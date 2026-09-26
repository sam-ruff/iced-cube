//! Floating content: dialogs, tooltips, toasts, popovers and menus.

#[cfg(any(
    feature = "popover",
    feature = "dropdown-menu",
    feature = "context-menu"
))]
pub mod anchored;
#[cfg(feature = "context-menu")]
pub mod context_menu;
#[cfg(feature = "dialog")]
pub mod dialog;
#[cfg(feature = "dropdown-menu")]
pub mod dropdown_menu;
#[cfg(feature = "popover")]
pub mod popover;
#[cfg(feature = "toast")]
pub mod toast;
#[cfg(feature = "tooltip")]
pub mod tooltip;

#[cfg(feature = "context-menu")]
pub use context_menu::{ContextMenu, context_menu};
#[cfg(feature = "dialog")]
pub use dialog::{AlertDialog, Dialog, alert_dialog, dialog};
#[cfg(feature = "dropdown-menu")]
pub use dropdown_menu::{DropdownMenu, dropdown_menu};
#[cfg(feature = "popover")]
pub use popover::{Popover, popover};
#[cfg(feature = "toast")]
pub use toast::{Toast, Toasts, toast, toasts};
#[cfg(feature = "tooltip")]
pub use tooltip::{Tooltip, tooltip};
