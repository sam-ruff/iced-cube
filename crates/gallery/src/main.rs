use gallery::app::Mode;
use gallery::{FONT, FONT_FILES, Gallery, ThemeChoice};

/// How the gallery was asked to start.
struct Launch {
    mode: Mode,
    story: Option<String>,
    theme: ThemeChoice,
    reports_state: bool,
}

fn main() -> iced::Result {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    iced_cube::theme::set_font(FONT);
    let launch = launch_options();

    FONT_FILES
        .into_iter()
        .fold(
            iced::application(
                move || {
                    Gallery::new(launch.mode, launch.story.as_deref(), launch.theme)
                        .reporting_state(launch.reports_state)
                },
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

/// Reads `?story=<id>&theme=<light|dark>&state=1` from the page URL.
#[cfg(target_arch = "wasm32")]
fn launch_options() -> Launch {
    let params = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .and_then(|search| web_sys::UrlSearchParams::new_with_str(&search).ok());

    let get = |key: &str| params.as_ref().and_then(|params| params.get(key));
    let theme = get("theme")
        .and_then(|value| ThemeChoice::parse(&value))
        .unwrap_or_default();

    Launch {
        mode: Mode::Single,
        story: get("story"),
        theme,
        reports_state: get("state").is_some_and(|value| value == "1"),
    }
}

/// Reads `[story-id] [--dark]` from the command line.
#[cfg(not(target_arch = "wasm32"))]
fn launch_options() -> Launch {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let theme = if args.iter().any(|arg| arg == "--dark") {
        ThemeChoice::Dark
    } else {
        ThemeChoice::Light
    };
    let story = args.into_iter().find(|arg| !arg.starts_with("--"));
    Launch {
        mode: Mode::Browser,
        story,
        theme,
        reports_state: false,
    }
}
