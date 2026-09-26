//! Transient notifications stacked in a corner of the window.
//!
//! Toasts usually come from outside the UI thread, so they arrive through a
//! channel. [`subscription`] creates a bounded channel, emits
//! [`Event::Ready`] with its [`Sender`] once, then delivers whatever
//! producers send in batches as [`Event::Received`]. The app keeps the
//! sender, hands clones to producers and feeds every [`Event`] into its
//! [`State`]. [`timer`] ticks only while a visible toast is counting down.
//!
//! ```no_run
//! use iced::widget::text;
//! use iced::{Element, Subscription};
//! use iced_cube::overlay::toast::{self, toasts};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Toast(toast::Event),
//! }
//!
//! #[derive(Default)]
//! struct App {
//!     toasts: toast::State,
//!     sender: Option<toast::Sender>,
//! }
//!
//! impl App {
//!     fn update(&mut self, message: Message) {
//!         let Message::Toast(event) = message;
//!         if let Some(toast::Output::Ready(sender)) = self.toasts.update(event) {
//!             self.sender = Some(sender);
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         toasts(&self.toasts, text("Content"))
//!             .on_event(Message::Toast)
//!             .into()
//!     }
//!
//!     fn subscription(&self) -> Subscription<Message> {
//!         Subscription::batch([toast::subscription(), toast::timer(&self.toasts)])
//!             .map(Message::Toast)
//!     }
//! }
//! ```

use std::collections::VecDeque;

use iced::futures::channel::mpsc;
use iced::futures::{Stream, StreamExt, stream};
use iced::keyboard::key::Named;
use iced::time::{Duration, Instant};
use iced::widget::{self, column, container, opaque, row, stack, text};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Shadow, Subscription, Theme, Vector,
};

use crate::icon::{Glyph, tinted};
use crate::keys::{self, Chord, Keymap};
use crate::primitives::button::{self, Size};
use crate::theme::{Tokens, fade, mix, radius, space, text_size};

/// How many toasts a producer can send before it has to wait.
pub const CHANNEL_CAPACITY: usize = 32;
/// The most toasts delivered in one [`Event::Received`].
pub const BATCH_SIZE: usize = 16;
/// How long a toast stays visible unless it sets its own duration.
pub const DEFAULT_DURATION: Duration = Duration::from_secs(5);
/// How many toasts are shown at once. The rest wait their turn.
pub const DEFAULT_LIMIT: usize = 3;
/// How often [`timer`] checks for expired toasts.
pub const TICK: Duration = Duration::from_millis(250);

const WIDTH: f32 = 356.0;

/// The sending half of the toast channel. Clone it for each producer.
pub type Sender = mpsc::Sender<Toast>;

/// Colour and icon of a toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variant {
    #[default]
    Default,
    Success,
    Destructive,
}

impl Variant {
    pub const ALL: [Variant; 3] = [Variant::Default, Variant::Success, Variant::Destructive];

    fn glyph(self) -> Option<Glyph> {
        match self {
            Variant::Default => None,
            Variant::Success => Some(crate::lucide!(CircleCheck)),
            Variant::Destructive => Some(crate::lucide!(CircleX)),
        }
    }
}

/// The corner or edge the toasts stack in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Position {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    #[default]
    BottomRight,
}

impl Position {
    pub const ALL: [Position; 6] = [
        Position::TopLeft,
        Position::TopCenter,
        Position::TopRight,
        Position::BottomLeft,
        Position::BottomCenter,
        Position::BottomRight,
    ];

    fn alignment(self) -> (Alignment, Alignment) {
        match self {
            Position::TopLeft => (Alignment::Start, Alignment::Start),
            Position::TopCenter => (Alignment::Center, Alignment::Start),
            Position::TopRight => (Alignment::End, Alignment::Start),
            Position::BottomLeft => (Alignment::Start, Alignment::End),
            Position::BottomCenter => (Alignment::Center, Alignment::End),
            Position::BottomRight => (Alignment::End, Alignment::End),
        }
    }
}

/// One notification.
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub title: String,
    pub description: Option<String>,
    pub variant: Variant,
    /// Label of the action button.
    pub action: Option<String>,
    /// How long the toast stays once shown. `None` keeps it until closed.
    pub duration: Option<Duration>,
}

/// Creates a toast with a title that closes after [`DEFAULT_DURATION`].
pub fn toast(title: impl Into<String>) -> Toast {
    Toast {
        title: title.into(),
        description: None,
        variant: Variant::default(),
        action: None,
        duration: Some(DEFAULT_DURATION),
    }
}

impl Toast {
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Adds an action button. Pressing it closes the toast and returns
    /// [`Output::Action`].
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.action = Some(label.into());
        self
    }

    /// How long the toast stays visible once it is shown.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Keeps the toast until it is closed.
    pub fn persistent(mut self) -> Self {
        self.duration = None;
        self
    }
}

/// Identifies a toast in a [`State`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(u64);

/// Everything that changes the toast queue.
#[derive(Debug, Clone)]
pub enum Event {
    /// The channel is open. Keep the sender and hand clones to producers.
    Ready(Sender),
    /// A batch of toasts from the channel, oldest first.
    Received(Vec<Toast>),
    Dismiss(Id),
    Action(Id),
    Tick(Instant),
}

/// What the app may need to act on after an [`Event`].
#[derive(Debug, Clone)]
pub enum Output {
    Ready(Sender),
    /// The action button of this toast was pressed.
    Action(Id, Toast),
}

#[derive(Debug, Clone)]
struct Entry {
    id: Id,
    toast: Toast,
    expires_at: Option<Instant>,
}

/// The toast queue. The first [`limit`](State::limit) toasts are visible and
/// counting down; the rest wait until a visible one closes.
#[derive(Debug, Clone)]
pub struct State {
    entries: VecDeque<Entry>,
    next_id: u64,
    limit: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            next_id: 0,
            limit: DEFAULT_LIMIT,
        }
    }
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets how many toasts are visible at once. At least one is.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Queues a toast and returns its id.
    pub fn push(&mut self, toast: Toast) -> Id {
        let id = Id(self.next_id);
        self.next_id += 1;
        self.entries.push_back(Entry {
            id,
            toast,
            expires_at: None,
        });
        id
    }

    /// Removes a toast, visible or waiting.
    pub fn dismiss(&mut self, id: Id) -> Option<Toast> {
        let index = self.entries.iter().position(|entry| entry.id == id)?;
        self.entries.remove(index).map(|entry| entry.toast)
    }

    /// Closes expired toasts and starts the countdown of newly visible ones.
    /// Returns the ids that closed.
    pub fn tick(&mut self, now: Instant) -> Vec<Id> {
        let mut expired = Vec::new();
        self.entries.retain(|entry| {
            let alive = entry.expires_at.is_none_or(|at| at > now);
            if !alive {
                expired.push(entry.id);
            }
            alive
        });

        let limit = self.limit;
        for entry in self.entries.iter_mut().take(limit) {
            if entry.expires_at.is_none() {
                entry.expires_at = entry.toast.duration.map(|duration| now + duration);
            }
        }
        expired
    }

    pub fn update(&mut self, event: Event) -> Option<Output> {
        match event {
            Event::Ready(sender) => Some(Output::Ready(sender)),
            Event::Received(batch) => {
                for toast in batch {
                    let _ = self.push(toast);
                }
                None
            }
            Event::Dismiss(id) => {
                let _ = self.dismiss(id);
                None
            }
            Event::Action(id) => self.dismiss(id).map(|toast| Output::Action(id, toast)),
            Event::Tick(now) => {
                let _ = self.tick(now);
                None
            }
        }
    }

    /// The visible toasts, oldest first.
    pub fn visible(&self) -> impl Iterator<Item = (Id, &Toast)> {
        self.entries
            .iter()
            .take(self.limit)
            .map(|entry| (entry.id, &entry.toast))
    }

    /// How many toasts are waiting behind the visible ones.
    pub fn waiting(&self) -> usize {
        self.entries.len().saturating_sub(self.limit)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether a visible toast still has to count down.
    pub fn needs_tick(&self) -> bool {
        self.visible().any(|(_, toast)| toast.duration.is_some())
    }

    /// Every queued toast id, visible first, oldest first.
    pub fn ids(&self) -> impl Iterator<Item = Id> {
        self.entries.iter().map(|entry| entry.id)
    }
}

/// What a toast keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    DismissLatest,
    DismissAll,
}

impl Action {
    /// The events this action sends to `state`. Empty when nothing is queued.
    pub fn events(self, state: &State) -> Vec<Event> {
        match self {
            Action::DismissLatest => state
                .visible()
                .last()
                .map(|(id, _)| Event::Dismiss(id))
                .into_iter()
                .collect(),
            Action::DismissAll => state.ids().map(Event::Dismiss).collect(),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[Action::DismissLatest, Action::DismissAll];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::DismissLatest => "DismissLatest",
            Action::DismissAll => "DismissAll",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::DismissLatest => "Closes the newest visible toast.",
            Action::DismissAll => "Closes every toast, including those waiting their turn.",
        }
    }
}

/// The default toast shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `Escape` | [`Action::DismissLatest`] |
/// | `Shift+Escape` | [`Action::DismissAll`] |
///
/// Escape often closes dialogs and menus too, so resolve the toast keymap
/// after any overlay that should take Escape first.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::Escape), Action::DismissLatest)
        .bind(Chord::named(Named::Escape).shift(), Action::DismissAll)
}

/// Owns the toast channel. Emits [`Event::Ready`] first, then batches.
pub fn subscription() -> Subscription<Event> {
    Subscription::run(stream)
}

/// The stream behind [`subscription`], for driving it without a runtime.
pub fn stream() -> impl Stream<Item = Event> {
    let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);
    stream::once(async move { Event::Ready(sender) }).chain(batches(receiver).map(Event::Received))
}

/// Groups whatever is already waiting in the channel into one batch.
pub fn batches<T>(receiver: impl Stream<Item = T>) -> impl Stream<Item = Vec<T>> {
    receiver.ready_chunks(BATCH_SIZE)
}

/// Ticks every [`TICK`] while a visible toast is counting down.
pub fn timer(state: &State) -> Subscription<Event> {
    if !state.needs_tick() {
        return Subscription::none();
    }
    every(TICK).map(|_| Event::Tick(Instant::now()))
}

#[cfg(any(target_arch = "wasm32", feature = "tokio"))]
fn every(period: Duration) -> Subscription<()> {
    iced::time::every(period).map(|_| ())
}

/// A sleeping thread feeds a channel, so no async runtime timer is needed.
#[cfg(not(any(target_arch = "wasm32", feature = "tokio")))]
fn every(period: Duration) -> Subscription<()> {
    Subscription::run_with(period, |period| {
        let period = *period;
        let (mut sender, receiver) = mpsc::channel(1);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(period);
                if sender
                    .try_send(())
                    .is_err_and(|error| error.is_disconnected())
                {
                    break;
                }
            }
        });
        receiver
    })
}

/// The widget id of a toast's close button, for tests and focus.
pub fn close_button_id(id: Id) -> widget::Id {
    widget::Id::from(format!("iced-cube-toast-close-{}", id.0))
}

/// Renders the visible toasts of a [`State`] over some content.
pub struct Toasts<'a, Message> {
    state: &'a State,
    content: Element<'a, Message>,
    position: Position,
    on_event: Option<Box<dyn Fn(Event) -> Message + 'a>>,
}

/// Stacks the toasts of `state` over `content`. Without
/// [`on_event`](Toasts::on_event) the buttons render disabled.
pub fn toasts<'a, Message>(
    state: &'a State,
    content: impl Into<Element<'a, Message>>,
) -> Toasts<'a, Message> {
    Toasts {
        state,
        content: content.into(),
        position: Position::default(),
        on_event: None,
    }
}

impl<Message> std::fmt::Debug for Toasts<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Toasts")
            .field("state", self.state)
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> Toasts<'a, Message> {
    pub fn position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

impl<'a, Message: Clone + 'a> From<Toasts<'a, Message>> for Element<'a, Message> {
    fn from(toasts: Toasts<'a, Message>) -> Self {
        let Toasts {
            state,
            content,
            position,
            on_event,
        } = toasts;
        let emit = |event: Event| on_event.as_ref().map(|on_event| on_event(event));

        let cards = state.visible().map(|(id, toast)| card(id, toast, &emit));

        let (align_x, align_y) = position.alignment();
        let layer = container(column(cards).spacing(space::SM).width(WIDTH))
            .padding(space::LG)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(align_x)
            .align_y(align_y);

        // The layer is always present so the content keeps its widget state
        // when the first toast appears.
        stack![content, layer]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn card<'a, Message: Clone + 'a>(
    id: Id,
    toast: &'a Toast,
    emit: &impl Fn(Event) -> Option<Message>,
) -> Element<'a, Message> {
    let variant = toast.variant;

    let mut body = column![
        text(&toast.title)
            .size(text_size::SM)
            .font(crate::theme::semibold())
    ]
    .spacing(2)
    .width(Length::Fill);
    if let Some(description) = &toast.description {
        body = body.push(
            text(description)
                .size(text_size::SM)
                .style(move |theme: &Theme| text::Style {
                    color: Some(Tokens::of(theme).muted_foreground),
                }),
        );
    }

    let mut content = row![].spacing(space::MD).align_y(Alignment::Center);
    if let Some(glyph) = variant.glyph() {
        content = content.push(tinted(glyph, 18.0, None).style(move |theme: &Theme, _| {
            widget::svg::Style {
                color: Some(accent(&Tokens::of(theme), variant)),
            }
        }));
    }
    content = content.push(body);
    if let Some(label) = &toast.action {
        content = content.push(
            button::button(label.as_str())
                .variant(button::Variant::Outline)
                .size(Size::Sm)
                .on_press_maybe(emit(Event::Action(id))),
        );
    }
    content = content.push(
        container(
            button::icon_button(crate::lucide!(X))
                .size(Size::Sm)
                .on_press_maybe(emit(Event::Dismiss(id))),
        )
        .id(close_button_id(id)),
    );

    opaque(
        container(content)
            .padding([space::MD, space::LG])
            .width(Length::Fill)
            .style(move |theme| style(&Tokens::of(theme), variant)),
    )
}

/// The colour of a variant's icon and tint.
pub fn accent(tokens: &Tokens, variant: Variant) -> Color {
    match variant {
        Variant::Default => tokens.foreground,
        Variant::Success => tokens.success,
        Variant::Destructive => tokens.destructive,
    }
}

/// The card style of a toast.
pub fn style(tokens: &Tokens, variant: Variant) -> container::Style {
    let (background, border) = match variant {
        Variant::Default => (tokens.background, tokens.border),
        Variant::Success | Variant::Destructive => {
            let accent = accent(tokens, variant);
            (
                mix(tokens.background, accent, 0.08),
                mix(tokens.border, accent, 0.45),
            )
        }
    };
    let shadow_alpha = if tokens.is_dark { 0.5 } else { 0.12 };

    container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(tokens.foreground),
        border: Border {
            color: border,
            width: 1.0,
            radius: radius::LG.into(),
        },
        shadow: Shadow {
            color: fade(Color::BLACK, shadow_alpha),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        snap: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};
    use futures::SinkExt;
    use futures::executor::block_on;

    fn titles(state: &State) -> Vec<&str> {
        state
            .visible()
            .map(|(_, toast)| toast.title.as_str())
            .collect()
    }

    #[test]
    fn builder_defaults() {
        let t = toast("Saved");
        assert_eq!(t.title, "Saved");
        assert_eq!(t.variant, Variant::Default);
        assert_eq!(t.duration, Some(DEFAULT_DURATION));
        assert!(t.description.is_none());
        assert!(t.action.is_none());
        assert!(toast("Sticky").persistent().duration.is_none());
    }

    #[test]
    fn queue_keeps_arrival_order_and_unique_ids() {
        let mut state = State::new();
        let a = state.push(toast("a"));
        let b = state.push(toast("b"));
        let _ = state.update(Event::Received(vec![toast("c"), toast("d")]));

        assert_ne!(a, b);
        assert_eq!(state.len(), 4);
        assert_eq!(titles(&state), ["a", "b", "c"]);
        assert_eq!(state.waiting(), 1);
    }

    #[test]
    fn limit_caps_visible_toasts_and_is_at_least_one() {
        let mut state = State::new().with_limit(2);
        for title in ["a", "b", "c"] {
            let _ = state.push(toast(title));
        }
        assert_eq!(titles(&state), ["a", "b"]);
        assert_eq!(State::new().with_limit(0).limit(), 1);
    }

    #[test]
    fn toasts_expire_using_injected_time() {
        let start = Instant::now();
        let mut state = State::new();
        let id = state.push(toast("a").duration(Duration::from_secs(2)));

        assert!(state.tick(start).is_empty());
        assert!(state.tick(start + Duration::from_millis(1999)).is_empty());
        assert_eq!(state.tick(start + Duration::from_secs(2)), vec![id]);
        assert!(state.is_empty());
    }

    #[test]
    fn waiting_toasts_start_counting_when_shown() {
        let start = Instant::now();
        let mut state = State::new().with_limit(1);
        let _ = state.push(toast("a").duration(Duration::from_secs(1)));
        let _ = state.push(toast("b").duration(Duration::from_secs(1)));

        let _ = state.tick(start);
        let _ = state.tick(start + Duration::from_secs(1));
        assert_eq!(titles(&state), ["b"]);

        // "b" became visible at 1s, so it lives until 2s.
        assert!(state.tick(start + Duration::from_millis(1500)).is_empty());
        assert_eq!(state.tick(start + Duration::from_secs(2)).len(), 1);
    }

    #[test]
    fn persistent_toasts_never_expire_and_need_no_timer() {
        let start = Instant::now();
        let mut state = State::new();
        let _ = state.push(toast("a").persistent());
        assert!(!state.needs_tick());

        let _ = state.tick(start + Duration::from_secs(3600));
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn empty_state_needs_no_timer() {
        let mut state = State::new();
        assert!(!state.needs_tick());
        let _ = state.push(toast("a"));
        assert!(state.needs_tick());
    }

    #[test]
    fn dismiss_and_action_remove_the_toast() {
        let mut state = State::new();
        let a = state.push(toast("a"));
        let b = state.push(toast("b").action("Undo"));

        assert!(state.update(Event::Dismiss(a)).is_none());
        let Some(Output::Action(id, toast)) = state.update(Event::Action(b)) else {
            panic!("expected an action output");
        };
        assert_eq!(id, b);
        assert_eq!(toast.action.as_deref(), Some("Undo"));
        assert!(state.is_empty());
        assert!(state.update(Event::Action(b)).is_none());
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_and_overrides() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "Escape"), Some(Action::DismissLatest));
        assert_eq!(press(&keymap, "Shift+Escape"), Some(Action::DismissAll));
        assert_eq!(press(&keymap, "Ctrl+Escape"), None);

        let custom = keymap
            .unbind(&"Shift+Escape".parse().unwrap())
            .bind("Ctrl+Shift+X".parse().unwrap(), Action::DismissAll);
        assert_eq!(press(&custom, "Shift+Escape"), None);
        assert_eq!(press(&custom, "ctrl+shift+x"), Some(Action::DismissAll));
    }

    #[test]
    fn dismiss_latest_closes_the_newest_visible_toast() {
        let mut state = State::new().with_limit(2);
        let _ = state.push(toast("a"));
        let b = state.push(toast("b"));
        let _ = state.push(toast("c"));

        let events = Action::DismissLatest.events(&state);
        assert!(matches!(events.as_slice(), [Event::Dismiss(id)] if *id == b));
        for event in events {
            let _ = state.update(event);
        }
        assert_eq!(titles(&state), ["a", "c"]);
        assert!(Action::DismissLatest.events(&State::new()).is_empty());
    }

    #[test]
    fn dismiss_all_includes_waiting_toasts() {
        let mut state = State::new().with_limit(1);
        for title in ["a", "b", "c"] {
            let _ = state.push(toast(title));
        }
        let events = Action::DismissAll.events(&state);
        assert_eq!(events.len(), 3);
        for event in events {
            let _ = state.update(event);
        }
        assert!(state.is_empty());
        assert!(Action::DismissAll.events(&state).is_empty());
    }

    #[test]
    fn ready_hands_the_sender_back() {
        let (sender, _receiver) = mpsc::channel(1);
        let output = State::new().update(Event::Ready(sender));
        assert!(matches!(output, Some(Output::Ready(_))));
    }

    #[test]
    fn stream_emits_ready_first_then_batches() {
        let mut events = Box::pin(stream());
        let Some(Event::Ready(mut sender)) = block_on(events.next()) else {
            panic!("first event must be Ready");
        };

        block_on(async {
            for title in ["a", "b", "c"] {
                sender.send(toast(title)).await.ok();
            }
        });
        let Some(Event::Received(batch)) = block_on(events.next()) else {
            panic!("expected a batch");
        };
        let titles: Vec<_> = batch.iter().map(|toast| toast.title.as_str()).collect();
        assert_eq!(titles, ["a", "b", "c"]);
    }

    #[test]
    fn batches_split_bursts_at_batch_size() {
        let items = stream::iter(0..BATCH_SIZE + 3);
        let sizes: Vec<_> = block_on(batches(items).map(|batch| batch.len()).collect());
        assert_eq!(sizes, [BATCH_SIZE, 3]);
    }

    #[test]
    fn default_style_uses_background_and_border_tokens() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let style = style(&tokens, Variant::Default);
            assert_eq!(style.background, Some(Background::Color(tokens.background)));
            assert_eq!(style.border.color, tokens.border);
            assert_eq!(style.text_color, Some(tokens.foreground));
        }
    }

    #[test]
    fn variants_are_tinted_differently_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let styles = Variant::ALL.map(|variant| style(&tokens, variant).border.color);
            assert_ne!(styles[0], styles[1]);
            assert_ne!(styles[1], styles[2]);
            assert_eq!(accent(&tokens, Variant::Destructive), tokens.destructive);
            assert_eq!(accent(&tokens, Variant::Success), tokens.success);
        }
    }

    #[test]
    fn dark_shadow_is_stronger() {
        let light = style(&Tokens::of(&light()), Variant::Default)
            .shadow
            .color
            .a;
        let dark = style(&Tokens::of(&dark()), Variant::Default).shadow.color.a;
        assert!(dark > light);
    }
}
