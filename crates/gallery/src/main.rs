use gallery::app::Mode;
use gallery::{FONT, FONT_FILES, Gallery, ThemeChoice};

fn main() -> iced::Result {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    iced_cube::theme::set_font(FONT);
    let (mode, story, theme) = launch_options();

    FONT_FILES
        .into_iter()
        .fold(
            iced::application(
                move || Gallery::new(mode, story.as_deref(), theme),
                Gallery::update,
                Gallery::view,
            ),
            |app, font| app.font(font),
        )
        .title("iced-cube gallery")
        .theme(Gallery::theme)
        .subscription(Gallery::subscription)
        .default_font(FONT)
        .run()
}

/// Reads `?story=<id>&theme=<light|dark>` from the page URL.
#[cfg(target_arch = "wasm32")]
fn launch_options() -> (Mode, Option<String>, ThemeChoice) {
    let params = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .and_then(|search| web_sys::UrlSearchParams::new_with_str(&search).ok());

    let get = |key: &str| params.as_ref().and_then(|params| params.get(key));
    let theme = get("theme")
        .and_then(|value| ThemeChoice::parse(&value))
        .unwrap_or_default();

    (Mode::Single, get("story"), theme)
}

/// Reads `[story-id] [--dark]` from the command line.
#[cfg(not(target_arch = "wasm32"))]
fn launch_options() -> (Mode, Option<String>, ThemeChoice) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let theme = if args.iter().any(|arg| arg == "--dark") {
        ThemeChoice::Dark
    } else {
        ThemeChoice::Light
    };
    let story = args.into_iter().find(|arg| !arg.starts_with("--"));
    (Mode::Browser, story, theme)
}
