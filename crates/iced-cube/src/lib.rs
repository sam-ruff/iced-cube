//! Themeable application components for [iced](https://iced.rs).
//!
//! Every component sits behind a Cargo feature named after it, such as
//! `button` or `scroll-area`, and each group (`primitives`, `forms`,
//! `layout`, `navigation`, `overlay`, `feedback`, `application`) enables its members. The
//! default `full` feature enables all of them.
//!
//! ```no_run
//! # #[cfg(feature = "button")]
//! # fn main() {
//! use iced_cube::{button, lucide};
//! use iced_cube::primitives::button::Variant;
//!
//! #[derive(Debug, Clone)]
//! enum Message { Save }
//!
//! let save: iced::Element<'_, Message> = button("Save")
//!     .icon(lucide!(Save))
//!     .variant(Variant::Primary)
//!     .on_press(Message::Save)
//!     .into();
//! # }
//! # #[cfg(not(feature = "button"))]
//! # fn main() {}
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod application;
pub mod feedback;
pub mod forms;
pub mod icon;
#[cfg(feature = "slider")]
mod inert;
pub mod keys;
pub mod layout;
#[cfg(any(feature = "badge", feature = "button", feature = "tabs"))]
mod natural;
pub mod navigation;
pub mod overlay;
pub mod primitives;
pub mod theme;

pub use icon::{Glyph, Icon};
pub use keys::{Chord, Keymap};

#[cfg(feature = "command-palette")]
pub use application::command_palette;
#[cfg(feature = "menubar")]
pub use application::menubar;
#[cfg(feature = "status-bar")]
pub use application::status_bar;
#[cfg(feature = "toolbar")]
pub use application::toolbar;

#[cfg(feature = "alert")]
pub use feedback::alert;
#[cfg(feature = "badge")]
pub use feedback::badge;
#[cfg(feature = "progress")]
pub use feedback::progress;
#[cfg(feature = "spinner")]
pub use feedback::spinner;

#[cfg(feature = "combobox")]
pub use forms::combobox;
#[cfg(feature = "select")]
pub use forms::select;
#[cfg(feature = "textarea")]
pub use forms::textarea;
#[cfg(feature = "field")]
pub use forms::{field, label};

#[cfg(feature = "accordion")]
pub use layout::accordion;
#[cfg(feature = "card")]
pub use layout::card;
#[cfg(feature = "stack")]
pub use layout::{hstack, vstack};

#[cfg(feature = "command")]
pub use navigation::command;
#[cfg(feature = "tabs")]
pub use navigation::{tab, tabs};

#[cfg(feature = "context-menu")]
pub use overlay::context_menu;
#[cfg(feature = "dropdown-menu")]
pub use overlay::dropdown_menu;
#[cfg(feature = "popover")]
pub use overlay::popover;
#[cfg(feature = "tooltip")]
pub use overlay::tooltip;
#[cfg(feature = "dialog")]
pub use overlay::{alert_dialog, dialog};
#[cfg(feature = "toast")]
pub use overlay::{toast, toasts};

#[cfg(feature = "button")]
pub use primitives::button;
#[cfg(feature = "checkbox")]
pub use primitives::checkbox;
#[cfg(feature = "icon-button")]
pub use primitives::icon_button;
#[cfg(feature = "input")]
pub use primitives::input;
#[cfg(feature = "scroll-area")]
pub use primitives::scroll_area;
#[cfg(feature = "slider")]
pub use primitives::slider;
#[cfg(feature = "switch")]
pub use primitives::switch;
#[cfg(feature = "radio")]
pub use primitives::{radio, radio_group};
#[cfg(feature = "separator")]
pub use primitives::{separator, vertical_separator};
