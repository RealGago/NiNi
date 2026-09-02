mod layout;
mod splash;
mod chat;
mod popup;

use crate::app::{App, Screen};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &App) {
    match &app.screen {
        Screen::Splash { input, pending, idx} => {
            splash::draw_splash(f, input, pending, *idx, app.tick, &app.theme)
        }
        Screen::Chat => chat::draw_chat(f, app),
    }
}
