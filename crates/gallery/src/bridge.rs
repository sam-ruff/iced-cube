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
}

/// Parses a posted message into an [`Event`].
pub fn parse(kind: &str, value: &str) -> Option<Event> {
    if kind != "theme" {
        return None;
    }
    ThemeChoice::parse(value).map(Event::Theme)
}

#[cfg(target_arch = "wasm32")]
pub fn subscription() -> Subscription<Event> {
    Subscription::run(web::listen)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn subscription() -> Subscription<Event> {
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
    fn ignores_unknown_kinds_and_values() {
        assert_eq!(parse("resize", "dark"), None);
        assert_eq!(parse("theme", "sepia"), None);
    }
}
