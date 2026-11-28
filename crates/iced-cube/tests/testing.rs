//! Component locators resolve against the trees components report.
#![cfg(all(
    feature = "icedwright",
    feature = "tree",
    feature = "tabs",
    feature = "document-tabs",
    feature = "dropdown-menu",
    feature = "virtual-list",
    feature = "timeline",
    feature = "image-compare"
))]

use iced::Element;
use iced::widget::{button, column, text};
use iced_cube::data::{image_compare, timeline, tree, virtual_list};
use iced_cube::navigation::{document_tabs, tabs};
use iced_cube::overlay::dropdown_menu;
use iced_cube::testing::{
    ComponentLocator, DocumentTabs, ImageCompare, Menu, ScrollTarget, Tabs, Timeline, Tree,
    VirtualList,
};
use icedwright_locator::{Locator, RoleKind, resolve};
use icedwright_semantics::SemanticTree;

#[path = "support/semantic.rs"]
mod semantic;

/// The name of the one node `locator` finds.
fn name_of<R: RoleKind>(tree: &SemanticTree, locator: &Locator<R>) -> Option<String> {
    let id = resolve(tree, locator).ok()?;
    tree.get(id).and_then(|node| node.name.clone())
}

#[test]
fn a_tree_item_is_found_by_its_path_and_its_expander() {
    let state = tree::State::new([
        tree::node(1, "Projects").folder().children([
            tree::node(2, "Roadmap").file(),
            tree::node(3, "Budget").file(),
        ]),
        tree::node(4, "Roadmap").file(),
    ])
    .with_expanded([1]);
    let element: Element<'_, tree::Event<u32>> = tree::tree(&state).on_event(|e| e).into();
    let found = semantic::Recorder::default().tree(element);

    let files = Tree::of_kind();
    let nested = files.item(["Projects", "Roadmap"]);
    let id = resolve(&found, &nested).expect("the nested item");
    let node = found.get(id).expect("a node");
    assert_eq!(node.set.level, Some(2));
    assert_eq!(
        name_of(&found, &files.item(["Roadmap"])).as_deref(),
        Some("Roadmap")
    );
    assert_eq!(
        name_of(&found, &files.expander(["Projects"])).as_deref(),
        Some("Collapse")
    );
    assert_eq!(files.selected().count(&found), 0);
}

#[test]
fn tabs_are_found_by_name_and_selection() {
    let state = tabs::State::new([tabs::tab(1u8, "Account"), tabs::tab(2, "Billing")]);
    let element: Element<'_, tabs::Event<u8>> = tabs::tabs(&state).on_event(|e| e).into();
    let found = semantic::Recorder::default().tree(element);

    let strip = Tabs::of_kind();
    assert_eq!(strip.tab("Billing").count(&found), 1);
    assert_eq!(
        name_of(&found, &strip.selected()).as_deref(),
        Some("Account")
    );
}

#[test]
fn document_tabs_find_close_buttons_and_unsaved_tabs() {
    let state = document_tabs::State::new([
        document_tabs::document(1u8, "alpha.rs"),
        document_tabs::document(2, "beta.rs").modified(true),
    ]);
    let element: Element<'_, document_tabs::Event<u8>> =
        document_tabs::document_tabs(&state).on_event(|e| e).into();
    let found = semantic::Recorder::default().tree(element);

    let strip = DocumentTabs::of_kind();
    assert_eq!(
        name_of(&found, &strip.selected()).as_deref(),
        Some("alpha.rs")
    );
    assert_eq!(
        name_of(&found, &strip.close_button("beta.rs")).as_deref(),
        Some("Close beta.rs")
    );
    let unsaved: Vec<_> = strip
        .unsaved(&found)
        .iter()
        .filter_map(|tab| name_of(&found, tab))
        .collect();
    assert_eq!(unsaved, ["beta.rs"]);
}

#[test]
fn an_open_menu_finds_plain_and_check_items() {
    let mut state = dropdown_menu::State::new([
        dropdown_menu::item(1u8, "Copy"),
        dropdown_menu::checkbox_item(2, "Show grid", false),
    ]);
    let _ = state.update(dropdown_menu::Event::Open);
    let element: Element<'_, dropdown_menu::Event<u8>> = column![
        dropdown_menu::dropdown_menu(
            &state,
            button(text("Actions")).on_press(dropdown_menu::Event::Toggle)
        )
        .on_event(|e| e)
    ]
    .into();
    let found = semantic::Recorder::default().tree(element);

    let menu = Menu::of_kind();
    assert_eq!(name_of(&found, &menu.item("Copy")).as_deref(), Some("Copy"));
    assert_eq!(
        name_of(&found, &menu.item("Show grid")).as_deref(),
        Some("Show grid")
    );
}

#[test]
fn a_virtual_list_finds_rendered_rows_and_scrolls_to_others() {
    let state = virtual_list::State::new(0..1000u32);
    let element: Element<'_, virtual_list::Event<u32>> =
        virtual_list::virtual_list(&state, |_, n: &u32| text(format!("Row {n}")).into())
            .height(90)
            .on_event(|e| e)
            .into();
    let found = semantic::Recorder::default().tree(element);

    let list = VirtualList::of_kind();
    let row = list.row(&found, 2).expect("row 2 is rendered");
    assert_eq!(name_of(&found, &row).as_deref(), Some("Row 2"));
    assert!(list.row(&found, 500).is_none());

    let Some(ScrollTarget { offset, .. }) = list.scroll_target(&found, 500) else {
        panic!("a scroll target");
    };
    let height = resolve(&found, &row)
        .ok()
        .and_then(|id| found.get(id))
        .map(|node| node.bounds.height)
        .expect("row bounds");
    assert_eq!(offset, 500.0 * height);
}

#[test]
fn a_timeline_and_an_image_compare_find_their_parts() {
    let picture = || iced::widget::image::Handle::from_rgba(2, 1, vec![128; 8]);
    let element: Element<'_, usize> = column![
        timeline::timeline([timeline::Tone::Neutral; 4], 2)
            .label("Steps")
            .on_select(|step| step),
        image_compare::image_compare(picture(), picture())
            .divider(0.25)
            .on_divider(|_| 0),
    ]
    .into();
    let found = semantic::Recorder::default().tree(element);

    let steps = Timeline::of_kind();
    assert_eq!(name_of(&found, &steps.slider()).as_deref(), Some("Steps"));

    let compare = ImageCompare::of_kind();
    assert_eq!(
        name_of(&found, &compare.before()).as_deref(),
        Some("Before")
    );
    assert_eq!(name_of(&found, &compare.after()).as_deref(), Some("After"));
    assert_eq!(
        name_of(&found, &compare.divider()).as_deref(),
        Some("Divider")
    );
}
