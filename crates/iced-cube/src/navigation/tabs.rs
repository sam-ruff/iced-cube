//! Switches between related views that share one place on screen.
//!
//! [`State`] owns the tabs and the selection. Its [`update`](State::update)
//! handles selecting, moving to the next or previous tab with wrap-around,
//! and skips disabled tabs. The app renders the selected view itself.
//!
//! Keyboard shortcuts resolve through a [`Keymap`] of [`Action`]s; see
//! [`default_keymap`] and [`crate::keys`].

use iced::keyboard::key::Named;
use iced::widget::{self, button::Status, column, container, row, rule, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Theme, Vector};

use crate::icon::{Glyph, opacity, themed};
use crate::keys::{self, Chord, Keymap};
use crate::theme::{Tokens, fade, mix, radius, space, text_size};

/// How the tab list looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variant {
    /// Plain labels over a rule, the selected one underlined.
    #[default]
    Underline,
    /// A segmented control, the selected tab raised.
    Pills,
}

impl Variant {
    pub const ALL: [Variant; 2] = [Variant::Underline, Variant::Pills];
}

/// One tab in a [`State`].
#[derive(Debug, Clone, PartialEq)]
pub struct Tab<Id> {
    pub id: Id,
    pub label: String,
    pub icon: Option<Glyph>,
    pub disabled: bool,
}

/// Creates an enabled tab.
pub fn tab<Id>(id: Id, label: impl Into<String>) -> Tab<Id> {
    Tab {
        id,
        label: label.into(),
        icon: None,
        disabled: false,
    }
}

impl<Id> Tab<Id> {
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
    Select(Id),
    Next,
    Previous,
}

/// The tabs and which one is selected.
#[derive(Debug, Clone, PartialEq)]
pub struct State<Id> {
    tabs: Vec<Tab<Id>>,
    selected: Option<usize>,
}

impl<Id: Copy + PartialEq> State<Id> {
    /// Selects the first enabled tab.
    pub fn new(tabs: impl IntoIterator<Item = Tab<Id>>) -> Self {
        let tabs: Vec<_> = tabs.into_iter().collect();
        let selected = tabs.iter().position(|tab| !tab.disabled);
        Self { tabs, selected }
    }

    /// Starts on `id` when it exists and is enabled.
    pub fn with_selected(mut self, id: Id) -> Self {
        let _ = self.update(Event::Select(id));
        self
    }

    pub fn tabs(&self) -> &[Tab<Id>] {
        &self.tabs
    }

    pub fn selected(&self) -> Option<Id> {
        self.selected.map(|index| self.tabs[index].id)
    }

    pub fn is_selected(&self, id: Id) -> bool {
        self.selected() == Some(id)
    }

    /// Enables or disables a tab. The selection stays where it is.
    pub fn set_disabled(&mut self, id: Id, disabled: bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) {
            tab.disabled = disabled;
        }
    }

    /// Applies an event and returns the newly selected id, if it changed.
    pub fn update(&mut self, event: Event<Id>) -> Option<Id> {
        let target = match event {
            Event::Select(id) => self
                .tabs
                .iter()
                .position(|tab| tab.id == id && !tab.disabled),
            Event::Next => self.step(true),
            Event::Previous => self.step(false),
        }?;

        if self.selected == Some(target) {
            return None;
        }
        self.selected = Some(target);
        Some(self.tabs[target].id)
    }

    /// The nearest enabled index in one direction, wrapping round.
    fn step(&self, forward: bool) -> Option<usize> {
        let len = self.tabs.len();
        if len == 0 {
            return None;
        }
        let start = self.selected.unwrap_or(if forward { len - 1 } else { 0 });
        let offset = if forward { 1 } else { len - 1 };
        (1..=len)
            .map(|n| (start + offset * n) % len)
            .find(|&index| !self.tabs[index].disabled)
    }
}

/// What a tabs keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
    First,
    Last,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when no tab
    /// is enabled.
    pub fn event<Id: Copy + PartialEq>(self, state: &State<Id>) -> Option<Event<Id>> {
        let mut enabled = state.tabs.iter().filter(|tab| !tab.disabled);
        match self {
            Action::Next => Some(Event::Next),
            Action::Previous => Some(Event::Previous),
            Action::First => enabled.next().map(|tab| Event::Select(tab.id)),
            Action::Last => enabled.next_back().map(|tab| Event::Select(tab.id)),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[Action::Next, Action::Previous, Action::First, Action::Last];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Next => "Next",
            Action::Previous => "Previous",
            Action::First => "First",
            Action::Last => "Last",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Selects the next enabled tab, wrapping at the end.",
            Action::Previous => "Selects the previous enabled tab, wrapping at the start.",
            Action::First => "Selects the first enabled tab.",
            Action::Last => "Selects the last enabled tab.",
        }
    }
}

/// The default tabs shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `Ctrl+Tab`, `ArrowRight` | [`Action::Next`] |
/// | `Ctrl+Shift+Tab`, `ArrowLeft` | [`Action::Previous`] |
/// | `Home` | [`Action::First`] |
/// | `End` | [`Action::Last`] |
///
/// Shortcuts are app-wide, not tied to focus. Arrow keys, Home and End
/// only reach the keymap when no widget captures them, but an app with
/// text inputs or other arrow-driven components may still want to unbind
/// them and keep only `Ctrl+Tab`.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::Tab).ctrl(), Action::Next)
        .bind(Chord::named(Named::ArrowRight), Action::Next)
        .bind(Chord::named(Named::Tab).ctrl().shift(), Action::Previous)
        .bind(Chord::named(Named::ArrowLeft), Action::Previous)
        .bind(Chord::named(Named::Home), Action::First)
        .bind(Chord::named(Named::End), Action::Last)
}

/// A tab list builder. Convert it into an [`Element`] to render.
pub struct Tabs<'a, Id, Message> {
    state: &'a State<Id>,
    variant: Variant,
    width: Option<Length>,
    on_event: Option<Box<dyn Fn(Event<Id>) -> Message + 'a>>,
}

/// Renders the tab list of `state`. Without
/// [`on_event`](Tabs::on_event) every tab renders disabled.
pub fn tabs<Id, Message>(state: &State<Id>) -> Tabs<'_, Id, Message> {
    Tabs {
        state,
        variant: Variant::default(),
        width: None,
        on_event: None,
    }
}

impl<Id: std::fmt::Debug, Message> std::fmt::Debug for Tabs<'_, Id, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tabs")
            .field("state", self.state)
            .field("variant", &self.variant)
            .finish_non_exhaustive()
    }
}

impl<'a, Id, Message> Tabs<'a, Id, Message> {
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Underline lists fill their row by default, pills shrink to fit.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Id>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

impl<'a, Id, Message> From<Tabs<'a, Id, Message>> for Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(tabs: Tabs<'a, Id, Message>) -> Self {
        let Tabs {
            state,
            variant,
            width,
            on_event,
        } = tabs;

        let items = state.tabs.iter().map(|tab| {
            let selected = state.is_selected(tab.id);
            let message = on_event
                .as_ref()
                .filter(|_| !tab.disabled)
                .map(|on_event| on_event(Event::Select(tab.id)));
            item(tab, variant, selected, message)
        });

        match variant {
            Variant::Underline => column![
                row(items).spacing(space::XS),
                rule::horizontal(1).style(|theme: &Theme| rule::Style {
                    color: Tokens::of(theme).border,
                    radius: 0.0.into(),
                    fill_mode: rule::FillMode::Full,
                    snap: true,
                }),
            ]
            .width(width.unwrap_or(Length::Fill))
            .into(),
            Variant::Pills => container(row(items).spacing(space::XS))
                .padding(space::XS)
                .width(width.unwrap_or(Length::Shrink))
                .style(move |theme| list_style(&Tokens::of(theme), variant))
                .into(),
        }
    }
}

fn item<'a, Id, Message: Clone + 'a>(
    tab: &'a Tab<Id>,
    variant: Variant,
    selected: bool,
    message: Option<Message>,
) -> Element<'a, Message> {
    let enabled = message.is_some();
    let mut label = row![].spacing(space::SM).align_y(Alignment::Center);
    if let Some(glyph) = tab.icon {
        let status = if enabled {
            Status::Active
        } else {
            Status::Disabled
        };
        label = label.push(themed(glyph, 16.0, opacity(enabled), move |theme| {
            foreground(&Tokens::of(theme), selected, status)
        }));
    }
    label = label.push(text(tab.label.as_str()).size(text_size::SM));

    let button = widget::button(label)
        .padding([6.0, space::MD])
        .on_press_maybe(message)
        .style(move |theme, status| style(&Tokens::of(theme), variant, selected, status));

    if variant == Variant::Pills {
        return button.into();
    }

    column![
        button,
        container(iced::widget::space())
            .width(Length::Fill)
            .height(2)
            .style(move |theme| indicator_style(&Tokens::of(theme), selected)),
    ]
    .width(Length::Shrink)
    .into()
}

fn foreground(tokens: &Tokens, selected: bool, status: Status) -> Color {
    let colour = match status {
        _ if selected => tokens.foreground,
        Status::Hovered | Status::Pressed => tokens.foreground,
        _ => tokens.muted_foreground,
    };
    if status == Status::Disabled {
        fade(colour, 0.5)
    } else {
        colour
    }
}

/// The style of one tab.
pub fn style(
    tokens: &Tokens,
    variant: Variant,
    selected: bool,
    status: Status,
) -> widget::button::Style {
    let raised = variant == Variant::Pills && selected;
    let background = raised.then(|| {
        if tokens.is_dark {
            mix(tokens.muted, tokens.foreground, 0.12)
        } else {
            tokens.background
        }
    });

    widget::button::Style {
        background: background.map(Background::Color),
        text_color: foreground(tokens, selected, status),
        border: Border {
            radius: radius::MD.into(),
            ..Border::default()
        },
        shadow: if raised {
            Shadow {
                color: fade(Color::BLACK, 0.1),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 3.0,
            }
        } else {
            Shadow::default()
        },
        snap: true,
    }
}

/// The underline below a tab.
pub fn indicator_style(tokens: &Tokens, selected: bool) -> container::Style {
    container::Style {
        background: selected.then_some(Background::Color(tokens.foreground)),
        ..container::Style::default()
    }
}

/// The container around the tab list.
pub fn list_style(tokens: &Tokens, variant: Variant) -> container::Style {
    match variant {
        Variant::Underline => container::Style::default(),
        Variant::Pills => container::Style {
            background: Some(Background::Color(tokens.muted)),
            border: Border {
                radius: radius::LG.into(),
                ..Border::default()
            },
            ..container::Style::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    fn state() -> State<u8> {
        State::new([
            tab(0, "A"),
            tab(1, "B").disabled(true),
            tab(2, "C"),
            tab(3, "D"),
        ])
    }

    #[test]
    fn new_selects_first_enabled_tab() {
        assert_eq!(state().selected(), Some(0));
        let skipped = State::new([tab(0, "A").disabled(true), tab(1, "B")]);
        assert_eq!(skipped.selected(), Some(1));
    }

    #[test]
    fn empty_and_all_disabled_have_no_selection() {
        let mut empty: State<u8> = State::new([]);
        assert_eq!(empty.selected(), None);
        assert_eq!(empty.update(Event::Next), None);
        assert_eq!(empty.update(Event::Previous), None);

        let mut disabled = State::new([tab(0, "A").disabled(true)]);
        assert_eq!(disabled.selected(), None);
        assert_eq!(disabled.update(Event::Next), None);
    }

    #[test]
    fn select_ignores_disabled_unknown_and_current() {
        let mut state = state();
        assert_eq!(state.update(Event::Select(1)), None);
        assert_eq!(state.update(Event::Select(9)), None);
        assert_eq!(state.update(Event::Select(0)), None);
        assert_eq!(state.update(Event::Select(3)), Some(3));
        assert_eq!(state.selected(), Some(3));
    }

    #[test]
    fn next_skips_disabled_and_wraps() {
        let mut state = state();
        assert_eq!(state.update(Event::Next), Some(2));
        assert_eq!(state.update(Event::Next), Some(3));
        assert_eq!(state.update(Event::Next), Some(0));
    }

    #[test]
    fn previous_skips_disabled_and_wraps() {
        let mut state = state();
        assert_eq!(state.update(Event::Previous), Some(3));
        assert_eq!(state.update(Event::Previous), Some(2));
        assert_eq!(state.update(Event::Previous), Some(0));
    }

    #[test]
    fn single_enabled_tab_never_moves() {
        let mut state = State::new([tab(0, "A"), tab(1, "B").disabled(true)]);
        assert_eq!(state.update(Event::Next), None);
        assert_eq!(state.update(Event::Previous), None);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn with_selected_and_set_disabled() {
        let mut state = state().with_selected(2);
        assert_eq!(state.selected(), Some(2));
        state.set_disabled(1, false);
        assert_eq!(state.update(Event::Previous), Some(1));
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "Ctrl+Tab"), Some(Action::Next));
        assert_eq!(press(&keymap, "ArrowRight"), Some(Action::Next));
        assert_eq!(press(&keymap, "Ctrl+Shift+Tab"), Some(Action::Previous));
        assert_eq!(press(&keymap, "ArrowLeft"), Some(Action::Previous));
        assert_eq!(press(&keymap, "Home"), Some(Action::First));
        assert_eq!(press(&keymap, "End"), Some(Action::Last));
        assert_eq!(press(&keymap, "Tab"), None);
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?}");
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&"ArrowRight".parse().unwrap())
            .unbind(&"ArrowLeft".parse().unwrap())
            .bind("Ctrl+PageDown".parse().unwrap(), Action::Next);
        assert_eq!(press(&keymap, "ArrowRight"), None);
        assert_eq!(press(&keymap, "Ctrl+PageDown"), Some(Action::Next));
        assert_eq!(press(&keymap, "Ctrl+Tab"), Some(Action::Next));
    }

    #[test]
    fn actions_map_to_events() {
        let mut state = state().with_selected(2);
        assert_eq!(Action::Next.event(&state), Some(Event::Next));
        assert_eq!(Action::Previous.event(&state), Some(Event::Previous));
        assert_eq!(Action::First.event(&state), Some(Event::Select(0)));
        assert_eq!(Action::Last.event(&state), Some(Event::Select(3)));

        let event = Action::First.event(&state).unwrap();
        assert_eq!(state.update(event), Some(0));

        let disabled = State::new([tab(0, "A").disabled(true)]);
        assert_eq!(Action::First.event(&disabled), None);
        assert_eq!(Action::Last.event(&disabled), None);
    }

    #[test]
    fn first_and_last_skip_disabled_tabs() {
        let state = State::new([
            tab(0, "A").disabled(true),
            tab(1, "B"),
            tab(2, "C"),
            tab(3, "D").disabled(true),
        ]);
        assert_eq!(Action::First.event(&state), Some(Event::Select(1)));
        assert_eq!(Action::Last.event(&state), Some(Event::Select(2)));
    }

    #[test]
    fn selected_tab_is_emphasised_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in Variant::ALL {
                let on = style(&tokens, variant, true, Status::Active);
                let off = style(&tokens, variant, false, Status::Active);
                assert_eq!(on.text_color, tokens.foreground);
                assert_eq!(off.text_color, tokens.muted_foreground);
            }
        }
    }

    #[test]
    fn only_selected_pill_is_raised() {
        let tokens = Tokens::of(&light());
        assert!(
            style(&tokens, Variant::Pills, true, Status::Active)
                .background
                .is_some()
        );
        assert!(
            style(&tokens, Variant::Pills, false, Status::Active)
                .background
                .is_none()
        );
        assert!(
            style(&tokens, Variant::Underline, true, Status::Active)
                .background
                .is_none()
        );
    }

    #[test]
    fn hover_and_disabled_change_the_label_colour() {
        let tokens = Tokens::of(&dark());
        let hovered = style(&tokens, Variant::Underline, false, Status::Hovered);
        let disabled = style(&tokens, Variant::Underline, false, Status::Disabled);
        assert_eq!(hovered.text_color, tokens.foreground);
        assert!(disabled.text_color.a < tokens.muted_foreground.a);
    }

    #[test]
    fn indicator_and_list_styles() {
        let tokens = Tokens::of(&light());
        assert!(indicator_style(&tokens, true).background.is_some());
        assert!(indicator_style(&tokens, false).background.is_none());
        assert!(list_style(&tokens, Variant::Pills).background.is_some());
        assert!(list_style(&tokens, Variant::Underline).background.is_none());
    }
}
