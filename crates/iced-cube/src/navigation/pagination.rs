//! Moves through a long list one page at a time.
//!
//! [`State`] holds the current page and the page count, both counted from
//! one. Its [`update`](State::update) handles selecting a page, stepping
//! to the next or previous one and jumping to either end, and never leaves
//! the range. [`slots`] decides which page numbers to show, so the control
//! keeps the same width as the current page moves.
//!
//! Keyboard shortcuts resolve through a [`Keymap`] of [`Action`]s; see
//! [`default_keymap`] and [`crate::keys`].

use std::ops::Range;
use std::rc::Rc;

use iced::keyboard::key::Named;
use iced::widget::{self, button::Status, container, responsive, row, text};
use iced::{Alignment, Element, Length, Padding, Theme};

use crate::icon::{Glyph, opacity, themed};
use crate::keys::{self, Chord, Keymap};
use crate::lucide;
use crate::primitives::button::{self, Size};
use crate::theme::{Tokens, space};

/// How the control is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variant {
    /// Previous and next buttons around page numbers, with gaps for the
    /// pages left out.
    #[default]
    Numbers,
    /// "Page 3 of 12" between first, previous, next and last buttons, for
    /// narrow spaces.
    Compact,
}

impl Variant {
    pub const ALL: [Variant; 2] = [Variant::Numbers, Variant::Compact];
}

/// One place in the row of page numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Page(usize),
    /// Pages left out between two shown ones.
    Gap,
}

/// The page numbers to show for `page` of `pages`, with `siblings` pages on
/// each side of the current one.
///
/// The first and last pages are always shown. A gap only replaces two or
/// more pages, since a gap for a single page takes as much room as the page
/// itself. Once there are enough pages to need a gap, the result always has
/// `2 * siblings + 5` slots, so the control does not change width as the
/// current page moves.
pub fn slots(page: usize, pages: usize, siblings: usize) -> Vec<Slot> {
    let total = 2 * siblings + 5;
    if pages <= total {
        return (1..=pages).map(Slot::Page).collect();
    }
    let page = page.clamp(1, pages);
    let left = page.saturating_sub(siblings).max(1);
    let right = (page + siblings).min(pages);
    let left_gap = left > 3;
    let right_gap = right + 2 < pages;
    // Slots on the side without a gap: the end page, the siblings, the
    // current page, and the two slots the missing gap and end page free up.
    let run = total - 2;

    let mut slots = Vec::with_capacity(total);
    match (left_gap, right_gap) {
        (false, _) => {
            slots.extend((1..=run).map(Slot::Page));
            slots.extend([Slot::Gap, Slot::Page(pages)]);
        }
        (true, false) => {
            slots.extend([Slot::Page(1), Slot::Gap]);
            slots.extend((pages + 1 - run..=pages).map(Slot::Page));
        }
        (true, true) => {
            slots.extend([Slot::Page(1), Slot::Gap]);
            slots.extend((left..=right).map(Slot::Page));
            slots.extend([Slot::Gap, Slot::Page(pages)]);
        }
    }
    slots
}

/// Changes to the current page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    /// Goes to a page, counted from 1. Pages out of range are ignored.
    Select(usize),
    Next,
    Previous,
    First,
    Last,
}

/// The current page and how many there are, both counted from 1. There is
/// always at least one page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct State {
    page: usize,
    pages: usize,
}

impl State {
    /// Starts on page 1 of `pages`, or of one page when `pages` is 0.
    pub fn new(pages: usize) -> Self {
        Self {
            page: 1,
            pages: pages.max(1),
        }
    }

    /// Enough pages for `items` at `per_page` each, starting on page 1.
    pub fn for_items(items: usize, per_page: usize) -> Self {
        Self::new(items.div_ceil(per_page.max(1)))
    }

    /// Starts on `page`, kept within range.
    pub fn with_page(mut self, page: usize) -> Self {
        self.page = page.clamp(1, self.pages);
        self
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn pages(&self) -> usize {
        self.pages
    }

    pub fn is_first(&self) -> bool {
        self.page == 1
    }

    pub fn is_last(&self) -> bool {
        self.page == self.pages
    }

    /// Changes the page count, such as after a filter, and moves the
    /// current page back into range. Returns the new page if it moved.
    pub fn set_pages(&mut self, pages: usize) -> Option<usize> {
        self.pages = pages.max(1);
        let page = self.page.min(self.pages);
        if page == self.page {
            return None;
        }
        self.page = page;
        Some(page)
    }

    /// The indices of the items on the current page, out of `items` at
    /// `per_page` each. Empty when the page is past the end.
    pub fn range(&self, items: usize, per_page: usize) -> Range<usize> {
        let start = ((self.page - 1) * per_page).min(items);
        let end = (start + per_page).min(items);
        start..end
    }

    /// Applies an event and returns the new page, if it changed.
    pub fn update(&mut self, event: Event) -> Option<usize> {
        let target = match event {
            Event::Select(page) => (1..=self.pages).contains(&page).then_some(page)?,
            Event::Next => (self.page + 1).min(self.pages),
            Event::Previous => self.page.saturating_sub(1).max(1),
            Event::First => 1,
            Event::Last => self.pages,
        };
        if target == self.page {
            return None;
        }
        self.page = target;
        Some(target)
    }
}

/// What a pagination keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
    First,
    Last,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when the page
    /// is already where the action would take it.
    pub fn event(self, state: &State) -> Option<Event> {
        match self {
            Action::Next | Action::Last if state.is_last() => None,
            Action::Previous | Action::First if state.is_first() => None,
            Action::Next => Some(Event::Next),
            Action::Previous => Some(Event::Previous),
            Action::First => Some(Event::First),
            Action::Last => Some(Event::Last),
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
            Action::Next => "Goes to the next page.",
            Action::Previous => "Goes to the previous page.",
            Action::First => "Goes to the first page.",
            Action::Last => "Goes to the last page.",
        }
    }
}

/// The default pagination shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `PageDown` | [`Action::Next`] |
/// | `PageUp` | [`Action::Previous`] |
/// | `Ctrl+Home` | [`Action::First`] |
/// | `Ctrl+End` | [`Action::Last`] |
///
/// These follow document viewers, and leave the arrow keys, Home and End to
/// tabs, sliders and lists on the same screen.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::PageDown), Action::Next)
        .bind(Chord::named(Named::PageUp), Action::Previous)
        .bind(Chord::named(Named::Home).ctrl(), Action::First)
        .bind(Chord::named(Named::End).ctrl(), Action::Last)
}

/// A pagination builder. Convert it into an [`Element`] to render.
pub struct Pagination<'a, Message> {
    state: &'a State,
    variant: Variant,
    size: Size,
    siblings: usize,
    on_event: Option<Box<dyn Fn(Event) -> Message + 'a>>,
}

/// Renders the controls for `state`. Without
/// [`on_event`](Pagination::on_event) every button renders disabled.
pub fn pagination<Message>(state: &State) -> Pagination<'_, Message> {
    Pagination {
        state,
        variant: Variant::default(),
        size: Size::Md,
        siblings: 1,
        on_event: None,
    }
}

impl<Message> std::fmt::Debug for Pagination<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pagination")
            .field("state", self.state)
            .field("variant", &self.variant)
            .field("size", &self.size)
            .field("siblings", &self.siblings)
            .field("enabled", &self.on_event.is_some())
            .finish()
    }
}

impl<'a, Message> Pagination<'a, Message> {
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// The button size: `Sm` is 32 pixels square, `Md` 36 and `Lg` 40.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// How many pages to show on each side of the current one. Defaults to
    /// 1, which shows at most seven page slots.
    pub fn siblings(mut self, siblings: usize) -> Self {
        self.siblings = siblings;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

type Handler<'a, Message> = Option<Rc<dyn Fn(Event) -> Message + 'a>>;

impl<'a, Message: Clone + 'a> From<Pagination<'a, Message>> for Element<'a, Message> {
    fn from(pagination: Pagination<'a, Message>) -> Self {
        let Pagination {
            state,
            variant,
            size,
            siblings,
            on_event,
        } = pagination;
        let state = *state;
        let on_event: Handler<'a, Message> = on_event.map(Rc::from);

        if variant == Variant::Compact {
            return compact(state, size, &on_event);
        }
        // Drops sibling pages until the row fits, then falls back to the
        // compact layout, so a phone never cuts the control off.
        responsive(move |available| {
            let fits = (0..=siblings)
                .rev()
                .find(|&siblings| numbers_width(state, siblings, size) <= available.width);
            match fits {
                Some(siblings) => numbers(state, siblings, size, &on_event),
                None => compact(state, size, &on_event),
            }
        })
        .width(Length::Shrink)
        .height(Length::Shrink)
        .into()
    }
}

/// The width of the numbered layout: two arrows, the page slots and the
/// gaps between them.
pub fn numbers_width(state: State, siblings: usize, size: Size) -> f32 {
    let side = size.metrics().height;
    let slots = slots(state.page, state.pages, siblings);
    let pages: f32 = slots
        .iter()
        .map(|slot| match slot {
            Slot::Page(page) => number_width(*page, size),
            Slot::Gap => side,
        })
        .sum();
    2.0 * side + pages + space::XS * (slots.len() + 1) as f32
}

fn send<Message>(on_event: &Handler<'_, Message>, event: Event, enabled: bool) -> Option<Message> {
    on_event
        .as_ref()
        .filter(|_| enabled)
        .map(|on_event| on_event(event))
}

fn numbers<'a, Message: Clone + 'a>(
    state: State,
    siblings: usize,
    size: Size,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let height = size.metrics().height;
    let step = |glyph: Glyph, event: Event, enabled: bool| {
        arrow(glyph, size, send(on_event, event, enabled))
    };

    let mut controls = row![step(
        lucide!(ChevronLeft),
        Event::Previous,
        !state.is_first()
    )]
    .spacing(space::XS)
    .align_y(Alignment::Center);
    for slot in slots(state.page, state.pages, siblings) {
        controls = controls.push(match slot {
            Slot::Page(page) => number(
                page,
                page == state.page,
                size,
                send(on_event, Event::Select(page), true),
            ),
            Slot::Gap => gap(height),
        });
    }
    controls
        .push(step(lucide!(ChevronRight), Event::Next, !state.is_last()))
        .into()
}

fn compact<'a, Message: Clone + 'a>(
    state: State,
    size: Size,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let step = |glyph: Glyph, event: Event, enabled: bool| {
        arrow(glyph, size, send(on_event, event, enabled))
    };
    let label = text(format!("Page {} of {}", state.page, state.pages))
        .size(size.metrics().text)
        .wrapping(text::Wrapping::None)
        .style(|theme: &Theme| text::Style {
            color: Some(Tokens::of(theme).foreground),
        });
    row![
        step(lucide!(ChevronsLeft), Event::First, !state.is_first()),
        step(lucide!(ChevronLeft), Event::Previous, !state.is_first()),
        container(label).padding(Padding::from([0.0, space::SM])),
        step(lucide!(ChevronRight), Event::Next, !state.is_last()),
        step(lucide!(ChevronsRight), Event::Last, !state.is_last()),
    ]
    .spacing(space::XS)
    .align_y(Alignment::Center)
    .into()
}

/// A square button with a chevron, for moving by one page or to an end.
fn arrow<'a, Message: Clone + 'a>(
    glyph: Glyph,
    size: Size,
    message: Option<Message>,
) -> Element<'a, Message> {
    let metrics = size.metrics();
    let enabled = message.is_some();
    let resting = if enabled {
        Status::Active
    } else {
        Status::Disabled
    };
    let icon = themed(glyph, metrics.icon, opacity(enabled), move |theme| {
        page_style(&Tokens::of(theme), false, resting).text_color
    });
    widget::button(icon)
        .width(Length::Fixed(metrics.height))
        .height(Length::Fixed(metrics.height))
        .padding(Padding::from((metrics.height - metrics.icon) / 2.0))
        .on_press_maybe(message)
        .style(|theme, status| page_style(&Tokens::of(theme), false, status))
        .into()
}

/// The width of a page button: square up to two digits, wider for longer
/// numbers so they keep some padding.
pub fn number_width(page: usize, size: Size) -> f32 {
    let metrics = size.metrics();
    let digits = page.max(1).ilog10() as f32 + 1.0;
    metrics
        .height
        .max(digits * metrics.text * 0.55 + 2.0 * space::SM)
}

/// A page number, centred in its button.
fn number<'a, Message: Clone + 'a>(
    page: usize,
    current: bool,
    size: Size,
    message: Option<Message>,
) -> Element<'a, Message> {
    let metrics = size.metrics();
    let label = text(page.to_string())
        .size(metrics.text)
        .wrapping(text::Wrapping::None);
    widget::button(container(label).center(Length::Fill))
        .width(Length::Fixed(number_width(page, size)))
        .height(Length::Fixed(metrics.height))
        .padding(0)
        .on_press_maybe(message)
        .style(move |theme, status| page_style(&Tokens::of(theme), current, status))
        .into()
}

/// The ellipsis that stands in for pages left out.
fn gap<'a, Message: 'a>(side: f32) -> Element<'a, Message> {
    container(themed(lucide!(Ellipsis), 16.0, 1.0, |theme| {
        Tokens::of(theme).muted_foreground
    }))
    .center(Length::Fixed(side))
    .into()
}

/// The style of a page button: outlined for the current page, ghost for
/// the rest and for the arrows. A disabled arrow fades like any button.
pub fn page_style(tokens: &Tokens, current: bool, status: Status) -> widget::button::Style {
    let variant = if current {
        button::Variant::Outline
    } else {
        button::Variant::Ghost
    };
    button::style(tokens, variant, status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    fn pages(slots: &[Slot]) -> String {
        slots
            .iter()
            .map(|slot| match slot {
                Slot::Page(page) => page.to_string(),
                Slot::Gap => String::from("."),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn few_pages_are_all_shown() {
        assert_eq!(pages(&slots(1, 1, 1)), "1");
        assert_eq!(pages(&slots(3, 7, 1)), "1 2 3 4 5 6 7");
        assert_eq!(pages(&slots(1, 0, 1)), "");
    }

    #[test]
    fn gaps_appear_on_the_side_with_hidden_pages() {
        assert_eq!(pages(&slots(1, 10, 1)), "1 2 3 4 5 . 10");
        assert_eq!(pages(&slots(4, 10, 1)), "1 2 3 4 5 . 10");
        assert_eq!(pages(&slots(5, 10, 1)), "1 . 4 5 6 . 10");
        assert_eq!(pages(&slots(6, 10, 1)), "1 . 5 6 7 . 10");
        assert_eq!(pages(&slots(7, 10, 1)), "1 . 6 7 8 9 10");
        assert_eq!(pages(&slots(10, 10, 1)), "1 . 6 7 8 9 10");
    }

    #[test]
    fn slot_count_stays_the_same_as_the_page_moves() {
        for siblings in 0..4 {
            for page in 1..=40 {
                let slots = slots(page, 40, siblings);
                assert_eq!(slots.len(), 2 * siblings + 5, "{page} {siblings}");
                assert!(slots.contains(&Slot::Page(page)), "{page} {siblings}");
                assert_eq!(slots.first(), Some(&Slot::Page(1)));
                assert_eq!(slots.last(), Some(&Slot::Page(40)));
            }
        }
    }

    #[test]
    fn a_gap_always_hides_at_least_two_pages() {
        for page in 1..=20 {
            let slots = slots(page, 20, 1);
            for window in slots.windows(3) {
                if let [Slot::Page(before), Slot::Gap, Slot::Page(after)] = window {
                    assert!(after - before > 2, "{page}: {before} . {after}");
                }
            }
        }
    }

    #[test]
    fn more_siblings_widen_the_window() {
        assert_eq!(pages(&slots(10, 20, 2)), "1 . 8 9 10 11 12 . 20");
        assert_eq!(pages(&slots(10, 20, 0)), "1 . 10 . 20");
    }

    #[test]
    fn out_of_range_pages_are_clamped_in_slots() {
        assert_eq!(slots(0, 10, 1), slots(1, 10, 1));
        assert_eq!(slots(99, 10, 1), slots(10, 10, 1));
    }

    #[test]
    fn new_starts_on_the_first_page_and_never_has_zero_pages() {
        let state = State::new(5);
        assert_eq!((state.page(), state.pages()), (1, 5));
        assert!(state.is_first());
        assert!(!state.is_last());
        let empty = State::new(0);
        assert_eq!((empty.page(), empty.pages()), (1, 1));
        assert!(empty.is_first() && empty.is_last());
    }

    #[test]
    fn for_items_rounds_up() {
        assert_eq!(State::for_items(0, 10).pages(), 1);
        assert_eq!(State::for_items(10, 10).pages(), 1);
        assert_eq!(State::for_items(11, 10).pages(), 2);
        assert_eq!(State::for_items(5, 0).pages(), 5);
    }

    #[test]
    fn with_page_clamps() {
        assert_eq!(State::new(5).with_page(3).page(), 3);
        assert_eq!(State::new(5).with_page(0).page(), 1);
        assert_eq!(State::new(5).with_page(9).page(), 5);
    }

    #[test]
    fn update_moves_within_range() {
        let mut state = State::new(3);
        assert_eq!(state.update(Event::Previous), None);
        assert_eq!(state.update(Event::First), None);
        assert_eq!(state.update(Event::Next), Some(2));
        assert_eq!(state.update(Event::Next), Some(3));
        assert_eq!(state.update(Event::Next), None);
        assert_eq!(state.update(Event::Last), None);
        assert_eq!(state.update(Event::Previous), Some(2));
        assert_eq!(state.update(Event::First), Some(1));
        assert_eq!(state.update(Event::Last), Some(3));
    }

    #[test]
    fn select_ignores_the_current_page_and_out_of_range_pages() {
        let mut state = State::new(4);
        assert_eq!(state.update(Event::Select(1)), None);
        assert_eq!(state.update(Event::Select(0)), None);
        assert_eq!(state.update(Event::Select(5)), None);
        assert_eq!(state.update(Event::Select(4)), Some(4));
        assert!(state.is_last());
    }

    #[test]
    fn set_pages_pulls_the_page_back_into_range() {
        let mut state = State::new(10).with_page(8);
        assert_eq!(state.set_pages(20), None);
        assert_eq!(state.set_pages(5), Some(5));
        assert_eq!(state.set_pages(0), Some(1));
        assert_eq!(state.pages(), 1);
    }

    #[test]
    fn range_covers_the_items_on_the_page() {
        let state = State::for_items(23, 10);
        assert_eq!(state.range(23, 10), 0..10);
        assert_eq!(state.with_page(3).range(23, 10), 20..23);
        assert_eq!(state.with_page(3).range(5, 10), 5..5);
        assert_eq!(State::new(1).range(0, 10), 0..0);
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "PageDown"), Some(Action::Next));
        assert_eq!(press(&keymap, "PageUp"), Some(Action::Previous));
        assert_eq!(press(&keymap, "Ctrl+Home"), Some(Action::First));
        assert_eq!(press(&keymap, "Ctrl+End"), Some(Action::Last));
        assert_eq!(press(&keymap, "Home"), None);
        assert_eq!(press(&keymap, "ArrowRight"), None);
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?}");
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&Chord::named(Named::PageDown))
            .bind("Alt+N".parse().unwrap(), Action::Next);
        assert_eq!(press(&keymap, "PageDown"), None);
        assert_eq!(press(&keymap, "Alt+N"), Some(Action::Next));
        assert_eq!(press(&keymap, "PageUp"), Some(Action::Previous));
        assert!(
            default_keymap()
                .unbind_action(&Action::Last)
                .chords(&Action::Last)
                .is_empty()
        );
    }

    #[test]
    fn actions_map_to_events_and_stop_at_the_ends() {
        let first = State::new(3);
        assert_eq!(Action::Next.event(&first), Some(Event::Next));
        assert_eq!(Action::Last.event(&first), Some(Event::Last));
        assert_eq!(Action::Previous.event(&first), None);
        assert_eq!(Action::First.event(&first), None);

        let mut last = first.with_page(3);
        assert_eq!(Action::Next.event(&last), None);
        assert_eq!(Action::Last.event(&last), None);
        assert_eq!(Action::Previous.event(&last), Some(Event::Previous));
        let event = Action::First.event(&last).unwrap();
        assert_eq!(last.update(event), Some(1));

        let single = State::new(1);
        for action in <Action as keys::Action>::ALL {
            assert_eq!(action.event(&single), None, "{action:?}");
        }
    }

    #[test]
    fn current_page_is_outlined_and_the_rest_are_ghosts() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let current = page_style(&tokens, true, Status::Active);
            let other = page_style(&tokens, false, Status::Active);
            assert!(current.border.width > 0.0);
            assert!(current.background.is_some());
            assert_eq!(other.border.width, 0.0);
            assert!(other.background.is_none());
        }
    }

    #[test]
    fn page_buttons_respond_to_hover_and_fade_when_disabled() {
        let tokens = Tokens::of(&light());
        for current in [false, true] {
            let rest = page_style(&tokens, current, Status::Active);
            let hovered = page_style(&tokens, current, Status::Hovered);
            let disabled = page_style(&tokens, current, Status::Disabled);
            assert_ne!(rest.background, hovered.background, "{current}");
            assert!(disabled.text_color.a < rest.text_color.a, "{current}");
        }
    }

    #[test]
    fn page_buttons_are_square_until_the_number_needs_more_room() {
        for size in Size::ALL {
            let side = size.metrics().height;
            assert_eq!(number_width(1, size), side);
            assert_eq!(number_width(99, size), side);
            assert!(number_width(10_000, size) > side);
            assert!(number_width(100_000, size) > number_width(10_000, size));
        }
    }

    #[test]
    fn the_numbered_width_counts_arrows_slots_and_gaps() {
        let state = State::new(10).with_page(5);
        // Two arrows and seven square slots, with eight gaps between nine
        // buttons.
        assert_eq!(numbers_width(state, 1, Size::Md), 9.0 * 36.0 + 8.0 * 4.0);
        assert_eq!(numbers_width(state, 0, Size::Md), 7.0 * 36.0 + 6.0 * 4.0);
        assert!(numbers_width(state, 1, Size::Sm) < numbers_width(state, 1, Size::Md));
        let long = State::new(2000).with_page(1000);
        assert!(numbers_width(long, 1, Size::Md) > numbers_width(state, 1, Size::Md));
    }

    #[test]
    fn builder_defaults_and_options() {
        let state = State::new(3);
        let p: Pagination<'_, Event> = pagination(&state);
        assert_eq!(p.variant, Variant::Numbers);
        assert_eq!(p.size, Size::Md);
        assert_eq!(p.siblings, 1);
        assert!(p.on_event.is_none());

        let p = p
            .variant(Variant::Compact)
            .size(Size::Sm)
            .siblings(2)
            .on_event(|event| event);
        assert_eq!(p.variant, Variant::Compact);
        assert_eq!(p.size, Size::Sm);
        assert_eq!(p.siblings, 2);
        assert!(p.on_event.is_some());
    }
}
