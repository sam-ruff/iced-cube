//! The docs site's demo as a desktop app: the same story, full window.
#![cfg_attr(windows, windows_subsystem = "windows")]

use gallery::app::Mode;
use gallery::{FONT, FONT_FILES, Gallery, ThemeChoice};

const STORY: &str = "showcase/demo";

fn main() -> iced::Result {
    iced_cube::theme::set_font(FONT);
    let theme = if std::env::args().any(|arg| arg == "--dark") {
        ThemeChoice::Dark
    } else {
        ThemeChoice::Light
    };

    FONT_FILES
        .into_iter()
        .fold(
            iced::application(
                move || Gallery::new(Mode::Single, Some(STORY), theme),
                Gallery::update,
                Gallery::view,
            ),
            |app, font| app.font(font),
        )
        .title("iced-cube demo")
        .window_size(iced::Size::new(1280.0, 800.0))
        .theme(Gallery::theme)
        .subscription(Gallery::subscription)
        .default_font(FONT)
        .run()
}
