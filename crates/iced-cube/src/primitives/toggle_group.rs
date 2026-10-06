//! A row of toggles where one, or several, can be on.
//!
//! [`State`] owns the items and the selection. In [`Mode::Single`] pressing
//! an item turns the others off, like a segmented control; in
//! [`Mode::Multiple`] each item switches on and off by itself, like the
//! formatting buttons of a text editor. Disabled items are skipped.
//!
//! Keyboard shortcuts resolve through a [`Keymap`] of [`Action`]s; see
//! [`default_keymap`] and [`crate::keys`].

use iced::border::Radius;
use iced::keyboard::key::Named;
use iced::widget::{self, button::Status, container, row};
use iced::{Background, Border, Element, Theme};

use crate::icon::Glyph;
use crate::keys::{self, Chord, Keymap};
use crate::natural::natural;
use crate::primitives::toggle::{self, Size};
use crate::theme::{Tokens, fade, radius, space};

/// Whether one or several items can be on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    /// At most one item is on. Pressing another moves the selection.
    #[default]
    Single,
    /// Every item switches on and off by itself.
    Multiple,
}

impl Mode {
    pub const ALL: [Mode; 2] = [Mode::Single, Mode::Multiple];
}

/// How the group is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variant {
    /// Separate toggles with a small gap, transparent until hovered.
    #[default]
    Default,
    /// Attached segments inside one border.
    Outline,
}

impl Variant {
    pub const ALL: [Variant; 2] = [Variant::Default, Variant::Outline];
}

/// One item in a [`State`].
#[derive(Debug, Clone, PartialEq)]
pub struct Item<Id> {
    pub id: Id,
    pub label: String,
    pub icon: Option<Glyph>,
    pub disabled: bool,
}

/// Creates an enabled item.
pub fn item<Id>(id: Id, label: impl Into<String>) -> Item<Id> {
    Item {
        id,
        label: label.into(),
        icon: None,
        disabled: false,
    }
}

impl<Id> Item<Id> {
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Changes to the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event<Id> {
    /// Switches an item on or off. In single mode, switching one on turns
    /// the others off.
    Toggle(Id),
    /// Single mode: moves the selection to the next enabled item, wrapping.
    Next,
    /// Single mode: moves the selection to the previous enabled item,
    /// wrapping.
    Previous,
}

/// The items and which of them are on.
#[derive(Debug, Clone, PartialEq)]
pub struct State<Id> {
    items: Vec<Item<Id>>,
    mode: Mode,
    required: bool,
    selected: Vec<bool>,
}

impl<Id: Copy + PartialEq> State<Id> {
    /// A group where at most one item is on, starting with none.
    pub fn single(items: impl IntoIterator<Item = Item<Id>>) -> Self {
        Self::new(items, Mode::Single)
    }

    /// A group where every item switches on and off by itself.
    pub fn multiple(items: impl IntoIterator<Item = Item<Id>>) -> Self {
        Self::new(items, Mode::Multiple)
    }

    fn new(items: impl IntoIterator<Item = Item<Id>>, mode: Mode) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        let selected = vec![false; items.len()];
        Self {
            items,
            mode,
            required: false,
            selected,
        }
    }

    /// Switches these items on, skipping unknown and disabled ones. In
    /// single mode the first that applies replaces the selection.
    pub fn with_selected(mut self, ids: impl IntoIterator<Item = Id>) -> Self {
        let mut replaced = false;
        for id in ids {
            let Some(index) = self.enabled_index(id) else {
                continue;
            };
            if self.mode == Mode::Single {
                if replaced {
                    break;
                }
                self.selected.fill(false);
                replaced = true;
            }
            self.selected[index] = true;
        }
        self
    }

    /// Keeps the last item in a single-mode group on, so pressing it again
    /// does nothing rather than leaving the group empty. When nothing is on
    /// yet, the first enabled item is switched on.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        if required
            && self.mode == Mode::Single
            && !self.selected.contains(&true)
            && let Some(index) = self.items.iter().position(|item| !item.disabled)
        {
            self.selected[index] = true;
        }
        self
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn is_required(&self) -> bool {
        self.required
    }

    pub fn items(&self) -> &[Item<Id>] {
        &self.items
    }

    /// The ids that are on, in item order.
    pub fn selected(&self) -> Vec<Id> {
        self.items
            .iter()
            .zip(&self.selected)
            .filter_map(|(item, on)| on.then_some(item.id))
            .collect()
    }

    /// The first id that is on, which in single mode is the only one.
    pub fn selected_one(&self) -> Option<Id> {
        let index = self.selected.iter().position(|on| *on)?;
        Some(self.items[index].id)
    }

    pub fn is_selected(&self, id: Id) -> bool {
        self.index(id).is_some_and(|index| self.selected[index])
    }

    /// Enables or disables an item. Whether it is on stays as it was.
    pub fn set_disabled(&mut self, id: Id, disabled: bool) {
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            item.disabled = disabled;
        }
    }

    /// Applies an event and returns the ids that are on afterwards, if
    /// anything changed.
    pub fn update(&mut self, event: Event<Id>) -> Option<Vec<Id>> {
        let changed = match event {
            Event::Toggle(id) => self.toggle(id),
            Event::Next => self.step(true),
            Event::Previous => self.step(false),
        };
        changed.then(|| self.selected())
    }

    fn toggle(&mut self, id: Id) -> bool {
        let Some(index) = self.enabled_index(id) else {
            return false;
        };
        let on = self.selected[index];
        match self.mode {
            Mode::Multiple => self.selected[index] = !on,
            Mode::Single if on && self.required => return false,
            Mode::Single => {
                self.selected.fill(false);
                self.selected[index] = !on;
            }
        }
        true
    }

    /// Moves a single-mode selection to the nearest enabled item in one
    /// direction, wrapping round.
    fn step(&mut self, forward: bool) -> bool {
        let len = self.items.len();
        if self.mode != Mode::Single || len == 0 {
            return false;
        }
        let current = self.selected.iter().position(|on| *on);
        let start = current.unwrap_or(if forward { len - 1 } else { 0 });
        let offset = if forward { 1 } else { len - 1 };
        let Some(target) = (1..=len)
            .map(|n| (start + offset * n) % len)
            .find(|&index| !self.items[index].disabled)
        else {
            return false;
        };
        if current == Some(target) {
            return false;
        }
        self.selected.fill(false);
        self.selected[target] = true;
        true
    }

    fn index(&self, id: Id) -> Option<usize> {
        self.items.iter().position(|item| item.id == id)
    }

    fn enabled_index(&self, id: Id) -> Option<usize> {
        self.index(id).filter(|&index| !self.items[index].disabled)
    }
}

/// What a toggle group keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
}

impl Action {
    /// The [`Event`] this action sends to `state`. Only single-mode groups
    /// move with the keyboard, so a multiple-mode group gets `None`.
    pub fn event<Id: Copy + PartialEq>(self, state: &State<Id>) -> Option<Event<Id>> {
        if state.mode != Mode::Single {
            return None;
        }
        Some(match self {
            Action::Next => Event::Next,
            Action::Previous => Event::Previous,
        })
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[Action::Next, Action::Previous];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Next => "Next",
            Action::Previous => "Previous",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Turns on the next enabled item of a single-choice group, wrapping.",
            Action::Previous => {
                "Turns on the previous enabled item of a single-choice group, wrapping."
            }
        }
    }
}

/// The default toggle group shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowRight` | [`Action::Next`] |
/// | `ArrowLeft` | [`Action::Previous`] |
///
/// These are the keys tabs use, since a single-choice group switches views
/// the same way. Shortcuts are app-wide, so an app showing tabs, a slider
/// or a second group alongside should rebind one of them.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowRight), Action::Next)
        .bind(Chord::named(Named::ArrowLeft), Action::Previous)
}

/// A toggle group builder. Convert it into an [`Element`] to render.
pub struct ToggleGroup<'a, Id, Message> {
    state: &'a State<Id>,
    variant: Variant,
    size: Size,
    on_event: Option<Box<dyn Fn(Event<Id>) -> Message + 'a>>,
}

/// Renders the items of `state`. Without
/// [`on_event`](ToggleGroup::on_event) every item renders disabled.
pub fn toggle_group<Id, Message>(state: &State<Id>) -> ToggleGroup<'_, Id, Message> {
    ToggleGroup {
        state,
        variant: Variant::default(),
        size: Size::default(),
        on_event: None,
    }
}

impl<Id: std::fmt::Debug, Message> std::fmt::Debug for ToggleGroup<'_, Id, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToggleGroup")
            .field("state", self.state)
            .field("variant", &self.variant)
            .field("size", &self.size)
            .field("enabled", &self.on_event.is_some())
            .finish()
    }
}

impl<'a, Id, Message> ToggleGroup<'a, Id, Message> {
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Id>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

/// Where an item sits in an outlined group, which decides its corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Only,
    First,
    Middle,
    Last,
}

impl Position {
    fn of(index: usize, len: usize) -> Self {
        match (index, len) {
            (_, 1) => Position::Only,
            (0, _) => Position::First,
            (index, len) if index + 1 == len => Position::Last,
            _ => Position::Middle,
        }
    }
}

impl<'a, Id, Message> From<ToggleGroup<'a, Id, Message>> for Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(group: ToggleGroup<'a, Id, Message>) -> Self {
        let ToggleGroup {
            state,
            variant,
            size,
            on_event,
        } = group;
        let len = state.items.len();
        // An outlined group draws its border outside the items, so they
        // shrink by the border to keep the group as tall as a button.
        let metrics = match variant {
            Variant::Default => size.metrics(),
            Variant::Outline => toggle::Metrics {
                height: size.metrics().height - 2.0 * BORDER,
                ..size.metrics()
            },
        };

        let items = state.items.iter().enumerate().map(|(index, entry)| {
            let pressed = state.selected[index];
            let message = on_event
                .as_ref()
                .filter(|_| !entry.disabled)
                .map(|on_event| on_event(Event::Toggle(entry.id)));
            let position = Position::of(index, len);
            toggle::button(
                entry.label.as_str(),
                entry.icon,
                pressed,
                metrics,
                message,
                move |tokens, status| item_style(tokens, variant, position, pressed, status),
            )
        });

        let enabled = on_event.is_some();
        // Separate toggles wrap onto new lines on a narrow screen; attached
        // segments stay in one row, so keep outlined groups to a few short
        // items.
        match variant {
            Variant::Default => row(items.map(natural)).spacing(space::XS).wrap().into(),
            Variant::Outline => natural(
                container(row(items))
                    .padding(BORDER)
                    .style(move |theme: &Theme| group_style(&Tokens::of(theme), variant, enabled)),
            ),
        }
    }
}

/// Width of an outlined group's border.
const BORDER: f32 = 1.0;

/// The style of one item: a toggle, with square inner corners and no gap
/// when the group is outlined.
pub fn item_style(
    tokens: &Tokens,
    variant: Variant,
    position: Position,
    pressed: bool,
    status: Status,
) -> widget::button::Style {
    let base = toggle::style(tokens, toggle::Variant::Default, pressed, status);
    if variant == Variant::Default {
        return base;
    }
    // The items sit inside the group's border, so their outer corners are
    // rounded by the border less than the group's.
    let outer = radius::MD - BORDER;
    let radius = match position {
        Position::Only => Radius::from(outer),
        Position::First => Radius::default().left(outer),
        Position::Middle => Radius::default(),
        Position::Last => Radius::default().right(outer),
    };
    widget::button::Style {
        border: Border {
            radius,
            ..base.border
        },
        ..base
    }
}

/// The container around the items: nothing for separate toggles, one
/// border for an outlined group.
pub fn group_style(tokens: &Tokens, variant: Variant, enabled: bool) -> container::Style {
    match variant {
        Variant::Default => container::Style::default(),
        Variant::Outline => container::Style {
            background: Some(Background::Color(tokens.background)),
            border: Border {
                color: if enabled {
                    tokens.border
                } else {
                    fade(tokens.border, 0.5)
                },
                width: BORDER,
                radius: radius::MD.into(),
            },
            ..container::Style::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    fn items() -> [Item<u8>; 4] {
        [
            item(0, "A"),
            item(1, "B").disabled(true),
            item(2, "C"),
            item(3, "D"),
        ]
    }

    #[test]
    fn groups_start_with_nothing_on() {
        for state in [State::single(items()), State::multiple(items())] {
            assert!(state.selected().is_empty());
            assert_eq!(state.selected_one(), None);
            assert!(!state.is_required());
        }
        assert_eq!(State::single(items()).mode(), Mode::Single);
        assert_eq!(State::multiple(items()).mode(), Mode::Multiple);
    }

    #[test]
    fn single_mode_moves_the_selection_and_can_clear_it() {
        let mut state = State::single(items());
        assert_eq!(state.update(Event::Toggle(0)), Some(vec![0]));
        assert_eq!(state.update(Event::Toggle(3)), Some(vec![3]));
        assert!(!state.is_selected(0));
        assert_eq!(state.update(Event::Toggle(3)), Some(vec![]));
        assert_eq!(state.selected_one(), None);
    }

    #[test]
    fn required_single_mode_never_empties() {
        let mut state = State::single(items()).required(true);
        assert_eq!(state.selected(), vec![0]);
        assert_eq!(state.update(Event::Toggle(0)), None);
        assert_eq!(state.update(Event::Toggle(2)), Some(vec![2]));
        assert_eq!(state.update(Event::Toggle(2)), None);

        let kept = State::single(items()).with_selected([3]).required(true);
        assert_eq!(kept.selected(), vec![3]);
        let replaced = State::single(items()).required(true).with_selected([3]);
        assert_eq!(replaced.selected(), vec![3]);
    }

    #[test]
    fn multiple_mode_switches_items_independently() {
        let mut state = State::multiple(items());
        assert_eq!(state.update(Event::Toggle(0)), Some(vec![0]));
        assert_eq!(state.update(Event::Toggle(3)), Some(vec![0, 3]));
        assert_eq!(state.update(Event::Toggle(0)), Some(vec![3]));
        assert_eq!(state.selected_one(), Some(3));
    }

    #[test]
    fn disabled_and_unknown_items_do_not_change() {
        for mut state in [State::single(items()), State::multiple(items())] {
            assert_eq!(state.update(Event::Toggle(1)), None);
            assert_eq!(state.update(Event::Toggle(9)), None);
            assert!(state.selected().is_empty());
        }
    }

    #[test]
    fn with_selected_skips_disabled_and_keeps_one_in_single_mode() {
        let single = State::single(items()).with_selected([1, 3, 0]);
        assert_eq!(single.selected(), vec![3]);
        let multiple = State::multiple(items()).with_selected([3, 1, 0]);
        assert_eq!(multiple.selected(), vec![0, 3]);
    }

    #[test]
    fn next_and_previous_skip_disabled_and_wrap() {
        let mut state = State::single(items()).with_selected([0]);
        assert_eq!(state.update(Event::Next), Some(vec![2]));
        assert_eq!(state.update(Event::Next), Some(vec![3]));
        assert_eq!(state.update(Event::Next), Some(vec![0]));
        assert_eq!(state.update(Event::Previous), Some(vec![3]));
        assert_eq!(state.update(Event::Previous), Some(vec![2]));
        assert_eq!(state.update(Event::Previous), Some(vec![0]));
    }

    #[test]
    fn next_from_nothing_picks_the_first_and_previous_the_last() {
        let mut forward = State::single(items());
        assert_eq!(forward.update(Event::Next), Some(vec![0]));
        let mut backward = State::single(items());
        assert_eq!(backward.update(Event::Previous), Some(vec![3]));
    }

    #[test]
    fn stepping_does_nothing_in_multiple_mode_or_without_enabled_items() {
        let mut multiple = State::multiple(items());
        assert_eq!(multiple.update(Event::Next), None);
        let mut empty: State<u8> = State::single([]);
        assert_eq!(empty.update(Event::Next), None);
        let mut disabled = State::single([item(0, "A").disabled(true)]);
        assert_eq!(disabled.update(Event::Previous), None);
        let mut lonely = State::single([item(0, "A")]).with_selected([0]);
        assert_eq!(lonely.update(Event::Next), None);
    }

    #[test]
    fn set_disabled_keeps_the_selection() {
        let mut state = State::multiple(items()).with_selected([0]);
        state.set_disabled(0, true);
        assert!(state.is_selected(0));
        assert_eq!(state.update(Event::Toggle(0)), None);
        state.set_disabled(1, false);
        assert_eq!(state.update(Event::Toggle(1)), Some(vec![0, 1]));
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "ArrowRight"), Some(Action::Next));
        assert_eq!(press(&keymap, "ArrowLeft"), Some(Action::Previous));
        assert_eq!(press(&keymap, "Tab"), None);
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?}");
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&Chord::named(Named::ArrowRight))
            .bind("Alt+ArrowDown".parse().unwrap(), Action::Next);
        assert_eq!(press(&keymap, "ArrowRight"), None);
        assert_eq!(press(&keymap, "Alt+ArrowDown"), Some(Action::Next));
        assert_eq!(press(&keymap, "ArrowLeft"), Some(Action::Previous));
    }

    #[test]
    fn actions_map_to_events_in_single_mode_only() {
        let mut single = State::single(items()).with_selected([0]);
        assert_eq!(Action::Next.event(&single), Some(Event::Next));
        assert_eq!(Action::Previous.event(&single), Some(Event::Previous));
        let event = Action::Next.event(&single).unwrap();
        assert_eq!(single.update(event), Some(vec![2]));

        let multiple = State::multiple(items());
        assert_eq!(Action::Next.event(&multiple), None);
        assert_eq!(Action::Previous.event(&multiple), None);
    }

    #[test]
    fn positions_follow_the_item_index() {
        assert_eq!(Position::of(0, 1), Position::Only);
        assert_eq!(Position::of(0, 3), Position::First);
        assert_eq!(Position::of(1, 3), Position::Middle);
        assert_eq!(Position::of(2, 3), Position::Last);
    }

    #[test]
    fn outlined_items_round_only_their_outer_corners() {
        let tokens = Tokens::of(&light());
        let corners = |position| {
            item_style(&tokens, Variant::Outline, position, true, Status::Active)
                .border
                .radius
        };
        let outer = radius::MD - 1.0;
        assert_eq!(corners(Position::Only), Radius::from(outer));
        assert_eq!(corners(Position::Middle), Radius::default());
        let first = corners(Position::First);
        assert_eq!((first.top_left, first.top_right), (outer, 0.0));
        let last = corners(Position::Last);
        assert_eq!((last.top_left, last.bottom_right), (0.0, outer));
    }

    #[test]
    fn items_never_draw_their_own_border() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in Variant::ALL {
                for pressed in [false, true] {
                    let style =
                        item_style(&tokens, variant, Position::Middle, pressed, Status::Hovered);
                    assert_eq!(style.border.width, 0.0);
                }
            }
        }
    }

    #[test]
    fn pressed_items_match_a_pressed_toggle() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let item = item_style(
                &tokens,
                Variant::Default,
                Position::Only,
                true,
                Status::Active,
            );
            let toggle = toggle::style(&tokens, toggle::Variant::Default, true, Status::Active);
            assert_eq!(item, toggle);
            assert_eq!(item.background, Some(Background::Color(tokens.accent)));
        }
    }

    #[test]
    fn only_the_outline_group_has_a_border_and_it_fades_when_disabled() {
        let tokens = Tokens::of(&dark());
        assert_eq!(
            group_style(&tokens, Variant::Default, true).border.width,
            0.0
        );
        let enabled = group_style(&tokens, Variant::Outline, true);
        let disabled = group_style(&tokens, Variant::Outline, false);
        assert_eq!(enabled.border.width, 1.0);
        assert_eq!(enabled.border.color, tokens.border);
        assert!(disabled.border.color.a < enabled.border.color.a);
    }
}
