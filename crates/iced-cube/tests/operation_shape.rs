//! What focusable controls report to iced's operations. Without the
//! `icedwright` feature the focus report is a leaf beside the content, as it
//! always was; with it, the content nests under the report so the recorder
//! can fold the focus into the control's annotation.
#![cfg(all(feature = "button", feature = "select"))]

use iced::Element;
use iced_cube::{button, select};
use icedwright_protocol::{RawCall, RawNode, RawTree};

#[path = "support/semantic.rs"]
mod semantic;

#[derive(Debug, Clone)]
enum Message {
    Pressed,
    Picked,
}

/// Every `focusable` call in the tree, depth first.
fn focusables(tree: &RawTree) -> Vec<&RawNode> {
    fn walk<'t>(node: &'t RawNode, found: &mut Vec<&'t RawNode>) {
        if matches!(node.call, RawCall::Focusable { .. }) {
            found.push(node);
        }
        for child in &node.children {
            walk(child, found);
        }
    }
    let mut found = Vec::new();
    for window in &tree.windows {
        walk(&window.root, &mut found);
    }
    found
}

fn controls() -> [Element<'static, Message>; 2] {
    [
        button("Save").on_press(Message::Pressed).into(),
        select(&["One", "Two"][..], Some("One"))
            .on_select(|_| Message::Picked)
            .into(),
    ]
}

#[test]
fn focus_reports_keep_their_shape() {
    for control in controls() {
        let raw = semantic::Recorder::default().raw(control);
        let found = focusables(&raw);
        assert_eq!(found.len(), 1, "{raw:#?}");
        let nested = found.iter().all(|node| !node.children.is_empty());
        if cfg!(feature = "icedwright") {
            assert!(nested, "the content nests under the report: {raw:#?}");
        } else {
            assert!(!nested, "the report stays a leaf: {raw:#?}");
        }
    }
}
