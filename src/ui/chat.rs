use crate::app::{App, ToolLogStatus};
use crate::theme::Theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use pulldown_cmark::{Event, Parser, Tag};
use super::popup;

pub fn markdown_to_lines(text: &str, theme: &Theme) -> Vec<Line<'static>> {
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

pub fn draw_chat(f: &mut Frame, app: &App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(1),
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
    if !app.tool_log.is_empty() {
        for entry in &app.tool_log {
            let (marker, marker_style, extra) = match &entry.status {
                ToolLogStatus::Running => ("→", Style::default().fg(Color::Yellow), String::new()),
                ToolLogStatus::Done { duration_ms } => (
                    "✓",
                    Style::default().fg(Color::Green),
                    format!(" — {}ms", duration_ms),
                ),
                ToolLogStatus::Error { duration_ms } => (
                    "✗",
                    Style::default().fg(Color::Red),
                    format!(" — {}ms", duration_ms),
                ),
            };

            let summary = if entry.args_summary.len() > 40 {
                format!("{}...", &entry.args_summary[..40])
            } else {
                entry.args_summary.clone()
            };

            let line = Line::from(vec![
                Span::raw("   "),
                Span::styled(format!("{} ", marker), marker_style),
                Span::styled(entry.name.clone(), Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(format!("({})", summary), Style::default().fg(Color::DarkGray)),
                Span::styled(extra, Style::default().fg(Color::DarkGray)),
            ]);
            lines.push(line);
        }
        lines.push(Line::from(""));
    }

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
            total_virtual_lines += line_width.div_ceil(inner_chat_width);
        } else {
            total_virtual_lines += 1;
        }
    }

    let max_scroll = total_virtual_lines.saturating_sub(inner_chat_height);
    let clamped_scroll = if app.scroll > max_scroll { max_scroll } else { app.scroll };
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

    if total_virtual_lines > inner_chat_height {
        let mut scrollbar_state =
            ScrollbarState::new(total_virtual_lines as usize).position(final_offset as usize);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .style(purple_style);

        f.render_stateful_widget(scrollbar, chunks[1], &mut scrollbar_state);
    }

    let input_title = if app.is_loading { " ( thinking... ) > " } else { " you > " };
    let input = Paragraph::new(app.input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(purple_style)
            .title(input_title),
    );
    f.render_widget(input, chunks[2]);

    f.set_cursor_position((chunks[2].x + 1 + app.cursor_position as u16, chunks[2].y + 1));

    let status_text = if clamped_scroll > 0 {
        format!("{}  •  ↕ scrolled up — PageDown to jump to latest", app.status)
    } else {
        app.status.clone()
    };
    let status = Paragraph::new(status_text).style(Style::default().fg(Color::Yellow));
    f.render_widget(status, chunks[3]);

    popup::draw_popup(f, app, size);
}
