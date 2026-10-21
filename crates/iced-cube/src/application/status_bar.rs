//! A thin bar along the bottom edge of a window, with start, centre and
//! end sections of small items: text, an icon with text, a clickable
//! item, a spinner, a progress bar and a badge.
//!
//! Status usually comes from outside the UI thread, such as a sync job, a
//! build or the editor's cursor, so items arrive through a channel.
//! [`subscription`] creates a bounded channel, emits [`Event::Ready`] with
//! its [`Sender`] once, then delivers whatever producers send in batches as
//! [`Event::Received`]. Producers send [`Update`]s: set an item by its id,
//! replacing any item with the same id, or remove one. [`animation`] ticks
//! only while a spinner is showing.
//!
//! ```no_run
//! use iced::{Element, Subscription};
//! use iced_cube::status_bar::{self, Section, Update, status_bar};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Status(status_bar::Event<&'static str>),
//! }
//!
//! #[derive(Default)]
//! struct App {
//!     status: status_bar::State<&'static str>,
//!     sender: Option<status_bar::Sender<&'static str>>,
//! }
//!
//! impl App {
//!     fn update(&mut self, message: Message) {
//!         let Message::Status(event) = message;
//!         if let Some(status_bar::Output::Ready(mut sender)) = self.status.update(event) {
//!             // Hand clones of the sender to worker threads or tasks.
//!             let _ = sender.try_send(Update::Set(
//!                 status_bar::text("cursor", "Ln 1, Col 1").section(Section::End),
//!             ));
//!             self.sender = Some(sender);
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         status_bar(&self.status).on_event(Message::Status).into()
//!     }
//!
//!     fn subscription(&self) -> Subscription<Message> {
//!         Subscription::batch([
//!             status_bar::subscription(),
//!             status_bar::animation(&self.status),
//!         ])
//!         .map(Message::Status)
//!     }
//! }
//! ```

use std::rc::Rc;

use iced::futures::channel::mpsc;
use iced::futures::{Stream, StreamExt, stream};
use iced::time::Instant;
use iced::widget::text::LineHeight;
use iced::widget::{self, column, container, responsive, row, space};
use iced::{Alignment, Background, Element, Length, Padding, Subscription, Theme, window};

use crate::feedback::badge::{self};
use crate::feedback::progress::{self, progress as progress_bar};
use crate::feedback::spinner::{self, advance, spinner as spinning};
use crate::icon::{Glyph, opacity, themed};
use crate::overlay::tooltip::{Position, tooltip};
use crate::primitives::button::{self, Variant};
use crate::theme::{Tokens, space as gap, text_size};

/// How many updates a producer can send before it has to wait.
pub const CHANNEL_CAPACITY: usize = 32;
/// The most updates delivered in one [`Event::Received`].
pub const BATCH_SIZE: usize = 16;
/// The id [`subscription`] runs under, so iced keeps one stream alive.
pub const SUBSCRIPTION_ID: &str = "iced-cube-status-bar";
/// Below this width the sections wrap onto several lines.
pub const COMPACT_WIDTH: f32 = 480.0;
/// Height of every item's line.
pub const LINE: f32 = 16.0;
/// Width of a progress item's bar.
pub const PROGRESS_WIDTH: f32 = 80.0;

const ICON_SIZE: f32 = 14.0;

/// Where an item sits along the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Section {
    #[default]
    Start,
    Centre,
    End,
}

impl Section {
    pub const ALL: [Section; 3] = [Section::Start, Section::Centre, Section::End];
}

/// What an item shows besides its label and icon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Content {
    /// Muted text.
    Text,
    /// A small ghost button that sends [`Event::Press`].
    Action,
    /// A spinner before the label, animated by [`animation`].
    Spinner,
    /// A short progress bar after the label, from 0.0 to 1.0.
    Progress(f32),
    /// A badge holding the label.
    Badge(badge::Variant),
}

/// One item of the bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Item<Id> {
    pub id: Id,
    pub section: Section,
    pub label: String,
    pub icon: Option<Glyph>,
    pub tooltip: Option<String>,
    pub content: Content,
}

fn new_item<Id>(id: Id, label: impl Into<String>, content: Content) -> Item<Id> {
    Item {
        id,
        section: Section::default(),
        label: label.into(),
        icon: None,
        tooltip: None,
        content,
    }
}

/// Muted text, such as a cursor position. Add `.icon(...)` for an icon.
pub fn text<Id>(id: Id, label: impl Into<String>) -> Item<Id> {
    new_item(id, label, Content::Text)
}

/// A clickable item, drawn as a small ghost button.
pub fn action<Id>(id: Id, label: impl Into<String>) -> Item<Id> {
    new_item(id, label, Content::Action)
}

/// A spinner with a label, for work in progress.
pub fn spinner<Id>(id: Id, label: impl Into<String>) -> Item<Id> {
    new_item(id, label, Content::Spinner)
}

/// A short progress bar, clamped to 0.0..=1.0. Add `.label(...)` to
/// show text before it.
pub fn progress<Id>(id: Id, value: f32) -> Item<Id> {
    new_item(id, "", Content::Progress(progress::clamp(value)))
}

/// A badge, such as an error count.
pub fn badge<Id>(id: Id, label: impl Into<String>, variant: badge::Variant) -> Item<Id> {
    new_item(id, label, Content::Badge(variant))
}

impl<Id> Item<Id> {
    pub fn section(mut self, section: Section) -> Self {
        self.section = section;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Adds a Lucide icon before the label.
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    /// Shown above the item while the pointer rests on it.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// What a producer sends through the channel.
#[derive(Debug, Clone, PartialEq)]
pub enum Update<Id> {
    /// Adds an item, or replaces the item with the same id where it stands.
    Set(Item<Id>),
    Remove(Id),
    Clear,
}

/// The sending half of the status channel. Clone it for each producer.
pub type Sender<Id> = mpsc::Sender<Update<Id>>;

/// Changes to a status bar.
#[derive(Debug, Clone)]
pub enum Event<Id> {
    /// The channel is open. Keep the sender and hand clones to producers.
    Ready(Sender<Id>),
    /// Updates from the channel, oldest first.
    Received(Vec<Update<Id>>),
    /// A window frame, which moves the spinners on.
    Frame(Instant),
    /// A clickable item was pressed.
    Press(Id),
}

/// What the app may need to act on after an [`Event`].
#[derive(Debug, Clone)]
pub enum Output<Id> {
    Ready(Sender<Id>),
    /// A clickable item was pressed.
    Pressed(Id),
}

/// The items and the spinners' phase.
#[derive(Debug, Clone, PartialEq)]
pub struct State<Id> {
    items: Vec<Item<Id>>,
    phase: f32,
    last_frame: Option<Instant>,
}

impl<Id> Default for State<Id> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            phase: 0.0,
            last_frame: None,
        }
    }
}

impl<Id: PartialEq> State<Id> {
    /// An empty bar.
    pub fn new() -> Self {
        Self::default()
    }

    /// A bar starting with `items`, as if each had been set in turn.
    pub fn with_items(mut self, items: impl IntoIterator<Item = Item<Id>>) -> Self {
        for item in items {
            self.apply(Update::Set(item));
        }
        self
    }

    /// Every item, in the order it was first set.
    pub fn items(&self) -> &[Item<Id>] {
        &self.items
    }

    pub fn item(&self, id: &Id) -> Option<&Item<Id>> {
        self.items.iter().find(|item| item.id == *id)
    }

    /// The items in one section, in order.
    pub fn section(&self, section: Section) -> impl Iterator<Item = &Item<Id>> {
        self.items
            .iter()
            .filter(move |item| item.section == section)
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The spinners' phase, in turns.
    pub fn phase(&self) -> f32 {
        self.phase
    }

    /// Whether any spinner is showing, so frames are needed.
    pub fn needs_frames(&self) -> bool {
        self.items
            .iter()
            .any(|item| item.content == Content::Spinner)
    }

    /// Applies one update, as the channel does.
    pub fn apply(&mut self, update: Update<Id>) {
        match update {
            Update::Set(item) => match self.items.iter_mut().find(|old| old.id == item.id) {
                Some(old) => *old = item,
                None => self.items.push(item),
            },
            Update::Remove(id) => self.items.retain(|item| item.id != id),
            Update::Clear => self.items.clear(),
        }
        if !self.needs_frames() {
            self.last_frame = None;
        }
    }

    /// Applies an event and returns what the app may need to act on.
    pub fn update(&mut self, event: Event<Id>) -> Option<Output<Id>> {
        match event {
            Event::Ready(sender) => return Some(Output::Ready(sender)),
            Event::Received(batch) => {
                for update in batch {
                    self.apply(update);
                }
            }
            Event::Frame(now) => {
                if let Some(last) = self.last_frame {
                    self.phase = advance(self.phase, now.saturating_duration_since(last));
                }
                self.last_frame = Some(now);
            }
            Event::Press(id) => {
                let pressable = self
                    .item(&id)
                    .is_some_and(|item| item.content == Content::Action);
                return pressable.then_some(Output::Pressed(id));
            }
        }
        None
    }
}

/// Owns the status channel. Emits [`Event::Ready`] first, then batches.
/// It runs under [`SUBSCRIPTION_ID`], so iced keeps the same stream
/// however often the app asks for it.
pub fn subscription<Id: Send + 'static>() -> Subscription<Event<Id>> {
    Subscription::run_with(SUBSCRIPTION_ID, |_| stream::<Id>())
}

/// The stream behind [`subscription`], for driving it without a runtime.
pub fn stream<Id: Send + 'static>() -> impl Stream<Item = Event<Id>> {
    let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);
    stream::once(async move { Event::Ready(sender) }).chain(batches(receiver).map(Event::Received))
}

/// Groups whatever is already waiting in the channel into one batch.
pub fn batches<T>(receiver: impl Stream<Item = T>) -> impl Stream<Item = Vec<T>> {
    receiver.ready_chunks(BATCH_SIZE)
}

/// Window frames while a spinner is showing, and nothing otherwise.
pub fn animation<Id: PartialEq + 'static>(state: &State<Id>) -> Subscription<Event<Id>> {
    if !state.needs_frames() {
        return Subscription::none();
    }
    window::frames().map(Event::Frame)
}

/// The bar: the page background with muted text, below a line along its
/// top edge. Progress tracks and badges read against it in both themes.
pub fn bar_style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens.background)),
        text_color: Some(tokens.muted_foreground),
        ..container::Style::default()
    }
}

/// The colour of text and icons in the bar.
pub fn text_colour(tokens: &Tokens) -> iced::Color {
    tokens.muted_foreground
}

type OnEvent<'a, Id, Message> = Rc<dyn Fn(Event<Id>) -> Message + 'a>;

/// A status bar builder. Convert it into an [`Element`] to render.
pub struct StatusBar<'a, Id, Message> {
    state: &'a State<Id>,
    compact_below: f32,
    on_event: Option<Box<dyn Fn(Event<Id>) -> Message + 'a>>,
}

impl<Id: std::fmt::Debug, Message> std::fmt::Debug for StatusBar<'_, Id, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatusBar")
            .field("state", self.state)
            .field("compact_below", &self.compact_below)
            .finish_non_exhaustive()
    }
}

/// Renders the items of `state`. Without [`on_event`](StatusBar::on_event)
/// clickable items render disabled.
pub fn status_bar<'a, Id, Message>(state: &'a State<Id>) -> StatusBar<'a, Id, Message> {
    StatusBar {
        state,
        compact_below: COMPACT_WIDTH,
        on_event: None,
    }
}

impl<'a, Id, Message> StatusBar<'a, Id, Message> {
    /// Below this width, the items flow onto as many lines as they need
    /// instead of splitting into three sections. Defaults to
    /// [`COMPACT_WIDTH`].
    pub fn compact_below(mut self, width: f32) -> Self {
        self.compact_below = width;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Id>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

impl<'a, Id, Message> From<StatusBar<'a, Id, Message>> for Element<'a, Message>
where
    Id: Clone + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(bar: StatusBar<'a, Id, Message>) -> Self {
        let StatusBar {
            state,
            compact_below,
            on_event,
        } = bar;
        let on_event: Option<OnEvent<'a, Id, Message>> = on_event.map(Rc::from);

        let content = responsive(move |size| {
            let items = |section: Section| -> Vec<Element<'a, Message>> {
                state
                    .section(section)
                    .map(|item| view_item(item, state.phase, on_event.as_ref()))
                    .collect()
            };
            if size.width < compact_below {
                let all = Section::ALL.into_iter().flat_map(items);
                return row(all)
                    .spacing(gap::MD)
                    .align_y(Alignment::Center)
                    .wrap()
                    .vertical_spacing(gap::XS)
                    .into();
            }
            let section = |section| {
                row(items(section))
                    .spacing(gap::MD)
                    .align_y(Alignment::Center)
            };
            row![
                section(Section::Start),
                space::horizontal(),
                section(Section::Centre),
                space::horizontal(),
                section(Section::End),
            ]
            .spacing(gap::MD)
            .align_y(Alignment::Center)
            .into()
        })
        .height(Length::Shrink);

        column![
            crate::application::edge(),
            container(content)
                .padding(Padding::from([gap::XS, gap::SM]))
                .width(Length::Fill)
                .style(|theme| bar_style(&Tokens::of(theme))),
        ]
        .width(Length::Fill)
        .into()
    }
}

fn muted_text<'a>(label: String) -> widget::Text<'a, Theme> {
    widget::text(label)
        .size(text_size::XS)
        .line_height(LineHeight::Absolute(LINE.into()))
        .wrapping(widget::text::Wrapping::None)
}

fn glyph<'a, Message: 'a>(glyph: Glyph, enabled: bool) -> Element<'a, Message> {
    themed(glyph, ICON_SIZE, opacity(enabled), |theme| {
        text_colour(&Tokens::of(theme))
    })
    .into()
}

fn view_item<'a, Id, Message>(
    item: &Item<Id>,
    phase: f32,
    on_event: Option<&OnEvent<'a, Id, Message>>,
) -> Element<'a, Message>
where
    Id: Clone + 'a,
    Message: Clone + 'a,
{
    let label = || -> Option<Element<'a, Message>> {
        (!item.label.is_empty()).then(|| {
            muted_text(item.label.clone())
                .style(|theme| widget::text::Style {
                    color: Some(text_colour(&Tokens::of(theme))),
                })
                .into()
        })
    };
    let icon = item.icon.map(|icon| glyph(icon, true));

    let element: Element<'a, Message> = match item.content {
        Content::Text => labelled(icon.into_iter().chain(label())),
        Content::Spinner => {
            labelled(std::iter::once(spinning(phase).size(spinner::Size::Sm).into()).chain(label()))
        }
        Content::Progress(value) => labelled(
            icon.into_iter().chain(label()).chain(std::iter::once(
                progress_bar(value)
                    .size(progress::Size::Sm)
                    .width(PROGRESS_WIDTH)
                    .into(),
            )),
        ),
        Content::Badge(variant) => {
            let badge = badge::badge(item.label.clone()).variant(variant);
            match item.icon {
                Some(icon) => badge.icon(icon).into(),
                None => badge.into(),
            }
        }
        Content::Action => action_button(item, on_event),
    };
    match &item.tooltip {
        Some(label) => tooltip(element, label.clone())
            .position(Position::Top)
            .into(),
        None => element,
    }
}

fn labelled<'a, Message: 'a>(
    parts: impl Iterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> {
    row(parts)
        .spacing(gap::XS + 2.0)
        .align_y(Alignment::Center)
        .into()
}

fn action_button<'a, Id, Message>(
    item: &Item<Id>,
    on_event: Option<&OnEvent<'a, Id, Message>>,
) -> Element<'a, Message>
where
    Id: Clone + 'a,
    Message: Clone + 'a,
{
    let enabled = on_event.is_some();
    let mut content = row![].spacing(gap::XS + 2.0).align_y(Alignment::Center);
    if let Some(icon) = item.icon {
        content = content.push(themed(icon, ICON_SIZE, opacity(enabled), |theme| {
            Tokens::of(theme).foreground
        }));
    }
    content = content.push(muted_text(item.label.clone()));
    widget::button(content)
        .padding(Padding::from([2.0, 6.0]))
        .style(|theme, status| button::style(&Tokens::of(theme), Variant::Ghost, status))
        .on_press_maybe(on_event.map(|on_event| on_event(Event::Press(item.id.clone()))))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};
    use iced::futures::SinkExt;
    use iced::futures::executor::block_on;

    fn sample() -> State<&'static str> {
        State::new().with_items([
            text("branch", "main").icon(crate::lucide!(GitBranch)),
            spinner("sync", "Syncing"),
            progress("build", 0.4)
                .label("Building")
                .section(Section::Centre),
            action("problems", "2 problems").section(Section::End),
            badge("mode", "Insert", badge::Variant::Secondary).section(Section::End),
        ])
    }

    #[test]
    fn builders_set_every_field() {
        let item = text("cursor", "Ln 1, Col 1")
            .icon(crate::lucide!(MousePointer))
            .section(Section::End)
            .tooltip("Go to line");
        assert_eq!(item.id, "cursor");
        assert_eq!(item.label, "Ln 1, Col 1");
        assert_eq!(item.icon, Some(crate::lucide!(MousePointer)));
        assert_eq!(item.section, Section::End);
        assert_eq!(item.tooltip.as_deref(), Some("Go to line"));
        assert_eq!(item.content, Content::Text);

        assert_eq!(action(1, "Run").content, Content::Action);
        assert_eq!(spinner(1, "Busy").content, Content::Spinner);
        assert_eq!(progress(1, 1.5).content, Content::Progress(1.0));
        assert_eq!(progress(1, f32::NAN).content, Content::Progress(0.0));
        assert!(progress(1, 0.5).label.is_empty());
        assert_eq!(
            badge(1, "3", badge::Variant::Destructive).content,
            Content::Badge(badge::Variant::Destructive)
        );
        assert_eq!(Section::default(), Section::Start);
    }

    #[test]
    fn items_keep_their_order_within_sections() {
        let state = sample();
        let start: Vec<_> = state.section(Section::Start).map(|item| item.id).collect();
        let end: Vec<_> = state.section(Section::End).map(|item| item.id).collect();
        assert_eq!(start, ["branch", "sync"]);
        assert_eq!(end, ["problems", "mode"]);
        assert_eq!(state.items().len(), 5);
        assert!(!state.is_empty());
    }

    #[test]
    fn set_replaces_in_place_and_remove_and_clear_delete() {
        let mut state = sample();
        state.apply(Update::Set(text("sync", "Synced")));
        assert_eq!(state.items()[1].label, "Synced");
        assert_eq!(state.items()[1].content, Content::Text);
        assert_eq!(state.items().len(), 5);

        state.apply(Update::Remove("build"));
        assert!(state.item(&"build").is_none());
        state.apply(Update::Remove("missing"));
        assert_eq!(state.items().len(), 4);

        state.apply(Update::Clear);
        assert!(state.is_empty());
    }

    #[test]
    fn a_batch_applies_in_order() {
        let mut state: State<&str> = State::new();
        let output = state.update(Event::Received(vec![
            Update::Set(text("a", "one")),
            Update::Set(text("b", "two")),
            Update::Set(text("a", "three")),
            Update::Remove("b"),
        ]));
        assert!(output.is_none());
        assert_eq!(state.items().len(), 1);
        assert_eq!(state.items()[0].label, "three");
        assert!(state.update(Event::Received(Vec::new())).is_none());
    }

    #[test]
    fn frames_move_the_spinners_only_while_one_shows() {
        let mut state = sample();
        assert!(state.needs_frames());
        let start = Instant::now();
        let _ = state.update(Event::Frame(start));
        assert_eq!(state.phase(), 0.0, "the first frame only starts the clock");
        let _ = state.update(Event::Frame(start + spinner::PERIOD / 4));
        assert!((state.phase() - 0.25).abs() < 1e-3);

        state.apply(Update::Remove("sync"));
        assert!(!state.needs_frames());
        assert!(
            state.last_frame.is_none(),
            "the clock stops with the spinner"
        );
    }

    #[test]
    fn only_clickable_items_report_presses() {
        let mut state = sample();
        assert!(matches!(
            state.update(Event::Press("problems")),
            Some(Output::Pressed("problems"))
        ));
        assert!(state.update(Event::Press("branch")).is_none());
        assert!(state.update(Event::Press("missing")).is_none());
    }

    #[test]
    fn ready_hands_the_sender_back() {
        let (sender, _receiver) = mpsc::channel(1);
        let output = State::<u8>::new().update(Event::Ready(sender));
        assert!(matches!(output, Some(Output::Ready(_))));
    }

    #[test]
    fn stream_emits_ready_first_then_batches() {
        let mut events = Box::pin(stream::<u8>());
        let Some(Event::Ready(mut sender)) = block_on(events.next()) else {
            panic!("first event must be Ready");
        };
        block_on(async {
            for n in 0..3 {
                sender.send(Update::Set(text(n, format!("{n}")))).await.ok();
            }
        });
        let Some(Event::Received(batch)) = block_on(events.next()) else {
            panic!("expected a batch");
        };
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn batches_split_bursts_at_batch_size() {
        let items = stream::iter(0..BATCH_SIZE + 3);
        let sizes: Vec<_> = block_on(batches(items).map(|batch| batch.len()).collect());
        assert_eq!(sizes, [BATCH_SIZE, 3]);
    }

    #[test]
    fn builder_defaults_and_overrides() {
        let state = sample();
        let bar: StatusBar<'_, &str, ()> = status_bar(&state);
        assert_eq!(bar.compact_below, COMPACT_WIDTH);
        assert!(bar.on_event.is_none());
        let bar: StatusBar<'_, &str, ()> = status_bar(&state).compact_below(0.0).on_event(|_| ());
        assert_eq!(bar.compact_below, 0.0);
        assert!(bar.on_event.is_some());
    }

    #[test]
    fn the_bar_has_muted_text_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let style = bar_style(&tokens);
            assert_eq!(style.background, Some(Background::Color(tokens.background)));
            assert_eq!(style.text_color, Some(tokens.muted_foreground));
            assert_eq!(text_colour(&tokens), tokens.muted_foreground);
        }
    }
}
