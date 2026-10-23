//! Messages from the page hosting a web preview.
//!
//! The docs site posts `{ "type": "theme", "value": "dark" }` to the preview
//! iframe. The subscription owns the receiving end of a channel and the
//! browser's `message` listener owns the sending end, so no state is shared.

use iced::Subscription;

use crate::app::ThemeChoice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Theme(ThemeChoice),
    /// Tab reached the gallery unused, so focus should leave the preview,
    /// backwards with Shift.
    TabOut {
        back: bool,
    },
}

/// Whether an event is a plain Tab or Shift+Tab, and if so whether it goes
/// backwards. Tab with Ctrl, Alt or the logo key belongs to the story.
pub fn tab_out(event: &iced::Event) -> Option<bool> {
    let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) = event
    else {
        return None;
    };
    if *key != iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab)
        || modifiers.control()
        || modifiers.alt()
        || modifiers.logo()
    {
        return None;
    }
    Some(modifiers.shift())
}

/// Parses a posted message into an [`Event`].
pub fn parse(kind: &str, value: &str) -> Option<Event> {
    if kind != "theme" {
        return None;
    }
    ThemeChoice::parse(value).map(Event::Theme)
}

/// Theme changes from the page, and Tab presses no widget used. A dialog
/// keeps Tab for its buttons; anywhere else Tab leaves the preview.
pub fn subscription() -> Subscription<Event> {
    Subscription::batch([
        messages(),
        iced::event::listen_with(|event, status, _| {
            if status == iced::event::Status::Captured {
                return None;
            }
            tab_out(&event).map(|back| Event::TabOut { back })
        }),
    ])
}

#[cfg(target_arch = "wasm32")]
fn messages() -> Subscription<Event> {
    Subscription::run(web::listen)
}

#[cfg(not(target_arch = "wasm32"))]
fn messages() -> Subscription<Event> {
    Subscription::none()
}

/// Tells the hosting page how tall the preview needs to be, as
/// `{ "type": "size", "height": <px> }`.
#[cfg(target_arch = "wasm32")]
pub fn report_height(height: f32) {
    web::post_height(height);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn report_height(_height: f32) {}

/// Tells the hosting page which theme the story picked for itself, as
/// `{ "type": "app-theme", "value": "light" | "dark" | "system" }`, so the
/// page chrome can follow it. `None` means the story follows the host again.
#[cfg(target_arch = "wasm32")]
pub fn report_theme(choice: Option<ThemeChoice>) {
    web::post("app-theme", theme_value(choice));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn report_theme(_choice: Option<ThemeChoice>) {}

/// Asks the hosting page to move focus out of the preview, as
/// `{ "type": "exit", "back": <bool> }`.
#[cfg(target_arch = "wasm32")]
pub fn report_exit(back: bool) {
    web::post_exit(back);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn report_exit(_back: bool) {}

/// The value [`report_theme`] posts.
pub fn theme_value(choice: Option<ThemeChoice>) -> &'static str {
    match choice {
        None => "system",
        Some(ThemeChoice::Light) => "light",
        Some(ThemeChoice::Dark) => "dark",
    }
}

/// Writes text to the clipboard. iced has no clipboard in the browser, so
/// there it goes through the page's clipboard API instead.
#[cfg(target_arch = "wasm32")]
pub fn copy<T>(text: String) -> iced::Task<T> {
    web::copy(&text);
    iced::Task::none()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn copy<T>(text: String) -> iced::Task<T> {
    iced::clipboard::write(text)
}

#[cfg(target_arch = "wasm32")]
mod web {
    use iced::futures::channel::mpsc;
    use iced::futures::{Stream, StreamExt};
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;

    use super::{Event, parse};

    const BUFFER: usize = 16;

    pub fn listen() -> impl Stream<Item = Event> {
        let (mut sender, receiver) = mpsc::channel(BUFFER);

        let on_message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(
            move |event: web_sys::MessageEvent| {
                let data = event.data();
                let field = |name: &str| {
                    js_sys::Reflect::get(&data, &name.into())
                        .ok()
                        .and_then(|value| value.as_string())
                };
                let (Some(kind), Some(value)) = (field("type"), field("value")) else {
                    return;
                };
                if let Some(event) = parse(&kind, &value) {
                    // A full buffer only drops a stale theme change.
                    let _ = sender.try_send(event);
                }
            },
        );

        if let Some(window) = web_sys::window() {
            let _ = window
                .add_event_listener_with_callback("message", on_message.as_ref().unchecked_ref());
        }
        // The listener lives as long as the page.
        on_message.forget();

        receiver.boxed()
    }

    pub fn post_height(height: f32) {
        let Some(parent) = web_sys::window().and_then(|window| window.parent().ok().flatten())
        else {
            return;
        };
        let message = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&message, &"type".into(), &"size".into());
        let _ = js_sys::Reflect::set(&message, &"height".into(), &f64::from(height).into());
        let _ = parent.post_message(&message, "*");
    }

    pub fn post_exit(back: bool) {
        let Some(parent) = web_sys::window().and_then(|window| window.parent().ok().flatten())
        else {
            return;
        };
        let message = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&message, &"type".into(), &"exit".into());
        let _ = js_sys::Reflect::set(&message, &"back".into(), &back.into());
        let _ = parent.post_message(&message, "*");
    }

    pub fn post(kind: &str, value: &str) {
        let Some(parent) = web_sys::window().and_then(|window| window.parent().ok().flatten())
        else {
            return;
        };
        let message = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&message, &"type".into(), &kind.into());
        let _ = js_sys::Reflect::set(&message, &"value".into(), &value.into());
        let _ = parent.post_message(&message, "*");
    }

    pub fn copy(text: &str) {
        let Some(window) = web_sys::window() else {
            return;
        };
        // The promise settles on its own; a refusal leaves the clipboard as it was.
        let _ = window.navigator().clipboard().write_text(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_theme_messages() {
        assert_eq!(
            parse("theme", "dark"),
            Some(Event::Theme(ThemeChoice::Dark))
        );
        assert_eq!(
            parse("theme", "light"),
            Some(Event::Theme(ThemeChoice::Light))
        );
    }

    #[test]
    fn app_theme_values_match_what_the_demo_page_reads() {
        assert_eq!(theme_value(None), "system");
        assert_eq!(theme_value(Some(ThemeChoice::Light)), "light");
        assert_eq!(theme_value(Some(ThemeChoice::Dark)), "dark");
    }

    fn press(key: iced::keyboard::Key, modifiers: iced::keyboard::Modifiers) -> iced::Event {
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: iced::keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    }

    #[test]
    fn only_plain_and_shift_tab_leave_the_preview() {
        use iced::keyboard::{Key, Modifiers, key::Named};

        let tab = Key::Named(Named::Tab);
        assert_eq!(
            tab_out(&press(tab.clone(), Modifiers::empty())),
            Some(false)
        );
        assert_eq!(tab_out(&press(tab.clone(), Modifiers::SHIFT)), Some(true));
        assert_eq!(tab_out(&press(tab.clone(), Modifiers::CTRL)), None);
        assert_eq!(tab_out(&press(tab, Modifiers::ALT)), None);
        assert_eq!(
            tab_out(&press(Key::Named(Named::Enter), Modifiers::empty())),
            None
        );
    }

    #[test]
    fn ignores_unknown_kinds_and_values() {
        assert_eq!(parse("resize", "dark"), None);
        assert_eq!(parse("theme", "sepia"), None);
    }
}
