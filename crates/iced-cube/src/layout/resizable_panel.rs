//! Two or more panels in a row or column, separated by handles that resize
//! them.
//!
//! [`State`] holds the panels' constraints and their sizes, as fractions of
//! the space the panels share, so an app can persist them with
//! [`State::sizes`] and restore them with [`State::with_sizes`]. Its pure
//! [`update`](State::update) applies drags, keyboard steps and collapsing,
//! enforcing each panel's [min, max and collapse threshold](Panel).
//!
//! Dragging a handle moves the panels next to it; when one reaches its
//! limit the change carries on to the panels beyond it. A
//! [collapsible](Panel::collapsible) panel snaps shut when dragged below its
//! threshold and snaps open again when dragged past it.
//!
//! # Keyboard
//!
//! iced 0.14 has no general focus model, but it does let widgets take part
//! in its focus operations. Each handle is focusable: pressing it focuses
//! it, as a text field is, and `iced::widget::operation::focus_next()` or
//! `focus(id)` reach it too. While a handle has focus the group resolves
//! its own [`Keymap`] of [`Action`]s and captures the keys it uses, so an
//! app needs no subscription; see [`default_keymap`]. A press anywhere
//! else, or Escape, takes the focus away.

use iced::Event as Input;
use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::keyboard::{self, key::Named};
use iced::widget;
use iced::{
    Background, Border, Color, Element, Length, Point, Rectangle, Renderer, Shadow, Size, Theme,
    Vector, mouse, touch,
};

use crate::keys::{self, Chord, Keymap};
use crate::theme::{Tokens, radius};

/// Space a handle takes up between two panels, in logical pixels.
pub const HANDLE: f32 = 1.0;

/// Width of the area around a handle that responds to the pointer.
pub const HIT_AREA: f32 = 9.0;

/// Width of the bar drawn over a highlighted handle.
const HIGHLIGHT: f32 = 3.0;

/// Size of the grip across and along the handle.
const GRIP: Size = Size::new(12.0, 18.0);

/// Sizes closer than this count as equal, in logical pixels.
const EPSILON: f32 = 0.01;

/// Whether panels sit side by side or one above the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Axis {
    /// A row of panels with upright handles between them.
    #[default]
    Horizontal,
    /// A column of panels with level handles between them.
    Vertical,
}

impl Axis {
    pub const ALL: [Axis; 2] = [Axis::Horizontal, Axis::Vertical];

    /// The coordinate of `point` along this axis.
    pub fn along(self, point: Point) -> f32 {
        match self {
            Axis::Horizontal => point.x,
            Axis::Vertical => point.y,
        }
    }

    /// The pointer shape over a handle.
    pub fn cursor(self) -> mouse::Interaction {
        match self {
            Axis::Horizontal => mouse::Interaction::ResizingColumn,
            Axis::Vertical => mouse::Interaction::ResizingRow,
        }
    }

    fn main(self, size: Size) -> f32 {
        match self {
            Axis::Horizontal => size.width,
            Axis::Vertical => size.height,
        }
    }

    fn cross(self, size: Size) -> f32 {
        match self {
            Axis::Horizontal => size.height,
            Axis::Vertical => size.width,
        }
    }

    fn size(self, main: f32, cross: f32) -> Size {
        match self {
            Axis::Horizontal => Size::new(main, cross),
            Axis::Vertical => Size::new(cross, main),
        }
    }

    fn point(self, main: f32) -> Point {
        match self {
            Axis::Horizontal => Point::new(main, 0.0),
            Axis::Vertical => Point::new(0.0, main),
        }
    }
}

/// A length along the group: a share of the space the panels have, or a
/// fixed number of logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Extent {
    /// A share of the space, from 0.0 to 1.0.
    Fraction(f32),
    Pixels(f32),
}

impl Extent {
    /// The length in pixels, out of `total` pixels.
    pub fn resolve(self, total: f32) -> f32 {
        let pixels = match self {
            Extent::Fraction(share) => share * total,
            Extent::Pixels(pixels) => pixels,
        };
        pixels.max(0.0)
    }
}

/// The constraints of one panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Panel {
    /// The starting size. Fractions share out the space the pixel panels
    /// leave.
    pub default: Extent,
    pub min: Extent,
    pub max: Extent,
    /// Whether the panel snaps shut below its threshold.
    pub collapsible: bool,
    /// The size of the panel while collapsed.
    pub collapsed_size: Extent,
    /// Below this size a collapsible panel snaps shut. Defaults to halfway
    /// between the collapsed size and the minimum.
    pub collapse_below: Option<Extent>,
    /// Whether the panel starts collapsed.
    pub collapsed: bool,
}

/// A panel starting at `default`, with no minimum or maximum.
pub fn panel(default: Extent) -> Panel {
    Panel {
        default,
        min: Extent::Pixels(0.0),
        max: Extent::Fraction(1.0),
        collapsible: false,
        collapsed_size: Extent::Pixels(0.0),
        collapse_below: None,
        collapsed: false,
    }
}

impl Panel {
    pub fn min(mut self, min: Extent) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: Extent) -> Self {
        self.max = max;
        self
    }

    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    /// The size while collapsed, such as the width of an icon rail.
    pub fn collapsed_size(mut self, size: Extent) -> Self {
        self.collapsed_size = size;
        self
    }

    pub fn collapse_below(mut self, threshold: Extent) -> Self {
        self.collapse_below = Some(threshold);
        self
    }

    /// Starts the panel collapsed. Only applies to a collapsible panel.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// The minimum, maximum and collapsed sizes in pixels. The maximum is
    /// never below the minimum.
    pub fn limits(&self, total: f32) -> Limits {
        let min = self.min.resolve(total);
        let collapsed = self.collapsed_size.resolve(total).min(min);
        Limits {
            min,
            max: self.max.resolve(total).max(min),
            collapsed,
            threshold: self
                .collapse_below
                .map_or((collapsed + min) / 2.0, |threshold| {
                    threshold.resolve(total).clamp(collapsed, min)
                }),
        }
    }

    /// The nearest size this panel can take to `size`: collapsed below the
    /// threshold when collapsible, otherwise within its minimum and maximum.
    pub fn settle(&self, size: f32, total: f32) -> f32 {
        let limits = self.limits(total);
        if self.collapsible && size < limits.threshold {
            return limits.collapsed;
        }
        size.clamp(limits.min, limits.max)
    }

    /// Whether a panel of `size` pixels is collapsed.
    pub fn is_collapsed_at(&self, size: f32, total: f32) -> bool {
        let limits = self.limits(total);
        self.collapsible && size <= limits.collapsed + EPSILON && limits.collapsed < limits.min
    }
}

/// A panel's constraints resolved to pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    pub min: f32,
    pub max: f32,
    pub collapsed: f32,
    pub threshold: f32,
}

/// The starting sizes of `panels` in pixels: pixel defaults as given, and
/// fraction defaults sharing out what is left in proportion, then fitted to
/// the panels' constraints.
pub fn defaults(panels: &[Panel], total: f32) -> Vec<f32> {
    let fixed: f32 = panels
        .iter()
        .filter_map(|panel| match panel.default {
            Extent::Pixels(pixels) => Some(pixels.max(0.0)),
            Extent::Fraction(_) => None,
        })
        .sum();
    let shares: f32 = panels
        .iter()
        .filter_map(|panel| match panel.default {
            Extent::Fraction(share) => Some(share.max(0.0)),
            Extent::Pixels(_) => None,
        })
        .sum();
    let left = (total - fixed).max(0.0);
    let sizes = panels
        .iter()
        .map(|panel| match panel.default {
            Extent::Pixels(pixels) => pixels.max(0.0),
            Extent::Fraction(share) if shares > 0.0 => left * share.max(0.0) / shares,
            Extent::Fraction(_) => 0.0,
        })
        .collect();
    let collapsed: Vec<bool> = panels
        .iter()
        .map(|panel| panel.collapsible && panel.collapsed)
        .collect();
    fit(panels, &collapsed, sizes, total)
}

/// Fits `sizes` into `total` pixels within each panel's limits.
///
/// Collapsed panels take their collapsed size. The rest grow or shrink in
/// proportion to their size until the sizes add up. When the minimums do
/// not fit, collapsible panels collapse, last first, and as a last resort
/// every panel scales down, so the sizes always add up to `total`.
pub fn fit(panels: &[Panel], collapsed: &[bool], sizes: Vec<f32>, total: f32) -> Vec<f32> {
    let mut shut: Vec<bool> = (0..panels.len())
        .map(|index| collapsed.get(index).copied().unwrap_or(false))
        .collect();
    let mut fitted = constrain(panels, &shut, &sizes, total);
    distribute(panels, &shut, &mut fitted, total);

    while fitted.iter().sum::<f32>() > total + EPSILON {
        let Some(index) = (0..panels.len())
            .rev()
            .find(|&index| panels[index].collapsible && !shut[index])
        else {
            break;
        };
        shut[index] = true;
        fitted = constrain(panels, &shut, &sizes, total);
        distribute(panels, &shut, &mut fitted, total);
    }

    let sum: f32 = fitted.iter().sum();
    if sum > EPSILON && (sum - total).abs() > EPSILON {
        for size in &mut fitted {
            *size *= total / sum;
        }
    }
    fitted
}

fn constrain(panels: &[Panel], shut: &[bool], sizes: &[f32], total: f32) -> Vec<f32> {
    panels
        .iter()
        .enumerate()
        .map(|(index, panel)| {
            let limits = panel.limits(total);
            if shut[index] && panel.collapsible {
                return limits.collapsed;
            }
            sizes
                .get(index)
                .copied()
                .unwrap_or(0.0)
                .clamp(limits.min, limits.max)
        })
        .collect()
}

/// Spreads the difference between `total` and the sizes over the open
/// panels that have room, in proportion to their size.
fn distribute(panels: &[Panel], shut: &[bool], sizes: &mut [f32], total: f32) {
    for _ in 0..=panels.len() {
        let gap = total - sizes.iter().sum::<f32>();
        if gap.abs() <= EPSILON {
            return;
        }
        let room = |index: usize| {
            let limits = panels[index].limits(total);
            let open = !(shut[index] && panels[index].collapsible);
            open && if gap > 0.0 {
                sizes[index] < limits.max - EPSILON
            } else {
                sizes[index] > limits.min + EPSILON
            }
        };
        let flexible: Vec<usize> = (0..panels.len()).filter(|&index| room(index)).collect();
        if flexible.is_empty() {
            return;
        }
        let weight: f32 = flexible.iter().map(|&index| sizes[index]).sum();
        for &index in &flexible {
            let share = if weight > EPSILON {
                sizes[index] / weight
            } else {
                1.0 / flexible.len() as f32
            };
            let limits = panels[index].limits(total);
            sizes[index] = (sizes[index] + gap * share).clamp(limits.min, limits.max);
        }
    }
}

/// The sizes after moving handle `handle` by `delta` pixels from `origin`.
///
/// The panel on the side the handle moves towards shrinks, and once it
/// reaches its minimum the panels beyond it shrink in turn. The panel on
/// the other side grows, passing any growth past its maximum on outwards.
/// Collapsible panels snap shut and open at their threshold. When the move
/// cannot be balanced, `origin` comes back unchanged.
pub fn resize(panels: &[Panel], origin: &[f32], handle: usize, delta: f32, total: f32) -> Vec<f32> {
    let count = panels.len();
    if handle + 1 >= count || origin.len() != count || delta.abs() < EPSILON {
        return origin.to_vec();
    }
    let before: Vec<usize> = (0..=handle).rev().collect();
    let after: Vec<usize> = (handle + 1..count).collect();
    let (grow, shrink) = if delta > 0.0 {
        (before, after)
    } else {
        (after, before)
    };

    let mut request = delta.abs();
    for _ in 0..3 {
        let (shrunk, given) = shrink_by(panels, origin, &shrink, request, total);
        if given < EPSILON {
            return origin.to_vec();
        }
        let (grown, taken) = grow_by(panels, shrunk, &grow, given, total);
        if (taken - given).abs() < EPSILON {
            return grown;
        }
        // Snapping made one side move more than the other: try again with
        // what the growing side accepts.
        request = taken;
    }
    origin.to_vec()
}

fn shrink_by(
    panels: &[Panel],
    origin: &[f32],
    order: &[usize],
    request: f32,
    total: f32,
) -> (Vec<f32>, f32) {
    let mut sizes = origin.to_vec();
    let mut given = 0.0;
    for &index in order {
        let remaining = request - given;
        if remaining < EPSILON {
            break;
        }
        let current = sizes[index];
        let settled = panels[index].settle(current - remaining, total);
        if settled < current {
            given += current - settled;
            sizes[index] = settled;
        }
    }
    (sizes, given)
}

fn grow_by(
    panels: &[Panel],
    mut sizes: Vec<f32>,
    order: &[usize],
    amount: f32,
    total: f32,
) -> (Vec<f32>, f32) {
    let mut taken = 0.0;
    for &index in order {
        let remaining = amount - taken;
        if remaining < EPSILON {
            break;
        }
        let current = sizes[index];
        let settled = panels[index].settle(current + remaining, total);
        if settled > current {
            taken += settled - current;
            sizes[index] = settled;
        } else if panels[index].is_collapsed_at(current, total) {
            // A collapsed panel that stays shut holds the handle in place.
            break;
        }
    }
    (sizes, taken)
}

/// Changes to a panel group.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// The group measured the space its panels share, in pixels. The
    /// widget sends it whenever that changes.
    Measure(f32),
    /// The pointer pressed `handle` at `position` along the axis.
    DragStart {
        handle: usize,
        position: f32,
    },
    /// The pointer moved to `position` along the axis while dragging.
    Drag(f32),
    DragEnd,
    /// Moves `handle` by `delta` pixels, towards the end when positive.
    Resize {
        handle: usize,
        delta: f32,
    },
    Collapse(usize),
    Expand(usize),
    /// Collapses an open panel or expands a collapsed one.
    Toggle(usize),
    /// Puts every panel back to its default size.
    Reset,
}

/// What changed after an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    /// The sizes changed. Read them with [`State::sizes`].
    Resized,
    /// This panel collapsed.
    Collapsed(usize),
    /// This panel expanded.
    Expanded(usize),
}

/// A drag in progress: the handle, where the pointer started and the
/// sizes at that moment.
#[derive(Debug, Clone, PartialEq)]
struct Drag {
    handle: usize,
    start: f32,
    origin: Vec<f32>,
}

/// The panels, their sizes and any drag in progress.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    panels: Vec<Panel>,
    /// Fractions of the shared space, adding up to 1. Empty until the
    /// group is first measured or sizes are restored.
    sizes: Vec<f32>,
    collapsed: Vec<bool>,
    /// The size each collapsed panel had before it collapsed.
    restore: Vec<Option<f32>>,
    total: f32,
    /// Sizes came from [`State::with_sizes`], so which panels are collapsed
    /// is worked out on the first measurement.
    restored: bool,
    drag: Option<Drag>,
    step: Extent,
    large_step: Extent,
}

impl State {
    /// A group of `panels` at their default sizes.
    pub fn new(panels: impl IntoIterator<Item = Panel>) -> Self {
        let panels: Vec<Panel> = panels.into_iter().collect();
        let collapsed = panels
            .iter()
            .map(|panel| panel.collapsible && panel.collapsed)
            .collect();
        Self {
            restore: vec![None; panels.len()],
            panels,
            sizes: Vec::new(),
            collapsed,
            total: 0.0,
            restored: false,
            drag: None,
            step: Extent::Fraction(0.05),
            large_step: Extent::Fraction(0.2),
        }
    }

    /// Starts from saved sizes, as returned by [`State::sizes`]. Sizes of
    /// the wrong length or adding up to nothing are ignored; the rest are
    /// scaled to add up to 1.
    pub fn with_sizes(mut self, sizes: impl IntoIterator<Item = f32>) -> Self {
        self.set_sizes(sizes);
        self
    }

    /// Replaces the sizes, as [`State::with_sizes`] does, fitting them to
    /// the panels' limits once the group has been measured.
    pub fn set_sizes(&mut self, sizes: impl IntoIterator<Item = f32>) {
        let sizes: Vec<f32> = sizes.into_iter().map(|size| size.max(0.0)).collect();
        let sum: f32 = sizes.iter().sum();
        if sizes.len() != self.panels.len() || sum <= 0.0 {
            return;
        }
        self.sizes = sizes.iter().map(|size| size / sum).collect();
        self.collapsed = self
            .panels
            .iter()
            .zip(&self.sizes)
            .map(|(panel, size)| panel.collapsible && *size <= EPSILON / 100.0)
            .collect();
        self.restored = true;
        if self.total > 0.0 {
            self.settle_restored();
        }
    }

    /// How far the arrow keys move a handle. Defaults to 5% of the space.
    pub fn step(mut self, step: Extent) -> Self {
        self.step = step;
        self
    }

    /// How far Shift and an arrow key move a handle. Defaults to 20%.
    pub fn large_step(mut self, step: Extent) -> Self {
        self.large_step = step;
        self
    }

    pub fn panels(&self) -> &[Panel] {
        &self.panels
    }

    /// Each panel's share of the space, adding up to 1, for saving and
    /// restoring with [`State::with_sizes`]. Before the group is first
    /// measured this is the defaults as fractions, or empty when a default
    /// is in pixels.
    pub fn sizes(&self) -> Vec<f32> {
        if !self.sizes.is_empty() {
            return self.sizes.clone();
        }
        let fractions: Option<Vec<f32>> = self
            .panels
            .iter()
            .map(|panel| match panel.default {
                Extent::Fraction(share) => Some(share),
                Extent::Pixels(_) => None,
            })
            .collect();
        fractions.unwrap_or_default()
    }

    /// The pixels the panels share, as last measured. 0 before the first
    /// measurement.
    pub fn total(&self) -> f32 {
        self.total
    }

    pub fn is_collapsed(&self, panel: usize) -> bool {
        self.collapsed.get(panel).copied().unwrap_or(false)
    }

    /// The handle being dragged, if any.
    pub fn dragging(&self) -> Option<usize> {
        self.drag.as_ref().map(|drag| drag.handle)
    }

    /// Each panel's size in pixels, out of `total`.
    pub fn resolve(&self, total: f32) -> Vec<f32> {
        if self.sizes.is_empty() {
            let mut sizes = defaults(&self.panels, total);
            self.collapse_marked(&mut sizes, total);
            return sizes;
        }
        let sizes = self.sizes.iter().map(|size| size * total).collect();
        fit(&self.panels, &self.collapsed, sizes, total)
    }

    fn collapse_marked(&self, sizes: &mut [f32], total: f32) {
        let marked = self.collapsed.iter().any(|collapsed| *collapsed);
        if marked {
            let fitted = fit(&self.panels, &self.collapsed, sizes.to_vec(), total);
            sizes.copy_from_slice(&fitted);
        }
    }

    /// The pixel sizes for the current measurement.
    fn current(&self) -> Vec<f32> {
        self.resolve(self.total)
    }

    /// Applies an event and says what changed.
    pub fn update(&mut self, event: Event) -> Option<Output> {
        if let Event::Measure(total) = event {
            self.measure(total);
            return None;
        }
        if self.total <= 0.0 {
            return None;
        }
        match event {
            Event::Measure(_) => None,
            Event::DragStart { handle, position } => {
                if handle + 1 >= self.panels.len() {
                    return None;
                }
                self.drag = Some(Drag {
                    handle,
                    start: position,
                    origin: self.current(),
                });
                None
            }
            Event::Drag(position) => {
                let drag = self.drag.clone()?;
                let sizes = resize(
                    &self.panels,
                    &drag.origin,
                    drag.handle,
                    position - drag.start,
                    self.total,
                );
                self.apply(sizes, &drag.origin)
            }
            Event::DragEnd => {
                self.drag = None;
                None
            }
            Event::Resize { handle, delta } => {
                let origin = self.current();
                let sizes = resize(&self.panels, &origin, handle, delta, self.total);
                self.apply(sizes, &origin)
            }
            Event::Collapse(panel) => self.collapse(panel),
            Event::Expand(panel) => self.expand(panel),
            Event::Toggle(panel) if self.is_collapsed(panel) => self.expand(panel),
            Event::Toggle(panel) => self.collapse(panel),
            Event::Reset => {
                let origin = self.current();
                self.collapsed = self
                    .panels
                    .iter()
                    .map(|panel| panel.collapsible && panel.collapsed)
                    .collect();
                self.restore = vec![None; self.panels.len()];
                self.sizes.clear();
                let sizes = self.current();
                self.store(&sizes);
                (!same(&sizes, &origin)).then_some(Output::Resized)
            }
        }
    }

    fn measure(&mut self, total: f32) {
        if total <= 0.0 || !total.is_finite() {
            return;
        }
        if self.sizes.is_empty() {
            let sizes = self.resolve(total);
            self.total = total;
            self.store(&sizes);
            return;
        }
        self.total = total;
        if self.restored {
            self.settle_restored();
        }
    }

    /// Fits restored sizes to the panels. A size below a collapsible
    /// panel's threshold would snap shut, so that panel starts collapsed.
    fn settle_restored(&mut self) {
        self.restored = false;
        let total = self.total;
        self.collapsed = self
            .panels
            .iter()
            .zip(&self.sizes)
            .map(|(panel, size)| panel.collapsible && size * total < panel.limits(total).threshold)
            .collect();
        let sizes = self.current();
        self.store(&sizes);
    }

    /// The handle to move to resize `panel`, and whether growing the panel
    /// moves it towards the end.
    fn handle_for(&self, panel: usize) -> Option<(usize, bool)> {
        if panel + 1 < self.panels.len() {
            return Some((panel, true));
        }
        (panel > 0 && panel < self.panels.len()).then(|| (panel - 1, false))
    }

    fn collapse(&mut self, panel: usize) -> Option<Output> {
        let settings = self.panels.get(panel)?;
        if !settings.collapsible || self.is_collapsed(panel) {
            return None;
        }
        let (handle, forward) = self.handle_for(panel)?;
        let origin = self.current();
        let shrink = origin[panel] - settings.limits(self.total).collapsed;
        let delta = if forward { -shrink } else { shrink };
        let sizes = resize(&self.panels, &origin, handle, delta, self.total);
        self.apply(sizes, &origin)
    }

    fn expand(&mut self, panel: usize) -> Option<Output> {
        let settings = self.panels.get(panel)?;
        if !self.is_collapsed(panel) {
            return None;
        }
        let (handle, forward) = self.handle_for(panel)?;
        let origin = self.current();
        let limits = settings.limits(self.total);
        let wanted = self
            .restore
            .get(panel)
            .copied()
            .flatten()
            .map_or_else(
                || settings.default.resolve(self.total),
                |size| size * self.total,
            )
            .clamp(limits.min, limits.max);
        let grow = wanted - origin[panel];
        let delta = if forward { grow } else { -grow };
        let sizes = resize(&self.panels, &origin, handle, delta, self.total);
        self.apply(sizes, &origin)
    }

    /// Stores new pixel sizes and reports what changed against `origin`.
    fn apply(&mut self, sizes: Vec<f32>, origin: &[f32]) -> Option<Output> {
        if same(&sizes, &self.current()) {
            return None;
        }
        let was = self.collapsed.clone();
        self.store(&sizes);
        let changed = (0..self.panels.len()).find(|&index| was[index] != self.collapsed[index]);
        match changed {
            Some(index) if self.collapsed[index] => {
                if let (Some(slot), Some(size)) = (self.restore.get_mut(index), origin.get(index)) {
                    *slot = Some(size / self.total);
                }
                Some(Output::Collapsed(index))
            }
            Some(index) => Some(Output::Expanded(index)),
            None => Some(Output::Resized),
        }
    }

    fn store(&mut self, sizes: &[f32]) {
        let sum: f32 = sizes.iter().sum();
        if sum <= 0.0 {
            return;
        }
        self.sizes = sizes.iter().map(|size| size / sum).collect();
        self.collapsed = self
            .panels
            .iter()
            .zip(sizes)
            .map(|(panel, size)| panel.is_collapsed_at(*size, self.total.max(sum)))
            .collect();
    }
}

fn same(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| (a - b).abs() < EPSILON)
}

/// What a handle keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Moves the handle towards the start by one step.
    Decrease,
    /// Moves the handle towards the end by one step.
    Increase,
    DecreaseMore,
    IncreaseMore,
    /// Shrinks the panel before the handle to its minimum.
    Min,
    /// Grows the panel before the handle to its maximum.
    Max,
    /// Collapses or expands the collapsible panel next to the handle.
    Toggle,
    /// Takes the focus away from the handle.
    Release,
}

impl Action {
    /// The [`Event`] this action sends for `handle`, or `None` when it has
    /// nothing to do, as for [`Action::Release`], which the group handles
    /// itself, or [`Action::Toggle`] beside panels that cannot collapse.
    pub fn event(self, state: &State, handle: usize) -> Option<Event> {
        if handle + 1 >= state.panels.len() {
            return None;
        }
        let total = state.total;
        let size = || state.current().get(handle).copied().unwrap_or(0.0);
        let limits = || state.panels[handle].limits(total);
        let delta = match self {
            Action::Decrease => -state.step.resolve(total),
            Action::Increase => state.step.resolve(total),
            Action::DecreaseMore => -state.large_step.resolve(total),
            Action::IncreaseMore => state.large_step.resolve(total),
            Action::Min => limits().min - size(),
            Action::Max => limits().max - size(),
            Action::Toggle => {
                return [handle, handle + 1]
                    .into_iter()
                    .find(|&panel| state.panels[panel].collapsible)
                    .map(Event::Toggle);
            }
            Action::Release => return None,
        };
        Some(Event::Resize { handle, delta })
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Decrease,
        Action::Increase,
        Action::DecreaseMore,
        Action::IncreaseMore,
        Action::Min,
        Action::Max,
        Action::Toggle,
        Action::Release,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Decrease => "Decrease",
            Action::Increase => "Increase",
            Action::DecreaseMore => "DecreaseMore",
            Action::IncreaseMore => "IncreaseMore",
            Action::Min => "Min",
            Action::Max => "Max",
            Action::Toggle => "Toggle",
            Action::Release => "Release",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Decrease => "Moves the focused handle one step towards the start.",
            Action::Increase => "Moves the focused handle one step towards the end.",
            Action::DecreaseMore => "Moves the focused handle a large step towards the start.",
            Action::IncreaseMore => "Moves the focused handle a large step towards the end.",
            Action::Min => "Shrinks the panel before the handle to its minimum.",
            Action::Max => "Grows the panel before the handle to its maximum.",
            Action::Toggle => "Collapses or expands the collapsible panel next to the handle.",
            Action::Release => "Takes the focus away from the handle.",
        }
    }
}

/// The default handle shortcuts, active while a handle has focus:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowLeft`, `ArrowUp` | [`Action::Decrease`] |
/// | `ArrowRight`, `ArrowDown` | [`Action::Increase`] |
/// | `Shift+ArrowLeft`, `Shift+ArrowUp` | [`Action::DecreaseMore`] |
/// | `Shift+ArrowRight`, `Shift+ArrowDown` | [`Action::IncreaseMore`] |
/// | `Home` | [`Action::Min`] |
/// | `End` | [`Action::Max`] |
/// | `Enter` | [`Action::Toggle`] |
/// | `Escape` | [`Action::Release`] |
///
/// The group resolves these itself and captures them, so they never reach
/// `keys::subscription` while a handle has focus.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowLeft), Action::Decrease)
        .bind(Chord::named(Named::ArrowUp), Action::Decrease)
        .bind(Chord::named(Named::ArrowRight), Action::Increase)
        .bind(Chord::named(Named::ArrowDown), Action::Increase)
        .bind(Chord::named(Named::ArrowLeft).shift(), Action::DecreaseMore)
        .bind(Chord::named(Named::ArrowUp).shift(), Action::DecreaseMore)
        .bind(
            Chord::named(Named::ArrowRight).shift(),
            Action::IncreaseMore,
        )
        .bind(Chord::named(Named::ArrowDown).shift(), Action::IncreaseMore)
        .bind(Chord::named(Named::Home), Action::Min)
        .bind(Chord::named(Named::End), Action::Max)
        .bind(Chord::named(Named::Enter), Action::Toggle)
        .bind(Chord::named(Named::Escape), Action::Release)
}

/// How a handle is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HandleStatus {
    #[default]
    Idle,
    /// Under the pointer.
    Hovered,
    Dragged,
    /// Has keyboard focus.
    Focused,
    /// The group has no message, so the handle does nothing.
    Disabled,
}

impl HandleStatus {
    pub const ALL: [HandleStatus; 5] = [
        HandleStatus::Idle,
        HandleStatus::Hovered,
        HandleStatus::Dragged,
        HandleStatus::Focused,
        HandleStatus::Disabled,
    ];
}

/// The colours of a handle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandleStyle {
    /// The one pixel line between the panels.
    pub line: Color,
    /// The wider bar drawn over the line while highlighted.
    pub highlight: Option<Color>,
    pub grip: Color,
    pub grip_border: Color,
    pub grip_dots: Color,
}

/// Resolves a handle's colours for a status.
pub fn handle_style(tokens: &Tokens, status: HandleStatus) -> HandleStyle {
    let highlight = match status {
        HandleStatus::Idle | HandleStatus::Disabled => None,
        HandleStatus::Hovered => Some(tokens.border),
        HandleStatus::Dragged | HandleStatus::Focused => Some(tokens.ring),
    };
    HandleStyle {
        line: tokens.border,
        highlight,
        grip: tokens.background,
        grip_border: match status {
            HandleStatus::Focused | HandleStatus::Dragged => tokens.ring,
            _ => tokens.border,
        },
        grip_dots: tokens.muted_foreground,
    }
}

/// A panel group builder. Convert it into an [`Element`] to render.
pub struct ResizablePanel<'a, Message> {
    state: &'a State,
    panes: Vec<Element<'a, Message>>,
    axis: Axis,
    stack_below: Option<f32>,
    grip: bool,
    width: Length,
    height: Length,
    id: Option<widget::Id>,
    keymap: Keymap<Action>,
    on_event: Option<Box<dyn Fn(Event) -> Message + 'a>>,
}

impl<Message> std::fmt::Debug for ResizablePanel<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResizablePanel")
            .field("state", self.state)
            .field("panes", &self.panes.len())
            .field("axis", &self.axis)
            .field("stack_below", &self.stack_below)
            .field("grip", &self.grip)
            .field("keymap", &self.keymap)
            .finish_non_exhaustive()
    }
}

/// Lays out `panes`, one per panel of `state`, with handles between them.
/// Without [`on_event`](ResizablePanel::on_event) the handles do nothing.
pub fn resizable_panel<'a, Message>(
    state: &'a State,
    panes: impl IntoIterator<Item = Element<'a, Message>>,
) -> ResizablePanel<'a, Message> {
    ResizablePanel {
        state,
        panes: panes.into_iter().collect(),
        axis: Axis::default(),
        stack_below: None,
        grip: false,
        width: Length::Fill,
        height: Length::Fill,
        id: None,
        keymap: default_keymap(),
        on_event: None,
    }
}

impl<'a, Message> ResizablePanel<'a, Message> {
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Stacks a horizontal group into a column while it is narrower than
    /// `width`, as on a phone.
    pub fn stack_below(mut self, width: f32) -> Self {
        self.stack_below = Some(width);
        self
    }

    /// Draws a grip on each handle so it is easier to find.
    pub fn grip(mut self, grip: bool) -> Self {
        self.grip = grip;
        self
    }

    /// Defaults to [`Length::Fill`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Defaults to [`Length::Fill`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// The id of the first handle, for focusing it with
    /// `iced::widget::operation::focus`.
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Replaces the [`default_keymap`] a focused handle resolves keys with.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    /// The axis panels are laid out along in `width` pixels.
    pub fn axis_at(&self, width: f32) -> Axis {
        match self.stack_below {
            Some(breakpoint) if self.axis == Axis::Horizontal && width < breakpoint => {
                Axis::Vertical
            }
            _ => self.axis,
        }
    }
}

impl<'a, Message: 'a> From<ResizablePanel<'a, Message>> for Element<'a, Message> {
    fn from(group: ResizablePanel<'a, Message>) -> Self {
        Element::new(group)
    }
}

/// Whether one handle has keyboard focus, and whether to show it. A press
/// focuses a handle without showing it; a key or a focus operation shows
/// it.
#[derive(Debug, Clone, Copy, Default)]
struct HandleFocus {
    focused: bool,
    visible: bool,
}

impl Focusable for HandleFocus {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
        self.visible = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
        self.visible = false;
    }
}

/// What the widget remembers between events.
#[derive(Debug, Default)]
struct Memory {
    axis: Axis,
    total: f32,
    /// Where each panel starts along the axis, and its length, relative
    /// to the group.
    slots: Vec<(f32, f32)>,
    hovered: Option<usize>,
    handles: Vec<HandleFocus>,
}

impl Memory {
    fn focused(&self) -> Option<usize> {
        self.handles.iter().position(|handle| handle.focused)
    }

    /// The focused handle, if its focus should be drawn.
    fn visible(&self) -> Option<usize> {
        self.handles
            .iter()
            .position(|handle| handle.focused && handle.visible)
    }

    /// Focuses one handle, or none, without showing the focus.
    fn focus(&mut self, index: Option<usize>) {
        for (i, handle) in self.handles.iter_mut().enumerate() {
            handle.focused = Some(i) == index;
            handle.visible = false;
        }
    }

    fn show_focus(&mut self) {
        for handle in &mut self.handles {
            handle.visible = handle.focused;
        }
    }

    /// The space panel `index` has, in window coordinates.
    fn slot(&self, bounds: Rectangle, index: usize) -> Option<Rectangle> {
        let (start, length) = *self.slots.get(index)?;
        Some(match self.axis {
            Axis::Horizontal => Rectangle {
                x: bounds.x + start,
                width: length,
                ..bounds
            },
            Axis::Vertical => Rectangle {
                y: bounds.y + start,
                height: length,
                ..bounds
            },
        })
    }

    /// The line of each handle, after every panel but the last.
    fn lines(&self, bounds: Rectangle) -> Vec<Rectangle> {
        let count = self.slots.len().saturating_sub(1);
        self.slots
            .iter()
            .take(count)
            .map(|(start, length)| match self.axis {
                Axis::Horizontal => Rectangle {
                    x: bounds.x + start + length,
                    width: HANDLE,
                    ..bounds
                },
                Axis::Vertical => Rectangle {
                    y: bounds.y + start + length,
                    height: HANDLE,
                    ..bounds
                },
            })
            .collect()
    }

    fn handle_at(&self, bounds: Rectangle, point: Point) -> Option<usize> {
        self.lines(bounds)
            .into_iter()
            .position(|line| hit_area(line, self.axis).contains(point))
    }
}

/// The area around a handle's line that responds to the pointer.
fn hit_area(line: Rectangle, axis: Axis) -> Rectangle {
    let grow = (HIT_AREA - HANDLE) / 2.0;
    match axis {
        Axis::Horizontal => Rectangle {
            x: line.x - grow,
            width: HIT_AREA,
            ..line
        },
        Axis::Vertical => Rectangle {
            y: line.y - grow,
            height: HIT_AREA,
            ..line
        },
    }
}

fn pointer(event: &Input, cursor: mouse::Cursor) -> Option<Point> {
    match event {
        Input::Touch(
            touch::Event::FingerPressed { position, .. }
            | touch::Event::FingerMoved { position, .. },
        ) => Some(*position),
        _ => cursor.position(),
    }
}

impl<Message> Widget<Message, Theme, Renderer> for ResizablePanel<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Memory>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Memory::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn children(&self) -> Vec<Tree> {
        self.panes.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.panes);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        let max = limits.max();
        let axis = self.axis_at(max.width);
        let count = self.panes.len();
        let handles = count.saturating_sub(1) as f32 * HANDLE;
        let main = axis.main(max);
        let total = if main.is_finite() {
            (main - handles).max(0.0)
        } else {
            0.0
        };
        let cross = axis.cross(max);
        let sizes = self.state.resolve(total);

        let mut start = 0.0_f32;
        let mut used = 0.0_f32;
        let mut widest = 0.0_f32;
        let mut nodes = Vec::with_capacity(count);
        let mut slots = Vec::with_capacity(count);
        for (index, (pane, tree)) in self.panes.iter_mut().zip(&mut tree.children).enumerate() {
            used += sizes.get(index).copied().unwrap_or(0.0);
            let end = used.round();
            let length = (end - start).max(0.0);
            let offset = start + index as f32 * HANDLE;
            let bounds = axis.size(length, cross);
            let node = pane
                .as_widget_mut()
                .layout(tree, renderer, &layout::Limits::new(Size::ZERO, bounds))
                .move_to(axis.point(offset));
            widest = widest.max(axis.cross(node.size()));
            nodes.push(node);
            slots.push((offset, length));
            start = end;
        }

        let cross = if cross.is_finite() { cross } else { widest };
        let memory = tree.state.downcast_mut::<Memory>();
        memory.axis = axis;
        memory.total = total;
        memory.slots = slots;
        memory
            .handles
            .resize(count.saturating_sub(1), HandleFocus::default());

        let size = limits.resolve(self.width, self.height, axis.size(total + handles, cross));
        layout::Node::with_children(size, nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        let Tree {
            state, children, ..
        } = tree;
        let memory = state.downcast_mut::<Memory>();
        let lines = memory.lines(layout.bounds());
        let id = self.id.as_ref();
        operation.traverse(&mut |operation| {
            for (index, (handle, line)) in memory.handles.iter_mut().zip(&lines).enumerate() {
                let id = id.filter(|_| index == 0);
                operation.focusable(id, *line, handle);
            }
            for ((pane, tree), layout) in self
                .panes
                .iter_mut()
                .zip(children.iter_mut())
                .zip(layout.children())
            {
                pane.as_widget_mut()
                    .operate(tree, layout, renderer, operation);
            }
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Input,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Some(on_event) = &self.on_event {
            let memory = tree.state.downcast_mut::<Memory>();
            if handle_event(
                self.state,
                &self.keymap,
                on_event,
                memory,
                event,
                layout,
                cursor,
                shell,
            ) {
                return;
            }
        }

        for ((pane, tree), layout) in self
            .panes
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            pane.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let memory = tree.state.downcast_ref::<Memory>();
        if self.on_event.is_some() {
            let over = cursor
                .position()
                .and_then(|point| memory.handle_at(layout.bounds(), point));
            if self.state.dragging().is_some() || over.is_some() {
                return memory.axis.cursor();
            }
        }
        self.panes
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((pane, tree), layout)| {
                pane.as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;

        let memory = tree.state.downcast_ref::<Memory>();
        let bounds = layout.bounds();
        for (index, ((pane, tree), layout)) in self
            .panes
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
        {
            let Some(slot) = memory.slot(bounds, index) else {
                continue;
            };
            if slot.width < 1.0 || slot.height < 1.0 {
                continue;
            }
            let Some(clip) = slot.intersection(viewport) else {
                continue;
            };
            renderer.with_layer(clip, |renderer| {
                pane.as_widget()
                    .draw(tree, renderer, theme, style, layout, cursor, &clip);
            });
        }

        let tokens = Tokens::of(theme);
        let focused = memory.visible();
        let over = cursor
            .position()
            .and_then(|point| memory.handle_at(bounds, point));
        for (index, line) in memory.lines(bounds).into_iter().enumerate() {
            let status = if self.on_event.is_none() {
                HandleStatus::Disabled
            } else if self.state.dragging() == Some(index) {
                HandleStatus::Dragged
            } else if focused == Some(index) {
                HandleStatus::Focused
            } else if over == Some(index) {
                HandleStatus::Hovered
            } else {
                HandleStatus::Idle
            };
            draw_handle(
                renderer,
                line,
                memory.axis,
                handle_style(&tokens, status),
                self.grip,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.panes,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

/// Handles pointer and key events on the handles. Returns whether the
/// event was used, so the panes never see it.
#[allow(clippy::too_many_arguments)]
fn handle_event<Message>(
    state: &State,
    keymap: &Keymap<Action>,
    on_event: &dyn Fn(Event) -> Message,
    memory: &mut Memory,
    event: &Input,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    shell: &mut Shell<'_, Message>,
) -> bool {
    if memory.total > 0.0 && (state.total() - memory.total).abs() > 0.5 {
        shell.publish(on_event(Event::Measure(memory.total)));
    }
    let axis = memory.axis;

    match event {
        Input::Mouse(mouse::Event::CursorMoved { .. })
        | Input::Touch(touch::Event::FingerMoved { .. }) => {
            let Some(point) = pointer(event, cursor) else {
                return false;
            };
            if state.dragging().is_some() {
                shell.publish(on_event(Event::Drag(axis.along(point))));
                shell.capture_event();
                return true;
            }
            let hovered = memory.handle_at(layout.bounds(), point);
            if hovered != memory.hovered {
                memory.hovered = hovered;
                shell.request_redraw();
            }
            false
        }
        Input::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        | Input::Touch(touch::Event::FingerPressed { .. }) => {
            let hit = pointer(event, cursor).and_then(|point| {
                memory
                    .handle_at(layout.bounds(), point)
                    .map(|handle| (handle, point))
            });
            let Some((handle, point)) = hit else {
                if memory.focused().is_some() {
                    memory.focus(None);
                    shell.request_redraw();
                }
                return false;
            };
            memory.focus(Some(handle));
            shell.publish(on_event(Event::DragStart {
                handle,
                position: axis.along(point),
            }));
            shell.capture_event();
            true
        }
        Input::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | Input::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. }) => {
            if state.dragging().is_none() {
                return false;
            }
            shell.publish(on_event(Event::DragEnd));
            shell.capture_event();
            true
        }
        Input::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
            let Some(handle) = memory.focused() else {
                return false;
            };
            let Some(action) = keymap.resolve(key, *modifiers) else {
                return false;
            };
            if action == Action::Release {
                memory.focus(None);
            } else {
                memory.show_focus();
                if let Some(event) = action.event(state, handle) {
                    shell.publish(on_event(event));
                }
            }
            shell.request_redraw();
            shell.capture_event();
            true
        }
        _ => false,
    }
}

fn draw_handle(
    renderer: &mut Renderer,
    line: Rectangle,
    axis: Axis,
    style: HandleStyle,
    grip: bool,
) {
    use iced::advanced::Renderer as _;

    let quad = |bounds: Rectangle, radius: f32, border: Option<Color>| renderer::Quad {
        bounds,
        border: Border {
            color: border.unwrap_or(Color::TRANSPARENT),
            width: if border.is_some() { 1.0 } else { 0.0 },
            radius: radius.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    };

    renderer.fill_quad(quad(line, 0.0, None), Background::Color(style.line));

    if let Some(colour) = style.highlight {
        let grow = (HIGHLIGHT - HANDLE) / 2.0;
        let bar = match axis {
            Axis::Horizontal => Rectangle {
                x: line.x - grow,
                width: HIGHLIGHT,
                ..line
            },
            Axis::Vertical => Rectangle {
                y: line.y - grow,
                height: HIGHLIGHT,
                ..line
            },
        };
        renderer.fill_quad(quad(bar, 0.0, None), Background::Color(colour));
    }

    if !grip {
        return;
    }
    let size = match axis {
        Axis::Horizontal => GRIP,
        Axis::Vertical => Size::new(GRIP.height, GRIP.width),
    };
    let centre = line.center();
    let bounds = Rectangle::new(
        Point::new(
            (centre.x - size.width / 2.0).round(),
            (centre.y - size.height / 2.0).round(),
        ),
        size,
    );
    renderer.fill_quad(
        quad(bounds, radius::SM, Some(style.grip_border)),
        Background::Color(style.grip),
    );

    // Two rows of three dots, turned to follow the handle.
    let dot = 2.0;
    for column in [-1.0_f32, 1.0] {
        for row in [-1.0_f32, 0.0, 1.0] {
            let (dx, dy) = match axis {
                Axis::Horizontal => (column * 2.0, row * 4.0),
                Axis::Vertical => (row * 4.0, column * 2.0),
            };
            let position = Point::new(
                (centre.x + dx - dot / 2.0).round(),
                (centre.y + dy - dot / 2.0).round(),
            );
            renderer.fill_quad(
                quad(Rectangle::new(position, Size::new(dot, dot)), 1.0, None),
                Background::Color(style.grip_dots),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    const TOTAL: f32 = 1000.0;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.05
    }

    fn assert_sizes(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len(), "{actual:?}");
        for (a, b) in actual.iter().zip(expected) {
            assert!(close(*a, *b), "{actual:?} != {expected:?}");
        }
    }

    fn measured(panels: impl IntoIterator<Item = Panel>) -> State {
        let mut state = State::new(panels);
        let _ = state.update(Event::Measure(TOTAL));
        state
    }

    fn pixels(state: &State) -> Vec<f32> {
        state.resolve(state.total())
    }

    fn sidebar_and_content() -> [Panel; 2] {
        [
            panel(Extent::Fraction(0.3))
                .min(Extent::Pixels(200.0))
                .max(Extent::Fraction(0.5))
                .collapsible(true),
            panel(Extent::Fraction(0.7)).min(Extent::Pixels(300.0)),
        ]
    }

    #[test]
    fn panel_defaults() {
        let panel = panel(Extent::Fraction(0.5));
        assert_eq!(panel.min, Extent::Pixels(0.0));
        assert_eq!(panel.max, Extent::Fraction(1.0));
        assert!(!panel.collapsible);
        assert_eq!(panel.collapsed_size, Extent::Pixels(0.0));
        assert_eq!(panel.collapse_below, None);
    }

    #[test]
    fn extents_resolve_to_pixels() {
        assert_eq!(Extent::Fraction(0.25).resolve(800.0), 200.0);
        assert_eq!(Extent::Pixels(120.0).resolve(800.0), 120.0);
        assert_eq!(Extent::Pixels(-5.0).resolve(800.0), 0.0);
    }

    #[test]
    fn limits_keep_max_above_min_and_threshold_between() {
        let limits = panel(Extent::Fraction(0.5))
            .min(Extent::Pixels(300.0))
            .max(Extent::Pixels(100.0))
            .collapsible(true)
            .collapsed_size(Extent::Pixels(40.0))
            .limits(TOTAL);
        assert_eq!(limits.min, 300.0);
        assert_eq!(limits.max, 300.0);
        assert_eq!(limits.collapsed, 40.0);
        assert_eq!(limits.threshold, 170.0);

        let custom = panel(Extent::Fraction(0.5))
            .min(Extent::Pixels(200.0))
            .collapsible(true)
            .collapse_below(Extent::Pixels(500.0))
            .limits(TOTAL);
        assert_eq!(custom.threshold, 200.0, "never above the minimum");
    }

    #[test]
    fn settle_clamps_and_snaps_at_the_threshold() {
        let plain = panel(Extent::Fraction(0.5))
            .min(Extent::Pixels(100.0))
            .max(Extent::Pixels(400.0));
        assert_eq!(plain.settle(50.0, TOTAL), 100.0);
        assert_eq!(plain.settle(250.0, TOTAL), 250.0);
        assert_eq!(plain.settle(900.0, TOTAL), 400.0);

        let collapsible = plain.collapsible(true);
        assert_eq!(
            collapsible.settle(60.0, TOTAL),
            100.0,
            "above the threshold"
        );
        assert_eq!(collapsible.settle(49.0, TOTAL), 0.0, "below the threshold");
        assert!(collapsible.is_collapsed_at(0.0, TOTAL));
        assert!(!plain.is_collapsed_at(0.0, TOTAL));
    }

    #[test]
    fn defaults_share_what_pixel_panels_leave() {
        let panels = [
            panel(Extent::Pixels(250.0)),
            panel(Extent::Fraction(1.0)),
            panel(Extent::Fraction(1.0)),
        ];
        assert_sizes(&defaults(&panels, TOTAL), &[250.0, 375.0, 375.0]);
    }

    #[test]
    fn defaults_respect_limits_and_initial_collapse() {
        let panels = [
            panel(Extent::Fraction(0.1)).min(Extent::Pixels(200.0)),
            panel(Extent::Fraction(0.9)),
        ];
        assert_sizes(&defaults(&panels, TOTAL), &[200.0, 800.0]);

        let collapsed = [
            panel(Extent::Fraction(0.3))
                .collapsible(true)
                .collapsed_size(Extent::Pixels(48.0))
                .min(Extent::Pixels(200.0))
                .collapsed(true),
            panel(Extent::Fraction(0.7)),
        ];
        assert_sizes(&defaults(&collapsed, TOTAL), &[48.0, 952.0]);
    }

    #[test]
    fn fit_always_adds_up() {
        let panels = sidebar_and_content();
        for total in [1200.0, 600.0, 450.0, 320.0, 100.0] {
            let sizes = fit(
                &panels,
                &[false, false],
                vec![0.3 * total, 0.7 * total],
                total,
            );
            let sum: f32 = sizes.iter().sum();
            assert!(close(sum, total), "{total}: {sizes:?}");
        }
    }

    #[test]
    fn fit_collapses_side_panels_when_minimums_do_not_fit() {
        let panels = sidebar_and_content();
        // 200 + 300 minimums do not fit in 360, so the sidebar collapses.
        let sizes = fit(&panels, &[false, false], vec![108.0, 252.0], 360.0);
        assert_sizes(&sizes, &[0.0, 360.0]);
    }

    #[test]
    fn fit_scales_as_a_last_resort() {
        let panels = [
            panel(Extent::Fraction(0.5)).min(Extent::Pixels(300.0)),
            panel(Extent::Fraction(0.5)).min(Extent::Pixels(300.0)),
        ];
        let sizes = fit(&panels, &[false, false], vec![200.0, 200.0], 400.0);
        assert_sizes(&sizes, &[200.0, 200.0]);
    }

    #[test]
    fn state_starts_from_defaults_once_measured() {
        let state = State::new(sidebar_and_content());
        assert_eq!(state.total(), 0.0);
        assert_sizes(&state.sizes(), &[0.3, 0.7]);
        assert_sizes(&state.resolve(TOTAL), &[300.0, 700.0]);

        let state = measured(sidebar_and_content());
        assert_eq!(state.total(), TOTAL);
        assert_sizes(&state.sizes(), &[0.3, 0.7]);
    }

    #[test]
    fn unmeasured_state_ignores_changes() {
        let mut state = State::new(sidebar_and_content());
        assert_eq!(
            state.update(Event::Resize {
                handle: 0,
                delta: 50.0
            }),
            None
        );
        assert_eq!(state.update(Event::Collapse(0)), None);
        let _ = state.update(Event::Measure(-1.0));
        let _ = state.update(Event::Measure(f32::INFINITY));
        assert_eq!(state.total(), 0.0);
    }

    #[test]
    fn saved_sizes_round_trip_and_bad_ones_are_ignored() {
        let state = State::new(sidebar_and_content()).with_sizes([2.0, 6.0]);
        assert_sizes(&state.sizes(), &[0.25, 0.75]);

        let wrong_length = State::new(sidebar_and_content()).with_sizes([1.0]);
        assert_sizes(&wrong_length.sizes(), &[0.3, 0.7]);
        let empty = State::new(sidebar_and_content()).with_sizes([0.0, 0.0]);
        assert_sizes(&empty.sizes(), &[0.3, 0.7]);

        let collapsed = State::new(sidebar_and_content()).with_sizes([0.0, 1.0]);
        assert!(collapsed.is_collapsed(0));
    }

    #[test]
    fn dragging_moves_the_handle_from_where_it_started() {
        let mut state = measured(sidebar_and_content());
        let _ = state.update(Event::DragStart {
            handle: 0,
            position: 300.0,
        });
        assert_eq!(state.dragging(), Some(0));
        assert_eq!(state.update(Event::Drag(340.0)), Some(Output::Resized));
        assert_sizes(&pixels(&state), &[340.0, 660.0]);
        assert_eq!(state.update(Event::Drag(320.0)), Some(Output::Resized));
        assert_sizes(&pixels(&state), &[320.0, 680.0]);
        let _ = state.update(Event::DragEnd);
        assert_eq!(state.dragging(), None);
        assert_eq!(state.update(Event::Drag(500.0)), None);
    }

    #[test]
    fn drags_clamp_to_min_and_max() {
        let mut state = measured(sidebar_and_content());
        let _ = state.update(Event::DragStart {
            handle: 0,
            position: 300.0,
        });
        let _ = state.update(Event::Drag(900.0));
        assert_sizes(&pixels(&state), &[500.0, 500.0]);
        let _ = state.update(Event::Drag(160.0));
        assert_sizes(&pixels(&state), &[200.0, 800.0]);
    }

    #[test]
    fn collapsible_panels_snap_shut_below_the_threshold_and_open_past_it() {
        let mut state = measured(sidebar_and_content());
        let _ = state.update(Event::DragStart {
            handle: 0,
            position: 300.0,
        });
        // Threshold is 100: halfway between collapsed (0) and min (200).
        let _ = state.update(Event::Drag(110.0));
        assert_sizes(&pixels(&state), &[200.0, 800.0]);
        assert_eq!(state.update(Event::Drag(90.0)), Some(Output::Collapsed(0)));
        assert!(state.is_collapsed(0));
        assert_sizes(&pixels(&state), &[0.0, 1000.0]);
        assert_eq!(state.update(Event::Drag(120.0)), Some(Output::Expanded(0)));
        assert_sizes(&pixels(&state), &[200.0, 800.0]);
        let _ = state.update(Event::DragEnd);

        let _ = state.update(Event::Collapse(0));
        let _ = state.update(Event::DragStart {
            handle: 0,
            position: 0.0,
        });
        assert_eq!(
            state.update(Event::Drag(60.0)),
            None,
            "still below the threshold"
        );
        assert!(state.is_collapsed(0));
        assert_eq!(state.update(Event::Drag(101.0)), Some(Output::Expanded(0)));
        assert_sizes(&pixels(&state), &[200.0, 800.0]);
    }

    #[test]
    fn shrinking_carries_on_past_a_panel_at_its_minimum() {
        let mut state = measured([
            panel(Extent::Fraction(0.4)),
            panel(Extent::Fraction(0.3)).min(Extent::Pixels(200.0)),
            panel(Extent::Fraction(0.3)).min(Extent::Pixels(100.0)),
        ]);
        let _ = state.update(Event::Resize {
            handle: 0,
            delta: 250.0,
        });
        assert_sizes(&pixels(&state), &[650.0, 200.0, 150.0]);
        let _ = state.update(Event::Resize {
            handle: 0,
            delta: 500.0,
        });
        assert_sizes(&pixels(&state), &[700.0, 200.0, 100.0]);
    }

    #[test]
    fn growth_past_a_maximum_passes_outwards() {
        let origin = [200.0, 300.0, 500.0];
        let panels = [
            panel(Extent::Fraction(0.2)),
            panel(Extent::Fraction(0.3)).max(Extent::Pixels(350.0)),
            panel(Extent::Fraction(0.5)),
        ];
        let sizes = resize(&panels, &origin, 1, 100.0, TOTAL);
        assert_sizes(&sizes, &[250.0, 350.0, 400.0]);
    }

    #[test]
    fn resize_ignores_bad_handles_and_zero_moves() {
        let panels = sidebar_and_content();
        let origin = [300.0, 700.0];
        assert_eq!(resize(&panels, &origin, 1, 50.0, TOTAL), origin);
        assert_eq!(resize(&panels, &origin, 0, 0.0, TOTAL), origin);
        assert_eq!(resize(&panels, &[300.0], 0, 50.0, TOTAL), [300.0]);
    }

    #[test]
    fn toggle_collapses_and_restores_the_previous_size() {
        let mut state = measured(sidebar_and_content());
        let _ = state.update(Event::Resize {
            handle: 0,
            delta: 100.0,
        });
        assert_eq!(state.update(Event::Toggle(0)), Some(Output::Collapsed(0)));
        assert_sizes(&pixels(&state), &[0.0, 1000.0]);
        assert_eq!(state.update(Event::Collapse(0)), None);
        assert_eq!(state.update(Event::Toggle(0)), Some(Output::Expanded(0)));
        assert_sizes(&pixels(&state), &[400.0, 600.0]);
        assert_eq!(state.update(Event::Expand(0)), None);
    }

    #[test]
    fn the_last_panel_collapses_through_the_handle_before_it() {
        let mut state = measured([
            panel(Extent::Fraction(0.6)),
            panel(Extent::Fraction(0.4))
                .collapsible(true)
                .collapsed_size(Extent::Pixels(40.0))
                .min(Extent::Pixels(200.0)),
        ]);
        assert_eq!(state.update(Event::Collapse(1)), Some(Output::Collapsed(1)));
        assert_sizes(&pixels(&state), &[960.0, 40.0]);
        assert_eq!(state.update(Event::Expand(1)), Some(Output::Expanded(1)));
        assert_sizes(&pixels(&state), &[600.0, 400.0]);
    }

    #[test]
    fn panels_that_cannot_collapse_ignore_it() {
        let mut state = measured(sidebar_and_content());
        assert_eq!(state.update(Event::Collapse(1)), None);
        assert_eq!(state.update(Event::Toggle(7)), None);
    }

    #[test]
    fn reset_puts_the_defaults_back() {
        let mut state = measured(sidebar_and_content());
        let _ = state.update(Event::Collapse(0));
        assert_eq!(state.update(Event::Reset), Some(Output::Resized));
        assert!(!state.is_collapsed(0));
        assert_sizes(&pixels(&state), &[300.0, 700.0]);
        assert_eq!(state.update(Event::Reset), None);
    }

    #[test]
    fn keyboard_steps_follow_the_step_sizes() {
        let state = measured(sidebar_and_content());
        let resize = |action: Action| action.event(&state, 0);
        assert_eq!(
            resize(Action::Increase),
            Some(Event::Resize {
                handle: 0,
                delta: 50.0
            })
        );
        assert_eq!(
            resize(Action::Decrease),
            Some(Event::Resize {
                handle: 0,
                delta: -50.0
            })
        );
        assert_eq!(
            resize(Action::IncreaseMore),
            Some(Event::Resize {
                handle: 0,
                delta: 200.0
            })
        );
        assert_eq!(
            resize(Action::DecreaseMore),
            Some(Event::Resize {
                handle: 0,
                delta: -200.0
            })
        );

        let pixels = measured(sidebar_and_content())
            .step(Extent::Pixels(10.0))
            .large_step(Extent::Pixels(40.0));
        assert_eq!(
            Action::Increase.event(&pixels, 0),
            Some(Event::Resize {
                handle: 0,
                delta: 10.0
            })
        );
        assert_eq!(
            Action::DecreaseMore.event(&pixels, 0),
            Some(Event::Resize {
                handle: 0,
                delta: -40.0
            })
        );
    }

    #[test]
    fn home_and_end_go_to_min_and_max() {
        let mut state = measured(sidebar_and_content());
        let home = Action::Min.event(&state, 0);
        assert_eq!(
            home,
            Some(Event::Resize {
                handle: 0,
                delta: -100.0
            })
        );
        if let Some(event) = home {
            let _ = state.update(event);
        }
        assert_sizes(&pixels(&state), &[200.0, 800.0]);
        if let Some(event) = Action::Max.event(&state, 0) {
            let _ = state.update(event);
        }
        assert_sizes(&pixels(&state), &[500.0, 500.0]);
    }

    #[test]
    fn toggle_and_release_actions() {
        let state = measured(sidebar_and_content());
        assert_eq!(Action::Toggle.event(&state, 0), Some(Event::Toggle(0)));
        assert_eq!(Action::Release.event(&state, 0), None);
        assert_eq!(Action::Increase.event(&state, 1), None, "no such handle");

        let neither = measured([panel(Extent::Fraction(0.5)), panel(Extent::Fraction(0.5))]);
        assert_eq!(Action::Toggle.event(&neither, 0), None);
        let after = measured([
            panel(Extent::Fraction(0.5)),
            panel(Extent::Fraction(0.5)).collapsible(true),
        ]);
        assert_eq!(Action::Toggle.event(&after, 0), Some(Event::Toggle(1)));
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "ArrowLeft"), Some(Action::Decrease));
        assert_eq!(press(&keymap, "ArrowUp"), Some(Action::Decrease));
        assert_eq!(press(&keymap, "ArrowRight"), Some(Action::Increase));
        assert_eq!(press(&keymap, "ArrowDown"), Some(Action::Increase));
        assert_eq!(
            press(&keymap, "Shift+ArrowLeft"),
            Some(Action::DecreaseMore)
        );
        assert_eq!(
            press(&keymap, "Shift+ArrowDown"),
            Some(Action::IncreaseMore)
        );
        assert_eq!(press(&keymap, "Home"), Some(Action::Min));
        assert_eq!(press(&keymap, "End"), Some(Action::Max));
        assert_eq!(press(&keymap, "Enter"), Some(Action::Toggle));
        assert_eq!(press(&keymap, "Escape"), Some(Action::Release));
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?}");
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&"Enter".parse().unwrap())
            .bind("Space".parse().unwrap(), Action::Toggle);
        assert_eq!(press(&keymap, "Enter"), None);
        assert_eq!(press(&keymap, "Space"), Some(Action::Toggle));
        assert!(keymap.clear().is_empty());
    }

    #[test]
    fn stacking_only_turns_narrow_horizontal_groups() {
        let state = State::new(sidebar_and_content());
        let group: ResizablePanel<'_, ()> = resizable_panel(&state, []).stack_below(480.0);
        assert_eq!(group.axis_at(360.0), Axis::Vertical);
        assert_eq!(group.axis_at(720.0), Axis::Horizontal);
        let plain: ResizablePanel<'_, ()> = resizable_panel(&state, []);
        assert_eq!(plain.axis_at(360.0), Axis::Horizontal);
        let vertical: ResizablePanel<'_, ()> = resizable_panel(&state, [])
            .axis(Axis::Vertical)
            .stack_below(480.0);
        assert_eq!(vertical.axis_at(360.0), Axis::Vertical);
    }

    #[test]
    fn builder_defaults() {
        let state = State::new(sidebar_and_content());
        let group: ResizablePanel<'_, ()> = resizable_panel(&state, []);
        assert_eq!(group.axis, Axis::Horizontal);
        assert!(!group.grip);
        assert_eq!(group.width, Length::Fill);
        assert_eq!(group.height, Length::Fill);
        assert!(group.on_event.is_none());
        assert_eq!(group.keymap, default_keymap());
    }

    #[test]
    fn axes_pick_their_coordinate_and_cursor() {
        let point = Point::new(3.0, 7.0);
        assert_eq!(Axis::Horizontal.along(point), 3.0);
        assert_eq!(Axis::Vertical.along(point), 7.0);
        assert_eq!(
            Axis::Horizontal.cursor(),
            mouse::Interaction::ResizingColumn
        );
        assert_eq!(Axis::Vertical.cursor(), mouse::Interaction::ResizingRow);
    }

    #[test]
    fn a_press_focuses_quietly_and_keys_or_operations_show_it() {
        let mut memory = Memory {
            handles: vec![HandleFocus::default(); 2],
            ..Memory::default()
        };
        memory.focus(Some(1));
        assert_eq!(memory.focused(), Some(1));
        assert_eq!(memory.visible(), None);
        memory.show_focus();
        assert_eq!(memory.visible(), Some(1));
        memory.focus(None);
        assert_eq!(memory.focused(), None);

        memory.handles[0].focus();
        assert_eq!(memory.visible(), Some(0), "focus operations show it");
        memory.handles[0].unfocus();
        assert_eq!(memory.focused(), None);
    }

    #[test]
    fn saved_sizes_below_the_threshold_restore_collapsed() {
        let panels = [
            panel(Extent::Pixels(240.0))
                .min(Extent::Pixels(180.0))
                .collapsible(true)
                .collapsed_size(Extent::Pixels(48.0)),
            panel(Extent::Fraction(1.0)),
        ];
        let mut state = State::new(panels).with_sizes([0.048, 0.952]);
        let _ = state.update(Event::Measure(TOTAL));
        assert!(state.is_collapsed(0));
        assert_sizes(&pixels(&state), &[48.0, 952.0]);

        // A wider window keeps the rail at its pixel size.
        let _ = state.update(Event::Measure(2000.0));
        assert_sizes(&state.resolve(2000.0), &[48.0, 1952.0]);
        assert_eq!(state.update(Event::Expand(0)), Some(Output::Expanded(0)));
        assert_sizes(&state.resolve(2000.0), &[240.0, 1760.0]);
    }

    #[test]
    fn handles_highlight_on_hover_drag_and_focus() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let idle = handle_style(&tokens, HandleStatus::Idle);
            assert_eq!(idle.line, tokens.border);
            assert_eq!(idle.highlight, None);
            assert_eq!(
                handle_style(&tokens, HandleStatus::Disabled).highlight,
                None
            );
            assert_eq!(
                handle_style(&tokens, HandleStatus::Hovered).highlight,
                Some(tokens.border)
            );
            for status in [HandleStatus::Dragged, HandleStatus::Focused] {
                let style = handle_style(&tokens, status);
                assert_eq!(style.highlight, Some(tokens.ring));
                assert_eq!(style.grip_border, tokens.ring);
            }
        }
    }
}
