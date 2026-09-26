//! Floating content: tooltips and toasts.

pub mod toast;
pub mod tooltip;

pub use toast::{Toast, Toasts, toast, toasts};
pub use tooltip::{Tooltip, tooltip};
