//! Locators that know how iced-cube components are built, for icedwright
//! tests. Behind the `icedwright` feature.
//!
//! Each one wraps a locator for a component's root node and builds locators
//! for its parts, so a test says `tree.item(["Projects", "Roadmap"])` rather
//! than chaining roles and names. They are pure queries: bind one to a
//! harness to act on it.
//!
//! ```no_run
//! use iced_cube::testing::{ComponentLocator, Tree};
//! use icedwright_locator::{Scope, get_by_role, role};
//!
//! let tree = Tree::from_locator(get_by_role(role::Tree).name("Projects").any_role());
//! let roadmap = tree.item(["Projects", "Roadmap"]);
//! # let _ = roadmap;
//! ```

use icedwright_locator::{
    Locator, Role, RoleKind, Scope, TextMatch, get_by_role_dyn, of_type, role,
};
use icedwright_semantics::{Node, SemanticTree};

/// A component's locator, built over the locator for its root node.
pub trait ComponentLocator: Sized {
    /// The kind the component's annotations report, such as `"iced_cube::tree"`.
    const KIND: &'static str;

    /// The role of the component's root node.
    const ROOT: Role;

    /// Wraps a locator that finds the component's root.
    fn from_locator(root: Locator) -> Self;

    /// The locator for the component's root.
    fn locator(&self) -> &Locator;

    /// Every component of this kind on the page. Its parts report the same
    /// kind, so this matches the root role as well.
    fn of_kind() -> Self {
        Self::from_locator(get_by_role_dyn(Self::ROOT).and(of_type(Self::KIND)))
    }
}

macro_rules! component {
    ($(#[$doc:meta])* $name:ident, $kind:literal, $root:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            root: Locator,
        }

        impl ComponentLocator for $name {
            const KIND: &'static str = $kind;
            const ROOT: Role = Role::$root;

            fn from_locator(root: Locator) -> Self {
                Self { root }
            }

            fn locator(&self) -> &Locator {
                &self.root
            }
        }
    };
}

component!(
    /// A [`tree`](mod@crate::tree): items found by their path of labels.
    Tree,
    "iced_cube::tree",
    Tree
);

impl Tree {
    /// The item at `path`, each label an exact name, following each item
    /// into the items it owns.
    pub fn item(&self, path: impl IntoIterator<Item = impl AsRef<str>>) -> Locator<role::TreeItem> {
        let mut path = path.into_iter();
        let Some(first) = path.next() else {
            return self.root.clone().get_by_role(role::TreeItem).first();
        };
        path.fold(
            item_named(self.root.clone(), first.as_ref()),
            |item, label| item_named(item.any_role(), label.as_ref()),
        )
    }

    /// The button that expands or collapses the item at `path`.
    pub fn expander(
        &self,
        path: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Locator<role::Button> {
        self.item(path).get_by_role(role::Button).first()
    }

    /// The selected items.
    pub fn selected(&self) -> Locator<role::TreeItem> {
        self.root.clone().get_by_role(role::TreeItem).selected(true)
    }
}

fn item_named(scope: Locator, label: &str) -> Locator<role::TreeItem> {
    scope
        .get_by_role(role::TreeItem)
        .name(TextMatch::exact(label))
        .first()
}

component!(
    /// [`tabs`](mod@crate::tabs): a tab strip.
    Tabs,
    "iced_cube::tabs",
    TabList
);

impl Tabs {
    /// The tab labelled `name`.
    pub fn tab(&self, name: &str) -> Locator<role::Tab> {
        self.root
            .clone()
            .get_by_role(role::Tab)
            .name(TextMatch::exact(name))
    }

    /// The selected tab.
    pub fn selected(&self) -> Locator<role::Tab> {
        self.root.clone().get_by_role(role::Tab).selected(true)
    }
}

component!(
    /// [`document_tabs`](mod@crate::document_tabs): open documents, each with a
    /// close button.
    DocumentTabs,
    "iced_cube::document_tabs",
    TabList
);

impl DocumentTabs {
    /// The tab labelled `name`.
    pub fn tab(&self, name: &str) -> Locator<role::Tab> {
        self.root
            .clone()
            .get_by_role(role::Tab)
            .name(TextMatch::exact(name))
    }

    /// The button that closes the tab labelled `name`.
    pub fn close_button(&self, name: &str) -> Locator<role::Button> {
        self.tab(name)
            .get_by_role(role::Button)
            .name(TextMatch::exact(format!("Close {name}")))
    }

    /// The selected, active tab.
    pub fn selected(&self) -> Locator<role::Tab> {
        self.root.clone().get_by_role(role::Tab).selected(true)
    }

    /// The tabs with unsaved changes in `tree`, one locator each. Locators
    /// cannot match props, so this reads them from a tree.
    pub fn unsaved(&self, tree: &SemanticTree) -> Vec<Locator<role::Tab>> {
        let tabs = self.root.clone().get_by_role(role::Tab);
        tabs.all(tree)
            .filter(|tab| {
                resolved(tree, tab).is_some_and(|node| {
                    node.props.get("unsaved").map(String::as_str) == Some("true")
                })
            })
            .collect()
    }
}

component!(
    /// An open menu. [`of_kind`](ComponentLocator::of_kind) finds
    /// [`dropdown_menu`](mod@crate::dropdown_menu) panels; wrap a locator for
    /// the `Menu` node to use it on a [`context_menu`](mod@crate::context_menu)
    /// or menu bar panel.
    Menu,
    "iced_cube::dropdown_menu",
    Menu
);

impl Menu {
    /// The item labelled `name`, including check and radio items.
    pub fn item(&self, name: &str) -> Locator {
        let named =
            |role: Role| icedwright_locator::get_by_role_dyn(role).name(TextMatch::exact(name));
        self.root.clone().locator(
            named(Role::MenuItem)
                .or(named(Role::MenuItemCheckBox))
                .or(named(Role::MenuItemRadio)),
        )
    }
}

component!(
    /// A [`virtual_list`](mod@crate::virtual_list): only the rows on screen are
    /// in the tree, each with its position in the whole list.
    VirtualList,
    "iced_cube::virtual_list",
    List
);

/// How far to scroll a list to bring a row into view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollTarget {
    /// The position of the scrollable around the list among the
    /// interface's scrollables, or `None` when the list scrolls itself, as
    /// the virtual list does, and is driven by its own events.
    pub ordinal: Option<u32>,
    /// The vertical offset, in logical pixels, that puts the row at the top.
    pub offset: f32,
}

impl VirtualList {
    fn rows(&self) -> Locator<role::ListItem> {
        self.root.clone().get_by_role(role::ListItem)
    }

    /// The rendered row at `index`, counted from 0 over the whole list, or
    /// `None` when that row is not rendered in `tree`.
    pub fn row(&self, tree: &SemanticTree, index: u32) -> Option<Locator<role::ListItem>> {
        self.rows()
            .all(tree)
            .find(|row| resolved(tree, row).and_then(|n| n.set.position) == Some(index + 1))
    }

    /// The scroll that brings row `index` to the top of the list, from a
    /// rendered row's height, since every row has the same height. `None`
    /// when no row is rendered.
    pub fn scroll_target(&self, tree: &SemanticTree, index: u32) -> Option<ScrollTarget> {
        let row = self.rows().all(tree).find_map(|row| resolved(tree, &row))?;
        Some(ScrollTarget {
            ordinal: row.scroll_ancestors.last().map(|scroll| scroll.ordinal),
            offset: index as f32 * row.bounds.height,
        })
    }
}

component!(
    /// A [`timeline`](mod@crate::data::timeline): one slider over its steps,
    /// whose value is the current step.
    Timeline,
    "iced_cube::timeline",
    Slider
);

impl Timeline {
    /// The slider itself, for setting or reading the current step.
    pub fn slider(&self) -> Locator<role::Slider> {
        self.root.clone().into_role::<role::Slider>()
    }
}

component!(
    /// An [`image_compare`](mod@crate::data::image_compare): a group of the
    /// before and after images and, in slider mode, the divider.
    ImageCompare,
    "iced_cube::image_compare",
    Group
);

impl ImageCompare {
    /// The before image, when one is shown.
    pub fn before(&self) -> Locator<role::Image> {
        self.image("Before")
    }

    /// The after image, or the only one.
    pub fn after(&self) -> Locator<role::Image> {
        self.image("After")
    }

    /// The divider, a splitter whose value runs from 0 to 100.
    pub fn divider(&self) -> Locator<role::Splitter> {
        self.root
            .clone()
            .get_by_role(role::Splitter)
            .name(TextMatch::exact("Divider"))
    }

    fn image(&self, name: &str) -> Locator<role::Image> {
        self.root
            .clone()
            .get_by_role(role::Image)
            .name(TextMatch::exact(name))
    }
}

fn resolved<'t, R: RoleKind>(tree: &'t SemanticTree, locator: &Locator<R>) -> Option<&'t Node> {
    let id = icedwright_locator::resolve(tree, locator).ok()?;
    tree.get(id)
}
