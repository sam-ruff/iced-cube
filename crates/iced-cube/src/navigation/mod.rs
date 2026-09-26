//! Moving between views: tabs.

#[cfg(feature = "tabs")]
pub mod tabs;

#[cfg(feature = "tabs")]
pub use tabs::{Tabs, tab, tabs};
