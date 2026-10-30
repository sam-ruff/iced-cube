//! A bar of three to five destinations along the bottom of a phone screen.
//!
//! Each destination is an icon above a short label, sharing the width
//! equally. The active one sits on a pill. The app owns which destination
//! is active and changes it in `on_select`; selecting the active one sends
//! `on_select` too, which apps use to go back to the top of that screen.
//!
//! Destinations respond to taps without capturing the finger, as a
//! [`tappable`](mod@crate::primitives::tappable) does.
//!
//! Keyboard shortcuts resolve through a [`Keymap`] of [`Action`]s; see
//! [`default_keymap`] and [`crate::keys`].
//!
//! ```no_run
//! use iced_cube::{bottom_nav, lucide};
//!
//! #[derive(Debug, Clone, Copy, PartialEq)]
//! enum Screen { Notes, Search, Settings }
//!
//! #[derive(Debug, Clone)]
//! enum Message { Go(Screen) }
//!
//! let bar: iced::Element<'_, Message> = bottom_nav(Screen::Notes)
//!     .item(Screen::Notes, lucide!(NotebookText), "Notes")
//!     .item(Screen::Search, lucide!(Search), "Search")
//!     .item(Screen::Settings, lucide!(Settings), "Settings")
//!     .on_select(Message::Go)
//!     .into();
//! ```

use std::fmt;

use iced::widget::{column, container, row, space, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};

use crate::icon::{Glyph, opacity, themed};
use crate::keys::{self, Chord, Keymap};
use crate::primitives::tappable::{self, Status, tappable};
use crate::theme::{Tokens, fade, radius, text_size};

/// Height of the bar above any inset, its top border included.
pub const HEIGHT: f32 = 80.0;
/// Size of the pill behind the active icon.
pub const PILL: (f32, f32) = (64.0, 32.0);
/// Icon size.
pub const ICON: f32 = 24.0;
/// Label size.
pub const LABEL: f32 = text_size::XS;
/// The narrowest a destination gets: a comfortable touch target.
pub const MIN_SLOT_WIDTH: f32 = 48.0;
/// The most destinations a bar shows. Later items are dropped.
pub const MAX_ITEMS: usize = 5;

/// One destination.
#[derive(Debug, Clone, PartialEq)]
pub struct Item<Id> {
    pub id: Id,
    pub glyph: Glyph,
    pub label: String,
}

/// A bottom navigation builder. Convert it into an [`Element`] to render.
///
/// Without [`on_select`](BottomNav::on_select) the destinations do nothing
/// and are drawn faded.
pub struct BottomNav<'a, Id, Message> {
    active: Id,
    items: Vec<Item<Id>>,
    on_select: Option<Box<dyn Fn(Id) -> Message + 'a>>,
    inset: f32,
}

/// Creates a bar with `active` as the current destination.
pub fn bottom_nav<'a, Id: Copy + PartialEq, Message>(active: Id) -> BottomNav<'a, Id, Message> {
    BottomNav {
        active,
        items: Vec::new(),
        on_select: None,
        inset: 0.0,
    }
}

impl<Id: fmt::Debug, Message> fmt::Debug for BottomNav<'_, Id, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BottomNav")
            .field("active", &self.active)
            .field("items", &self.items)
            .field("enabled", &self.on_select.is_some())
            .field("inset", &self.inset)
            .finish()
    }
}

impl<'a, Id: Copy + PartialEq, Message> BottomNav<'a, Id, Message> {
    /// Adds a destination. Use three to five; past five the rest are not
    /// shown.
    pub fn item(mut self, id: Id, glyph: Glyph, label: impl Into<String>) -> Self {
        self.items.push(Item {
            id,
            glyph,
            label: label.into(),
        });
        self
    }

    /// The message for selecting a destination, the active one included.
    pub fn on_select(mut self, on_select: impl Fn(Id) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// Extra space below the destinations, such as the height of a system
    /// navigation bar drawn over the window. Defaults to 0.
    pub fn inset(mut self, bottom: f32) -> Self {
        self.inset = bottom.max(0.0);
        self
    }

    pub fn items(&self) -> &[Item<Id>] {
        &self.items
    }

    pub fn is_enabled(&self) -> bool {
        self.on_select.is_some()
    }
}

impl<'a, Id, Message> From<BottomNav<'a, Id, Message>> for Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(nav: BottomNav<'a, Id, Message>) -> Self {
        let BottomNav {
            active,
            items,
            on_select,
            inset,
        } = nav;
        let enabled = on_select.is_some();

        let slots = items.into_iter().take(MAX_ITEMS).map(|item| {
            let on = item.id == active;
            let message = on_select.as_ref().map(|on_select| on_select(item.id));
            slot(item, on, enabled, message)
        });

        let border = container(space())
            .width(Length::Fill)
            .height(1)
            .style(|theme: &Theme| container::Style {
                background: Some(Background::Color(Tokens::of(theme).border)),
                ..container::Style::default()
            });

        container(column![
            border,
            row(slots).width(Length::Fill).height(HEIGHT - 1.0)
        ])
        .width(Length::Fill)
        .padding(Padding {
            bottom: inset,
            ..Padding::ZERO
        })
        .style(|theme: &Theme| bar_style(&Tokens::of(theme)))
        .into()
    }
}

fn slot<'a, Id, Message: Clone + 'a>(
    item: Item<Id>,
    on: bool,
    enabled: bool,
    message: Option<Message>,
) -> Element<'a, Message> {
    let icon = themed(item.glyph, ICON, opacity(enabled), move |theme| {
        colours(&Tokens::of(theme), on).icon
    });
    let pill = container(icon)
        .center_x(PILL.0)
        .center_y(PILL.1)
        .style(move |theme: &Theme| pill_style(&Tokens::of(theme), on));
    let label = text(item.label)
        .size(LABEL)
        .font(if on {
            crate::theme::semibold()
        } else {
            crate::theme::font()
        })
        .wrapping(text::Wrapping::None)
        .style(move |theme: &Theme| {
            let colour = colours(&Tokens::of(theme), on).label;
            text::Style {
                color: Some(if enabled { colour } else { fade(colour, 0.5) }),
            }
        });

    let content = container(column![pill, label].spacing(4).align_x(Alignment::Center))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .clip(true);

    let mut slot = tappable(content)
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .style(|theme, status| slot_style(&Tokens::of(theme), status));
    if let Some(message) = message {
        slot = slot.on_tap(message);
    }
    slot.into()
}

/// Icon and label colours of a destination.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colours {
    pub icon: Color,
    pub label: Color,
}

/// The active destination's icon reads on its pill and its label in full
/// foreground; the others are muted but readable in both themes.
pub fn colours(tokens: &Tokens, active: bool) -> Colours {
    if active {
        return Colours {
            icon: tokens.accent_foreground,
            label: tokens.foreground,
        };
    }
    Colours {
        icon: tokens.muted_foreground,
        label: tokens.muted_foreground,
    }
}

/// The pill behind the active icon, and nothing behind the others.
pub fn pill_style(tokens: &Tokens, active: bool) -> container::Style {
    container::Style {
        background: active.then_some(Background::Color(tokens.accent)),
        border: Border {
            radius: radius::FULL.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// A destination's touch feedback: a faint wash of the foreground on hover
/// and a stronger one while pressed, visible on any bar colour.
pub fn slot_style(tokens: &Tokens, status: Status) -> container::Style {
    let alpha = match status {
        Status::Idle => return container::Style::default(),
        Status::Hovered => 0.04,
        Status::Pressed => 0.1,
    };
    container::Style {
        background: Some(Background::Color(fade(tokens.foreground, alpha))),
        ..tappable::style(tokens, status)
    }
}

/// The bar itself.
pub fn bar_style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens.background)),
        text_color: Some(tokens.foreground),
        ..container::Style::default()
    }
}

/// What a bottom navigation keyboard shortcut does: go to the destination
/// at an index, counting from zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Go(usize),
}

impl Action {
    /// The destination this action selects, if there is one at its index.
    pub fn target<Id: Copy>(self, ids: &[Id]) -> Option<Id> {
        let Action::Go(index) = self;
        ids.get(index).copied()
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Go(0),
        Action::Go(1),
        Action::Go(2),
        Action::Go(3),
        Action::Go(4),
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Go(0) => "First",
            Action::Go(1) => "Second",
            Action::Go(2) => "Third",
            Action::Go(3) => "Fourth",
            Action::Go(_) => "Fifth",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Go(0) => "Selects the first destination.",
            Action::Go(1) => "Selects the second destination.",
            Action::Go(2) => "Selects the third destination.",
            Action::Go(3) => "Selects the fourth destination.",
            Action::Go(_) => "Selects the fifth destination.",
        }
    }
}

/// The default bottom navigation shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `Alt+1` to `Alt+5` | [`Action::Go`] with index 0 to 4 |
///
/// Alt and a number is how many desktop apps jump between their main
/// views, so a phone layout shown on a desktop keeps that habit.
pub fn default_keymap() -> Keymap<Action> {
    ['1', '2', '3', '4', '5']
        .into_iter()
        .enumerate()
        .fold(Keymap::new(), |keymap, (index, key)| {
            keymap.bind(Chord::character(key).alt(), Action::Go(index))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn builder_defaults() {
        let nav: BottomNav<'_, u8, ()> = bottom_nav(0);
        assert!(nav.items().is_empty());
        assert!(!nav.is_enabled());
        assert_eq!(nav.inset, 0.0);
        let nav: BottomNav<'_, u8, u8> = bottom_nav(1)
            .item(0, crate::lucide!(House), "Home")
            .item(1, crate::lucide!(Search), "Search")
            .inset(-4.0)
            .on_select(|id| id);
        assert_eq!(nav.items().len(), 2);
        assert_eq!(nav.items()[1].label, "Search");
        assert!(nav.is_enabled());
        assert_eq!(nav.inset, 0.0);
    }

    #[test]
    fn the_active_destination_stands_out_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let on = colours(&tokens, true);
            let off = colours(&tokens, false);
            assert_ne!(on.label, off.label);
            assert_eq!(off.label, tokens.muted_foreground);
            assert!(pill_style(&tokens, true).background.is_some());
            assert!(pill_style(&tokens, false).background.is_none());
        }
    }

    #[test]
    fn inactive_labels_are_readable_in_dark_mode() {
        let tokens = Tokens::of(&dark());
        let label = colours(&tokens, false).label;
        let lightness = (label.r + label.g + label.b) / 3.0;
        let background = (tokens.background.r + tokens.background.g + tokens.background.b) / 3.0;
        assert!(lightness - background > 0.4, "{label:?}");
    }

    #[test]
    fn slot_feedback_is_clear_at_rest_and_stronger_when_pressed() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert!(slot_style(&tokens, Status::Idle).background.is_none());
            let alpha = |status| match slot_style(&tokens, status).background {
                Some(Background::Color(colour)) => colour.a,
                _ => 0.0,
            };
            assert!(alpha(Status::Pressed) > alpha(Status::Hovered));
            assert!(alpha(Status::Hovered) > 0.0);
        }
    }

    #[test]
    fn the_bar_uses_the_background() {
        let tokens = Tokens::of(&light());
        assert_eq!(
            bar_style(&tokens).background,
            Some(Background::Color(tokens.background))
        );
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "Alt+1"), Some(Action::Go(0)));
        assert_eq!(press(&keymap, "Alt+5"), Some(Action::Go(4)));
        assert_eq!(press(&keymap, "1"), None);
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?}");
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&Chord::character('1').alt())
            .bind("Ctrl+1".parse().unwrap(), Action::Go(0));
        assert_eq!(press(&keymap, "Alt+1"), None);
        assert_eq!(press(&keymap, "Ctrl+1"), Some(Action::Go(0)));
    }

    #[test]
    fn actions_pick_the_destination_at_their_index() {
        let ids = ['a', 'b', 'c'];
        assert_eq!(Action::Go(0).target(&ids), Some('a'));
        assert_eq!(Action::Go(2).target(&ids), Some('c'));
        assert_eq!(Action::Go(4).target(&ids), None);
    }
}
