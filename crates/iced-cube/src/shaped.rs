//! The renderer's paragraph and the glyphs it was shaped into, for widgets
//! that work out carets and selections themselves.

use iced::Renderer;
use iced::advanced::graphics::text::cosmic_text::Buffer;
use iced::advanced::text;

/// The renderer's paragraph: the shaped text of a backend, or nothing when
/// the library is built without one.
pub type Paragraph = <Renderer as text::Renderer>::Paragraph;

/// Access to the glyphs a paragraph was shaped into.
pub trait Shaped {
    fn shaped(&self) -> Option<&Buffer>;
}

impl Shaped for () {
    fn shaped(&self) -> Option<&Buffer> {
        None
    }
}

impl Shaped for iced::advanced::graphics::text::Paragraph {
    fn shaped(&self) -> Option<&Buffer> {
        Some(self.buffer())
    }
}
