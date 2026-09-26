//! Floating content: tooltips and toasts.

#[cfg(feature = "toast")]
pub mod toast;
#[cfg(feature = "tooltip")]
pub mod tooltip;

#[cfg(feature = "toast")]
pub use toast::{Toast, Toasts, toast, toasts};
#[cfg(feature = "tooltip")]
pub use tooltip::{Tooltip, tooltip};
