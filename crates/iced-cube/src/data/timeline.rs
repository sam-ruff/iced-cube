//! A timeline across a sequence of steps, such as the actions of a recorded
//! session or the frames of a run: a tick per step coloured by its tone, a
//! playhead on the current step, and optional lanes below the track that
//! show spans over ranges of steps, such as tasks that ran across several.
//!
//! The app owns the current step. Pressing or dragging along the track, or
//! a shortcut from [`default_keymap`], picks another one. Shortcuts act
//! only while the timeline has keyboard focus (Tab reaches it) or was the
//! last thing pressed, so several on one screen never fight over keys.

use std::borrow::Cow;

use iced::advanced::layout::{self, Layout};
use iced::advanced::text as core_text;
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, renderer};
use iced::alignment::Vertical;
use iced::keyboard::{self, key::Named};
use iced::widget::text::{self, LineHeight, Shaping, Wrapping};
use iced::{
    Border, Color, Element, Event, Length, Pixels, Point, Rectangle, Renderer, Size, Theme, mouse,
    touch,
};

use crate::focus;
use crate::keys::{self, Chord, Keymap};
use crate::theme::{self, Tokens, fade, mix, radius, space, text_size};

/// Height of the track row.
pub const TRACK_HEIGHT: f32 = 28.0;
/// Height of one lane row.
pub const LANE_HEIGHT: f32 = 20.0;
/// Width of the lane labels when the timeline has lanes.
pub const LABEL_WIDTH: f32 = 96.0;
/// Radius of the playhead.
pub const PLAYHEAD_RADIUS: f32 = 6.0;
/// Steps that a page action moves.
pub const PAGE: usize = 10;

const RAIL_HEIGHT: f32 = 4.0;
const TICK_WIDTH: f32 = 2.0;
const BAR_HEIGHT: f32 = 8.0;

/// What a step or span signals.
pub use crate::theme::Tone;

/// A run of steps in a lane, from `start` to `end` inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub tone: Tone,
}

impl Span {
    /// A neutral span from `start` to `end`, in either order.
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start: start.min(end),
            end: start.max(end),
            tone: Tone::Neutral,
        }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

/// A labelled row of spans below the track.
#[derive(Debug, Clone, PartialEq)]
pub struct Lane<'a> {
    label: text::Fragment<'a>,
    spans: Vec<Span>,
}

/// Creates a lane named `label` holding `spans`.
pub fn lane<'a>(
    label: impl text::IntoFragment<'a>,
    spans: impl IntoIterator<Item = Span>,
) -> Lane<'a> {
    Lane {
        label: label.into_fragment(),
        spans: spans.into_iter().collect(),
    }
}

impl Lane<'_> {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn spans(&self) -> &[Span] {
        &self.spans
    }
}

/// A timeline builder. Convert it into an [`Element`] to render.
///
/// A timeline without a message is rendered disabled.
pub struct Timeline<'a, Message> {
    steps: Vec<Tone>,
    current: usize,
    lanes: Vec<Lane<'a>>,
    label_width: f32,
    width: Length,
    name: text::Fragment<'a>,
    keymap: Keymap<Action>,
    on_select: Option<Box<dyn Fn(usize) -> Message + 'a>>,
}

impl<Message> std::fmt::Debug for Timeline<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Timeline")
            .field("steps", &self.steps.len())
            .field("current", &self.current)
            .field("lanes", &self.lanes)
            .field("label_width", &self.label_width)
            .field("width", &self.width)
            .field("name", &self.name)
            .field("keymap", &self.keymap)
            .field("enabled", &self.on_select.is_some())
            .finish()
    }
}

/// Creates a timeline over `steps`, one tone per step, with `current`
/// selected. The current step is clamped to the steps.
pub fn timeline<'a, Message>(
    steps: impl IntoIterator<Item = Tone>,
    current: usize,
) -> Timeline<'a, Message> {
    let steps: Vec<Tone> = steps.into_iter().collect();
    Timeline {
        current: current.min(steps.len().saturating_sub(1)),
        steps,
        lanes: Vec::new(),
        label_width: LABEL_WIDTH,
        width: Length::Fill,
        name: Cow::Borrowed("Timeline"),
        keymap: default_keymap(),
        on_select: None,
    }
}

impl<'a, Message> Timeline<'a, Message> {
    /// Adds a lane of spans below the track.
    pub fn lane(mut self, lane: Lane<'a>) -> Self {
        self.lanes.push(lane);
        self
    }

    /// Adds several lanes below the track.
    pub fn lanes(mut self, lanes: impl IntoIterator<Item = Lane<'a>>) -> Self {
        self.lanes.extend(lanes);
        self
    }

    /// Width of the lane labels. Only used when there are lanes.
    pub fn label_width(mut self, width: f32) -> Self {
        self.label_width = width.max(0.0);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// The name tests and assistive tools know it by. "Timeline" by default.
    pub fn label(mut self, label: impl text::IntoFragment<'a>) -> Self {
        self.name = label.into_fragment();
        self
    }

    /// Sets the message sent with the step under the pointer while
    /// pressing or dragging, or the step a shortcut moves to.
    pub fn on_select(mut self, on_select: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// Replaces the [`default_keymap`] the timeline resolves keys with
    /// while it has keyboard focus or was the last thing pressed.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn current(&self) -> usize {
        self.current
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn is_enabled(&self) -> bool {
        self.on_select.is_some()
    }

    fn labels(&self) -> f32 {
        if self.lanes.is_empty() {
            0.0
        } else {
            self.label_width
        }
    }
}

/// The step under `x`, for a track `width` wide holding `count` steps
/// placed evenly from its left edge to its right edge.
pub fn index_at(x: f32, width: f32, count: usize) -> usize {
    if count < 2 || width <= 0.0 {
        return 0;
    }
    let last = count - 1;
    let fraction = (x / width).clamp(0.0, 1.0);
    let index = (fraction * last as f32).round();
    (index as usize).min(last)
}

/// Where step `index` sits along a track `width` wide.
pub fn position_of(index: usize, width: f32, count: usize) -> f32 {
    if count < 2 {
        return 0.0;
    }
    width * index.min(count - 1) as f32 / (count - 1) as f32
}

/// The left and right edges of a span along a track `width` wide: half a
/// step either side of its first and last steps, kept on the track.
pub fn span_extent(span: Span, width: f32, count: usize) -> (f32, f32) {
    let half = if count < 2 {
        width / 2.0
    } else {
        width / (count - 1) as f32 / 2.0
    };
    let left = (position_of(span.start, width, count) - half).max(0.0);
    let right = (position_of(span.end, width, count) + half).min(width);
    (left, right.max(left))
}

/// The height of a timeline with `lanes` lanes.
pub fn height(lanes: usize) -> f32 {
    TRACK_HEIGHT + lanes as f32 * LANE_HEIGHT
}

/// The part of `bounds` a step maps onto: right of the labels, and inset by
/// the playhead radius so the playhead never leaves the widget.
pub fn track_bounds(bounds: Rectangle, labels: f32) -> Rectangle {
    let left = if labels > 0.0 {
        labels + space::SM
    } else {
        0.0
    };
    let x = bounds.x + left + PLAYHEAD_RADIUS;
    Rectangle {
        x,
        y: bounds.y,
        width: (bounds.width - left - PLAYHEAD_RADIUS * 2.0).max(0.0),
        height: TRACK_HEIGHT,
    }
}

/// What a timeline shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Previous,
    Next,
    PageBack,
    PageForward,
    First,
    Last,
}

impl Action {
    /// The step after this action, from `current` in a timeline of `count`
    /// steps. Send it through the timeline's `on_select` message.
    pub fn apply(self, current: usize, count: usize) -> usize {
        let last = count.saturating_sub(1);
        let current = current.min(last);
        match self {
            Action::Previous => current.saturating_sub(1),
            Action::Next => (current + 1).min(last),
            Action::PageBack => current.saturating_sub(PAGE),
            Action::PageForward => (current + PAGE).min(last),
            Action::First => 0,
            Action::Last => last,
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Previous,
        Action::Next,
        Action::PageBack,
        Action::PageForward,
        Action::First,
        Action::Last,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Previous => "Previous",
            Action::Next => "Next",
            Action::PageBack => "Page back",
            Action::PageForward => "Page forward",
            Action::First => "First",
            Action::Last => "Last",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Previous => "Moves to the step before the current one.",
            Action::Next => "Moves to the step after the current one.",
            Action::PageBack => "Moves ten steps back.",
            Action::PageForward => "Moves ten steps forward.",
            Action::First => "Moves to the first step.",
            Action::Last => "Moves to the last step.",
        }
    }
}

/// The default timeline shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowLeft` | [`Action::Previous`] |
/// | `ArrowRight` | [`Action::Next`] |
/// | `Shift+ArrowLeft`, `PageUp` | [`Action::PageBack`] |
/// | `Shift+ArrowRight`, `PageDown` | [`Action::PageForward`] |
/// | `Home` | [`Action::First`] |
/// | `End` | [`Action::Last`] |
///
/// The timeline resolves them itself while it has keyboard focus or was the
/// last thing pressed. Pass a changed keymap with [`Timeline::keymap`].
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowLeft), Action::Previous)
        .bind(Chord::named(Named::ArrowRight), Action::Next)
        .bind(Chord::named(Named::ArrowLeft).shift(), Action::PageBack)
        .bind(Chord::named(Named::PageUp), Action::PageBack)
        .bind(Chord::named(Named::ArrowRight).shift(), Action::PageForward)
        .bind(Chord::named(Named::PageDown), Action::PageForward)
        .bind(Chord::named(Named::Home), Action::First)
        .bind(Chord::named(Named::End), Action::Last)
}

/// How the pointer is interacting with a timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Status {
    #[default]
    Active,
    Hovered,
    Dragged,
    Disabled,
}

impl Status {
    pub const ALL: [Status; 4] = [
        Status::Active,
        Status::Hovered,
        Status::Dragged,
        Status::Disabled,
    ];
}

/// Colours of a timeline in one status.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub rail: Color,
    pub lane: Color,
    pub label: Color,
    pub playhead: Color,
    pub playhead_border: Color,
    pub playhead_border_width: f32,
    /// The line from the playhead down through the lanes.
    pub cursor: Color,
    /// Multiplies the alpha of tone colours.
    pub tone_alpha: f32,
}

/// Resolves the colours of a timeline for a status.
pub fn style(tokens: &Tokens, status: Status) -> Style {
    let border = match status {
        Status::Active | Status::Disabled => tokens.primary,
        Status::Hovered => mix(tokens.primary, tokens.background, 0.15),
        Status::Dragged => mix(tokens.primary, tokens.background, 0.3),
    };
    let style = Style {
        rail: mix(tokens.muted, tokens.border, 0.5),
        lane: tokens.muted,
        label: tokens.muted_foreground,
        playhead: tokens.background,
        playhead_border: border,
        playhead_border_width: match status {
            Status::Hovered | Status::Dragged => 2.5,
            Status::Active | Status::Disabled => 2.0,
        },
        cursor: fade(tokens.primary, 0.4),
        tone_alpha: 1.0,
    };
    if status != Status::Disabled {
        return style;
    }
    Style {
        rail: fade(style.rail, 0.5),
        lane: fade(style.lane, 0.5),
        label: fade(style.label, 0.5),
        playhead_border: fade(style.playhead_border, 0.5),
        cursor: fade(style.cursor, 0.5),
        tone_alpha: 0.5,
        ..style
    }
}

#[derive(Debug, Default)]
struct State {
    dragging: bool,
    hovered: bool,
    finger: Option<touch::Finger>,
    last: Option<usize>,
    focus: focus::State,
    /// The last press landed on the timeline, so its keys apply.
    active: bool,
}

impl<Message> Timeline<'_, Message> {
    fn select(&self, state: &mut State, x: f32, track: Rectangle, shell: &mut Shell<'_, Message>) {
        let Some(on_select) = &self.on_select else {
            return;
        };
        let index = index_at(x - track.x, track.width, self.steps.len());
        if state.last != Some(index) && index != self.current {
            shell.publish(on_select(index));
        }
        state.last = Some(index);
    }

    fn status(&self, state: &State, layout: Layout<'_>, cursor: mouse::Cursor) -> Status {
        if self.on_select.is_none() {
            return Status::Disabled;
        }
        if state.dragging {
            return Status::Dragged;
        }
        if cursor.is_over(layout.bounds()) {
            return Status::Hovered;
        }
        Status::Active
    }
}

/// The track row with room for the playhead, which focus reports and the
/// ring surrounds.
fn focus_target(track: Rectangle) -> Rectangle {
    let middle = track.center_y();
    Rectangle {
        x: track.x - PLAYHEAD_RADIUS,
        y: middle - PLAYHEAD_RADIUS,
        width: track.width + PLAYHEAD_RADIUS * 2.0,
        height: PLAYHEAD_RADIUS * 2.0,
    }
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
        colour,
    );
}

fn label(renderer: &mut Renderer, content: &str, bounds: Rectangle, colour: Color) {
    <Renderer as core_text::Renderer>::fill_text(
        renderer,
        core_text::Text {
            content: content.to_owned(),
            bounds: Size::new(f32::INFINITY, bounds.height),
            size: Pixels(text_size::XS),
            line_height: LineHeight::Absolute(Pixels(LANE_HEIGHT)),
            font: theme::font(),
            align_x: text::Alignment::Left,
            align_y: Vertical::Center,
            shaping: Shaping::Advanced,
            wrapping: Wrapping::None,
        },
        Point::new(bounds.x, bounds.center_y()),
        colour,
        bounds,
    );
}

impl<Message> Widget<Message, Theme, Renderer> for Timeline<'_, Message> {
    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Fixed(height(self.lanes.len())))
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let height = height(self.lanes.len());
        layout::Node::new(limits.resolve(self.width, Length::Fixed(height), Size::new(0.0, height)))
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let tokens = Tokens::of(theme);
        let state = tree.state.downcast_ref::<State>();
        let style = style(&tokens, self.status(state, layout, cursor));
        let bounds = layout.bounds();
        let labels = self.labels();
        let track = track_bounds(bounds, labels);
        let count = self.steps.len();
        let middle = track.center_y();

        fill(
            renderer,
            Rectangle {
                x: track.x - PLAYHEAD_RADIUS,
                y: middle - RAIL_HEIGHT / 2.0,
                width: track.width + PLAYHEAD_RADIUS * 2.0,
                height: RAIL_HEIGHT,
            },
            style.rail,
            radius::FULL,
        );
        if state.focus.is_focused() {
            focus::draw_ring(renderer, theme, focus_target(track), radius::FULL);
        }

        for (index, lane) in self.lanes.iter().enumerate() {
            let row = Rectangle {
                x: bounds.x,
                y: bounds.y + TRACK_HEIGHT + index as f32 * LANE_HEIGHT,
                width: bounds.width,
                height: LANE_HEIGHT,
            };
            label(
                renderer,
                &lane.label,
                Rectangle {
                    width: labels,
                    ..row
                },
                style.label,
            );
            let bar_y = row.center_y() - BAR_HEIGHT / 2.0;
            fill(
                renderer,
                Rectangle {
                    x: track.x - PLAYHEAD_RADIUS,
                    y: bar_y,
                    width: track.width + PLAYHEAD_RADIUS * 2.0,
                    height: BAR_HEIGHT,
                },
                style.lane,
                radius::FULL,
            );
            for span in &lane.spans {
                if span.start >= count {
                    continue;
                }
                let (left, right) = span_extent(*span, track.width, count);
                fill(
                    renderer,
                    Rectangle {
                        x: track.x + left,
                        y: bar_y,
                        width: (right - left).max(BAR_HEIGHT),
                        height: BAR_HEIGHT,
                    },
                    fade(span.tone.colour(&tokens), style.tone_alpha),
                    radius::FULL,
                );
            }
        }

        for (index, tone) in self.steps.iter().enumerate() {
            let x = track.x + position_of(index, track.width, count);
            // No taller than the playhead, which covers the current tick.
            let tall = if *tone == Tone::Neutral {
                8.0
            } else {
                PLAYHEAD_RADIUS * 2.0
            };
            fill(
                renderer,
                Rectangle {
                    x: x - TICK_WIDTH / 2.0,
                    y: middle - tall / 2.0,
                    width: TICK_WIDTH,
                    height: tall,
                },
                fade(tone.colour(&tokens), style.tone_alpha),
                0.0,
            );
        }

        if count == 0 {
            return;
        }
        let x = track.x + position_of(self.current, track.width, count);
        if !self.lanes.is_empty() {
            fill(
                renderer,
                Rectangle {
                    x: x - 0.5,
                    y: middle,
                    width: 1.0,
                    height: bounds.y + bounds.height - middle,
                },
                style.cursor,
                0.0,
            );
        }
        <Renderer as renderer::Renderer>::fill_quad(
            renderer,
            renderer::Quad {
                bounds: Rectangle {
                    x: x - PLAYHEAD_RADIUS,
                    y: middle - PLAYHEAD_RADIUS,
                    width: PLAYHEAD_RADIUS * 2.0,
                    height: PLAYHEAD_RADIUS * 2.0,
                },
                border: Border {
                    color: style.playhead_border,
                    width: style.playhead_border_width,
                    radius: PLAYHEAD_RADIUS.into(),
                },
                ..renderer::Quad::default()
            },
            style.playhead,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if self.on_select.is_none() || self.steps.is_empty() {
            return;
        }
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        let track = track_bounds(bounds, self.labels());
        // Presses on the lane labels pick nothing.
        let pressable = Rectangle {
            x: track.x - PLAYHEAD_RADIUS,
            width: track.width + PLAYHEAD_RADIUS * 2.0,
            ..bounds
        };
        let before = (
            state.dragging,
            state.hovered,
            state.focus.is_focused(),
            state.active,
        );
        state.hovered = cursor.is_over(bounds);
        let _ = state.focus.update(event);
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(button)) => {
                let position = cursor.position_over(pressable);
                state.active = position.is_some();
                if let (mouse::Button::Left, Some(position)) = (button, position) {
                    state.dragging = true;
                    state.last = None;
                    self.select(state, position.x, track, shell);
                    shell.capture_event();
                }
            }
            Event::Touch(touch::Event::FingerPressed { id, position }) => {
                state.active = pressable.contains(*position);
                if state.active && !state.dragging {
                    state.dragging = true;
                    state.finger = Some(*id);
                    state.last = None;
                    self.select(state, position.x, track, shell);
                    shell.capture_event();
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })
                if state.active || state.focus.is_focused() =>
            {
                if let Some(action) = self.keymap.resolve(key, *modifiers) {
                    let next = action.apply(self.current, self.steps.len());
                    if let Some(on_select) = &self.on_select
                        && next != self.current
                    {
                        shell.publish(on_select(next));
                    }
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position })
                if state.dragging && state.finger.is_none() =>
            {
                self.select(state, position.x, track, shell);
            }
            Event::Touch(touch::Event::FingerMoved { id, position })
                if state.finger == Some(*id) =>
            {
                self.select(state, position.x, track, shell);
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.dragging && state.finger.is_none() =>
            {
                state.dragging = false;
                state.last = None;
                shell.capture_event();
            }
            Event::Touch(
                touch::Event::FingerLifted { id, .. } | touch::Event::FingerLost { id, .. },
            ) if state.finger == Some(*id) => {
                state.dragging = false;
                state.finger = None;
                state.last = None;
                shell.capture_event();
            }
            _ => {}
        }
        let after = (
            state.dragging,
            state.hovered,
            state.focus.is_focused(),
            state.active,
        );
        if after != before {
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
        if self.on_select.is_none() {
            return mouse::Interaction::None;
        }
        if tree.state.downcast_ref::<State>().dragging {
            return mouse::Interaction::Grabbing;
        }
        if cursor.is_over(layout.bounds()) {
            return mouse::Interaction::Pointer;
        }
        mouse::Interaction::None
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        let target = focus_target(track_bounds(layout.bounds(), self.labels()));
        let enabled = self.on_select.is_some();
        #[cfg(feature = "icedwright")]
        {
            use icedwright_probe::{Annotation, report, role};

            let last = self.steps.len().saturating_sub(1) as f64;
            report(operation, target, || {
                let annotation = Annotation::new()
                    .role(role::Slider)
                    .name(self.name.to_string())
                    .range(self.current as f64, 0.0..=last)
                    .component("iced_cube::timeline");
                if enabled {
                    annotation
                } else {
                    annotation.disabled(true)
                }
            });
        }
        // Reported after the annotation, at the same bounds, so a recorder
        // merges the two into one node.
        if enabled && !self.steps.is_empty() {
            let state = tree.state.downcast_mut::<State>();
            focus::report(operation, None, target, &mut state.focus);
        }
    }
}

impl<'a, Message: 'a> From<Timeline<'a, Message>> for Element<'a, Message> {
    fn from(timeline: Timeline<'a, Message>) -> Self {
        Element::new(timeline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::Action as _;
    use crate::theme::{dark, light};

    fn bounds() -> Rectangle {
        Rectangle {
            x: 10.0,
            y: 20.0,
            width: 400.0,
            height: 100.0,
        }
    }

    #[test]
    fn default_builder_fills_the_width_and_is_disabled() {
        let t: Timeline<'_, ()> = timeline([Tone::Neutral; 3], 1);
        assert_eq!(t.width, Length::Fill);
        assert_eq!(t.name, "Timeline");
        assert_eq!(t.label_width, LABEL_WIDTH);
        assert!(t.lanes.is_empty());
        assert!(!t.is_enabled());
        assert!(t.on_select(|_| ()).is_enabled());
    }

    #[test]
    fn the_current_step_is_clamped_to_the_steps() {
        let t: Timeline<'_, ()> = timeline([Tone::Neutral; 3], 9);
        assert_eq!(t.current(), 2);
        let t: Timeline<'_, ()> = timeline([], 4);
        assert_eq!(t.current(), 0);
        assert!(t.is_empty());
    }

    #[test]
    fn spans_are_ordered_and_take_a_tone() {
        let span = Span::new(5, 2).tone(Tone::Warning);
        assert_eq!((span.start, span.end, span.tone), (2, 5, Tone::Warning));
        assert_eq!(Span::new(1, 1).tone, Tone::Neutral);
    }

    #[test]
    fn positions_and_indices_agree_at_every_step() {
        for count in [1, 2, 3, 10, 57] {
            for index in 0..count {
                let x = position_of(index, 600.0, count);
                assert_eq!(index_at(x, 600.0, count), index, "{index} of {count}");
            }
        }
    }

    #[test]
    fn points_outside_the_track_clamp_to_the_ends() {
        assert_eq!(index_at(-50.0, 100.0, 5), 0);
        assert_eq!(index_at(150.0, 100.0, 5), 4);
        assert_eq!(index_at(10.0, 0.0, 5), 0);
        assert_eq!(index_at(10.0, 100.0, 0), 0);
    }

    #[test]
    fn points_go_to_the_nearest_step() {
        assert_eq!(index_at(24.0, 100.0, 5), 1);
        assert_eq!(index_at(26.0, 100.0, 5), 1);
        assert_eq!(index_at(38.0, 100.0, 5), 2);
    }

    #[test]
    fn a_span_reaches_half_a_step_either_side_and_stays_on_the_track() {
        assert_eq!(span_extent(Span::new(1, 2), 100.0, 5), (12.5, 62.5));
        assert_eq!(span_extent(Span::new(0, 4), 100.0, 5), (0.0, 100.0));
        assert_eq!(span_extent(Span::new(3, 3), 100.0, 1), (0.0, 50.0));
    }

    #[test]
    fn the_track_sits_right_of_the_labels_and_inside_the_playhead() {
        let track = track_bounds(bounds(), 0.0);
        assert_eq!(track.x, 10.0 + PLAYHEAD_RADIUS);
        assert_eq!(track.width, 400.0 - PLAYHEAD_RADIUS * 2.0);
        assert_eq!(track.height, TRACK_HEIGHT);
        let track = track_bounds(bounds(), 96.0);
        assert_eq!(track.x, 10.0 + 96.0 + space::SM + PLAYHEAD_RADIUS);
        let narrow = Rectangle {
            width: 20.0,
            ..bounds()
        };
        assert_eq!(track_bounds(narrow, 96.0).width, 0.0);
    }

    #[test]
    fn height_grows_by_a_row_per_lane() {
        assert_eq!(height(0), TRACK_HEIGHT);
        assert_eq!(height(2), TRACK_HEIGHT + 2.0 * LANE_HEIGHT);
    }

    #[test]
    fn actions_move_within_the_steps() {
        assert_eq!(Action::Previous.apply(3, 10), 2);
        assert_eq!(Action::Previous.apply(0, 10), 0);
        assert_eq!(Action::Next.apply(3, 10), 4);
        assert_eq!(Action::Next.apply(9, 10), 9);
        assert_eq!(Action::PageBack.apply(4, 30), 0);
        assert_eq!(Action::PageBack.apply(14, 30), 4);
        assert_eq!(Action::PageForward.apply(25, 30), 29);
        assert_eq!(Action::PageForward.apply(5, 30), 15);
        assert_eq!(Action::First.apply(7, 10), 0);
        assert_eq!(Action::Last.apply(2, 10), 9);
        assert_eq!(Action::Last.apply(0, 0), 0);
        assert_eq!(Action::Next.apply(50, 10), 9);
    }

    fn resolve(keymap: &Keymap<Action>, chord: Chord) -> Option<Action> {
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_binds_arrows_pages_and_ends() {
        let keymap = default_keymap();
        assert_eq!(
            resolve(&keymap, Chord::named(Named::ArrowLeft)),
            Some(Action::Previous)
        );
        assert_eq!(
            resolve(&keymap, Chord::named(Named::ArrowRight).shift()),
            Some(Action::PageForward)
        );
        assert_eq!(
            resolve(&keymap, Chord::named(Named::PageUp)),
            Some(Action::PageBack)
        );
        assert_eq!(
            resolve(&keymap, Chord::named(Named::End)),
            Some(Action::Last)
        );
        assert_eq!(Keymap::<Action>::defaults(), keymap);
        for action in Action::ALL {
            assert!(!action.name().is_empty());
            assert!(action.description().ends_with('.'));
        }
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .bind(Chord::character('j'), Action::Next)
            .unbind(&Chord::named(Named::Home));
        assert_eq!(resolve(&keymap, Chord::character('j')), Some(Action::Next));
        assert_eq!(resolve(&keymap, Chord::named(Named::Home)), None);
        assert_eq!(
            resolve(&keymap, Chord::named(Named::End)),
            Some(Action::Last)
        );
    }

    #[test]
    fn style_follows_the_status_in_light_and_dark() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let active = style(&tokens, Status::Active);
            assert_eq!(active.playhead_border, tokens.primary);
            assert_eq!(active.playhead, tokens.background);
            assert_eq!(active.tone_alpha, 1.0);
            let hovered = style(&tokens, Status::Hovered);
            assert_ne!(hovered.playhead_border, active.playhead_border);
            assert!(hovered.playhead_border_width > active.playhead_border_width);
            let dragged = style(&tokens, Status::Dragged);
            assert_ne!(dragged.playhead_border, hovered.playhead_border);
            let disabled = style(&tokens, Status::Disabled);
            assert!(disabled.rail.a < active.rail.a);
            assert!(disabled.playhead_border.a < active.playhead_border.a);
            assert_eq!(disabled.tone_alpha, 0.5);
            assert_eq!(Status::ALL.len(), 4);
        }
    }
}
