//! Floating content: dialogs, tooltips, toasts, popovers and menus.

#[cfg(feature = "popover")]
pub mod anchored;
#[cfg(feature = "dialog")]
pub mod dialog;
#[cfg(feature = "popover")]
pub mod popover;
#[cfg(feature = "toast")]
pub mod toast;
#[cfg(feature = "tooltip")]
pub mod tooltip;

#[cfg(feature = "dialog")]
pub use dialog::{AlertDialog, Dialog, alert_dialog, dialog};
#[cfg(feature = "popover")]
pub use popover::{Popover, popover};
#[cfg(feature = "toast")]
pub use toast::{Toast, Toasts, toast, toasts};
#[cfg(feature = "tooltip")]
pub use tooltip::{Tooltip, tooltip};
