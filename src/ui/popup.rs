use crate::app::{App, Popup};
use crate::providers;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use super::layout::{centered_rect, centered_rect_fixed};

pub fn draw_popup(f: &mut Frame, app: &App, size: Rect) {
    let purple_style = Style::default().fg(app.theme.primary());

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

            let help_lines = vec![
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

            let provider_items: Vec<ListItem> = providers::PROVIDERS
                .iter()
                .enumerate()
                .map(|(i, p)| ListItem::new(format!("{}, {}", i + 1, p.display_name)))
                .collect();

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

            let title = format!(" Provider: {} ", provider.label);
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

        Popup::ConfirmRunCommand { command } => {
            let area = centered_rect_fixed(60, 6, size);
            f.render_widget(Clear, area);
            let text = format!("Run this command?\n{}\n(y/n)", command);
            let confirm = Paragraph::new(text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                .block(
                    Block::default()
                        .title(" Confirm Command ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Red)),
                );
            f.render_widget(confirm, area);
        }
    }
}
