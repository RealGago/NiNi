use crate::app::PendingKey;
use crate::theme::Theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use super::layout::h_center;

const ASCII_ART: &str = "\
_|      _|  _|_|_|  _|      _|  _|_|_|  \n\
_|_|    _|    _|    _|_|    _|    _|    \n\
_|  _|  _|    _|    _|  _|  _|    _|    \n\
_|    _|_|    _|    _|    _|_|    _|    \n\
_|      _|  _|_|_|  _|      _|  _|_|_|  ";

const WARNING_TEXT: &str =
    "DO NOT INPUT API KEYS OR SENSITIVE CONTENT. YOU ARE SOLELY RESPONSIBLE FOR EVERYTHING YOU TYPE INTO THESE MODELS!";

fn wave_color(offset: f32) -> Color {
    let t = (offset.sin() + 1.0) / 2.0;
    let lerp = |a: f32, b: f32| (a + (b - a) * t) as u8;
    Color::Rgb(lerp(88.0, 200.0), lerp(60.0, 170.0), lerp(150.0, 255.0))
}

fn animated_art_lines(tick: u64) -> Vec<Line<'static>> {
    let raw_lines: Vec<&str> = ASCII_ART.split('\n').collect();
    let reveal_speed = 3;
    let revealed_lines = (tick / reveal_speed).min(raw_lines.len() as u64) as usize;

    raw_lines
        .iter()
        .enumerate()
        .map(|(row, line)| {
            if row >= revealed_lines {
                return Line::from("");
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

pub fn draw_splash(f: &mut Frame, input: &str, pending: &[PendingKey], idx: usize, tick: u64, theme: &Theme) {
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
        .constraints([Constraint::Fill(1), Constraint::Length(content_height), Constraint::Fill(1)])
        .split(size);
    let content_area = outer[1];

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(content_area);

    let art = Paragraph::new(animated_art_lines(tick)).alignment(Alignment::Center);
    f.render_widget(art, rows[0]);

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

    let warning_row_idx = if has_pending {
        let key_area = h_center(64, rows[4]);
        let safe_idx = idx.min(pending.len().saturating_sub(1));
        let current = &pending[safe_idx];
        let masked: String = "*".repeat(input.chars().count());
        let title = format!(" {} ", current.label);
        let input_widget = Paragraph::new(masked).alignment(Alignment::Center).block(
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

    let warning_area = h_center(70, rows[warning_row_idx]);
    let warning = Paragraph::new(WARNING_TEXT)
        .style(Style::default().fg(theme.error()).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.error())),
        );
    f.render_widget(warning, warning_area);
}
