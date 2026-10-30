//! Moving between views and actions: bottom navigation, breadcrumbs,
//! command lists, document tabs, pagination, the sidebar and tabs.

#[cfg(feature = "bottom-nav")]
pub mod bottom_nav;
#[cfg(feature = "breadcrumb")]
pub mod breadcrumb;
#[cfg(feature = "command")]
pub mod command;
#[cfg(feature = "document-tabs")]
pub mod document_tabs;
#[cfg(feature = "pagination")]
pub mod pagination;
#[cfg(feature = "sidebar")]
pub mod sidebar;
#[cfg(feature = "tabs")]
pub mod tabs;

#[cfg(feature = "bottom-nav")]
pub use bottom_nav::{BottomNav, bottom_nav};
#[cfg(feature = "breadcrumb")]
pub use breadcrumb::{Breadcrumb, breadcrumb, crumb};
#[cfg(feature = "command")]
pub use command::{Command, command};
#[cfg(feature = "document-tabs")]
pub use document_tabs::{DocumentTabs, document_tabs};
#[cfg(feature = "pagination")]
pub use pagination::{Pagination, pagination};
#[cfg(feature = "sidebar")]
pub use sidebar::{Sidebar, sidebar};
#[cfg(feature = "tabs")]
pub use tabs::{Tabs, tab, tabs};
