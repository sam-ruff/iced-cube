//! A text field with a list of suggestions that filters as you type.
//!
//! [`State`] holds the options, the query, the highlighted suggestion and
//! the selected value. The app owns it and passes every [`Event`] to
//! [`State::update`], which returns the value when one is chosen.
//!
//! While the field has focus or the list is open, the combobox handles its
//! own keys through a [`Keymap`] of [`Action`]s: arrows move the highlight,
//! Enter chooses and Escape closes. See [`default_keymap`]. The list is
//! drawn in the shared [menu look](crate::overlay::menu) on the
//! [anchored layer](crate::overlay::anchored), so a click outside closes
//! only the list, even inside a dialog or popover.
//!
//! ```no_run
//! use iced::Element;
//! use iced_cube::forms::combobox::{self, combobox};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Fruit(combobox::Event),
//! }
//!
//! struct App {
//!     fruit: combobox::State<&'static str>,
//! }
//!
//! impl App {
//!     fn update(&mut self, message: Message) {
//!         let Message::Fruit(event) = message;
//!         if let Some(fruit) = self.fruit.update(event) {
//!             println!("Chose {fruit}");
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         combobox(&self.fruit)
//!             .placeholder("Search fruit...")
//!             .on_event(Message::Fruit)
//!             .into()
//!     }
//! }
//! ```

use std::fmt::{self, Display};
use std::rc::Rc;

use iced::keyboard::key::Named;
use iced::mouse::ScrollDelta;
use iced::widget::text::LineHeight;
use iced::widget::text_input::{self, Status};
use iced::widget::{self, Column, container, mouse_area, stack};
use iced::{Alignment, Color, Element, Length, Padding, Theme};

use crate::icon::tinted;
use crate::keys::{self, Chord, Keymap};
use crate::overlay::anchored::{Align, Placement, Side, anchored};
use crate::overlay::menu::{self, DISABLED_ICON_OPACITY, Parts, RowStatus, Trailing};
use crate::primitives::input::{self, Size};
use crate::theme::{Tokens, space};

/// How many suggestions show at once unless the state sets its own number.
pub const VISIBLE_ROWS: usize = 6;
/// Shown when no option matches the query, unless the builder sets its own.
pub const DEFAULT_EMPTY: &str = "No results found.";

// Matches the line height `input` uses, so the field is the same height.
const LINE_HEIGHT: f32 = 1.3;

/// Everything that changes a combobox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The query was edited. Opens the list and highlights the first match.
    Input(String),
    Open,
    /// Closes the list and clears the query.
    Close,
    /// Highlights the next match, wrapping round. Opens a closed list.
    Next,
    /// Highlights the previous match, wrapping round. Opens a closed list.
    Previous,
    /// Chooses the highlighted match.
    ActivateHighlighted,
    /// Highlights an option, by its index in [`State::options`].
    Highlight(usize),
    /// Chooses an option, by its index in [`State::options`].
    Activate(usize),
    /// Scrolls the list by a number of rows, down when positive.
    Scroll(i32),
}

/// The options, the query and what is highlighted and selected.
#[derive(Debug, Clone, PartialEq)]
pub struct State<T> {
    options: Vec<T>,
    labels: Vec<String>,
    query: String,
    /// Indices into `options` of the options that match `query`.
    matches: Vec<usize>,
    /// Position in `matches`.
    highlighted: usize,
    /// Position in `matches` of the first visible row.
    offset: usize,
    selected: Option<usize>,
    open: bool,
    rows: usize,
}

impl<T: Display> State<T> {
    /// Creates a closed combobox with nothing selected. Options are
    /// labelled with their `Display` text.
    pub fn new(options: impl IntoIterator<Item = T>) -> Self {
        let options: Vec<T> = options.into_iter().collect();
        let labels = options
            .iter()
            .map(|option| option.to_string().to_lowercase())
            .collect();
        let matches = (0..options.len()).collect();
        Self {
            options,
            labels,
            query: String::new(),
            matches,
            highlighted: 0,
            offset: 0,
            selected: None,
            open: false,
            rows: VISIBLE_ROWS,
        }
    }
}

impl<T: PartialEq> State<T> {
    /// Starts with `value` selected, when it is one of the options.
    pub fn with_selected(mut self, value: &T) -> Self {
        self.selected = self.options.iter().position(|option| option == value);
        self
    }
}

impl<T> State<T> {
    /// Sets how many suggestions show at once. At least one does.
    pub fn with_visible_rows(mut self, rows: usize) -> Self {
        self.rows = rows.max(1);
        self.reveal();
        self
    }

    pub fn options(&self) -> &[T] {
        &self.options
    }

    pub fn selected(&self) -> Option<&T> {
        self.selected.and_then(|index| self.options.get(index))
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The options that match the query, in their original order.
    pub fn matches(&self) -> impl Iterator<Item = &T> {
        self.matches
            .iter()
            .filter_map(|&index| self.options.get(index))
    }

    /// The highlighted option, if any option matches.
    pub fn highlighted(&self) -> Option<&T> {
        self.matches
            .get(self.highlighted)
            .and_then(|&index| self.options.get(index))
    }

    /// The matches in view, with their index in [`State::options`].
    pub fn visible(&self) -> impl Iterator<Item = (usize, &T)> {
        self.matches
            .iter()
            .skip(self.offset)
            .take(self.rows)
            .filter_map(|&index| self.options.get(index).map(|option| (index, option)))
    }

    fn is_highlighted(&self, index: usize) -> bool {
        self.matches.get(self.highlighted) == Some(&index)
    }

    /// Applies an event and returns the chosen value, if one was chosen.
    pub fn update(&mut self, event: Event) -> Option<T>
    where
        T: Clone,
    {
        match event {
            Event::Input(query) => {
                self.query = query;
                self.open = true;
                self.filter();
                self.highlight(0);
            }
            Event::Open => self.open(),
            Event::Close => self.close(),
            Event::Next | Event::Previous if !self.open => self.open(),
            Event::Next => self.step(1),
            Event::Previous => self.step(-1),
            Event::ActivateHighlighted => {
                if !self.open {
                    return None;
                }
                let index = *self.matches.get(self.highlighted)?;
                return self.choose(index);
            }
            Event::Activate(index) => return self.choose(index),
            Event::Highlight(index) => {
                if let Some(position) = self.matches.iter().position(|&i| i == index) {
                    self.highlight(position);
                }
            }
            Event::Scroll(rows) => {
                let last = self.matches.len().saturating_sub(self.rows);
                let offset = (self.offset as i64).saturating_add(rows.into());
                self.offset = offset.clamp(0, last as i64) as usize;
            }
        }
        None
    }

    fn filter(&mut self) {
        let query = self.query.trim().to_lowercase();
        self.matches = self
            .labels
            .iter()
            .enumerate()
            .filter(|(_, label)| label.contains(&query))
            .map(|(index, _)| index)
            .collect();
    }

    fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.filter();
        let selected = self
            .selected
            .and_then(|index| self.matches.iter().position(|&i| i == index));
        self.offset = 0;
        self.highlight(selected.unwrap_or(0));
    }

    fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.filter();
        self.offset = 0;
    }

    fn choose(&mut self, index: usize) -> Option<T>
    where
        T: Clone,
    {
        let value = self.options.get(index)?.clone();
        self.selected = Some(index);
        self.close();
        Some(value)
    }

    fn step(&mut self, delta: isize) {
        let len = self.matches.len() as isize;
        if len == 0 {
            return;
        }
        let next = (self.highlighted as isize + delta).rem_euclid(len);
        self.highlight(next as usize);
    }

    fn highlight(&mut self, position: usize) {
        self.highlighted = position.min(self.matches.len().saturating_sub(1));
        self.reveal();
    }

    /// Moves the window so the highlighted row is in view.
    fn reveal(&mut self) {
        if self.highlighted < self.offset {
            self.offset = self.highlighted;
        } else if self.highlighted >= self.offset + self.rows {
            self.offset = self.highlighted + 1 - self.rows;
        }
        self.offset = self
            .offset
            .min(self.matches.len().saturating_sub(self.rows));
    }
}

/// What a combobox keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
    Activate,
    Close,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it would
    /// do nothing, so the key can reach the text field instead.
    pub fn event<T>(self, state: &State<T>) -> Option<Event> {
        match self {
            Action::Next => Some(Event::Next),
            Action::Previous => Some(Event::Previous),
            Action::Activate => state.open.then_some(Event::ActivateHighlighted),
            Action::Close => state.open.then_some(Event::Close),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Next,
        Action::Previous,
        Action::Activate,
        Action::Close,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Next => "Next",
            Action::Previous => "Previous",
            Action::Activate => "Activate",
            Action::Close => "Close",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Highlights the next suggestion, opening the list if it is closed.",
            Action::Previous => {
                "Highlights the previous suggestion, opening the list if it is closed."
            }
            Action::Activate => "Chooses the highlighted suggestion and closes the list.",
            Action::Close => "Closes the list and clears the query.",
        }
    }
}

/// The default combobox shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowDown` | [`Action::Next`] |
/// | `ArrowUp` | [`Action::Previous`] |
/// | `Enter` | [`Action::Activate`] |
/// | `Escape` | [`Action::Close`] |
///
/// The combobox handles these itself while its field has focus or its list
/// is open, so the app does not need to subscribe to key presses. Pass a
/// changed keymap with [`Combobox::keymap`].
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowDown), Action::Next)
        .bind(Chord::named(Named::ArrowUp), Action::Previous)
        .bind(Chord::named(Named::Enter), Action::Activate)
        .bind(Chord::named(Named::Escape), Action::Close)
}

/// A combobox builder. Convert it into an [`Element`] to render.
///
/// A combobox without [`on_event`](Combobox::on_event) is rendered disabled.
pub struct Combobox<'a, T, Message> {
    state: &'a State<T>,
    placeholder: String,
    empty: String,
    width: Length,
    size: Size,
    id: Option<widget::Id>,
    keymap: Keymap<Action>,
    on_event: Option<Box<dyn Fn(Event) -> Message + 'a>>,
}

/// Renders the field and, while it is open, the suggestions of `state`.
pub fn combobox<'a, T, Message>(state: &'a State<T>) -> Combobox<'a, T, Message> {
    Combobox {
        state,
        placeholder: String::new(),
        empty: DEFAULT_EMPTY.to_owned(),
        width: Length::Fixed(240.0),
        size: Size::default(),
        id: None,
        keymap: default_keymap(),
        on_event: None,
    }
}

impl<T: fmt::Debug, Message> fmt::Debug for Combobox<'_, T, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Combobox")
            .field("state", self.state)
            .field("placeholder", &self.placeholder)
            .field("empty", &self.empty)
            .field("width", &self.width)
            .field("size", &self.size)
            .field("enabled", &self.on_event.is_some())
            .finish_non_exhaustive()
    }
}

impl<'a, T, Message> Combobox<'a, T, Message> {
    /// Text shown while the field is empty and nothing is selected.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Text shown in the list when no option matches the query.
    pub fn empty(mut self, empty: impl Into<String>) -> Self {
        self.empty = empty.into();
        self
    }

    /// Overrides the width. Defaults to 240 pixels.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// The field's height, padding and text size, shared with
    /// [`input`](crate::primitives::input). Defaults to [`Size::Md`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Sets the widget id of the text field, to focus it or find it in tests.
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Replaces the [`default_keymap`].
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    /// Maps combobox events to your message. Without it the combobox is
    /// disabled.
    pub fn on_event(mut self, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_event.is_some()
    }
}

impl<'a, T, Message> From<Combobox<'a, T, Message>> for Element<'a, Message>
where
    T: Display + 'a,
    Message: Clone + 'a,
{
    fn from(combobox: Combobox<'a, T, Message>) -> Self {
        let Combobox {
            state,
            placeholder,
            empty,
            width,
            size,
            id,
            keymap,
            on_event,
        } = combobox;
        let selected = state.selected().map(ToString::to_string);

        let Some(on_event) = on_event else {
            let value = selected.unwrap_or_default();
            return container(field(&placeholder, &value, size, id, None, false))
                .width(width)
                .into();
        };
        let on_event: Rc<dyn Fn(Event) -> Message + 'a> = Rc::from(on_event);

        let open = state.open;
        let (value, hint) = if open {
            (state.query.clone(), selected.unwrap_or(placeholder))
        } else {
            (selected.unwrap_or_default(), placeholder)
        };

        let list = open.then(|| suggestions(state, &empty, &*on_event));
        let on_anchor_press = (!open).then(|| on_event(Event::Open));
        let on_dismiss = open.then(|| on_event(Event::Close));
        let on_input: OnInput<'a, Message> = {
            let on_event = Rc::clone(&on_event);
            Box::new(move |query| on_event(Event::Input(query)))
        };
        let field = container(field(&hint, &value, size, id, Some(on_input), open)).width(width);

        anchored(field)
            .content(list)
            .placement(Placement::new(Side::Bottom, Align::Start))
            .match_width(true)
            .dismiss_keys([])
            .dismiss_on_blur(true)
            .on_dismiss_maybe(on_dismiss)
            .on_anchor_press_maybe(on_anchor_press)
            .on_key(move |key| {
                keymap
                    .resolve_event(key)
                    .and_then(|action| action.event(state))
                    .map(&*on_event)
            })
            .into()
    }
}

type OnInput<'a, Message> = Box<dyn Fn(String) -> Message + 'a>;

fn field<'a, Message: Clone + 'a>(
    placeholder: &str,
    value: &str,
    size: Size,
    id: Option<widget::Id>,
    on_input: Option<OnInput<'a, Message>>,
    open: bool,
) -> Element<'a, Message> {
    let enabled = on_input.is_some();
    let metrics = size.metrics();
    let mut padding = metrics.padding(false);
    padding.right += metrics.icon + space::SM;

    let mut input = widget::text_input(placeholder, value)
        .size(metrics.text)
        .line_height(LineHeight::Relative(LINE_HEIGHT))
        .padding(padding)
        .width(Length::Fill)
        .style(move |theme, status| style(&Tokens::of(theme), status, open));
    if let Some(id) = id {
        input = input.id(id);
    }
    if let Some(on_input) = on_input {
        input = input.on_input(on_input);
    }

    let chevron = tinted(crate::lucide!(ChevronsUpDown), metrics.icon, None)
        .style(move |theme: &Theme, _| widget::svg::Style {
            color: Some(chevron_colour(&Tokens::of(theme))),
        })
        .opacity(chevron_opacity(enabled));

    stack![
        input,
        container(chevron)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::End)
            .align_y(Alignment::Center)
            .padding(Padding::ZERO.right(metrics.padding_x)),
    ]
    .into()
}

fn suggestions<'a, T: Display, Message: Clone + 'a>(
    state: &'a State<T>,
    empty: &str,
    on_event: &dyn Fn(Event) -> Message,
) -> Element<'a, Message> {
    let rows: Vec<Element<'a, Message>> = state
        .visible()
        .map(|(index, option)| {
            let status = if state.is_highlighted(index) {
                RowStatus::Highlighted
            } else {
                RowStatus::Idle
            };
            let label = option.to_string();
            let trailing = if state.selected == Some(index) {
                Trailing::Check
            } else {
                Trailing::None
            };
            let parts = Parts {
                trailing,
                ..Parts::label(&label)
            };
            let messages = (
                on_event(Event::Highlight(index)),
                on_event(Event::Activate(index)),
            );
            menu::item(parts, status, false, Some(messages))
        })
        .collect();

    let body: Element<'a, Message> = if rows.is_empty() {
        menu::empty(empty)
    } else {
        Column::from_vec(rows).width(Length::Fill).into()
    };

    let down = on_event(Event::Scroll(1));
    let up = on_event(Event::Scroll(-1));
    mouse_area(menu::surface(body).width(Length::Fill))
        .on_scroll(move |delta| match delta {
            ScrollDelta::Lines { y, .. } | ScrollDelta::Pixels { y, .. } if y < 0.0 => down.clone(),
            _ => up.clone(),
        })
        .into()
}

/// The field style: the input style, drawn focused while the list is open.
pub fn style(tokens: &Tokens, status: Status, open: bool) -> text_input::Style {
    let status = match status {
        Status::Active if open => Status::Focused { is_hovered: false },
        Status::Hovered if open => Status::Focused { is_hovered: true },
        status => status,
    };
    input::style(tokens, status, false)
}

/// The colour of the chevron at the end of the field. It stays opaque;
/// [`chevron_opacity`] fades it when the combobox is disabled.
pub fn chevron_colour(tokens: &Tokens) -> Color {
    tokens.muted_foreground
}

/// The chevron's svg opacity. iced ignores the alpha of an svg tint, so a
/// disabled chevron fades through its opacity instead.
pub fn chevron_opacity(enabled: bool) -> f32 {
    if enabled { 1.0 } else { DISABLED_ICON_OPACITY }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    const FRUITS: [&str; 5] = ["Apple", "Banana", "Blueberry", "Cherry", "Grape"];

    fn fruits() -> State<&'static str> {
        State::new(FRUITS)
    }

    fn matches(state: &State<&'static str>) -> Vec<&'static str> {
        state.matches().copied().collect()
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn new_state_is_closed_with_every_option() {
        let state = fruits();
        assert!(!state.is_open());
        assert!(state.selected().is_none());
        assert_eq!(state.query(), "");
        assert_eq!(matches(&state), FRUITS);
        assert_eq!(state.visible().count(), 5);
    }

    #[test]
    fn with_selected_ignores_unknown_values() {
        assert_eq!(
            fruits().with_selected(&"Cherry").selected(),
            Some(&"Cherry")
        );
        assert!(fruits().with_selected(&"Kiwi").selected().is_none());
    }

    #[test]
    fn typing_filters_case_insensitively_and_keeps_order() {
        let mut state = fruits();
        assert!(state.update(Event::Input("B".into())).is_none());
        assert!(state.is_open());
        assert_eq!(matches(&state), ["Banana", "Blueberry"]);
        assert_eq!(state.highlighted(), Some(&"Banana"));

        let _ = state.update(Event::Input("  ERR ".into()));
        assert_eq!(matches(&state), ["Blueberry", "Cherry"]);
    }

    #[test]
    fn no_match_leaves_nothing_highlighted() {
        let mut state = fruits();
        let _ = state.update(Event::Input("kiwi".into()));
        assert!(state.matches().next().is_none());
        assert!(state.highlighted().is_none());
        assert!(state.update(Event::ActivateHighlighted).is_none());
        let _ = state.update(Event::Next);
        assert!(state.highlighted().is_none());
    }

    #[test]
    fn arrows_open_then_wrap_round() {
        let mut state = fruits();
        let _ = state.update(Event::Next);
        assert!(state.is_open());
        assert_eq!(state.highlighted(), Some(&"Apple"));

        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(&"Grape"));
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(&"Apple"));
    }

    #[test]
    fn opening_highlights_the_selection() {
        let mut state = fruits().with_selected(&"Cherry");
        let _ = state.update(Event::Open);
        assert_eq!(state.highlighted(), Some(&"Cherry"));
    }

    #[test]
    fn confirm_chooses_closes_and_clears_the_query() {
        let mut state = fruits();
        let _ = state.update(Event::Input("an".into()));
        assert_eq!(state.update(Event::ActivateHighlighted), Some("Banana"));
        assert!(!state.is_open());
        assert_eq!(state.query(), "");
        assert_eq!(state.selected(), Some(&"Banana"));
        assert_eq!(matches(&state).len(), FRUITS.len());
    }

    #[test]
    fn confirm_does_nothing_while_closed() {
        let mut state = fruits();
        assert!(state.update(Event::ActivateHighlighted).is_none());
        assert!(state.selected().is_none());
    }

    #[test]
    fn pick_and_highlight_use_option_indices() {
        let mut state = fruits();
        let _ = state.update(Event::Input("berry".into()));
        let _ = state.update(Event::Highlight(2));
        assert_eq!(state.highlighted(), Some(&"Blueberry"));
        // Highlighting an option that does not match is ignored.
        let _ = state.update(Event::Highlight(0));
        assert_eq!(state.highlighted(), Some(&"Blueberry"));

        assert_eq!(state.update(Event::Activate(3)), Some("Cherry"));
        assert!(state.update(Event::Activate(99)).is_none());
        assert_eq!(state.selected(), Some(&"Cherry"));
    }

    #[test]
    fn close_keeps_the_selection() {
        let mut state = fruits().with_selected(&"Apple");
        let _ = state.update(Event::Input("gr".into()));
        let _ = state.update(Event::Close);
        assert!(!state.is_open());
        assert_eq!(state.query(), "");
        assert_eq!(state.selected(), Some(&"Apple"));
    }

    #[test]
    fn the_window_follows_the_highlight() {
        let mut state = fruits().with_visible_rows(2);
        let _ = state.update(Event::Open);
        let visible = |state: &State<&'static str>| -> Vec<&str> {
            state.visible().map(|(_, fruit)| *fruit).collect()
        };
        assert_eq!(visible(&state), ["Apple", "Banana"]);

        let _ = state.update(Event::Next);
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(&"Blueberry"));
        assert_eq!(visible(&state), ["Banana", "Blueberry"]);

        let _ = state.update(Event::Previous);
        let _ = state.update(Event::Previous);
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(&"Grape"));
        assert_eq!(visible(&state), ["Cherry", "Grape"]);
    }

    #[test]
    fn scrolling_stays_within_the_matches() {
        let mut state = fruits().with_visible_rows(2);
        let _ = state.update(Event::Open);
        let _ = state.update(Event::Scroll(10));
        assert_eq!(state.visible().map(|(i, _)| i).collect::<Vec<_>>(), [3, 4]);
        let _ = state.update(Event::Scroll(-10));
        assert_eq!(state.visible().map(|(i, _)| i).collect::<Vec<_>>(), [0, 1]);
        assert_eq!(State::<&str>::new([]).with_visible_rows(0).rows, 1);
    }

    #[test]
    fn empty_options_are_safe() {
        let mut state = State::<&str>::new([]);
        let _ = state.update(Event::Next);
        let _ = state.update(Event::Previous);
        let _ = state.update(Event::Scroll(3));
        assert!(state.update(Event::ActivateHighlighted).is_none());
        assert!(state.highlighted().is_none());
    }

    #[test]
    fn default_keymap_and_overrides() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "ArrowDown"), Some(Action::Next));
        assert_eq!(press(&keymap, "ArrowUp"), Some(Action::Previous));
        assert_eq!(press(&keymap, "Enter"), Some(Action::Activate));
        assert_eq!(press(&keymap, "Escape"), Some(Action::Close));

        let custom = keymap
            .bind("Ctrl+N".parse().unwrap(), Action::Next)
            .unbind(&Chord::named(Named::Escape));
        assert_eq!(press(&custom, "ctrl+n"), Some(Action::Next));
        assert_eq!(press(&custom, "Escape"), None);
    }

    #[test]
    fn actions_map_to_events_only_when_they_apply() {
        let mut state = fruits();
        assert_eq!(Action::Next.event(&state), Some(Event::Next));
        assert_eq!(Action::Previous.event(&state), Some(Event::Previous));
        assert_eq!(Action::Activate.event(&state), None);
        assert_eq!(Action::Close.event(&state), None);

        let _ = state.update(Event::Open);
        assert_eq!(
            Action::Activate.event(&state),
            Some(Event::ActivateHighlighted)
        );
        assert_eq!(Action::Close.event(&state), Some(Event::Close));
    }

    #[test]
    fn default_builder_is_disabled_with_the_default_empty_text() {
        let state = fruits();
        let c: Combobox<'_, &str, Event> = combobox(&state);
        assert!(!c.is_enabled());
        assert_eq!(c.empty, DEFAULT_EMPTY);
        assert_eq!(c.width, Length::Fixed(240.0));
        assert_eq!(c.size, Size::Md);
        assert_eq!(c.keymap, default_keymap());
        let c = c
            .placeholder("Pick")
            .empty("None")
            .size(Size::Sm)
            .on_event(|e| e);
        assert_eq!(c.size, Size::Sm);
        assert!(c.is_enabled());
        assert_eq!((c.placeholder.as_str(), c.empty.as_str()), ("Pick", "None"));
    }

    const STATES: [Status; 5] = [
        Status::Active,
        Status::Hovered,
        Status::Focused { is_hovered: false },
        Status::Focused { is_hovered: true },
        Status::Disabled,
    ];

    #[test]
    fn open_field_draws_the_focused_border() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let focused = input::style(&tokens, Status::Focused { is_hovered: false }, false);
            for status in STATES {
                let closed = style(&tokens, status, false);
                assert_eq!(closed, input::style(&tokens, status, false), "{status:?}");
                let open = style(&tokens, status, true);
                if status != Status::Disabled {
                    assert_eq!(open.border.color, focused.border.color, "{status:?}");
                }
            }
        }
    }

    #[test]
    fn disabled_chevron_fades_through_its_opacity() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(chevron_colour(&tokens), tokens.muted_foreground);
            assert_eq!(chevron_colour(&tokens).a, 1.0, "svg tints ignore alpha");
        }
        assert_eq!(chevron_opacity(true), 1.0);
        assert!(chevron_opacity(false) < 1.0);
    }
}
