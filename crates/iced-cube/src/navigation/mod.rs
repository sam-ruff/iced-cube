//! Moving between views and actions: tabs, command lists and the sidebar.

#[cfg(feature = "command")]
pub mod command;
#[cfg(feature = "sidebar")]
pub mod sidebar;
#[cfg(feature = "tabs")]
pub mod tabs;

#[cfg(feature = "command")]
pub use command::{Command, command};
#[cfg(feature = "sidebar")]
pub use sidebar::{Sidebar, sidebar};
#[cfg(feature = "tabs")]
pub use tabs::{Tabs, tab, tabs};
