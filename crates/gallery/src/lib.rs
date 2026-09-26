//! Live examples for every iced-cube component.
//!
//! Runs natively as a browsable gallery, and in the browser as a
//! single-story preview embedded by the docs site.

pub mod app;
pub mod bridge;
pub mod keymaps;
pub mod registry;
pub mod stories;

pub use app::{Gallery, Message, ThemeChoice};

use iced::Font;

/// Inter matches the docs site, and is bundled because browsers expose no system fonts to wasm.
pub const FONT: Font = Font::with_name("Inter");

pub const FONT_FILES: [&[u8]; 2] = [
    include_bytes!("../fonts/Inter-Regular.ttf"),
    include_bytes!("../fonts/Inter-SemiBold.ttf"),
];
