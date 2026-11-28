//! A searchable field that collects several values, each shown as a
//! removable [chip](mod@crate::feedback::chip) above the field.
//!
//! It works like the [combobox](mod@crate::forms::combobox) and shares its
//! field, list and filtering, with these differences:
//!
//! - Choosing a suggestion adds it and keeps the list open, with the query
//!   cleared so the next name can be typed straight away.
//! - Chosen suggestions stay in the list with a tick, in their usual place,
//!   so the list does not jump. Choosing a ticked suggestion removes it.
//! - The chosen values sit above the field as chips, in the order they were
//!   chosen, and wrap onto new lines when the field is narrow. A chip's x
//!   removes it, and so does Backspace in an empty field, starting with the
//!   last chip.
//!
//! [`State`] holds the options, the query and the chosen values. The app
//! owns it and passes every [`Event`] to [`State::update`], which returns
//! an [`Output`] when a value is added or removed.
//!
//! ```no_run
//! use iced::Element;
//! use iced_cube::forms::multi_combobox::{self, Output, multi_combobox};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Guests(multi_combobox::Event),
//! }
//!
//! struct App {
//!     guests: multi_combobox::State<&'static str>,
//! }
//!
//! impl App {
//!     fn update(&mut self, message: Message) {
//!         let Message::Guests(event) = message;
//!         match self.guests.update(event) {
//!             Some(Output::Added(name)) => println!("Invited {name}"),
//!             Some(Output::Removed(name)) => println!("Uninvited {name}"),
//!             None => {}
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         multi_combobox(&self.guests)
//!             .placeholder("Add guests...")
//!             .on_event(Message::Guests)
//!             .into()
//!     }
//! }
//! ```

use std::fmt::{self, Display};
use std::rc::Rc;

use iced::keyboard::key::Named;
use iced::widget::{self, column, container, row};
use iced::{Element, Length};

use crate::feedback::chip::{self, chip};
use crate::forms::combobox::{DEFAULT_EMPTY, Editing, ListEvent, Suggestions, field, suggestions};
use crate::keys::{self, Chord, Keymap};
use crate::overlay::anchored::{Align, Placement, Side, anchored};
use crate::primitives::input::{Shortcuts, Size};

pub use crate::forms::combobox::VISIBLE_ROWS;

/// Space between chips, across and down.
const CHIP_GAP: f32 = 4.0;
/// Space between the chips and the field.
const GAP: f32 = 6.0;

/// Everything that changes a multi-select combobox.
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
    /// Adds the highlighted match, or removes it when it is already chosen.
    ActivateHighlighted,
    /// Highlights an option, by its index in [`State::options`].
    Highlight(usize),
    /// Adds an option, or removes it when it is already chosen, by its
    /// index in [`State::options`].
    Activate(usize),
    /// Scrolls the list by a number of rows, down when positive.
    Scroll(i32),
    /// Removes a chosen option, by its index in [`State::options`].
    Remove(usize),
    /// Removes the most recently chosen option.
    RemoveLast,
}

/// What an [`Event`] changed in the chosen values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output<T> {
    Added(T),
    Removed(T),
}

/// The options, the query, the highlight and the chosen values.
#[derive(Debug, Clone, PartialEq)]
pub struct State<T> {
    list: Suggestions<T>,
    /// Indices into the options, in the order they were chosen.
    chosen: Vec<usize>,
}

impl<T: Display> State<T> {
    /// Creates a closed combobox with nothing chosen. Options are labelled
    /// with their `Display` text.
    pub fn new(options: impl IntoIterator<Item = T>) -> Self {
        Self {
            list: Suggestions::new(options),
            chosen: Vec::new(),
        }
    }
}

impl<T: PartialEq> State<T> {
    /// Starts with `values` chosen, in that order. Values that are not
    /// options, and repeats, are skipped.
    pub fn with_chosen<'v>(mut self, values: impl IntoIterator<Item = &'v T>) -> Self
    where
        T: 'v,
    {
        for value in values {
            let Some(index) = self.list.position(value) else {
                continue;
            };
            if !self.chosen.contains(&index) {
                self.chosen.push(index);
            }
        }
        self
    }
}

impl<T> State<T> {
    /// Sets how many suggestions show at once. At least one does.
    pub fn with_visible_rows(mut self, rows: usize) -> Self {
        self.list.set_rows(rows);
        self
    }

    pub fn options(&self) -> &[T] {
        self.list.options()
    }

    /// The chosen values, in the order they were chosen.
    pub fn chosen(&self) -> impl Iterator<Item = &T> {
        self.chosen
            .iter()
            .filter_map(|&index| self.list.option(index))
    }

    /// The chosen values' indices in [`State::options`], in the order they
    /// were chosen.
    pub fn chosen_indices(&self) -> &[usize] {
        &self.chosen
    }

    /// Whether the option at `index` in [`State::options`] is chosen.
    pub fn is_chosen(&self, index: usize) -> bool {
        self.chosen.contains(&index)
    }

    pub fn query(&self) -> &str {
        self.list.query()
    }

    pub fn is_open(&self) -> bool {
        self.list.is_open()
    }

    /// The options that match the query, chosen or not, in their original
    /// order.
    pub fn matches(&self) -> impl Iterator<Item = &T> {
        self.list.matches()
    }

    /// The highlighted option, if any option matches.
    pub fn highlighted(&self) -> Option<&T> {
        self.list.highlighted()
    }

    /// The matches in view, with their index in [`State::options`].
    pub fn visible(&self) -> impl Iterator<Item = (usize, &T)> {
        self.list.visible()
    }

    /// Applies an event and returns what changed in the chosen values.
    pub fn update(&mut self, event: Event) -> Option<Output<T>>
    where
        T: Clone,
    {
        match event {
            Event::Input(query) => self.list.input(query),
            Event::Open => self.list.open(None),
            Event::Close => self.list.close(),
            Event::Next | Event::Previous if !self.list.is_open() => self.list.open(None),
            Event::Next => self.list.step(1),
            Event::Previous => self.list.step(-1),
            Event::ActivateHighlighted => {
                if !self.list.is_open() {
                    return None;
                }
                let index = self.list.highlighted_index()?;
                return self.toggle(index);
            }
            Event::Activate(index) => return self.toggle(index),
            Event::Highlight(index) => self.list.highlight_option(index),
            Event::Scroll(rows) => self.list.scroll(rows),
            Event::Remove(index) => {
                let position = self.chosen.iter().position(|&i| i == index)?;
                return self.remove_at(position);
            }
            Event::RemoveLast => {
                let position = self.chosen.len().checked_sub(1)?;
                return self.remove_at(position);
            }
        }
        None
    }

    fn toggle(&mut self, index: usize) -> Option<Output<T>>
    where
        T: Clone,
    {
        let value = self.list.option(index)?.clone();
        self.list.clear_query();
        if let Some(position) = self.chosen.iter().position(|&i| i == index) {
            self.chosen.remove(position);
            return Some(Output::Removed(value));
        }
        self.chosen.push(index);
        Some(Output::Added(value))
    }

    fn remove_at(&mut self, position: usize) -> Option<Output<T>>
    where
        T: Clone,
    {
        let index = self.chosen.remove(position);
        self.list.option(index).cloned().map(Output::Removed)
    }
}

/// What a multi-select combobox keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
    Activate,
    Close,
    RemoveLast,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it would
    /// do nothing, so the key can reach the text field instead.
    pub fn event<T>(self, state: &State<T>) -> Option<Event> {
        match self {
            Action::Next => Some(Event::Next),
            Action::Previous => Some(Event::Previous),
            Action::Activate => state.is_open().then_some(Event::ActivateHighlighted),
            Action::Close => state.is_open().then_some(Event::Close),
            Action::RemoveLast => {
                (state.query().is_empty() && !state.chosen.is_empty()).then_some(Event::RemoveLast)
            }
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Next,
        Action::Previous,
        Action::Activate,
        Action::Close,
        Action::RemoveLast,
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
            Action::RemoveLast => "Remove last",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Highlights the next suggestion, opening the list if it is closed.",
            Action::Previous => {
                "Highlights the previous suggestion, opening the list if it is closed."
            }
            Action::Activate => {
                "Adds the highlighted suggestion, or removes it if it is already chosen. The list stays open."
            }
            Action::Close => "Closes the list and clears the query.",
            Action::RemoveLast => "Removes the last chip while the field is empty.",
        }
    }
}

/// The default multi-select combobox shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowDown` | [`Action::Next`] |
/// | `ArrowUp` | [`Action::Previous`] |
/// | `Enter` | [`Action::Activate`] |
/// | `Escape` | [`Action::Close`] |
/// | `Backspace` | [`Action::RemoveLast`] |
///
/// The combobox handles these itself while its field has focus or its list
/// is open. Backspace only removes a chip while the query is empty, so it
/// still deletes typed text. Pass a changed keymap with
/// [`MultiCombobox::keymap`].
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowDown), Action::Next)
        .bind(Chord::named(Named::ArrowUp), Action::Previous)
        .bind(Chord::named(Named::Enter), Action::Activate)
        .bind(Chord::named(Named::Escape), Action::Close)
        .bind(Chord::named(Named::Backspace), Action::RemoveLast)
}

/// The widget id of the x on the chip for the option at `index` in
/// [`State::options`], in the multi-select combobox with
/// [`MultiCombobox::id`] `key`.
pub fn remove_id(key: &str, index: usize) -> widget::Id {
    chip::remove_id(&chip_key(key, index))
}

fn chip_key(key: &str, index: usize) -> String {
    format!("{key}-{index}")
}

/// The chip size that suits a field size.
pub fn chip_size(size: Size) -> chip::Size {
    match size {
        Size::Sm => chip::Size::Sm,
        Size::Md | Size::Lg => chip::Size::Md,
    }
}

/// A multi-select combobox builder. Convert it into an [`Element`] to
/// render.
///
/// One without [`on_event`](MultiCombobox::on_event) is rendered disabled,
/// with its chips and no x buttons.
pub struct MultiCombobox<'a, T, Message> {
    state: &'a State<T>,
    placeholder: String,
    empty: String,
    width: Length,
    size: Size,
    key: Option<String>,
    invalid: bool,
    keymap: Keymap<Action>,
    shortcuts: Shortcuts,
    on_event: Option<Box<dyn Fn(Event) -> Message + 'a>>,
}

/// Renders the chosen values as chips, the field under them and, while it
/// is open, the suggestions of `state`.
pub fn multi_combobox<'a, T, Message>(state: &'a State<T>) -> MultiCombobox<'a, T, Message> {
    MultiCombobox {
        state,
        placeholder: String::new(),
        empty: DEFAULT_EMPTY.to_owned(),
        width: Length::Fixed(320.0),
        size: Size::default(),
        key: None,
        invalid: false,
        keymap: default_keymap(),
        shortcuts: Shortcuts::all(),
        on_event: None,
    }
}

impl<T: fmt::Debug, Message> fmt::Debug for MultiCombobox<'_, T, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MultiCombobox")
            .field("state", self.state)
            .field("placeholder", &self.placeholder)
            .field("empty", &self.empty)
            .field("width", &self.width)
            .field("size", &self.size)
            .field("key", &self.key)
            .field("invalid", &self.invalid)
            .field("shortcuts", &self.shortcuts)
            .field("enabled", &self.on_event.is_some())
            .finish_non_exhaustive()
    }
}

impl<'a, T, Message> MultiCombobox<'a, T, Message> {
    /// Text shown while the field is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Text shown in the list when no option matches the query.
    pub fn empty(mut self, empty: impl Into<String>) -> Self {
        self.empty = empty.into();
        self
    }

    /// Overrides the width. Defaults to 320 pixels.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// The field's height, padding and text size, shared with
    /// [`input`](fn@crate::primitives::input). Small fields get small chips.
    /// Defaults to [`Size::Md`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// A key, unique on screen, that becomes the text field's widget id and
    /// gives each chip's x the id [`remove_id`]`(key, index)`, to focus the
    /// field or find the buttons in tests.
    pub fn id(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Draws the field with the destructive border, as
    /// [`input`](fn@crate::primitives::input) does.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Replaces the [`default_keymap`].
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    /// Sets which editing [`Shortcut`](crate::primitives::input::Shortcut)s
    /// the field handles, as
    /// [`Input::shortcuts`](crate::primitives::input::Input::shortcuts) does.
    /// All of them by default.
    pub fn shortcuts(mut self, shortcuts: Shortcuts) -> Self {
        self.shortcuts = shortcuts;
        self
    }

    /// Maps events to your message. Without it the combobox is disabled.
    pub fn on_event(mut self, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_event.is_some()
    }
}

fn list_event(event: ListEvent) -> Event {
    match event {
        ListEvent::Highlight(index) => Event::Highlight(index),
        ListEvent::Activate(index) => Event::Activate(index),
        ListEvent::Scroll(rows) => Event::Scroll(rows),
    }
}

#[cfg(feature = "icedwright")]
const KIND: &str = "iced_cube::multi_combobox";

#[cfg(feature = "icedwright")]
use crate::forms::combobox::{Report, annotate_field, annotate_list};

impl<'a, T, Message> From<MultiCombobox<'a, T, Message>> for Element<'a, Message>
where
    T: Display + 'a,
    Message: Clone + 'a,
{
    fn from(combobox: MultiCombobox<'a, T, Message>) -> Self {
        let MultiCombobox {
            state,
            placeholder,
            empty,
            width,
            size,
            key,
            invalid,
            keymap,
            shortcuts,
            on_event,
        } = combobox;
        let on_event: Option<Rc<dyn Fn(Event) -> Message + 'a>> = on_event.map(Rc::from);
        let id = key.clone().map(widget::Id::from);

        let chips = state.chosen.iter().filter_map(|&index| {
            let option = state.list.option(index)?;
            let mut chip = chip(option.to_string())
                .size(chip_size(size))
                .on_remove_maybe(on_event.as_ref().map(|f| f(Event::Remove(index))));
            if let Some(key) = &key {
                chip = chip.id(chip_key(key, index));
            }
            Some(Element::from(chip))
        });
        let chips: Element<'a, Message> = row(chips)
            .spacing(CHIP_GAP)
            .wrap()
            .vertical_spacing(CHIP_GAP)
            .into();
        let has_chips = !state.chosen.is_empty();

        let Some(on_event) = on_event else {
            let field = field(&placeholder, "", size, id, None, false, invalid);
            let content = if has_chips {
                column![chips, field].spacing(GAP)
            } else {
                column![field]
            };
            let element: Element<'a, Message> = container(content).width(width).into();
            #[cfg(feature = "icedwright")]
            let element = annotate_field(
                element,
                Report {
                    value: String::new(),
                    placeholder,
                    open: false,
                    enabled: false,
                    invalid,
                    list: None,
                },
                KIND,
            );
            return element;
        };

        let open = state.is_open();
        #[cfg(feature = "icedwright")]
        let list_key = open.then(icedwright_probe::Key::unique);
        let list = open.then(|| {
            suggestions(
                &state.list,
                &empty,
                &|index| state.is_chosen(index),
                &|event| on_event(list_event(event)),
            )
        });
        #[cfg(feature = "icedwright")]
        let list = list.map(|list| match &list_key {
            Some(key) => annotate_list(list, key.clone(), KIND),
            None => list,
        });
        #[cfg(feature = "icedwright")]
        let report = Report {
            value: state.query().to_owned(),
            placeholder: placeholder.clone(),
            open,
            enabled: true,
            invalid,
            list: list_key,
        };
        let editing = Editing {
            on_input: {
                let on_event = Rc::clone(&on_event);
                Box::new(move |query| on_event(Event::Input(query)))
            },
            shortcuts,
        };
        let field = field(
            &placeholder,
            state.query(),
            size,
            id,
            Some(editing),
            open,
            invalid,
        );
        // Only a press on the field opens the list, not one on a chip's x.
        let field = anchored(field).on_anchor_press_maybe((!open).then(|| on_event(Event::Open)));
        let content = if has_chips {
            column![chips, field].spacing(GAP)
        } else {
            column![field]
        };
        let content: Element<'a, Message> = container(content).width(width).into();
        #[cfg(feature = "icedwright")]
        let content = annotate_field(content, report, KIND);

        anchored(content)
            .kind("iced_cube::multi_combobox")
            .content(list)
            .placement(Placement::new(Side::Bottom, Align::Start))
            .match_width(true)
            .dismiss_keys([])
            .dismiss_on_blur(true)
            .on_dismiss_maybe(open.then(|| on_event(Event::Close)))
            .on_key(move |key| {
                keymap
                    .resolve_event(key)
                    .and_then(|action| action.event(state))
                    .map(&*on_event)
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEOPLE: [&str; 5] = ["Ada", "Alan", "Grace", "Linus", "Margaret"];

    fn people() -> State<&'static str> {
        State::new(PEOPLE)
    }

    fn chosen(state: &State<&'static str>) -> Vec<&'static str> {
        state.chosen().copied().collect()
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().ok()?;
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn new_state_is_closed_with_nothing_chosen() {
        let state = people();
        assert!(!state.is_open());
        assert!(chosen(&state).is_empty());
        assert_eq!(state.query(), "");
        assert_eq!(state.matches().count(), PEOPLE.len());
    }

    #[test]
    fn with_chosen_keeps_order_and_skips_unknowns_and_repeats() {
        let state = people().with_chosen(&["Linus", "Kiwi", "Ada", "Linus"]);
        assert_eq!(chosen(&state), ["Linus", "Ada"]);
        assert_eq!(state.chosen_indices(), [3, 0]);
        assert!(state.is_chosen(3));
        assert!(!state.is_chosen(1));
    }

    #[test]
    fn choosing_adds_keeps_the_list_open_and_clears_the_query() {
        let mut state = people();
        let _ = state.update(Event::Input("gr".into()));
        assert_eq!(state.highlighted(), Some(&"Grace"));
        assert_eq!(
            state.update(Event::ActivateHighlighted),
            Some(Output::Added("Grace"))
        );
        assert!(state.is_open());
        assert_eq!(state.query(), "");
        assert_eq!(state.matches().count(), PEOPLE.len());
        assert_eq!(state.highlighted(), Some(&"Grace"), "highlight stays put");
        assert_eq!(chosen(&state), ["Grace"]);
    }

    #[test]
    fn choosing_a_chosen_option_removes_it() {
        let mut state = people().with_chosen(&["Ada", "Alan"]);
        let _ = state.update(Event::Open);
        assert_eq!(
            state.update(Event::Activate(0)),
            Some(Output::Removed("Ada"))
        );
        assert_eq!(chosen(&state), ["Alan"]);
        assert!(state.is_open());
    }

    #[test]
    fn values_are_kept_in_the_order_chosen() {
        let mut state = people();
        let _ = state.update(Event::Open);
        let _ = state.update(Event::Activate(4));
        let _ = state.update(Event::Activate(1));
        let _ = state.update(Event::Activate(2));
        assert_eq!(chosen(&state), ["Margaret", "Alan", "Grace"]);
    }

    #[test]
    fn remove_takes_out_one_chip_by_option_index() {
        let mut state = people().with_chosen(&["Ada", "Grace", "Linus"]);
        assert_eq!(
            state.update(Event::Remove(2)),
            Some(Output::Removed("Grace"))
        );
        assert_eq!(chosen(&state), ["Ada", "Linus"]);
        assert_eq!(state.update(Event::Remove(2)), None, "already removed");
        assert_eq!(state.update(Event::Remove(99)), None);
        assert!(!state.is_open(), "removing does not open the list");
    }

    #[test]
    fn remove_last_takes_the_most_recent_chip() {
        let mut state = people().with_chosen(&["Linus", "Ada"]);
        assert_eq!(
            state.update(Event::RemoveLast),
            Some(Output::Removed("Ada"))
        );
        assert_eq!(
            state.update(Event::RemoveLast),
            Some(Output::Removed("Linus"))
        );
        assert_eq!(state.update(Event::RemoveLast), None);
    }

    #[test]
    fn closed_list_ignores_activate_highlighted() {
        let mut state = people();
        assert_eq!(state.update(Event::ActivateHighlighted), None);
        assert!(chosen(&state).is_empty());
    }

    #[test]
    fn arrows_open_then_wrap_round() {
        let mut state = people();
        let _ = state.update(Event::Previous);
        assert!(state.is_open());
        assert_eq!(state.highlighted(), Some(&"Ada"));
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(&"Margaret"));
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(&"Ada"));
    }

    #[test]
    fn close_clears_the_query_and_keeps_the_chips() {
        let mut state = people().with_chosen(&["Ada"]);
        let _ = state.update(Event::Input("li".into()));
        let _ = state.update(Event::Close);
        assert!(!state.is_open());
        assert_eq!(state.query(), "");
        assert_eq!(chosen(&state), ["Ada"]);
    }

    #[test]
    fn no_match_leaves_nothing_to_choose() {
        let mut state = people();
        let _ = state.update(Event::Input("zz".into()));
        assert!(state.highlighted().is_none());
        assert_eq!(state.update(Event::ActivateHighlighted), None);
    }

    #[test]
    fn highlight_and_scroll_follow_the_combobox() {
        let mut state = people().with_visible_rows(2);
        let _ = state.update(Event::Open);
        let _ = state.update(Event::Highlight(1));
        assert_eq!(state.highlighted(), Some(&"Alan"));
        let _ = state.update(Event::Scroll(10));
        assert_eq!(state.visible().map(|(i, _)| i).collect::<Vec<_>>(), [3, 4]);
    }

    #[test]
    fn empty_options_are_safe() {
        let mut state = State::<&str>::new([]);
        let _ = state.update(Event::Next);
        assert_eq!(state.update(Event::ActivateHighlighted), None);
        assert_eq!(state.update(Event::Activate(0)), None);
        assert_eq!(state.update(Event::RemoveLast), None);
    }

    #[test]
    fn default_keymap_overrides_and_unbinding() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "ArrowDown"), Some(Action::Next));
        assert_eq!(press(&keymap, "ArrowUp"), Some(Action::Previous));
        assert_eq!(press(&keymap, "Enter"), Some(Action::Activate));
        assert_eq!(press(&keymap, "Escape"), Some(Action::Close));
        assert_eq!(press(&keymap, "Backspace"), Some(Action::RemoveLast));

        let custom = keymap
            .bind(Chord::character('n').ctrl(), Action::Next)
            .unbind(&Chord::named(Named::Backspace));
        assert_eq!(press(&custom, "ctrl+n"), Some(Action::Next));
        assert_eq!(press(&custom, "Backspace"), None);
    }

    #[test]
    fn actions_map_to_events_only_when_they_apply() {
        let mut state = people();
        assert_eq!(Action::Next.event(&state), Some(Event::Next));
        assert_eq!(Action::Previous.event(&state), Some(Event::Previous));
        assert_eq!(Action::Activate.event(&state), None);
        assert_eq!(Action::Close.event(&state), None);
        assert_eq!(Action::RemoveLast.event(&state), None, "no chips");

        let _ = state.update(Event::Open);
        let _ = state.update(Event::ActivateHighlighted);
        assert_eq!(
            Action::Activate.event(&state),
            Some(Event::ActivateHighlighted)
        );
        assert_eq!(Action::Close.event(&state), Some(Event::Close));
        assert_eq!(Action::RemoveLast.event(&state), Some(Event::RemoveLast));

        let _ = state.update(Event::Input("a".into()));
        assert_eq!(
            Action::RemoveLast.event(&state),
            None,
            "Backspace edits the query"
        );
    }

    #[test]
    fn default_builder_is_disabled() {
        let state = people();
        let c: MultiCombobox<'_, &str, Event> = multi_combobox(&state);
        assert!(!c.is_enabled());
        assert_eq!(c.empty, DEFAULT_EMPTY);
        assert_eq!(c.width, Length::Fixed(320.0));
        assert_eq!(c.size, Size::Md);
        assert_eq!(c.keymap, default_keymap());
        assert_eq!(c.shortcuts, Shortcuts::all());
        assert!(c.key.is_none());
        let c = c
            .placeholder("Add guests")
            .empty("Nobody")
            .size(Size::Sm)
            .id("guests")
            .invalid(true)
            .on_event(|e| e);
        assert!(c.is_enabled());
        assert!(c.invalid);
        assert_eq!(c.key.as_deref(), Some("guests"));
        assert_eq!(
            (c.placeholder.as_str(), c.empty.as_str()),
            ("Add guests", "Nobody")
        );
    }

    #[test]
    fn chip_sizes_follow_the_field() {
        assert_eq!(chip_size(Size::Sm), chip::Size::Sm);
        assert_eq!(chip_size(Size::Md), chip::Size::Md);
        assert_eq!(chip_size(Size::Lg), chip::Size::Md);
    }

    #[test]
    fn remove_ids_are_per_combobox_and_option() {
        assert_eq!(remove_id("guests", 1), remove_id("guests", 1));
        assert_ne!(remove_id("guests", 1), remove_id("guests", 2));
        assert_ne!(remove_id("guests", 1), remove_id("members", 1));
    }
}
