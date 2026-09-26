//! Every component's default shortcuts, exported for the docs site.

use std::collections::BTreeMap;

use iced_cube::forms::select;
use iced_cube::keys::{Action, Keymap};
use iced_cube::layout::accordion;
use iced_cube::navigation::tabs;
use iced_cube::overlay::toast;
use iced_cube::primitives::{checkbox, slider, switch};
use serde::Serialize;

/// One action and the chords that trigger it by default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Binding {
    pub keys: Vec<String>,
    pub action: &'static str,
    pub description: &'static str,
}

/// The default bindings of one action type, grouped by action.
pub fn bindings<A: Action>() -> Vec<Binding> {
    Keymap::<A>::defaults()
        .grouped()
        .into_iter()
        .map(|(action, chords)| Binding {
            keys: chords.iter().map(ToString::to_string).collect(),
            action: action.name(),
            description: action.description(),
        })
        .collect()
}

/// Default bindings keyed by docs page slug.
pub fn all() -> BTreeMap<&'static str, Vec<Binding>> {
    BTreeMap::from([
        ("accordion", bindings::<accordion::Action>()),
        ("checkbox", bindings::<checkbox::Action>()),
        ("select", bindings::<select::Action>()),
        ("slider", bindings::<slider::Action>()),
        ("switch", bindings::<switch::Action>()),
        ("tabs", bindings::<tabs::Action>()),
        ("toast", bindings::<toast::Action>()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords_for_one_action_share_an_entry() {
        let tabs = bindings::<tabs::Action>();
        assert_eq!(tabs.len(), 4);
        assert_eq!(tabs[0].action, "Next");
        assert_eq!(tabs[0].keys, ["Ctrl+Tab", "ArrowRight"]);
        assert_eq!(tabs[1].keys, ["Ctrl+Shift+Tab", "ArrowLeft"]);
    }

    #[test]
    fn every_action_is_exported_with_a_sentence() {
        for (slug, bindings) in all() {
            assert!(!bindings.is_empty(), "{slug}");
            for binding in bindings {
                assert!(!binding.keys.is_empty(), "{slug} {}", binding.action);
                assert!(
                    binding.description.ends_with('.'),
                    "{slug} {}",
                    binding.action
                );
            }
        }
    }

    #[test]
    fn serialises_in_the_site_shape() {
        let json = serde_json::to_value(all()).unwrap();
        let first = &json["toast"][0];
        assert_eq!(first["keys"][0], "Escape");
        assert_eq!(first["action"], "DismissLatest");
        assert!(first["description"].is_string());
    }
}
