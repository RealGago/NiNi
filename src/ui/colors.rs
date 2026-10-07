use ratatui::style::Color;

// status semântico (fora do theme.toml, fixo)
pub const SUCCESS: Color = Color::Rgb(111, 191, 115);
pub const ERROR: Color = Color::Rgb(224, 115, 107);
pub const RUNNING: Color = Color::Rgb(212, 183, 106);
pub const DIM: Color = Color::Rgb(107, 107, 112);
pub const USER: Color = Color::Cyan; // ou troque por um tom próprio depois
pub const ON_ACCENT: Color = Color::Black; // texto sobre fundo colorido (highlight)
pub const CODE_ACCENT: Color = Color::Rgb(255, 180, 100); // inline code hoje
pub const INPUT_BG: Color = Color::Rgb(28, 28, 32);
pub const FG: Color = Color::Rgb(212, 212, 216);
