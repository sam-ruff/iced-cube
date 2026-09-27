//! Two views, a start and an end, with one handle between them.
//!
//! A split pane is a [resizable panel](mod@crate::layout::resizable_panel)
//! group of exactly two panels, described by one ratio: the start view's
//! share of the space. [`State`] wraps the group's state and speaks in that
//! ratio, and the view is the same widget, so dragging, keyboard resizing,
//! collapsing and stacking on narrow screens all behave identically. The
//! events, outputs, [`Action`]s and [`default_keymap`] are the resizable
//! panel's.

use iced::widget;
use iced::{Element, Length};

use crate::keys::Keymap;
use crate::layout::resizable_panel::{self, Panel, panel};

pub use crate::layout::resizable_panel::{Action, Axis, Event, Extent, Output, default_keymap};

/// The start view's share of the space and the two panels' limits.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    group: resizable_panel::State,
}

impl State {
    /// A split with the start view taking `ratio` of the space, from 0.0
    /// to 1.0. Out of range ratios are clamped.
    pub fn new(ratio: f32) -> Self {
        let ratio = clamp_ratio(ratio);
        Self::with_panels(
            panel(Extent::Fraction(ratio)),
            panel(Extent::Fraction(1.0 - ratio)),
        )
    }

    /// A split from two panels, for limits or collapsing beyond what
    /// [`State::min`] and [`State::collapsible`] set.
    pub fn with_panels(start: Panel, end: Panel) -> Self {
        Self {
            group: resizable_panel::State::new([start, end]),
        }
    }

    /// The smallest each view may get.
    pub fn min(self, start: Extent, end: Extent) -> Self {
        self.map_panels(|[first, second]| [first.min(start), second.min(end)])
    }

    /// The largest each view may get.
    pub fn max(self, start: Extent, end: Extent) -> Self {
        self.map_panels(|[first, second]| [first.max(start), second.max(end)])
    }

    /// Lets the start view, the end view or both snap shut when dragged
    /// below their threshold.
    pub fn collapsible(self, start: bool, end: bool) -> Self {
        self.map_panels(|[first, second]| [first.collapsible(start), second.collapsible(end)])
    }

    fn map_panels(self, change: impl FnOnce([Panel; 2]) -> [Panel; 2]) -> Self {
        let panels = self.group.panels();
        let (Some(first), Some(second)) = (panels.first(), panels.get(1)) else {
            return self;
        };
        let ratio = self.ratio();
        let [start, end] = change([*first, *second]);
        let mut state = Self::with_panels(start, end);
        if !self.group.sizes().is_empty() {
            state.group.set_sizes([ratio, 1.0 - ratio]);
        }
        state
    }

    /// Starts from a saved ratio, as returned by [`State::ratio`].
    pub fn with_ratio(mut self, ratio: f32) -> Self {
        self.set_ratio(ratio);
        self
    }

    /// Moves the handle so the start view takes `ratio` of the space,
    /// within both views' limits once the split has been measured.
    pub fn set_ratio(&mut self, ratio: f32) {
        let ratio = clamp_ratio(ratio);
        self.group.set_sizes([ratio, 1.0 - ratio]);
    }

    /// The start view's share of the space, from 0.0 to 1.0.
    pub fn ratio(&self) -> f32 {
        ratio_of(&self.group.sizes())
    }

    pub fn is_start_collapsed(&self) -> bool {
        self.group.is_collapsed(0)
    }

    pub fn is_end_collapsed(&self) -> bool {
        self.group.is_collapsed(1)
    }

    /// The panel group underneath, for anything the ratio does not cover.
    pub fn group(&self) -> &resizable_panel::State {
        &self.group
    }

    /// Applies an event from the view or a keyboard action.
    pub fn update(&mut self, event: Event) -> Option<Output> {
        self.group.update(event)
    }
}

impl Default for State {
    /// An even split.
    fn default() -> Self {
        Self::new(0.5)
    }
}

fn clamp_ratio(ratio: f32) -> f32 {
    if ratio.is_nan() {
        return 0.5;
    }
    ratio.clamp(0.0, 1.0)
}

/// The start panel's share of `sizes`, or an even split when there is
/// nothing to go on.
pub fn ratio_of(sizes: &[f32]) -> f32 {
    let total: f32 = sizes.iter().sum();
    match sizes.first() {
        Some(first) if total > 0.0 => clamp_ratio(first / total),
        _ => 0.5,
    }
}

/// A split pane builder. Convert it into an [`Element`] to render.
pub struct SplitPane<'a, Message> {
    state: &'a State,
    start: Element<'a, Message>,
    end: Element<'a, Message>,
    axis: Axis,
    stack_below: Option<f32>,
    grip: bool,
    width: Length,
    height: Length,
    id: Option<widget::Id>,
    keymap: Keymap<Action>,
    on_event: Option<Box<dyn Fn(Event) -> Message + 'a>>,
}

impl<Message> std::fmt::Debug for SplitPane<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SplitPane")
            .field("state", self.state)
            .field("axis", &self.axis)
            .field("stack_below", &self.stack_below)
            .field("grip", &self.grip)
            .finish_non_exhaustive()
    }
}

/// Shows `start` and `end` with a handle between them, side by side unless
/// the [axis](SplitPane::axis) says otherwise. Without
/// [`on_event`](SplitPane::on_event) the handle does nothing.
pub fn split_pane<'a, Message>(
    state: &'a State,
    start: impl Into<Element<'a, Message>>,
    end: impl Into<Element<'a, Message>>,
) -> SplitPane<'a, Message> {
    SplitPane {
        state,
        start: start.into(),
        end: end.into(),
        axis: Axis::default(),
        stack_below: None,
        grip: true,
        width: Length::Fill,
        height: Length::Fill,
        id: None,
        keymap: default_keymap(),
        on_event: None,
    }
}

impl<'a, Message> SplitPane<'a, Message> {
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Stacks a horizontal split into a column while it is narrower than
    /// `width`.
    pub fn stack_below(mut self, width: f32) -> Self {
        self.stack_below = Some(width);
        self
    }

    /// Whether the handle shows a grip. On by default, since a split pane
    /// has a single handle to find.
    pub fn grip(mut self, grip: bool) -> Self {
        self.grip = grip;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// The handle's id, for focusing it with `iced::widget::operation::focus`.
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

impl<'a, Message: 'a> From<SplitPane<'a, Message>> for Element<'a, Message> {
    fn from(split: SplitPane<'a, Message>) -> Self {
        let SplitPane {
            state,
            start,
            end,
            axis,
            stack_below,
            grip,
            width,
            height,
            id,
            keymap,
            on_event,
        } = split;

        let mut group = resizable_panel::resizable_panel(&state.group, [start, end])
            .axis(axis)
            .grip(grip)
            .width(width)
            .height(height)
            .keymap(keymap);
        if let Some(breakpoint) = stack_below {
            group = group.stack_below(breakpoint);
        }
        if let Some(id) = id {
            group = group.id(id);
        }
        if let Some(on_event) = on_event {
            group = group.on_event(on_event);
        }
        group.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    fn measured(state: State) -> State {
        let mut state = state;
        let _ = state.update(Event::Measure(1000.0));
        state
    }

    #[test]
    fn ratio_is_clamped_and_defaults_to_even() {
        assert!(close(State::new(0.3).ratio(), 0.3));
        assert!(close(State::new(1.5).ratio(), 1.0));
        assert!(close(State::new(-1.0).ratio(), 0.0));
        assert!(close(State::new(f32::NAN).ratio(), 0.5));
        assert!(close(State::default().ratio(), 0.5));
    }

    #[test]
    fn ratio_of_sizes() {
        assert!(close(ratio_of(&[300.0, 700.0]), 0.3));
        assert!(close(ratio_of(&[1.0, 3.0]), 0.25));
        assert!(close(ratio_of(&[]), 0.5));
        assert!(close(ratio_of(&[0.0, 0.0]), 0.5));
    }

    #[test]
    fn dragging_changes_the_ratio() {
        let mut state = measured(State::new(0.5));
        let _ = state.update(Event::DragStart {
            handle: 0,
            position: 500.0,
        });
        assert_eq!(state.update(Event::Drag(650.0)), Some(Output::Resized));
        assert!(close(state.ratio(), 0.65));
    }

    #[test]
    fn limits_hold_the_ratio_in_range() {
        let mut state = measured(State::new(0.5).min(Extent::Pixels(200.0), Extent::Fraction(0.4)));
        let _ = state.update(Event::Resize {
            handle: 0,
            delta: 400.0,
        });
        assert!(close(state.ratio(), 0.6), "end keeps 40%");
        let _ = state.update(Event::Resize {
            handle: 0,
            delta: -900.0,
        });
        assert!(close(state.ratio(), 0.2), "start keeps 200px");

        let capped = measured(State::new(0.9).max(Extent::Fraction(0.7), Extent::Fraction(1.0)));
        assert!(close(capped.ratio(), 0.7));
    }

    #[test]
    fn set_ratio_fits_the_limits_once_measured() {
        let mut state = measured(State::new(0.5).min(Extent::Pixels(100.0), Extent::Pixels(300.0)));
        state.set_ratio(0.9);
        assert!(close(state.ratio(), 0.7));
        let restored = State::new(0.5).with_ratio(0.25);
        assert!(close(restored.ratio(), 0.25));
    }

    #[test]
    fn limits_keep_a_ratio_set_earlier() {
        let state = State::new(0.5)
            .with_ratio(0.3)
            .min(Extent::Pixels(10.0), Extent::Pixels(10.0));
        assert!(close(state.ratio(), 0.3));
    }

    #[test]
    fn either_side_can_collapse() {
        let mut state = measured(
            State::new(0.5)
                .min(Extent::Pixels(200.0), Extent::Pixels(200.0))
                .collapsible(false, true),
        );
        assert_eq!(state.update(Event::Toggle(0)), None);
        assert_eq!(state.update(Event::Toggle(1)), Some(Output::Collapsed(1)));
        assert!(state.is_end_collapsed());
        assert!(!state.is_start_collapsed());
        assert!(close(state.ratio(), 1.0));
    }

    #[test]
    fn builder_defaults_show_a_grip() {
        let state = State::default();
        let split: SplitPane<'_, ()> = split_pane(&state, "start", "end");
        assert!(split.grip);
        assert_eq!(split.axis, Axis::Horizontal);
        assert_eq!(split.stack_below, None);
        assert!(split.on_event.is_none());
    }
}
