//! Form building blocks: labels and fields, comboboxes, selects, textareas
//! and the rich text editor.

#[cfg(feature = "combobox")]
pub mod combobox;
#[cfg(feature = "field")]
pub mod label;
#[cfg(feature = "rich-text-editor")]
pub mod rich_text_editor;
#[cfg(feature = "select")]
pub mod select;
#[cfg(feature = "textarea")]
pub mod textarea;

#[cfg(feature = "combobox")]
pub use combobox::{Combobox, combobox};
#[cfg(feature = "field")]
pub use label::{Field, Label, field, label};
#[cfg(feature = "rich-text-editor")]
pub use rich_text_editor::{RichTextEditor, rich_text_editor};
#[cfg(feature = "select")]
pub use select::{Select, select};
#[cfg(feature = "textarea")]
pub use textarea::{Textarea, textarea};
