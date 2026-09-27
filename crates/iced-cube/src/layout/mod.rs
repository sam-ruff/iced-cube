//! Arranging content: stacks, cards, accordions and resizable panels.

#[cfg(feature = "accordion")]
pub mod accordion;
#[cfg(feature = "card")]
pub mod card;
#[cfg(feature = "resizable-panel")]
pub mod resizable_panel;
#[cfg(feature = "split-pane")]
pub mod split_pane;
#[cfg(feature = "stack")]
pub mod stack;

#[cfg(feature = "accordion")]
pub use accordion::{Accordion, accordion};
#[cfg(feature = "card")]
pub use card::{Card, card};
#[cfg(feature = "resizable-panel")]
pub use resizable_panel::{ResizablePanel, resizable_panel};
#[cfg(feature = "split-pane")]
pub use split_pane::{SplitPane, split_pane};
#[cfg(feature = "stack")]
pub use stack::{Stack, hstack, vstack};
