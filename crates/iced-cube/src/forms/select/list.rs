//! The open list of a select: an optional search field above the option
//! rows, which scroll inside the menu surface once there are more than fit.
//!
//! The list keeps its own state (the query, the highlight and the scroll
//! position) in the widget tree, so the app only sees the chosen value.
//! Rows are drawn content rather than buttons: the list decides what a
//! press, a drag or a key does. The search field is the library's text
//! field, built from the query each time it is needed, whose edits come
//! back to the list rather than to the app.

use iced::advanced::layout::{self, Layout};
use iced::advanced::text as core_text;
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::operation::scrollable::{AbsoluteOffset, RelativeOffset, Scrollable};
use iced::advanced::widget::{Id, Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, renderer};
use iced::alignment::Vertical;
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::time::{Duration, Instant};
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::widget::text_input;
use iced::widget::{container, stack};
use iced::{
    Alignment, Background, Border, Color, Element, Event, Length, Padding, Pixels, Point,
    Rectangle, Renderer, Size, Theme, Vector, mouse, touch, window,
};

use super::navigate::{
    TypeAhead, centred_offset, clamp_offset, moves, offset_for_thumb, step, thumb,
};
use crate::forms::matching;
use crate::icon::{opacity, themed};
use crate::overlay::menu::{self, RowStatus, reveal_offset};
use crate::primitives::input::{self, Shortcut, Shortcuts};
use crate::primitives::scroll_area::{self, Interaction, THICKNESS};
use crate::theme::{self, Tokens, fade, radius, space, text_size};

/// Height of the search field, with the rule under it and the gap below.
pub(super) const SEARCH_HEIGHT: f32 = ROW_HEIGHT + 1.0 + space::XS;
/// Height of a list row.
const ROW_HEIGHT: f32 = 32.0;
/// Height of the line shown in place of rows when none are shown.
const MESSAGE_HEIGHT: f32 = 40.0;
/// Height of the not-valid line above the rows.
const NOTICE_HEIGHT: f32 = 28.0;
/// Space between the scrollbar and the surface's edges.
const BAR_MARGIN: f32 = 2.0;
/// Width the rows give up to the scrollbar while the list scrolls.
const GUTTER: f32 = THICKNESS + 2.0 * BAR_MARGIN;
/// Pixels scrolled for one line of a mouse wheel.
const WHEEL_LINE: f32 = 60.0;
/// How far a finger moves before a touch becomes a scroll.
const TOUCH_SLOP: f32 = 8.0;
/// Left edge of the search icon: the rows' inset and padding.
const TEXT_X: f32 = space::XS + space::SM;

/// The search field: typing filters the options, offers a typed value, or
/// both.
pub(super) struct Search<'a, Message> {
    pub(super) filters: bool,
    pub(super) on_custom: Option<Box<dyn Fn(String) -> Option<Message> + 'a>>,
    pub(super) placeholder: String,
    pub(super) shortcuts: Shortcuts,
    /// The field is the closed control itself rather than a line in the
    /// list, so the list draws no field of its own.
    pub(super) on_anchor: bool,
    /// Enter asks `on_custom` before taking the highlighted match.
    pub(super) prefer_custom: bool,
    /// Tab or a press outside enters the typed text as Enter would.
    pub(super) commit_on_leave: bool,
}

/// The id of the open list of a select, reported to operations so tests
/// can find it. Only one list is open at a time.
pub fn list_id() -> Id {
    Id::new("iced-cube-select-list")
}

/// The id of the search field in the open list of a select.
pub fn search_id() -> Id {
    Id::new("iced-cube-select-search")
}

/// The list's state in `tree` or anywhere below it.
pub(super) fn state_mut(tree: &mut Tree) -> Option<&mut ListState> {
    if tree.tag == tree::Tag::of::<ListState>() {
        return Some(tree.state.downcast_mut::<ListState>());
    }
    tree.children.iter_mut().find_map(state_mut)
}

/// The list's state in `tree` or anywhere below it.
pub(super) fn state_ref(tree: &Tree) -> Option<&ListState> {
    if tree.tag == tree::Tag::of::<ListState>() {
        return Some(tree.state.downcast_ref::<ListState>());
    }
    tree.children.iter().find_map(state_ref)
}

/// The rows of a select's open list.
pub(super) struct List<'a, Message> {
    pub(super) rows: Vec<Element<'a, Message>>,
    /// Lowercase labels, one per row.
    pub(super) labels: Vec<String>,
    pub(super) choices: Vec<Option<Message>>,
    pub(super) selected: Option<usize>,
    /// The most rows shown before the list scrolls; `None` never scrolls.
    pub(super) max_rows: Option<usize>,
    pub(super) search: Option<Search<'a, Message>>,
    pub(super) pause: Duration,
    pub(super) empty: String,
    pub(super) no_results: String,
    /// Shown when the app turned the typed value down.
    pub(super) not_valid: String,
}

/// What the list remembers while it is open.
#[derive(Debug, Default)]
pub(super) struct ListState {
    query: String,
    /// The query was typed since the list opened.
    edited: bool,
    /// The query changed outside the list and is filtered on the next
    /// layout.
    refilter: bool,
    /// The keys moved the highlight since the query last changed.
    steered: bool,
    /// The closed control's bounds on screen.
    anchor: Option<Rectangle>,
    /// The app turned down the typed value.
    rejected: bool,
    /// Indices of the options shown, in order.
    shown: Vec<usize>,
    /// The highlighted option's index.
    highlight: Option<usize>,
    offset: f32,
    reveal: Option<Reveal>,
    typed: TypeAhead,
    /// When the latest frame was drawn.
    frame: Option<Instant>,
    press: Option<Target>,
    /// Distance from the thumb's top to the pointer while dragging it.
    grab: Option<f32>,
    finger: Option<Finger>,
    over_bar: bool,
    viewport: f32,
    content: f32,
}

/// How the highlighted row is brought into view after the next layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reveal {
    Centre,
    Nearest,
}

/// What a press landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Row(usize),
    Custom,
}

#[derive(Debug, Clone, Copy)]
struct Finger {
    id: touch::Finger,
    y: f32,
    offset: f32,
    target: Option<Target>,
    moved: bool,
}

impl ListState {
    /// Takes new text for the query. The list filters it on its next
    /// layout.
    pub(super) fn edit(&mut self, query: String) {
        self.query = query;
        self.edited = true;
        self.rejected = false;
        self.steered = false;
        self.refilter = true;
    }

    /// The text typed since the list opened, if any was.
    pub(super) fn edited_query(&self) -> Option<&str> {
        self.edited.then_some(self.query.as_str())
    }

    pub(super) fn is_rejected(&self) -> bool {
        self.rejected
    }

    pub(super) fn set_anchor(&mut self, anchor: Rectangle) {
        self.anchor = Some(anchor);
    }

    fn page_rows(&self) -> usize {
        (self.viewport / ROW_HEIGHT).floor().max(1.0) as usize
    }

    fn position(&self, option: Option<usize>) -> Option<usize> {
        option.and_then(|option| self.shown.iter().position(|&shown| shown == option))
    }
}

impl Scrollable for ListState {
    fn snap_to(&mut self, offset: RelativeOffset<Option<f32>>) {
        if let Some(y) = offset.y {
            self.offset = clamp_offset(
                y * (self.content - self.viewport),
                self.content,
                self.viewport,
            );
        }
    }

    fn scroll_to(&mut self, offset: AbsoluteOffset<Option<f32>>) {
        if let Some(y) = offset.y {
            self.offset = clamp_offset(y, self.content, self.viewport);
        }
    }

    fn scroll_by(&mut self, offset: AbsoluteOffset, _bounds: Rectangle, _content: Rectangle) {
        self.offset = clamp_offset(self.offset + offset.y, self.content, self.viewport);
    }
}

/// What the list does with an event.
#[derive(Debug, Clone, PartialEq)]
enum Outcome<Message> {
    /// Used here; nothing behind the list sees it.
    Used,
    /// A choice was sent. A key is left so the select closes the list; a
    /// pointer release is captured so the panel closes.
    Chosen(Message),
    /// A key for the search field.
    Field,
    /// Not the list's.
    Ignored,
}

/// The search field for `query`: an iced text input with the library's
/// editing shortcuts, behind a search icon. Its messages are the new text.
fn field(
    search_placeholder: &str,
    query: &str,
    rejected: bool,
    shortcuts: Shortcuts,
) -> Element<'static, String> {
    let vertical = (ROW_HEIGHT - menu::LINE_HEIGHT) / 2.0;
    let text = iced::widget::text_input(search_placeholder, query)
        .id(search_id())
        .size(text_size::SM)
        .line_height(LineHeight::Absolute(Pixels(menu::LINE_HEIGHT)))
        .padding(Padding {
            top: vertical,
            bottom: vertical,
            left: TEXT_X + menu::ICON_SIZE + space::SM,
            right: TEXT_X,
        })
        .width(Length::Fill)
        .style(move |theme, _status| field_style(&Tokens::of(theme), rejected, true));
    // Escape closes the list, so the field never keeps it.
    let shortcuts = shortcuts.without(Shortcut::EscapeUnfocus);
    let input = input::with_shortcuts(
        text,
        query,
        false,
        Some(Box::new(|value| value)),
        input::default_keymap(),
        shortcuts,
    );
    let icon = themed(
        crate::lucide!(Search),
        menu::ICON_SIZE,
        opacity(true),
        |theme| Tokens::of(theme).muted_foreground,
    );
    stack![
        input,
        container(icon)
            .height(Length::Fill)
            .align_y(Alignment::Center)
            .padding(Padding::ZERO.left(TEXT_X)),
    ]
    .into()
}

/// The search field's look: no box of its own, red text once the app has
/// turned the typed value down, faded while disabled. The field is built
/// afresh for every draw, so iced reports no status worth reading and
/// `enabled` says whether it takes input.
pub(super) fn field_style(tokens: &Tokens, rejected: bool, enabled: bool) -> text_input::Style {
    let value = match (rejected, enabled) {
        (true, _) => tokens.destructive_text,
        (false, false) => fade(tokens.foreground, 0.5),
        (false, true) => tokens.foreground,
    };
    let placeholder = if enabled {
        tokens.muted_foreground
    } else {
        fade(tokens.muted_foreground, 0.5)
    };
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: tokens.muted_foreground,
        placeholder,
        value,
        selection: fade(tokens.primary, 0.25),
    }
}

/// Focuses the search field if it lost focus, so typing always reaches it
/// while the list is open.
struct FocusField;

impl Operation for FocusField {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        if !state.is_focused() {
            state.focus();
        }
    }
}

/// Passes on everything a nested shell asked for except its messages.
pub(super) fn relay<Message, Inner>(shell: &mut Shell<'_, Message>, inner: &Shell<'_, Inner>) {
    if inner.is_event_captured() {
        shell.capture_event();
    }
    shell.request_redraw_at(inner.redraw_request());
    if inner.is_layout_invalid() {
        shell.invalidate_layout();
    }
    if inner.are_widgets_invalid() {
        shell.invalidate_widgets();
    }
    shell.input_method_mut().merge(inner.input_method());
}

/// Where a mouse or touch press landed, if the event is one.
fn press_position(event: &Event, cursor: mouse::Cursor) -> Option<Point> {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => cursor.position(),
        Event::Touch(touch::Event::FingerPressed { position, .. }) => Some(*position),
        _ => None,
    }
}

/// Whether an event goes on to the search field.
fn for_field(event: &Event, on_field: bool) -> bool {
    match event {
        Event::Window(_) | Event::InputMethod(_) => true,
        Event::Keyboard(keyboard::Event::KeyPressed { .. }) => false,
        Event::Keyboard(_) => true,
        Event::Mouse(mouse::Event::ButtonPressed(_)) | Event::Touch(_) => on_field,
        Event::Mouse(_) => true,
    }
}

impl<Message: Clone> List<'_, Message> {
    /// Whether the list draws a search field of its own.
    fn has_field(&self) -> bool {
        self.search.as_ref().is_some_and(|search| !search.on_anchor)
    }

    fn field_on_anchor(&self) -> bool {
        self.search.as_ref().is_some_and(|search| search.on_anchor)
    }

    fn first_row(&self) -> usize {
        usize::from(self.has_field())
    }

    fn search_height(&self) -> f32 {
        if self.has_field() { SEARCH_HEIGHT } else { 0.0 }
    }

    /// Whether the not-valid line shows above the rows. With no rows it
    /// shows in their place instead.
    fn shows_notice(&self, state: &ListState) -> bool {
        state.rejected && !state.shown.is_empty()
    }

    /// The top of the rows: below the search field and the not-valid line.
    fn top(&self, state: &ListState) -> f32 {
        let notice = if self.shows_notice(state) {
            NOTICE_HEIGHT
        } else {
            0.0
        };
        self.search_height() + notice
    }

    fn filters(&self) -> bool {
        self.search.as_ref().is_some_and(|search| search.filters)
    }

    fn offers_custom(&self) -> bool {
        self.search
            .as_ref()
            .is_some_and(|search| search.on_custom.is_some())
    }

    fn prefers_custom(&self) -> bool {
        self.search
            .as_ref()
            .is_some_and(|search| search.prefer_custom)
    }

    fn commits_on_leave(&self) -> bool {
        self.search
            .as_ref()
            .is_some_and(|search| search.commit_on_leave)
    }

    fn shown_for(&self, query: &str) -> Vec<usize> {
        if self.filters() {
            matching::filter(&self.labels, query)
        } else {
            (0..self.rows.len()).collect()
        }
    }

    /// The search field for the current query.
    fn field(&self, state: &ListState) -> Option<Element<'static, String>> {
        let search = self.search.as_ref().filter(|search| !search.on_anchor)?;
        Some(field(
            &search.placeholder,
            &state.query,
            state.rejected,
            search.shortcuts,
        ))
    }

    /// Whether the line in place of rows offers the typed value.
    fn offers_typed(&self, state: &ListState) -> bool {
        self.offers_custom()
            && !state.rejected
            && state.shown.is_empty()
            && !state.query.trim().is_empty()
    }

    fn choice(&self, option: Option<usize>) -> Option<Message> {
        option.and_then(|option| self.choices.get(option).cloned().flatten())
    }

    /// The message the app gives the typed value, trimmed, if it takes it.
    fn custom_message(&self, state: &ListState) -> Option<Message> {
        let on_custom = self.search.as_ref()?.on_custom.as_ref()?;
        on_custom(state.query.trim().to_owned())
    }

    /// The typed value, sent through the custom value handler.
    fn custom(&self, state: &mut ListState) -> Outcome<Message> {
        if !self.offers_custom() {
            return Outcome::Used;
        }
        match self.custom_message(state) {
            Some(message) => Outcome::Chosen(message),
            None => {
                state.rejected = true;
                Outcome::Used
            }
        }
    }

    /// Enter: the highlighted option, or the typed value when it matches
    /// no option. With custom values preferred, a whole label is still
    /// that option, then the app is asked, and only a value it turns down
    /// falls back to the highlighted match. A highlight the keys moved
    /// always wins.
    fn submit(&self, state: &mut ListState) -> Outcome<Message> {
        let typed = !state.query.trim().is_empty();
        if !typed || !self.offers_custom() {
            return self
                .choice(state.highlight)
                .map_or(Outcome::Used, Outcome::Chosen);
        }
        let matched = if self.filters() {
            state.highlight.filter(|_| !state.shown.is_empty())
        } else {
            None
        };
        let exact = matching::exact(&self.labels, &state.query);
        if self.prefers_custom() && !state.steered {
            if let Some(message) = self
                .choice(exact)
                .or_else(|| self.custom_message(state))
                .or_else(|| self.choice(matched))
            {
                return Outcome::Chosen(message);
            }
            state.rejected = true;
            return Outcome::Used;
        }
        let option = if self.filters() { matched } else { exact };
        match self.choice(option) {
            Some(message) => Outcome::Chosen(message),
            None => self.custom(state),
        }
    }

    /// Leaving the list by Tab or a press elsewhere. When the select
    /// commits on leaving, typed text is entered as Enter would: a value
    /// the app turns down keeps the list open (`Used`), and text that
    /// chooses nothing is dropped (`Ignored`).
    fn leave(&self, state: &mut ListState) -> Outcome<Message> {
        if !self.commits_on_leave() || state.query.trim().is_empty() {
            return Outcome::Ignored;
        }
        match self.submit(state) {
            Outcome::Chosen(message) => Outcome::Chosen(message),
            _ if state.rejected => Outcome::Used,
            _ => Outcome::Ignored,
        }
    }

    /// Takes the search field's new text and filters by it.
    fn set_query(&self, state: &mut ListState, query: String) {
        state.edit(query);
        self.refilter(state);
    }

    /// Shows the options matching the query. While filtering, the first
    /// match is highlighted and the list goes back to its top.
    fn refilter(&self, state: &mut ListState) {
        state.refilter = false;
        state.shown = self.shown_for(&state.query);
        if self.filters() {
            state.highlight = state.shown.first().copied();
            state.offset = 0.0;
        }
        state.reveal = Some(Reveal::Nearest);
    }

    /// The line drawn in place of rows when none show.
    fn message(&self, state: &ListState) -> String {
        if self.offers_typed(state) {
            return format!("Use \u{201c}{}\u{201d}", state.query.trim());
        }
        if state.rejected {
            self.not_valid.clone()
        } else if self.rows.is_empty() {
            self.empty.clone()
        } else {
            self.no_results.clone()
        }
    }

    /// Where the not-valid line above the rows shows.
    fn notice_bounds(&self, layout: Layout<'_>) -> Rectangle {
        let bounds = layout.bounds();
        Rectangle::new(
            Point::new(bounds.x + TEXT_X, bounds.y + self.search_height()),
            Size::new((bounds.width - 2.0 * TEXT_X).max(0.0), NOTICE_HEIGHT),
        )
    }

    fn key(
        &self,
        state: &mut ListState,
        key: &Key,
        modifiers: Modifiers,
        text: Option<&str>,
    ) -> Outcome<Message> {
        let searching = self.search.is_some();
        let plain = !(modifiers.control() || modifiers.alt() || modifiers.logo());
        if let Key::Named(named) = key
            && moves(*named)
            && plain
            && !modifiers.shift()
            // Home and End move the caret in the search field.
            && !(searching && matches!(named, Named::Home | Named::End))
        {
            let position = state.position(state.highlight);
            let next = step(position, state.shown.len(), state.page_rows(), *named);
            state.highlight = next.and_then(|position| state.shown.get(position).copied());
            state.reveal = Some(Reveal::Nearest);
            state.steered = true;
            state.typed.clear();
            return Outcome::Used;
        }
        match key {
            Key::Named(Named::Enter) if plain => return self.submit(state),
            Key::Named(Named::Tab) => return self.leave(state),
            Key::Named(Named::Escape) => return Outcome::Ignored,
            _ if searching => return Outcome::Field,
            _ if !plain => return Outcome::Ignored,
            Key::Named(Named::Space) if !state.typed.is_typing(state.frame, self.pause) => {
                return self
                    .choice(state.highlight)
                    .map_or(Outcome::Used, Outcome::Chosen);
            }
            _ => {}
        }
        let text = match key {
            Key::Named(Named::Space) => Some(" "),
            _ => text,
        };
        let Some(text) =
            text.filter(|text| !text.is_empty() && !text.chars().any(char::is_control))
        else {
            return Outcome::Ignored;
        };
        let labels: Vec<&str> = state
            .shown
            .iter()
            .filter_map(|&option| self.labels.get(option).map(String::as_str))
            .collect();
        let position = state.position(state.highlight);
        let found = state
            .typed
            .type_text(text, state.frame, self.pause, &labels, position);
        if let Some(option) = found.and_then(|position| state.shown.get(position).copied()) {
            state.highlight = Some(option);
            state.reveal = Some(Reveal::Nearest);
        }
        Outcome::Used
    }

    /// Where the rows show: below the search field, as tall as the viewport.
    fn area(&self, layout: Layout<'_>, state: &ListState) -> Rectangle {
        let bounds = layout.bounds();
        let top = self.top(state);
        Rectangle::new(
            Point::new(bounds.x, bounds.y + top),
            Size::new(bounds.width, state.viewport),
        )
    }

    /// The scrollbar's track, when the rows overflow.
    fn track(&self, layout: Layout<'_>, state: &ListState) -> Option<Rectangle> {
        if state.content <= state.viewport {
            return None;
        }
        let area = self.area(layout, state);
        Some(Rectangle::new(
            Point::new(
                area.x + area.width - BAR_MARGIN - THICKNESS,
                area.y + BAR_MARGIN,
            ),
            Size::new(THICKNESS, (area.height - 2.0 * BAR_MARGIN).max(0.0)),
        ))
    }

    fn thumb_bounds(&self, layout: Layout<'_>, state: &ListState) -> Option<Rectangle> {
        let track = self.track(layout, state)?;
        let (top, height) = thumb(state.offset, state.content, state.viewport, track.height)?;
        Some(Rectangle::new(
            Point::new(track.x, track.y + top),
            Size::new(track.width, height),
        ))
    }

    fn on_field(&self, layout: Layout<'_>, point: Point) -> bool {
        self.has_field()
            && layout
                .children()
                .next()
                .is_some_and(|field| field.bounds().contains(point))
    }

    /// The press target under a point on screen.
    fn target_at(&self, layout: Layout<'_>, state: &ListState, point: Point) -> Option<Target> {
        let area = self.area(layout, state);
        if !area.contains(point) {
            return None;
        }
        if state.shown.is_empty() {
            return self.offers_typed(state).then_some(Target::Custom);
        }
        let inside = point + Vector::new(0.0, state.offset);
        let first = self.first_row();
        state
            .shown
            .iter()
            .copied()
            .find(|&option| {
                layout
                    .children()
                    .nth(first + option)
                    .is_some_and(|row| row.bounds().contains(inside))
            })
            .map(Target::Row)
    }

    /// Highlights the row under the pointer, unless it is on the scrollbar.
    fn hover(&self, layout: Layout<'_>, state: &mut ListState, point: Point) {
        if state.grab.is_some() || state.over_bar {
            return;
        }
        if let Some(Target::Row(option)) = self.target_at(layout, state, point) {
            state.highlight = Some(option);
        }
    }

    /// The span of an option's row from the top of the rows.
    fn span(&self, layout: Layout<'_>, state: &ListState, option: usize) -> Option<(f32, f32)> {
        let row = layout.children().nth(self.first_row() + option)?;
        let top = row.bounds().y - layout.bounds().y - self.top(state);
        Some((top, top + row.bounds().height))
    }

    /// Scrolls to the highlight as asked. Centring on opening holds until
    /// the first input, since the panel may be laid out again at a
    /// fitted height before it shows. A centred list starts on a whole row.
    fn reveal_now(&self, layout: Layout<'_>, state: &mut ListState) {
        let reveal = match state.reveal {
            Some(Reveal::Centre) => Some(Reveal::Centre),
            _ => state.reveal.take(),
        };
        let Some(reveal) = reveal else {
            return;
        };
        let span = state
            .highlight
            .filter(|option| state.shown.contains(option))
            .and_then(|option| self.span(layout, state, option));
        if let Some((top, bottom)) = span {
            state.offset = match reveal {
                Reveal::Centre => {
                    let centred = centred_offset(top, bottom, state.content, state.viewport);
                    (centred / ROW_HEIGHT).round() * ROW_HEIGHT
                }
                Reveal::Nearest => reveal_offset(state.offset, state.viewport, top, bottom),
            };
        }
        state.offset = clamp_offset(state.offset, state.content, state.viewport);
    }

    /// A press outside the list. Typed text is entered when the select
    /// commits on leaving, unless the press is in the closed control's own
    /// field, where it only moves the caret. A value the app turns down
    /// captures the press, so the list stays open and nothing behind it
    /// acts.
    fn press_outside(
        &self,
        state: &mut ListState,
        position: Point,
        shell: &mut Shell<'_, Message>,
    ) {
        let in_field =
            self.field_on_anchor() && state.anchor.is_some_and(|anchor| anchor.contains(position));
        if in_field {
            return;
        }
        // The closed control shows the selected label again.
        if self.field_on_anchor() {
            shell.invalidate_layout();
        }
        match self.leave(state) {
            Outcome::Chosen(message) => shell.publish(message),
            Outcome::Used => shell.capture_event(),
            _ => {}
        }
    }

    fn choose(&self, target: Target, state: &mut ListState) -> Outcome<Message> {
        match target {
            Target::Row(option) => self
                .choice(Some(option))
                .map_or(Outcome::Used, Outcome::Chosen),
            Target::Custom => self.custom(state),
        }
    }

    /// Runs the search field on `event` and takes any new text from it.
    #[allow(clippy::too_many_arguments)]
    fn update_field(
        &self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> bool {
        let state = tree.state.downcast_mut::<ListState>();
        let (Some(mut field), Some(field_layout), Some(field_tree)) = (
            self.field(state),
            layout.children().next(),
            tree.children.first_mut(),
        ) else {
            return false;
        };
        let mut typed = Vec::new();
        let mut inner = Shell::new(&mut typed);
        field.as_widget_mut().update(
            field_tree,
            event,
            field_layout,
            cursor,
            renderer,
            clipboard,
            &mut inner,
            viewport,
        );
        relay(shell, &inner);
        // Focus stays in the field while the list is open.
        field
            .as_widget_mut()
            .operate(field_tree, field_layout, renderer, &mut FocusField);
        let Some(query) = typed.pop() else {
            return false;
        };
        self.set_query(state, query);
        true
    }

    /// The search field and the rule under it.
    fn draw_field(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let state = tree.state.downcast_ref::<ListState>();
        let (Some(field), Some(field_layout), Some(field_tree)) = (
            self.field(state),
            layout.children().next(),
            tree.children.first(),
        ) else {
            return;
        };
        let bounds = layout.bounds();
        field.as_widget().draw(
            field_tree,
            renderer,
            theme,
            style,
            field_layout,
            cursor,
            &bounds,
        );
        let tokens = Tokens::of(theme);
        let rule = if state.rejected {
            tokens.destructive
        } else {
            tokens.border
        };
        // Inside the surface's border, so a red rule does not run into it.
        fill(
            renderer,
            Rectangle::new(
                Point::new(bounds.x + 1.0, bounds.y + ROW_HEIGHT),
                Size::new((bounds.width - 2.0).max(0.0), 1.0),
            ),
            rule,
            0.0,
        );
    }

    /// The line in place of rows: the typed value to use, or why nothing
    /// shows.
    fn draw_message(
        &self,
        state: &ListState,
        renderer: &mut Renderer,
        tokens: &Tokens,
        area: Rectangle,
    ) {
        if self.offers_typed(state) {
            // Where the first row would be.
            let row = Rectangle::new(
                Point::new(area.x + space::XS, area.y),
                Size::new((area.width - 2.0 * space::XS).max(0.0), ROW_HEIGHT),
            );
            let style = menu::row_style(tokens, RowStatus::Highlighted, false);
            if let Some(background) = style.background {
                fill(renderer, row, background, radius::SM);
            }
            let label = self.message(state);
            let text = Rectangle::new(
                Point::new(row.x + space::SM, row.y),
                Size::new((row.width - 2.0 * space::SM).max(0.0), row.height),
            );
            draw_text(
                renderer,
                &label,
                text,
                style.text,
                core_text::Alignment::Left,
            );
            return;
        }
        let colour = if state.rejected {
            tokens.destructive_text
        } else {
            tokens.muted_foreground
        };
        draw_text(
            renderer,
            &self.message(state),
            area,
            colour,
            core_text::Alignment::Center,
        );
    }
}

/// Draws one line of 14 pixel text, centred vertically in `bounds` and cut
/// off at its edges. Left-aligned text is laid out without a width, so a
/// long line never wraps.
fn draw_text(
    renderer: &mut Renderer,
    content: &str,
    bounds: Rectangle,
    colour: Color,
    align: core_text::Alignment,
) {
    let (x, width) = match align {
        core_text::Alignment::Center => (bounds.center_x(), bounds.width),
        _ => (bounds.x, f32::INFINITY),
    };
    <Renderer as core_text::Renderer>::fill_text(
        renderer,
        core_text::Text {
            content: content.to_owned(),
            bounds: Size::new(width, bounds.height),
            size: Pixels(text_size::SM),
            line_height: LineHeight::Absolute(Pixels(menu::LINE_HEIGHT)),
            font: theme::font(),
            align_x: align,
            align_y: Vertical::Center,
            shaping: Shaping::Advanced,
            wrapping: Wrapping::None,
        },
        Point::new(x, bounds.center_y()),
        colour,
        bounds,
    );
}

fn fill(renderer: &mut Renderer, bounds: Rectangle, colour: Color, corner: f32) {
    <Renderer as renderer::Renderer>::fill_quad(
        renderer,
        renderer::Quad {
            bounds,
            border: Border {
                radius: corner.into(),
                ..Border::default()
            },
            ..renderer::Quad::default()
        },
        Background::Color(colour),
    );
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for List<'_, Message> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ListState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ListState {
            shown: (0..self.rows.len()).collect(),
            highlight: self.selected,
            reveal: Some(Reveal::Centre),
            ..ListState::default()
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.field(&ListState::default())
            .iter()
            .map(Tree::new)
            .chain(self.rows.iter().map(Tree::new))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        if tree.children.len() != self.first_row() + self.rows.len() {
            tree.children = self.children();
            return;
        }
        let state = tree.state.downcast_ref::<ListState>();
        let field = self.field(state);
        let (field_tree, row_trees) = tree.children.split_at_mut(self.first_row());
        if let (Some(field), Some(field_tree)) = (field, field_tree.first_mut()) {
            field_tree.diff(field.as_widget());
        }
        for (row, row_tree) in self.rows.iter().zip(row_trees) {
            row_tree.diff(row.as_widget());
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let first = self.first_row();
        let width = limits
            .resolve(Length::Fill, Length::Shrink, Size::ZERO)
            .width;
        let state = tree.state.downcast_mut::<ListState>();
        if state.refilter {
            self.refilter(state);
        } else {
            state.shown = self.shown_for(&state.query);
        }
        let search_height = self.top(state);
        if state
            .highlight
            .is_some_and(|option| !state.shown.contains(&option))
        {
            state.highlight = state.shown.first().copied();
        }
        let shown = state.shown.clone();
        let field = self.field(state);
        let (field_tree, row_trees) = tree.children.split_at_mut(first);

        let mut lay_out = |width: f32| {
            let mut visible = vec![false; self.rows.len()];
            for &option in &shown {
                if let Some(slot) = visible.get_mut(option) {
                    *slot = true;
                }
            }
            let row_limits =
                layout::Limits::new(Size::new(width, 0.0), Size::new(width, f32::INFINITY));
            let mut y = 0.0;
            let mut heights = Vec::with_capacity(shown.len());
            let nodes: Vec<layout::Node> = self
                .rows
                .iter_mut()
                .zip(row_trees.iter_mut())
                .zip(visible)
                .map(|((row, tree), visible)| {
                    if !visible {
                        return layout::Node::new(Size::ZERO).move_to((0.0, search_height));
                    }
                    let node = row
                        .as_widget_mut()
                        .layout(tree, renderer, &row_limits)
                        .move_to((0.0, search_height + y));
                    y += node.size().height;
                    heights.push(node.size().height);
                    node
                })
                .collect();
            (nodes, y, heights)
        };

        let (mut nodes, mut content, heights) = lay_out(width);
        let room = (limits.max().height - search_height).max(0.0);
        let viewport = match self.max_rows {
            _ if shown.is_empty() => MESSAGE_HEIGHT,
            None => content,
            Some(rows) => {
                let cap: f32 = heights.iter().take(rows.max(1)).sum();
                content.min(cap).min(room)
            }
        };
        if content > viewport + 0.5 {
            (nodes, content, _) = lay_out((width - GUTTER).max(0.0));
        }

        let mut children = Vec::with_capacity(nodes.len() + first);
        if let (Some(mut field), Some(field_tree)) = (field, field_tree.first_mut()) {
            let size = Size::new(width, ROW_HEIGHT);
            let node = field.as_widget_mut().layout(
                field_tree,
                renderer,
                &layout::Limits::new(size, size),
            );
            field.as_widget_mut().operate(
                field_tree,
                Layout::new(&node),
                renderer,
                &mut FocusField,
            );
            children.push(node);
        }
        children.extend(nodes);

        let state = tree.state.downcast_mut::<ListState>();
        state.viewport = viewport;
        state.content = if shown.is_empty() { 0.0 } else { content };
        let node =
            layout::Node::with_children(Size::new(width, search_height + viewport), children);
        self.reveal_now(Layout::new(&node), state);
        node
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<ListState>();
        let tokens = Tokens::of(theme);
        self.draw_field(tree, renderer, theme, style, layout, cursor);
        if self.shows_notice(state) {
            draw_text(
                renderer,
                &self.not_valid,
                self.notice_bounds(layout),
                tokens.destructive_text,
                core_text::Alignment::Left,
            );
        }
        let area = self.area(layout, state);
        if state.shown.is_empty() {
            self.draw_message(state, renderer, &tokens, area);
            return;
        }

        let offset = state.offset;
        let first = self.first_row();
        let inner_cursor = match cursor.position() {
            Some(position) if area.contains(position) => {
                mouse::Cursor::Available(position + Vector::new(0.0, offset))
            }
            _ => mouse::Cursor::Unavailable,
        };
        let inner_viewport = area + Vector::new(0.0, offset);
        let highlight = menu::row_style(&tokens, RowStatus::Highlighted, false);
        let idle = menu::row_style(&tokens, RowStatus::Idle, false);

        <Renderer as renderer::Renderer>::with_layer(renderer, area, |renderer| {
            <Renderer as renderer::Renderer>::with_translation(
                renderer,
                Vector::new(0.0, -offset),
                |renderer| {
                    for &option in &state.shown {
                        let (Some(row), Some(row_tree), Some(element)) = (
                            layout.children().nth(first + option),
                            tree.children.get(first + option),
                            self.rows.get(option),
                        ) else {
                            continue;
                        };
                        if !row.bounds().intersects(&inner_viewport) {
                            continue;
                        }
                        let highlighted = state.highlight == Some(option);
                        if highlighted && let Some(background) = highlight.background {
                            let inner = row.children().next().map_or(row.bounds(), |b| b.bounds());
                            fill(renderer, inner, background, radius::SM);
                        }
                        let text_color = if highlighted {
                            highlight.text
                        } else {
                            idle.text
                        };
                        element.as_widget().draw(
                            row_tree,
                            renderer,
                            theme,
                            &renderer::Style { text_color },
                            row,
                            inner_cursor,
                            &inner_viewport,
                        );
                    }
                },
            );
        });

        if let Some(bar) = self.thumb_bounds(layout, state) {
            let interaction = if state.grab.is_some() {
                Interaction::Dragged
            } else if state.over_bar {
                Interaction::Hovered
            } else {
                Interaction::Idle
            };
            fill(
                renderer,
                bar,
                scroll_area::scroller_colour(&tokens, interaction),
                radius::FULL,
            );
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let first = self.first_row();
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<ListState>();
        let mut field = self.field(state);
        let area = self.area(layout, state);
        let content = Rectangle::new(area.position(), Size::new(area.width, state.content));
        let translation = Vector::new(0.0, state.offset);
        let shown = state.shown.clone();
        // The lines the list draws itself, so tests can find them by text.
        let notice = self.shows_notice(state).then(|| self.notice_bounds(layout));
        let message = shown.is_empty().then(|| self.message(state));
        let (field_tree, row_trees) = children.split_at_mut(first);
        operation.container(Some(&list_id()), layout.bounds());
        operation.traverse(&mut |operation| {
            if let (Some(field), Some(field_tree), Some(field_layout)) = (
                field.as_mut(),
                field_tree.first_mut(),
                layout.children().next(),
            ) {
                field
                    .as_widget_mut()
                    .operate(field_tree, field_layout, renderer, operation);
            }
            if let Some(bounds) = notice {
                operation.text(None, bounds, &self.not_valid);
            }
            if let Some(message) = &message {
                operation.text(None, area, message);
            }
            operation.scrollable(None, area, content, translation, state);
            // The rows go one level down, so each row starts from the
            // scrolled view rather than from the list's.
            operation.traverse(&mut |operation| {
                for &option in &shown {
                    let (Some(row), Some(row_tree), Some(row_layout)) = (
                        self.rows.get_mut(option),
                        row_trees.get_mut(option),
                        layout.children().nth(first + option),
                    ) else {
                        continue;
                    };
                    row.as_widget_mut()
                        .operate(row_tree, row_layout, renderer, operation);
                }
            });
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<ListState>();
        let before = (
            state.highlight,
            state.offset,
            state.over_bar,
            state.query.len(),
            state.rejected,
        );
        let mut outcome = Outcome::Ignored;
        if matches!(
            event,
            Event::Keyboard(_) | Event::Mouse(_) | Event::Touch(_)
        ) && state.reveal == Some(Reveal::Centre)
        {
            state.reveal = None;
        }
        // The menu surface pads the list above and below.
        if let Some(position) = press_position(event, cursor)
            && !layout.bounds().expand(space::XS).contains(position)
        {
            self.press_outside(state, position, shell);
            if state.rejected != before.4 {
                shell.invalidate_layout();
                shell.request_redraw();
            }
            return;
        }

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                state.frame = Some(*now);
                // A frame once the pause is over ends the type-ahead burst.
                if let Some(stamped) = state.typed.stamp(*now) {
                    shell.request_redraw_at(stamped + self.pause);
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                text,
                ..
            }) => {
                // Escape closes the list, and the closed control then
                // shows the selected label again.
                if self.field_on_anchor() && *key == Key::Named(Named::Escape) {
                    shell.invalidate_layout();
                }
                outcome = self.key(state, key, *modifiers, text.as_deref());
                self.reveal_now(layout, state);
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let (Some(grab), Some(track)) = (state.grab, self.track(layout, state)) {
                    state.offset = offset_for_thumb(
                        position.y - grab - track.y,
                        state.content,
                        state.viewport,
                        track.height,
                    );
                }
                state.over_bar = self
                    .track(layout, state)
                    .is_some_and(|track| track.expand(BAR_MARGIN).contains(*position));
                self.hover(layout, state, *position);
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor.position_over(layout.bounds()) else {
                    return;
                };
                if self.on_field(layout, position) {
                    outcome = Outcome::Field;
                } else if let Some(track) = self.track(layout, state)
                    && track.expand(BAR_MARGIN).contains(position)
                {
                    let thumb = self.thumb_bounds(layout, state).unwrap_or(track);
                    let grab = if thumb.contains(position) {
                        position.y - thumb.y
                    } else {
                        thumb.height / 2.0
                    };
                    state.grab = Some(grab);
                    state.offset = offset_for_thumb(
                        position.y - grab - track.y,
                        state.content,
                        state.viewport,
                        track.height,
                    );
                    shell.capture_event();
                } else {
                    state.press = self.target_at(layout, state, position);
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.grab = None;
                let pressed = state.press.take();
                let released = cursor
                    .position()
                    .and_then(|position| self.target_at(layout, state, position));
                if let Some(target) = pressed.filter(|target| Some(*target) == released) {
                    outcome = self.choose(target, state);
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let Some(position) = cursor.position_over(layout.bounds()) else {
                    return;
                };
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * WHEEL_LINE,
                    mouse::ScrollDelta::Pixels { y, .. } => *y,
                };
                state.offset = clamp_offset(state.offset - dy, state.content, state.viewport);
                self.hover(layout, state, position);
                shell.capture_event();
            }
            Event::Touch(touch::Event::FingerPressed { id, position }) => {
                if !self.area(layout, state).contains(*position) {
                    outcome = Outcome::Field;
                } else {
                    state.finger = Some(Finger {
                        id: *id,
                        y: position.y,
                        offset: state.offset,
                        target: self.target_at(layout, state, *position),
                        moved: false,
                    });
                    shell.capture_event();
                }
            }
            Event::Touch(touch::Event::FingerMoved { id, position }) => {
                let Some(finger) = state.finger.as_mut().filter(|finger| finger.id == *id) else {
                    return;
                };
                let dy = position.y - finger.y;
                finger.moved |= dy.abs() > TOUCH_SLOP;
                if finger.moved {
                    let offset = finger.offset - dy;
                    state.offset = clamp_offset(offset, state.content, state.viewport);
                }
                shell.capture_event();
            }
            Event::Touch(touch::Event::FingerLifted { id, position }) => {
                let Some(finger) = state.finger.take().filter(|finger| finger.id == *id) else {
                    return;
                };
                if !finger.moved
                    && let Some(target) = finger.target
                    && self.target_at(layout, state, *position) == Some(target)
                {
                    outcome = self.choose(target, state);
                }
            }
            Event::Touch(touch::Event::FingerLost { .. }) => {
                state.finger = None;
            }
            _ => {}
        }

        let pointer = matches!(event, Event::Mouse(_) | Event::Touch(_));
        let on_field = cursor
            .position()
            .is_some_and(|position| self.on_field(layout, position));
        let to_field = matches!(outcome, Outcome::Field) || for_field(event, on_field);
        match outcome {
            // A pointer release that chose nothing, such as a typed value
            // the app turned down, is left uncaptured so the panel stays.
            Outcome::Used if !pointer => shell.capture_event(),
            Outcome::Chosen(message) => {
                shell.publish(message);
                if pointer {
                    shell.capture_event();
                }
            }
            _ => {}
        }
        let edited = to_field
            && self.update_field(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        if edited {
            shell.invalidate_layout();
        }

        let state = tree.state.downcast_ref::<ListState>();
        let after = (
            state.highlight,
            state.offset,
            state.over_bar,
            state.query.len(),
            state.rejected,
        );
        if after.4 != before.4 {
            shell.invalidate_layout();
        }
        if edited || before != after {
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<ListState>();
        let Some(position) = cursor.position() else {
            return mouse::Interaction::default();
        };
        if state.grab.is_some() || state.over_bar {
            return mouse::Interaction::Idle;
        }
        if self.target_at(layout, state, position).is_some() {
            return mouse::Interaction::Pointer;
        }
        if self.on_field(layout, position) {
            return mouse::Interaction::Text;
        }
        mouse::Interaction::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Pick(usize),
        Custom(String),
    }

    const DAYS: [&str; 7] = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];

    fn list<'a>(filters: bool, custom: bool, accepts: bool) -> List<'a, Message> {
        with_custom(filters, custom, move |text| {
            accepts.then_some(Message::Custom(text))
        })
    }

    fn with_custom<'a>(
        filters: bool,
        custom: bool,
        on_custom: impl Fn(String) -> Option<Message> + 'a,
    ) -> List<'a, Message> {
        let search = (filters || custom).then(|| Search {
            filters,
            on_custom: custom
                .then(|| Box::new(on_custom) as Box<dyn Fn(String) -> Option<Message> + 'a>),
            placeholder: "Search".to_owned(),
            shortcuts: Shortcuts::all(),
            on_anchor: false,
            prefer_custom: false,
            commit_on_leave: false,
        });
        List {
            rows: DAYS.iter().map(|_| iced::widget::space().into()).collect(),
            labels: matching::lowercase(&DAYS),
            choices: (0..DAYS.len())
                .map(|index| Some(Message::Pick(index)))
                .collect(),
            selected: Some(2),
            max_rows: Some(8),
            search,
            pause: Duration::from_secs(1),
            empty: "No options".to_owned(),
            no_results: "No results found.".to_owned(),
            not_valid: "Not valid".to_owned(),
        }
    }

    fn open(list: &List<'_, Message>) -> ListState {
        ListState {
            shown: (0..list.rows.len()).collect(),
            highlight: list.selected,
            viewport: 96.0,
            content: 224.0,
            ..ListState::default()
        }
    }

    fn press(
        list: &List<'_, Message>,
        state: &mut ListState,
        key: Key,
        text: Option<&str>,
    ) -> Outcome<Message> {
        list.key(state, &key, Modifiers::empty(), text)
    }

    fn named(list: &List<'_, Message>, state: &mut ListState, key: Named) -> Outcome<Message> {
        press(list, state, Key::Named(key), None)
    }

    fn typed(list: &List<'_, Message>, state: &mut ListState, text: &str) -> Outcome<Message> {
        press(list, state, Key::Character(text.into()), Some(text))
    }

    #[test]
    fn arrows_and_pages_move_the_highlight_and_ask_for_a_reveal() {
        let list = list(false, false, false);
        let mut state = open(&list);
        assert_eq!(named(&list, &mut state, Named::ArrowDown), Outcome::Used);
        assert_eq!(state.highlight, Some(3));
        assert_eq!(state.reveal, Some(Reveal::Nearest));
        named(&list, &mut state, Named::PageDown);
        assert_eq!(
            state.highlight,
            Some(6),
            "three rows to a page, stopping at the end"
        );
        named(&list, &mut state, Named::PageUp);
        assert_eq!(state.highlight, Some(3));
        named(&list, &mut state, Named::Home);
        assert_eq!(state.highlight, Some(0));
    }

    #[test]
    fn enter_or_space_choose_the_highlight_and_leave_the_key() {
        let list = list(false, false, false);
        let mut state = open(&list);
        assert_eq!(
            named(&list, &mut state, Named::Enter),
            Outcome::Chosen(Message::Pick(2))
        );
        assert_eq!(
            named(&list, &mut state, Named::Space),
            Outcome::Chosen(Message::Pick(2))
        );
        assert_eq!(named(&list, &mut state, Named::Tab), Outcome::Ignored);
        assert_eq!(named(&list, &mut state, Named::Escape), Outcome::Ignored);
    }

    #[test]
    fn typing_without_search_jumps_to_a_match() {
        let list = list(false, false, false);
        let mut state = open(&list);
        assert_eq!(typed(&list, &mut state, "t"), Outcome::Used);
        assert_eq!(state.highlight, Some(3), "the next T after Wednesday");
        typed(&list, &mut state, "u");
        assert_eq!(state.highlight, Some(1), "Tu");
        assert_eq!(
            named(&list, &mut state, Named::Space),
            Outcome::Used,
            "a space while typing is part of the search"
        );
        assert!(state.query.is_empty(), "type-ahead has no query");
    }

    #[test]
    fn with_a_search_field_typing_and_editing_keys_go_to_the_field() {
        let list = list(true, false, false);
        let mut state = open(&list);
        assert_eq!(typed(&list, &mut state, "s"), Outcome::Field);
        assert_eq!(named(&list, &mut state, Named::Space), Outcome::Field);
        assert_eq!(named(&list, &mut state, Named::Backspace), Outcome::Field);
        assert_eq!(named(&list, &mut state, Named::Home), Outcome::Field);
        let chord = list.key(
            &mut state,
            &Key::Character("v".into()),
            Modifiers::CTRL,
            Some("v"),
        );
        assert_eq!(chord, Outcome::Field, "paste and the editing shortcuts");
        assert_eq!(named(&list, &mut state, Named::ArrowDown), Outcome::Used);
        assert_eq!(named(&list, &mut state, Named::Escape), Outcome::Ignored);
        assert_eq!(named(&list, &mut state, Named::Tab), Outcome::Ignored);
    }

    #[test]
    fn search_filters_and_highlights_the_first_match() {
        let list = list(true, false, false);
        let mut state = open(&list);
        list.set_query(&mut state, "Sa".to_owned());
        assert_eq!(state.shown, vec![5]);
        assert_eq!(state.highlight, Some(5));
        assert_eq!(
            named(&list, &mut state, Named::Enter),
            Outcome::Chosen(Message::Pick(5))
        );
        list.set_query(&mut state, "S".to_owned());
        assert_eq!(state.shown, vec![1, 2, 3, 5, 6], "every day with an S");
        list.set_query(&mut state, "Sx".to_owned());
        assert!(state.shown.is_empty());
        assert_eq!(
            named(&list, &mut state, Named::Enter),
            Outcome::Used,
            "nothing to choose"
        );
    }

    #[test]
    fn a_typed_value_is_offered_when_nothing_matches() {
        let list = list(true, true, true);
        let mut state = open(&list);
        list.set_query(&mut state, "09:40".to_owned());
        assert!(list.offers_typed(&state));
        assert_eq!(
            named(&list, &mut state, Named::Enter),
            Outcome::Chosen(Message::Custom("09:40".to_owned()))
        );
        let mut state = open(&list);
        list.set_query(&mut state, "fri".to_owned());
        assert_eq!(
            named(&list, &mut state, Named::Enter),
            Outcome::Chosen(Message::Pick(4)),
            "a match wins over the typed value"
        );
    }

    #[test]
    fn a_rejected_value_stays_with_the_invalid_look_until_edited() {
        let list = list(true, true, false);
        let mut state = open(&list);
        list.set_query(&mut state, "25:00".to_owned());
        assert_eq!(named(&list, &mut state, Named::Enter), Outcome::Used);
        assert!(state.rejected);
        assert!(!list.offers_typed(&state), "not offered again");
        assert_eq!(state.query, "25:00", "the text is kept");
        list.set_query(&mut state, "25:0".to_owned());
        assert!(!state.rejected);
    }

    #[test]
    fn custom_values_without_search_keep_every_row_and_the_highlight() {
        let list = list(false, true, true);
        let mut state = open(&list);
        list.set_query(&mut state, "Fr".to_owned());
        assert_eq!(state.shown.len(), DAYS.len(), "nothing is filtered");
        assert_eq!(state.highlight, Some(2), "typing does not move it");
        assert_eq!(
            named(&list, &mut state, Named::Enter),
            Outcome::Chosen(Message::Custom("Fr".to_owned())),
            "only a whole label chooses an option"
        );
        list.set_query(&mut state, "friday".to_owned());
        assert_eq!(
            named(&list, &mut state, Named::Enter),
            Outcome::Chosen(Message::Pick(4))
        );
    }

    fn preferring(accepts: &'static [&'static str]) -> List<'static, Message> {
        let mut list = with_custom(true, true, move |text| {
            accepts
                .contains(&text.as_str())
                .then_some(Message::Custom(text))
        });
        if let Some(search) = list.search.as_mut() {
            search.prefer_custom = true;
            search.commit_on_leave = true;
        }
        list
    }

    #[test]
    fn preferring_custom_values_asks_the_app_before_the_highlight() {
        let list = preferring(&["s"]);
        let mut state = open(&list);
        list.set_query(&mut state, "s".to_owned());
        assert_eq!(state.highlight, Some(1), "Tuesday matches first");
        assert_eq!(
            named(&list, &mut state, Named::Enter),
            Outcome::Chosen(Message::Custom("s".to_owned()))
        );
        list.set_query(&mut state, "sat".to_owned());
        assert_eq!(
            named(&list, &mut state, Named::Enter),
            Outcome::Chosen(Message::Pick(5)),
            "turned down, so the highlight"
        );
        list.set_query(&mut state, "sunday".to_owned());
        assert_eq!(
            named(&list, &mut state, Named::Enter),
            Outcome::Chosen(Message::Pick(6)),
            "a whole label is that option"
        );
        list.set_query(&mut state, "zz".to_owned());
        assert_eq!(named(&list, &mut state, Named::Enter), Outcome::Used);
        assert!(state.rejected, "nothing to fall back to");
    }

    #[test]
    fn a_highlight_moved_by_the_keys_wins_over_a_preferred_custom_value() {
        let list = preferring(&["s"]);
        let mut state = open(&list);
        list.set_query(&mut state, "s".to_owned());
        named(&list, &mut state, Named::ArrowDown);
        assert_eq!(
            named(&list, &mut state, Named::Enter),
            Outcome::Chosen(Message::Pick(2))
        );
        list.set_query(&mut state, "s".to_owned());
        assert!(!state.steered, "editing hands it back to the text");
    }

    #[test]
    fn leaving_enters_typed_text_only_when_asked() {
        let committing = preferring(&["s"]);
        let mut state = open(&committing);
        assert_eq!(
            named(&committing, &mut state, Named::Tab),
            Outcome::Ignored,
            "nothing typed"
        );
        committing.set_query(&mut state, "s".to_owned());
        assert_eq!(
            named(&committing, &mut state, Named::Tab),
            Outcome::Chosen(Message::Custom("s".to_owned()))
        );
        committing.set_query(&mut state, "zz".to_owned());
        assert_eq!(named(&committing, &mut state, Named::Tab), Outcome::Used);
        assert!(state.rejected);

        let plain = list_with_search(true, true);
        let mut state = open(&plain);
        plain.set_query(&mut state, "zz".to_owned());
        assert_eq!(
            plain.leave(&mut state),
            Outcome::Ignored,
            "text that chooses nothing is dropped"
        );
        let off = list(true, true, true);
        let mut state = open(&off);
        off.set_query(&mut state, "s".to_owned());
        assert_eq!(off.leave(&mut state), Outcome::Ignored, "off by default");
    }

    fn list_with_search(filters: bool, commit: bool) -> List<'static, Message> {
        let mut list = list(filters, false, false);
        if let Some(search) = list.search.as_mut() {
            search.commit_on_leave = commit;
        }
        list
    }

    #[test]
    fn the_not_valid_line_shows_above_rows_and_in_place_of_none() {
        let custom = list(false, true, false);
        let mut state = open(&custom);
        assert_eq!(custom.top(&state), SEARCH_HEIGHT);
        custom.set_query(&mut state, "x".to_owned());
        named(&custom, &mut state, Named::Enter);
        assert!(custom.shows_notice(&state), "rows still show");
        assert_eq!(custom.top(&state), SEARCH_HEIGHT + NOTICE_HEIGHT);
        let filtering = list_with_search(true, false);
        let mut state = open(&filtering);
        filtering.set_query(&mut state, "x".to_owned());
        state.rejected = true;
        assert!(!filtering.shows_notice(&state), "said in place of rows");
        assert_eq!(filtering.message(&state), "Not valid");
    }

    #[test]
    fn the_line_in_place_of_rows_says_why() {
        let list = list(true, true, true);
        let mut state = open(&list);
        list.set_query(&mut state, "9:40".to_owned());
        assert_eq!(list.message(&state), "Use \u{201c}9:40\u{201d}");
        let plain = list_with_search(true, false);
        let mut state = open(&plain);
        plain.set_query(&mut state, "x".to_owned());
        assert_eq!(plain.message(&state), "No results found.");
        let mut empty = list_with_search(true, false);
        empty.rows.clear();
        empty.labels.clear();
        assert_eq!(empty.message(&ListState::default()), "No options");
    }

    #[test]
    fn a_field_on_the_closed_control_leaves_the_list_without_one() {
        let mut list = list(true, true, true);
        if let Some(search) = list.search.as_mut() {
            search.on_anchor = true;
        }
        assert!(!list.has_field());
        assert_eq!(list.first_row(), 0);
        assert_eq!(list.search_height(), 0.0);
        let mut state = open(&list);
        assert_eq!(typed(&list, &mut state, "s"), Outcome::Field);
        state.edit("sa".to_owned());
        assert!(state.refilter, "filtered on the next layout");
        assert_eq!(state.edited_query(), Some("sa"));
        list.refilter(&mut state);
        assert_eq!(state.shown, vec![5]);
    }

    #[test]
    fn without_a_search_field_chords_are_left_alone() {
        let list = list(false, false, false);
        let mut state = open(&list);
        let outcome = list.key(
            &mut state,
            &Key::Character("a".into()),
            Modifiers::CTRL,
            Some("a"),
        );
        assert_eq!(outcome, Outcome::Ignored);
        assert_eq!(state.highlight, Some(2));
    }

    #[test]
    fn the_field_text_turns_red_once_rejected() {
        let tokens = Tokens::of(&crate::theme::light());
        assert_eq!(field_style(&tokens, false, true).value, tokens.foreground);
        assert_eq!(
            field_style(&tokens, true, true).value,
            tokens.destructive_text
        );
        assert_eq!(
            field_style(&tokens, false, true).background,
            Background::Color(Color::TRANSPARENT)
        );
        assert_eq!(
            field_style(&tokens, false, false).value,
            fade(tokens.foreground, 0.5)
        );
    }

    #[test]
    fn only_some_events_reach_the_field() {
        let press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        assert!(for_field(&press, true));
        assert!(
            !for_field(&press, false),
            "presses on rows stay with the list"
        );
        let moved = Event::Mouse(mouse::Event::CursorMoved {
            position: Point::ORIGIN,
        });
        assert!(
            for_field(&moved, false),
            "a drag selecting text follows the pointer"
        );
        let key = Event::Keyboard(keyboard::Event::ModifiersChanged(Modifiers::SHIFT));
        assert!(for_field(&key, false));
    }

    #[test]
    fn programmatic_scrolls_stay_in_range() {
        let mut state = ListState {
            viewport: 100.0,
            content: 400.0,
            ..ListState::default()
        };
        state.scroll_to(AbsoluteOffset {
            x: None,
            y: Some(1000.0),
        });
        assert_eq!(state.offset, 300.0);
        state.snap_to(RelativeOffset {
            x: None,
            y: Some(0.5),
        });
        assert_eq!(state.offset, 150.0);
        state.scroll_by(
            AbsoluteOffset { x: 0.0, y: -500.0 },
            Rectangle::default(),
            Rectangle::default(),
        );
        assert_eq!(state.offset, 0.0);
    }
}
