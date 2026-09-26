//! Floating content: dialogs, tooltips and toasts.

#[cfg(feature = "dialog")]
pub mod dialog;
#[cfg(feature = "toast")]
pub mod toast;
#[cfg(feature = "tooltip")]
pub mod tooltip;

#[cfg(feature = "dialog")]
pub use dialog::{AlertDialog, Dialog, alert_dialog, dialog};
#[cfg(feature = "toast")]
pub use toast::{Toast, Toasts, toast, toasts};
#[cfg(feature = "tooltip")]
pub use tooltip::{Tooltip, tooltip};
