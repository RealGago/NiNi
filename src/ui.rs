use crate::app::{App, PendingKey, Popup, Screen};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};



const ASCII_ART: &str = "\
_|      _|  _|_|_|  _|      _|  _|_|_|  \n\
_|_|    _|    _|    _|_|    _|    _|    \n\
_|  _|  _|    _|    _|  _|  _|    _|    \n\
_|    _|_|    _|    _|    _|_|    _|    \n\
_|      _|  _|_|_|  _|      _|  _|_|_|  ";

fn markdown_to_lines(text: &str, theme: &crate::theme::Theme) -> Vec<Line<'static>> {
    use pulldown_cmark::{Event, Parser, Tag};

    let code_bg = theme.code_bg();
    let inline_code_style = Style::default().fg(Color::Rgb(255, 180, 100)).bg(code_bg);
    let code_block_style = Style::default().fg(theme.code()).bg(code_bg);
    let heading_style = Style::default().fg(theme.primary()).add_modifier(Modifier::BOLD);
    let bullet_style = Style::default().fg(theme.accent());

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut in_code_block = false;

    let flush = |current: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>| {
        if !current.is_empty() {
            lines.push(Line::from(std::mem::take(current)));
        }
    };

    for event in Parser::new(text) {
        match event {
            Event::Start(Tag::Heading(_, _, _)) => {
                flush(&mut current, &mut lines);
                current.push(Span::styled("▍ ", heading_style));
                bold = true;
            }
            Event::End(Tag::Heading(_, _, _)) => {
                bold = false;
                flush(&mut current, &mut lines);
            }
            Event::Start(Tag::Strong) => bold = true,
            Event::End(Tag::Strong) => bold = false,
            Event::Start(Tag::Emphasis) => italic = true,
            Event::End(Tag::Emphasis) => italic = false,
            Event::Start(Tag::CodeBlock(_)) => {
                flush(&mut current, &mut lines);
                in_code_block = true;
            }
            Event::End(Tag::CodeBlock(_)) => {
                in_code_block = false;
                flush(&mut current, &mut lines);
            }
            Event::Start(Tag::Item) => {
                flush(&mut current, &mut lines);
                current.push(Span::styled("  • ", bullet_style));
            }
            Event::End(Tag::Item) => flush(&mut current, &mut lines),
            Event::End(Tag::Paragraph) => flush(&mut current, &mut lines),
            Event::Code(t) => current.push(Span::styled(t.to_string(), inline_code_style)),
            Event::Text(t) => {
                if in_code_block {
                    for (i, l) in t.split('\n').enumerate() {
                        if i > 0 {
                            flush(&mut current, &mut lines);
                        }
                        if !l.is_empty() {
                            current.push(Span::styled(l.to_string(), code_block_style));
                        }
                    }
                } else {
                    let mut style = Style::default();
                    if bold {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if italic {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    current.push(Span::styled(t.to_string(), style));
                }
            }
            Event::SoftBreak => current.push(Span::raw(" ")),
            Event::HardBreak => flush(&mut current, &mut lines),
            Event::Rule => {
                flush(&mut current, &mut lines);
                lines.push(Line::from(Span::styled(
                    "─".repeat(40),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            _ => {}
        }
    }
    flush(&mut current, &mut lines);
    lines
}



pub fn draw(f: &mut Frame, app: &App) {
    match &app.screen {
        Screen::Splash { input, pending, idx } => draw_splash(f, input, pending, *idx, app.tick, &app.theme),
        Screen::Chat => draw_chat(f, app),
    }
}

fn wave_color(offset: f32) -> Color {
    let t = (offset.sin() + 1.0) / 2.0; // 0.0..1.0
    let lerp = |a: f32, b: f32| (a + (b - a) * t) as u8;
    Color::Rgb(lerp(88.0, 200.0), lerp(60.0, 170.0), lerp(150.0, 255.0))
}

fn animated_art_lines(tick: u64) -> Vec<Line<'static>> {
    let raw_lines: Vec<&str> = ASCII_ART.split('\n').collect();
    let reveal_speed = 3; // ticks por linha revelada
    let revealed_lines = (tick / reveal_speed).min(raw_lines.len() as u64) as usize;

    raw_lines
        .iter()
        .enumerate()
        .map(|(row, line)| {
            if row >= revealed_lines {
                return Line::from(""); // hasn't "typed" that line yet
            }
            let spans: Vec<Span> = line
                .chars()
                .enumerate()
                .map(|(col, c)| {
                    let offset = tick as f32 * 0.15 + col as f32 * 0.25 + row as f32 * 0.6;
                    Span::styled(c.to_string(), Style::default().fg(wave_color(offset)))
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}



const WARNING_TEXT: &str =
    "DO NOT INPUT API KEYS OR SENSITIVE CONTENT. YOU ARE SOLELY RESPONSIBLE FOR EVERYTHING YOU TYPE INTO THESE MODELS!";

fn draw_splash(f: &mut Frame, input: &str, pending: &[PendingKey], idx: usize, tick: u64, theme: &crate::theme::Theme) {
    let size = f.area();
    let purple_style = Style::default().fg(theme.primary());

    let shortcuts_left = [
        "Enter — send / confirm",
        "Esc — quit / cancel",
        "Tab — autocomplete",
        "F2 — copy last response",
    ];
    let shortcuts_right = [
        "/system — system prompt",
        "/usage — check usage",
        "/clear — clear chat",
        "/models — switch model",
    ];

    const ART_HEIGHT: u16 = 7;
    const SHORTCUTS_HEIGHT: u16 = 7;
    const SPACER_HEIGHT: u16 = 3;
    const KEY_BOX_HEIGHT: u16 = 3;
    const WARNING_SPACER: u16 = 2;
    const WARNING_HEIGHT: u16 = 5;

    let has_pending = !pending.is_empty();

    let mut constraints = vec![
        Constraint::Length(ART_HEIGHT),
        Constraint::Length(1),
        Constraint::Length(SHORTCUTS_HEIGHT),
    ];
    if has_pending {
        constraints.push(Constraint::Length(SPACER_HEIGHT));
        constraints.push(Constraint::Length(KEY_BOX_HEIGHT));
    } else {
        constraints.push(Constraint::Length(2));
    }
    constraints.push(Constraint::Length(WARNING_SPACER));
    constraints.push(Constraint::Length(WARNING_HEIGHT));

    let content_height: u16 = constraints
        .iter()
        .map(|c| if let Constraint::Length(n) = c { *n } else { 0 })
        .sum();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(content_height),
            Constraint::Fill(1),
        ])
        .split(size);
    let content_area = outer[1];

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(content_area);

    // ---- ASCII art ----
    let art = Paragraph::new(animated_art_lines(tick))
        .alignment(Alignment::Center);
    f.render_widget(art, rows[0]);

    // ---- Shortcuts, 2 columns ----
    let shortcuts_area = h_center(54, rows[2]);
    let shortcuts_block = Block::default()
        .borders(Borders::ALL)
        .border_style(purple_style)
        .title(" shortcuts ");
    let shortcuts_inner = shortcuts_block.inner(shortcuts_area);
    f.render_widget(shortcuts_block, shortcuts_area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(shortcuts_inner);

    let left = Paragraph::new(shortcuts_left.iter().map(|s| Line::from(*s)).collect::<Vec<_>>());
    let right = Paragraph::new(shortcuts_right.iter().map(|s| Line::from(*s)).collect::<Vec<_>>());
    f.render_widget(left, cols[0]);
    f.render_widget(right, cols[1]);

    // API key box: only appears if a key is actually missing.
    let warning_row_idx = if has_pending {
        let key_area = h_center(64, rows[4]);
        let safe_idx = idx.min(pending.len().saturating_sub(1));
        let current = &pending[safe_idx];
        let masked: String = "*".repeat(input.chars().count());
        let title = format!(" {} ", current.label);
        let input_widget = Paragraph::new(masked)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(title),
            );
        f.render_widget(input_widget, key_area);
        6
    } else {
        let ready = Paragraph::new("All Done!, press Enter")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));
        f.render_widget(ready, rows[3]);
        5
    };

    // ---- Warning, sits at the bottom, only visible on splash screen ----
    let warning_area = h_center(70, rows[warning_row_idx]);
   let warning = Paragraph::new(WARNING_TEXT)
        .style(Style::default().fg(theme.error()).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.error())),
        );    f.render_widget(warning, warning_area);
}

fn draw_chat(f: &mut Frame, app: &App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // banner
            Constraint::Min(3),    // chat history
            Constraint::Length(3), // input field
            Constraint::Length(1), // status bar
        ])
        .split(size);

    let purple_style = Style::default().fg(app.theme.primary());

    let banner_text = format!("NiNi — model: {}", app.model);
    let banner = Paragraph::new(banner_text)
        .style(purple_style)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(purple_style)
                .title(" nini-tui "),
        );
    f.render_widget(banner, chunks[0]);

    let user_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let assistant_style = Style::default().fg(app.theme.primary()).add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line> = Vec::new();
    for m in &app.messages {
        let (style, align, is_user) = match m.role.as_str() {
            "user" => (user_style, Alignment::Right, true),
            "assistant" => (assistant_style, Alignment::Left, false),
            _ => continue,
        };

        if is_user {
            let parts: Vec<&str> = m.content.split('\n').collect();
            let last_idx = parts.len().saturating_sub(1);
            for (i, part) in parts.iter().enumerate() {
                let mut spans = vec![Span::raw(part.to_string())];
                if i == last_idx {
                    spans.push(Span::styled(" : you", style));
                }
                lines.push(Line::from(spans).alignment(align));
            }
        } else {
            let mut md_lines = markdown_to_lines(&m.content, &app.theme);
            if let Some(first) = md_lines.first_mut() {
                first.spans.insert(0, Span::styled("nini: ", style));
            } else {
                md_lines.push(Line::from(Span::styled("nini: ", style)));
            }
            lines.extend(md_lines);
        }



        if !is_user {
            let copy_hint = Line::from(vec![
                Span::raw("      └─ "),
                Span::styled(" 📋 [F2] Copy Response ", Style::default().fg(Color::DarkGray)),
            ])
            .alignment(Alignment::Left);
            lines.push(copy_hint);
        }
        lines.push(Line::from("").alignment(align));
    }

    let inner_chat_width = chunks[1].width.saturating_sub(2);
    let inner_chat_height = chunks[1].height.saturating_sub(2);

    let mut total_virtual_lines: u16 = 0;
    for line in &lines {
        let line_width = line.width() as u16;
        if inner_chat_width > 0 && line_width > inner_chat_width {
            total_virtual_lines += (line_width + inner_chat_width - 1) / inner_chat_width;
        } else {
            total_virtual_lines += 1;
        }
    }

    let max_scroll = total_virtual_lines.saturating_sub(inner_chat_height);

    let clamped_scroll = if app.scroll > max_scroll {
        max_scroll
    } else {
        app.scroll
    };

    let final_offset = max_scroll.saturating_sub(clamped_scroll);

    let history = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((final_offset, 0))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(purple_style)
                .title(" chat "),
        );
    f.render_widget(history, chunks[1]);

    // ---- Scrollbar visual, mostra quanto ainda dá pra rolar ----
    if total_virtual_lines > inner_chat_height {
        let mut scrollbar_state =
            ScrollbarState::new(total_virtual_lines as usize).position(final_offset as usize);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .style(purple_style);

        f.render_stateful_widget(scrollbar, chunks[1], &mut scrollbar_state);
    }

    let input_title = if app.is_loading {
        " ( thinking... ) > "
    } else {
        " you > "
    };
    let input = Paragraph::new(app.input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(purple_style)
            .title(input_title),
    );
    f.render_widget(input, chunks[2]);

    f.set_cursor_position((
        chunks[2].x + 1 + app.cursor_position as u16,
        chunks[2].y + 1,
    ));

    let status_text = if clamped_scroll > 0 {
        format!("{}  •  ↕ scrolled up — PageDown to jump to latest", app.status)
    } else {
        app.status.clone()
    };
    let status = Paragraph::new(status_text).style(Style::default().fg(Color::Yellow));
    f.render_widget(status, chunks[3]);

    match &app.popup {
        Popup::None => {}

        Popup::Loading => {
            let area = centered_rect(35, 20, size);
            f.render_widget(Clear, area);

            let loading_widget = Paragraph::new("Fetching models from API...")
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(" Loading ")
                        .borders(Borders::ALL)
                        .border_style(purple_style),
                );
            f.render_widget(loading_widget, area);
        }

       Popup::ThemeEditor { field, channel, input } => {
            let area = centered_rect(60, 50, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .title(" Theme Editor — Ctrl+T ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary()));
            let inner = block.inner(area);
            f.render_widget(block, area);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(crate::app::ThemeField::ALL.len() as u16 + 1),
                    Constraint::Length(1),
                    Constraint::Length(2),
                    Constraint::Length(4),
                    Constraint::Fill(1),
                ])
                .split(inner);

            let mut field_lines: Vec<Line> = Vec::new();
            for f_item in crate::app::ThemeField::ALL {
                let (r, g, b) = app.theme.get(f_item);
                let is_selected = f_item == *field;
                let marker = if is_selected { "▶ " } else { "  " };
                let label_style = if is_selected {
                    Style::default().fg(app.theme.accent()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                field_lines.push(Line::from(vec![
                    Span::raw(marker),
                    Span::styled(format!("{:<28}", f_item.label()), label_style),
                    Span::styled("██", Style::default().fg(Color::Rgb(r, g, b))),
                    Span::raw(format!(" ({}, {}, {})", r, g, b)),
                ]));
            }
            f.render_widget(Paragraph::new(field_lines), rows[0]);

            let (cr, cg, cb) = app.theme.get(*field);
            let current_val = match channel {
                crate::app::RgbChannel::R => cr,
                crate::app::RgbChannel::G => cg,
                crate::app::RgbChannel::B => cb,
            };
            let editing_text = format!(
                "Editing channel {} (current: {}) — typing: {}",
                channel.label(),
                current_val,
                if input.is_empty() { "_".to_string() } else { input.clone() }
            );
            let editing = Paragraph::new(editing_text).style(Style::default().fg(Color::Yellow));
            f.render_widget(editing, rows[2]);
            
            let help_lines = vec! [
                Line::from("↑ ↓  field"),
                Line::from("Tab  channel R/G/B"),
                Line::from("0-9  type"),
                Line::from("Enter confirm · R reset · Esc close"),
            ];

            let help = Paragraph::new(help_lines).style(Style::default().fg(Color::DarkGray));
            f.render_widget(help, rows[3]);
        }

        Popup::SelectProvider { selected } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);

            let provider_items = vec![
                ListItem::new("1. OpenRouter (Free Models)"),
                ListItem::new("2. OpenCode Zen"),
            ];

            let list = List::new(provider_items)
                .block(
                    Block::default()
                        .title(" Select Provider ")
                        .borders(Borders::ALL)
                        .border_style(purple_style),
                )
                .highlight_style(Style::default().bg(app.theme.primary()).fg(Color::Black))
                .highlight_symbol("> ");

            let mut state = ListState::default().with_selected(Some(*selected));
            f.render_stateful_widget(list, area, &mut state);
        }

        Popup::SelectModel { provider, models, selected } => {
            let area = centered_rect(70, 60, size);
            f.render_widget(Clear, area);

            let title = format!(" Provider: {} ", provider.label());
            let model_items: Vec<ListItem> = models.iter().map(|m| ListItem::new(m.clone())).collect();

            let list = List::new(model_items)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(purple_style),
                )
                .highlight_style(Style::default().bg(app.theme.primary()).fg(Color::Black))
                .highlight_symbol("> ");

            let mut state = ListState::default().with_selected(Some(*selected));
            f.render_stateful_widget(list, area, &mut state);
        }

        Popup::EditSystemPrompt => {
            let area = centered_rect(60, 35, size);
            f.render_widget(Clear, area);

            let prompt_input = Paragraph::new(app.system_prompt.as_str())
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .title(" [ Configure System Prompt Instructions ] ")
                        .title_alignment(Alignment::Center)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                );
            f.render_widget(prompt_input, area);
        }

        Popup::ConfirmClear => {
            let area = centered_rect_fixed(50, 4, size);
            f.render_widget(Clear, area);

            let confirm = Paragraph::new("Clear conversation history? (y/n)")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                .block(
                    Block::default()
                        .title(" Confirm ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Red)),
                );
            f.render_widget(confirm, area);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(width),
            Constraint::Fill(1),
        ])
        .split(popup_layout[1])[1]
}

fn h_center(width: u16, area: Rect) -> Rect {
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(width),
            Constraint::Fill(1),
        ])
        .split(area);
    horizontal[1] 
}
