//! Themeable application components for [iced](https://iced.rs).
//!
//! ```no_run
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
//! ```

pub mod feedback;
pub mod forms;
pub mod icon;
mod inert;
pub mod keys;
pub mod layout;
pub mod navigation;
pub mod overlay;
pub mod primitives;
pub mod theme;

pub use feedback::{alert, badge, progress, spinner};
pub use forms::{field, label, select, textarea};
pub use icon::{Glyph, Icon};
pub use keys::{Chord, Keymap};
pub use layout::{accordion, card, hstack, vstack};
pub use navigation::{tab, tabs};
pub use overlay::{toast, toasts, tooltip};
pub use primitives::{
    button, checkbox, icon_button, input, radio, radio_group, scroll_area, separator, slider,
    switch, vertical_separator,
};
