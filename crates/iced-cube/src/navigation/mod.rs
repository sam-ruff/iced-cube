//! Moving between views and actions: breadcrumbs, command lists, pagination,
//! the sidebar and tabs.

#[cfg(feature = "breadcrumb")]
pub mod breadcrumb;
#[cfg(feature = "command")]
pub mod command;
#[cfg(feature = "pagination")]
pub mod pagination;
#[cfg(feature = "sidebar")]
pub mod sidebar;
#[cfg(feature = "tabs")]
pub mod tabs;

#[cfg(feature = "breadcrumb")]
pub use breadcrumb::{Breadcrumb, breadcrumb, crumb};
#[cfg(feature = "command")]
pub use command::{Command, command};
#[cfg(feature = "pagination")]
pub use pagination::{Pagination, pagination};
#[cfg(feature = "sidebar")]
pub use sidebar::{Sidebar, sidebar};
#[cfg(feature = "tabs")]
pub use tabs::{Tabs, tab, tabs};
