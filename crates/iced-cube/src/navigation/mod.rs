//! Moving between views and actions: tabs and command lists.

#[cfg(feature = "command")]
pub mod command;
#[cfg(feature = "tabs")]
pub mod tabs;

#[cfg(feature = "command")]
pub use command::{Command, command};
#[cfg(feature = "tabs")]
pub use tabs::{Tabs, tab, tabs};
