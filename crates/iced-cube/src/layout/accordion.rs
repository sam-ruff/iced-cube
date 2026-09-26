//! Sections that expand and collapse under their headers.
//!
//! [`State`] tracks which sections are open. In [`Mode::Single`] opening a
//! section closes the others; in [`Mode::Multiple`] each one is independent.

use iced::keyboard::key::Named;
use iced::widget::{self, button::Status, column, container, row, rule, text};
use iced::{Alignment, Color, Element, Length, Theme};

use crate::icon::tinted;
use crate::keys::{self, Chord, Keymap};
use crate::theme::{Tokens, fade, space, text_size};

/// How many sections may be open at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    /// At most one section is open.
    #[default]
    Single,
    /// Any number of sections are open.
    Multiple,
}

impl Mode {
    pub const ALL: [Mode; 2] = [Mode::Single, Mode::Multiple];
}

/// Opens or closes a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event<Id> {
    Toggle(Id),
    Open(Id),
    Close(Id),
}

/// What changed after an [`Event`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output<Id> {
    Opened(Id),
    Closed(Id),
}

/// Which sections are open.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct State<Id> {
    mode: Mode,
    open: Vec<Id>,
}

impl<Id: Copy + PartialEq> State<Id> {
    /// Starts with every section closed.
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            open: Vec::new(),
        }
    }

    /// Starts with `id` open.
    pub fn with_open(mut self, id: Id) -> Self {
        let _ = self.update(Event::Open(id));
        self
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Switches mode. Going to single mode keeps only the first open section.
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        if mode == Mode::Single {
            self.open.truncate(1);
        }
    }

    pub fn is_open(&self, id: Id) -> bool {
        self.open.contains(&id)
    }

    /// The open sections, in the order they were opened.
    pub fn open(&self) -> &[Id] {
        &self.open
    }

    pub fn update(&mut self, event: Event<Id>) -> Option<Output<Id>> {
        match event {
            Event::Toggle(id) if self.is_open(id) => self.update(Event::Close(id)),
            Event::Toggle(id) | Event::Open(id) => {
                if self.is_open(id) {
                    return None;
                }
                if self.mode == Mode::Single {
                    self.open.clear();
                }
                self.open.push(id);
                Some(Output::Opened(id))
            }
            Event::Close(id) => {
                let index = self.open.iter().position(|open| *open == id)?;
                let _ = self.open.remove(index);
                Some(Output::Closed(id))
            }
        }
    }
}

/// What an accordion keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Moves the most recently opened section to the next one, wrapping.
    Next,
    /// Moves the most recently opened section to the previous one, wrapping.
    Previous,
    ExpandAll,
    CollapseAll,
}

impl Action {
    /// The events this action sends to `state`, given every section id in
    /// display order. `ExpandAll` does nothing in [`Mode::Single`].
    pub fn events<Id: Copy + PartialEq>(
        self,
        state: &State<Id>,
        sections: &[Id],
    ) -> Vec<Event<Id>> {
        match self {
            Action::Next => step(state, sections, true),
            Action::Previous => step(state, sections, false),
            Action::ExpandAll if state.mode == Mode::Single => Vec::new(),
            Action::ExpandAll => sections
                .iter()
                .filter(|id| !state.is_open(**id))
                .map(|id| Event::Open(*id))
                .collect(),
            Action::CollapseAll => state.open.iter().map(|id| Event::Close(*id)).collect(),
        }
    }
}

fn step<Id: Copy + PartialEq>(state: &State<Id>, sections: &[Id], forward: bool) -> Vec<Event<Id>> {
    let len = sections.len();
    if len == 0 {
        return Vec::new();
    }
    let current = state.open.last().copied();
    let index = current.and_then(|id| sections.iter().position(|section| *section == id));
    let target = match (index, forward) {
        (Some(index), true) => (index + 1) % len,
        (Some(index), false) => (index + len - 1) % len,
        (None, true) => 0,
        (None, false) => len - 1,
    };
    let target = sections[target];
    if current == Some(target) {
        return Vec::new();
    }
    current
        .map(Event::Close)
        .into_iter()
        .chain([Event::Open(target)])
        .collect()
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Next,
        Action::Previous,
        Action::ExpandAll,
        Action::CollapseAll,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Next => "Next",
            Action::Previous => "Previous",
            Action::ExpandAll => "ExpandAll",
            Action::CollapseAll => "CollapseAll",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => {
                "Opens the next section in place of the current one, wrapping at the end."
            }
            Action::Previous => {
                "Opens the previous section in place of the current one, wrapping at the start."
            }
            Action::ExpandAll => "Opens every section. Only applies when several may be open.",
            Action::CollapseAll => "Closes every open section.",
        }
    }
}

/// The default accordion shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowDown` | [`Action::Next`] |
/// | `ArrowUp` | [`Action::Previous`] |
/// | `Ctrl+Shift+ArrowDown` | [`Action::ExpandAll`] |
/// | `Ctrl+Shift+ArrowUp` | [`Action::CollapseAll`] |
///
/// Shortcuts are app-wide, so an app with scrolling content or other
/// arrow-driven components may want to unbind the plain arrows.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowDown), Action::Next)
        .bind(Chord::named(Named::ArrowUp), Action::Previous)
        .bind(
            Chord::named(Named::ArrowDown).ctrl().shift(),
            Action::ExpandAll,
        )
        .bind(
            Chord::named(Named::ArrowUp).ctrl().shift(),
            Action::CollapseAll,
        )
}

/// An accordion builder. Convert it into an [`Element`] to render.
pub struct Accordion<'a, Id, Message> {
    state: &'a State<Id>,
    items: Vec<(Id, text::Fragment<'a>, Element<'a, Message>)>,
    on_event: Option<Box<dyn Fn(Event<Id>) -> Message + 'a>>,
}

/// Renders sections whose open state comes from `state`. Without
/// [`on_event`](Accordion::on_event) the headers render disabled.
pub fn accordion<Id, Message>(state: &State<Id>) -> Accordion<'_, Id, Message> {
    Accordion {
        state,
        items: Vec::new(),
        on_event: None,
    }
}

impl<Id: std::fmt::Debug, Message> std::fmt::Debug for Accordion<'_, Id, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Accordion")
            .field("state", self.state)
            .field("items", &self.items.len())
            .finish_non_exhaustive()
    }
}

impl<'a, Id, Message> Accordion<'a, Id, Message> {
    /// Adds a section with a header and the content shown while open.
    pub fn item(
        mut self,
        id: Id,
        title: impl text::IntoFragment<'a>,
        content: impl Into<Element<'a, Message>>,
    ) -> Self {
        self.items.push((id, title.into_fragment(), content.into()));
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Id>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

impl<'a, Id, Message> From<Accordion<'a, Id, Message>> for Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(accordion: Accordion<'a, Id, Message>) -> Self {
        let Accordion {
            state,
            items,
            on_event,
        } = accordion;

        column(items.into_iter().map(|(id, title, content)| {
            let open = state.is_open(id);
            let message = on_event
                .as_ref()
                .map(|on_event| on_event(Event::Toggle(id)));
            section(title, content, open, message)
        }))
        .width(Length::Fill)
        .into()
    }
}

fn section<'a, Message: Clone + 'a>(
    title: text::Fragment<'a>,
    content: Element<'a, Message>,
    open: bool,
    message: Option<Message>,
) -> Element<'a, Message> {
    let enabled = message.is_some();
    let chevron = if open {
        crate::lucide!(ChevronUp)
    } else {
        crate::lucide!(ChevronDown)
    };

    let header = widget::button(
        row![
            text(title).size(text_size::SM).width(Length::Fill),
            tinted(chevron, 16.0, None).style(move |theme: &Theme, _| widget::svg::Style {
                color: Some(chevron_colour(&Tokens::of(theme), enabled)),
            }),
        ]
        .spacing(space::MD)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([space::MD, 0.0])
    .on_press_maybe(message)
    .style(|theme, status| style(&Tokens::of(theme), status));

    let mut body = column![header].width(Length::Fill);
    if open {
        body = body.push(
            container(content)
                .padding(iced::Padding::ZERO.bottom(space::LG))
                .width(Length::Fill),
        );
    }
    body.push(rule::horizontal(1).style(|theme: &Theme| rule::Style {
        color: Tokens::of(theme).border,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }))
    .into()
}

fn chevron_colour(tokens: &Tokens, enabled: bool) -> Color {
    if enabled {
        tokens.muted_foreground
    } else {
        fade(tokens.muted_foreground, 0.5)
    }
}

/// The style of a section header.
pub fn style(tokens: &Tokens, status: Status) -> widget::button::Style {
    let text_color = match status {
        Status::Active => tokens.foreground,
        Status::Hovered | Status::Pressed => tokens.muted_foreground,
        Status::Disabled => fade(tokens.foreground, 0.5),
    };
    widget::button::Style {
        background: None,
        text_color,
        ..widget::button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn starts_closed_in_single_mode_by_default() {
        let state: State<u8> = State::default();
        assert_eq!(state.mode(), Mode::Single);
        assert!(state.open().is_empty());
    }

    fn apply(state: &mut State<u8>, action: Action, sections: &[u8]) {
        for event in action.events(state, sections) {
            let _ = state.update(event);
        }
    }

    #[test]
    fn next_and_previous_move_the_open_section_and_wrap() {
        let sections = [1, 2, 3];
        let mut state = State::new(Mode::Single);
        apply(&mut state, Action::Next, &sections);
        assert_eq!(state.open(), [1]);
        apply(&mut state, Action::Next, &sections);
        assert_eq!(state.open(), [2]);
        apply(&mut state, Action::Previous, &sections);
        apply(&mut state, Action::Previous, &sections);
        assert_eq!(state.open(), [3]);

        let mut closed = State::new(Mode::Single);
        apply(&mut closed, Action::Previous, &sections);
        assert_eq!(closed.open(), [3]);
    }

    #[test]
    fn next_in_multiple_mode_moves_only_the_latest_section() {
        let mut state = State::new(Mode::Multiple).with_open(1).with_open(2);
        apply(&mut state, Action::Next, &[1, 2, 3]);
        assert_eq!(state.open(), [1, 3]);
    }

    #[test]
    fn next_with_one_or_no_sections() {
        let state = State::new(Mode::Single).with_open(1);
        assert!(Action::Next.events(&state, &[1]).is_empty());
        assert!(Action::Next.events(&state, &[]).is_empty());
    }

    #[test]
    fn expand_and_collapse_all() {
        let sections = [1, 2, 3];
        let mut state = State::new(Mode::Multiple).with_open(2);
        assert_eq!(
            Action::ExpandAll.events(&state, &sections),
            [Event::Open(1), Event::Open(3)]
        );
        apply(&mut state, Action::ExpandAll, &sections);
        assert_eq!(state.open(), [2, 1, 3]);
        apply(&mut state, Action::CollapseAll, &sections);
        assert!(state.open().is_empty());

        let single = State::new(Mode::Single).with_open(1);
        assert!(Action::ExpandAll.events(&single, &sections).is_empty());
        assert_eq!(
            Action::CollapseAll.events(&single, &sections),
            [Event::Close(1)]
        );
    }

    #[test]
    fn default_keymap_and_overrides() {
        let keymap = default_keymap();
        let resolve = |keymap: &Keymap<Action>, chord: &str| {
            let chord: Chord = chord.parse().unwrap();
            keymap.resolve(chord.key(), chord.modifiers())
        };
        assert_eq!(resolve(&keymap, "ArrowDown"), Some(Action::Next));
        assert_eq!(resolve(&keymap, "ArrowUp"), Some(Action::Previous));
        assert_eq!(
            resolve(&keymap, "Ctrl+Shift+ArrowDown"),
            Some(Action::ExpandAll)
        );
        assert_eq!(
            resolve(&keymap, "Ctrl+Shift+ArrowUp"),
            Some(Action::CollapseAll)
        );
        let custom = keymap
            .unbind_action(&Action::Next)
            .bind("N".parse().unwrap(), Action::Next);
        assert_eq!(resolve(&custom, "ArrowDown"), None);
        assert_eq!(resolve(&custom, "n"), Some(Action::Next));
    }

    #[test]
    fn single_mode_keeps_one_section_open() {
        let mut state = State::new(Mode::Single);
        assert_eq!(state.update(Event::Toggle(1)), Some(Output::Opened(1)));
        assert_eq!(state.update(Event::Toggle(2)), Some(Output::Opened(2)));
        assert_eq!(state.open(), [2]);
        assert_eq!(state.update(Event::Toggle(2)), Some(Output::Closed(2)));
        assert!(state.open().is_empty());
    }

    #[test]
    fn multiple_mode_opens_sections_independently() {
        let mut state = State::new(Mode::Multiple);
        let _ = state.update(Event::Toggle(1));
        let _ = state.update(Event::Toggle(2));
        assert!(state.is_open(1) && state.is_open(2));
        let _ = state.update(Event::Toggle(1));
        assert_eq!(state.open(), [2]);
    }

    #[test]
    fn open_and_close_are_idempotent() {
        let mut state = State::new(Mode::Multiple).with_open(1);
        assert_eq!(state.update(Event::Open(1)), None);
        assert_eq!(state.update(Event::Close(3)), None);
        assert_eq!(state.update(Event::Close(1)), Some(Output::Closed(1)));
        assert_eq!(state.update(Event::Close(1)), None);
    }

    #[test]
    fn switching_to_single_keeps_the_first_open_section() {
        let mut state = State::new(Mode::Multiple).with_open(3).with_open(1);
        state.set_mode(Mode::Single);
        assert_eq!(state.open(), [3]);
        let _ = state.update(Event::Open(1));
        assert_eq!(state.open(), [1]);
    }

    #[test]
    fn header_style_in_every_status_and_theme() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(style(&tokens, Status::Active).text_color, tokens.foreground);
            assert_eq!(
                style(&tokens, Status::Hovered).text_color,
                tokens.muted_foreground
            );
            assert_eq!(
                style(&tokens, Status::Pressed).text_color,
                tokens.muted_foreground
            );
            assert!(style(&tokens, Status::Disabled).text_color.a < 1.0);
            assert!(style(&tokens, Status::Active).background.is_none());
        }
    }

    #[test]
    fn chevron_fades_when_disabled() {
        let tokens = Tokens::of(&light());
        assert!(chevron_colour(&tokens, false).a < chevron_colour(&tokens, true).a);
    }
}
