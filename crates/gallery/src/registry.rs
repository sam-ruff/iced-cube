//! Type-erased access to every story.

use serde::Serialize;

/// Preview height in logical pixels when a story does not set one.
pub const DEFAULT_HEIGHT: u32 = 280;

/// Width of the preview frame and snapshots in logical pixels.
pub const WIDTH: u32 = 720;

/// Static description of a story, also exported to the docs site.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Meta {
    pub id: &'static str,
    pub component: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// Path of the example file relative to the gallery crate.
    pub file: &'static str,
    /// The example's source code, shown on the docs site.
    pub source: &'static str,
    /// Height of the preview frame and snapshot in logical pixels.
    pub height: u32,
    /// Whether the story fills the preview edge to edge, with no gallery padding.
    pub edge: bool,
}

/// Declares the story registry: an `AnyStory` enum wrapping every example,
/// an `AnyMessage` enum wrapping their messages, and the `ALL` metadata list.
///
/// Optional settings follow `file`, in this order:
/// - `height: 200` sets the preview height, which defaults to [`DEFAULT_HEIGHT`].
/// - `subscription: true` for a story with `subscription(&self) -> Subscription<Message>`.
/// - `task: true` for a story whose `update` returns `Task<Message>`, such as
///   one that writes to the clipboard.
/// - `theme: true` for a story with `theme(&self) -> Option<Theme>`, which
///   replaces the gallery theme while the story is open.
/// - `edge: true` drops the gallery padding so the story fills the preview,
///   for full-window layers such as a dialog's scrim.
#[macro_export]
macro_rules! stories {
    ($(
        $variant:ident => $($module:ident)::+ {
            id: $id:literal, component: $component:literal, title: $title:literal,
            description: $description:literal, file: $file:literal
            $(, height: $height:literal)?
            $(, subscription: $subscription:tt)?
            $(, task: $task:tt)?
            $(, theme: $theme:tt)?
            $(, edge: $edge:literal)? $(,)?
        }
    )*) => {
        /// Metadata for every story, in declaration order.
        pub const ALL: &[$crate::registry::Meta] = &[$(
            $crate::registry::Meta {
                id: $id,
                component: $component,
                title: $title,
                description: $description,
                file: concat!("src/stories/", $file),
                source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/stories/", $file)),
                height: $crate::story_height!($($height)?),
                edge: $crate::story_edge!($($edge)?),
            },
        )*];

        /// Stories are boxed because a full app story is far larger than the rest.
        #[derive(Debug)]
        pub enum AnyStory {
            $($variant(Box<$($module)::+::Example>),)*
        }

        #[derive(Debug, Clone)]
        pub enum AnyMessage {
            $($variant($($module)::+::Message),)*
        }

        impl AnyStory {
            /// Creates the story with the given id.
            pub fn new(id: &str) -> Option<Self> {
                match id {
                    $($id => Some(Self::$variant(Box::default())),)*
                    _ => None,
                }
            }

            pub fn update(&mut self, message: AnyMessage) -> iced::Task<AnyMessage> {
                match (self, message) {
                    $((Self::$variant(story), AnyMessage::$variant(message)) => {
                        $crate::story_update!(story, message, $variant $(, $task)?)
                    })*
                    #[allow(unreachable_patterns)]
                    _ => iced::Task::none(),
                }
            }

            pub fn view(&self) -> iced::Element<'_, AnyMessage> {
                match self {
                    $(Self::$variant(story) => story.view().map(AnyMessage::$variant),)*
                }
            }

            pub fn subscription(&self) -> iced::Subscription<AnyMessage> {
                match self {
                    $(Self::$variant(_story) => {
                        $crate::story_subscription!(_story, $variant $(, $subscription)?)
                    })*
                }
            }

            /// The story's own theme, if it sets one.
            pub fn theme(&self) -> Option<iced::Theme> {
                match self {
                    $(Self::$variant(_story) => {
                        $crate::story_theme!(_story $(, $theme)?)
                    })*
                }
            }
        }
    };
}

/// Expands to a story's height, or the default when it has none.
#[doc(hidden)]
#[macro_export]
macro_rules! story_height {
    ($height:literal) => {
        $height
    };
    () => {
        $crate::registry::DEFAULT_HEIGHT
    };
}

/// Expands to whether a story fills the preview, which defaults to false.
#[doc(hidden)]
#[macro_export]
macro_rules! story_edge {
    ($edge:literal) => {
        $edge
    };
    () => {
        false
    };
}

/// Expands to a story's subscription, or none when it has not opted in.
#[doc(hidden)]
#[macro_export]
macro_rules! story_subscription {
    ($story:ident, $variant:ident, true) => {
        $story.subscription().map(AnyMessage::$variant)
    };
    ($story:ident, $variant:ident $(, false)?) => {
        iced::Subscription::none()
    };
}

/// Expands to a story's update, wrapping a plain one in `Task::none()`.
#[doc(hidden)]
#[macro_export]
macro_rules! story_update {
    ($story:ident, $message:ident, $variant:ident, true) => {
        $story.update($message).map(AnyMessage::$variant)
    };
    ($story:ident, $message:ident, $variant:ident $(, false)?) => {{
        $story.update($message);
        iced::Task::none()
    }};
}

/// Expands to a story's theme, or none when it has not opted in.
#[doc(hidden)]
#[macro_export]
macro_rules! story_theme {
    ($story:ident, true) => {
        $story.theme()
    };
    ($story:ident $(, false)?) => {
        None
    };
}
