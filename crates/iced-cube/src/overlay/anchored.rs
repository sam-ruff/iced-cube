//! Floats a panel next to an anchor element: the one layer popovers, menus,
//! the combobox and select lists, and the command list's keys are built on.
//!
//! [`anchored`] wraps an anchor, such as a trigger button or a text field,
//! and while it has content draws that content above everything else.
//! [`place`] is the pure positioning logic: it puts the panel on one
//! [`Side`] of the anchor, aligned by [`Align`], then flips and shifts it to
//! stay inside the window.
//!
//! Every open panel follows the same rules:
//!
//! - Events reach the content first, including panels nested inside it, so
//!   Escape closes the innermost layer.
//! - [`on_key`](Anchored::on_key) sees the key presses the content leaves,
//!   while the panel is open or something in the anchor has focus.
//! - [`on_dismiss`](Anchored::on_dismiss) is sent at most once per event: on
//!   a [dismiss chord](Anchored::dismiss_keys), a press outside the panel and
//!   the anchor, or, with [`dismiss_on_blur`](Anchored::dismiss_on_blur),
//!   when the anchor loses focus.
//! - That outside press is captured, so nothing underneath sees it, unless
//!   the panel [passes it through](Anchored::pass_through).
//! - Presses on the anchor are left to the anchor, so a trigger can toggle
//!   the panel itself. Presses inside the panel never reach the widgets
//!   underneath it.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Id, Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard::{self, key::Named};
use iced::widget::container;
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Shadow, Size,
    Theme, Vector, mouse, touch,
};

use crate::keys::{self, Chord};
use crate::theme::{Tokens, fade, radius};

/// Default space between the anchor and the panel, in logical pixels.
pub const GAP: f32 = 4.0;

/// Space kept between the panel and the edges of the window.
pub const MARGIN: f32 = 8.0;

/// The side of the anchor the panel appears on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Side {
    Top,
    #[default]
    Bottom,
    Left,
    Right,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::Top, Side::Bottom, Side::Left, Side::Right];

    pub fn opposite(self) -> Side {
        match self {
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    fn is_vertical(self) -> bool {
        matches!(self, Side::Top | Side::Bottom)
    }
}

/// How the panel lines up with the anchor along that side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Align {
    /// Left edges meet (or top edges, on the left and right sides).
    Start,
    #[default]
    Center,
    /// Right edges meet (or bottom edges, on the left and right sides).
    End,
}

impl Align {
    pub const ALL: [Align; 3] = [Align::Start, Align::Center, Align::End];

    fn mirrored(self) -> Align {
        match self {
            Align::Start => Align::End,
            Align::Center => Align::Center,
            Align::End => Align::Start,
        }
    }
}

/// Where a panel sits relative to its anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub side: Side,
    pub align: Align,
    /// Space between the anchor and the panel.
    pub gap: f32,
    /// Shift along the side, away from the aligned edge.
    pub offset: f32,
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            side: Side::default(),
            align: Align::default(),
            gap: GAP,
            offset: 0.0,
        }
    }
}

impl Placement {
    pub fn new(side: Side, align: Align) -> Self {
        Self {
            side,
            align,
            ..Self::default()
        }
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }
}

/// Positions a panel of `size` next to `anchor`, keeping it inside
/// `viewport` less [`MARGIN`].
///
/// The panel moves to the opposite side when it does not fit on the
/// requested one and does fit there, and to the mirrored alignment when it
/// would overflow along the side. Whatever still overflows is shifted back
/// inside, pinned to the top left if the panel is larger than the window.
pub fn place(
    anchor: Rectangle,
    size: Size,
    viewport: Rectangle,
    placement: Placement,
) -> Rectangle {
    let area = viewport.shrink(MARGIN);
    let room = |side: Side| match side {
        Side::Top => anchor.y - placement.gap - area.y,
        Side::Bottom => area.y + area.height - (anchor.y + anchor.height + placement.gap),
        Side::Left => anchor.x - placement.gap - area.x,
        Side::Right => area.x + area.width - (anchor.x + anchor.width + placement.gap),
    };
    let needed = |side: Side| {
        if side.is_vertical() {
            size.height
        } else {
            size.width
        }
    };

    let requested = placement.side;
    let opposite = requested.opposite();
    let side = if room(requested) >= needed(requested) {
        requested
    } else if room(opposite) >= needed(opposite) || room(opposite) > room(requested) {
        opposite
    } else {
        requested
    };

    let main = match side {
        Side::Top => anchor.y - placement.gap - size.height,
        Side::Bottom => anchor.y + anchor.height + placement.gap,
        Side::Left => anchor.x - placement.gap - size.width,
        Side::Right => anchor.x + anchor.width + placement.gap,
    };

    let (start, length, extent, low, high) = if side.is_vertical() {
        (
            anchor.x,
            anchor.width,
            size.width,
            area.x,
            area.x + area.width,
        )
    } else {
        (
            anchor.y,
            anchor.height,
            size.height,
            area.y,
            area.y + area.height,
        )
    };
    let cross_at = |align: Align| match align {
        Align::Start => start + placement.offset,
        Align::Center => start + (length - extent) / 2.0 + placement.offset,
        Align::End => start + length - extent - placement.offset,
    };
    let fits = |value: f32| value >= low && value + extent <= high;
    let preferred = cross_at(placement.align);
    let mirrored = cross_at(placement.align.mirrored());
    let cross = if !fits(preferred) && fits(mirrored) {
        mirrored
    } else {
        preferred
    };

    let (x, y) = if side.is_vertical() {
        (cross, main)
    } else {
        (main, cross)
    };
    Rectangle::new(
        Point::new(
            clamp(x, area.x, area.x + area.width - size.width),
            clamp(y, area.y, area.y + area.height - size.height),
        ),
        size,
    )
}

/// Keeps `value` within `low..=high`, preferring `low` when the range is empty.
fn clamp(value: f32, low: f32, high: f32) -> f32 {
    value.min(high).max(low)
}

/// The floating surface shared by popovers, menus, lists and dialogs: the
/// popover colour, a subtle border and a restrained shadow.
pub fn surface_style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens.popover)),
        text_color: Some(tokens.foreground),
        border: Border {
            color: tokens.border,
            width: 1.0,
            radius: radius::MD.into(),
        },
        shadow: shadow(tokens),
        snap: true,
    }
}

/// The shadow under every floating surface, stronger on a dark page.
pub fn shadow(tokens: &Tokens) -> Shadow {
    Shadow {
        color: fade(Color::BLACK, if tokens.is_dark { 0.5 } else { 0.1 }),
        offset: Vector::new(0.0, 4.0),
        blur_radius: 12.0,
    }
}

type OnKey<'a, Message> = Box<dyn Fn(&keys::Event) -> Option<Message> + 'a>;

/// How an open panel behaves. Shared with the widgets in this crate that
/// float a panel of their own.
pub(crate) struct Behaviour<'a, Message> {
    pub(crate) placement: Placement,
    pub(crate) point: Option<Point>,
    pub(crate) match_width: bool,
    pub(crate) on_dismiss: Option<Message>,
    pub(crate) dismiss_keys: Vec<Chord>,
    pub(crate) dismiss_on_anchor_press: bool,
    pub(crate) dismiss_on_blur: bool,
    pub(crate) pass_through: bool,
    /// The panel opens and closes itself through [`State::open`]: a
    /// dismissal or a press the content captured closes it.
    pub(crate) closes_itself: bool,
    pub(crate) on_key: Option<OnKey<'a, Message>>,
    pub(crate) on_anchor_press: Option<Message>,
}

impl<Message> Default for Behaviour<'_, Message> {
    fn default() -> Self {
        Self {
            placement: Placement::default(),
            point: None,
            match_width: false,
            on_dismiss: None,
            dismiss_keys: vec![Chord::named(Named::Escape)],
            dismiss_on_anchor_press: false,
            dismiss_on_blur: false,
            pass_through: false,
            closes_itself: false,
            on_key: None,
            on_anchor_press: None,
        }
    }
}

impl<Message> Behaviour<'_, Message> {
    fn can_dismiss(&self) -> bool {
        self.on_dismiss.is_some() || self.closes_itself
    }

    fn watches_focus(&self) -> bool {
        self.on_key.is_some() || self.dismiss_on_blur
    }
}

/// What an anchor remembers between events.
#[derive(Debug, Default)]
pub(crate) struct State {
    focused: bool,
    /// An outside press dismissed the panel, so the anchor gives up focus
    /// on the next event, without a second dismissal.
    unfocus: bool,
    /// Whether a panel that [closes itself](Behaviour::closes_itself) is open.
    pub(crate) open: bool,
}

/// An anchor with a panel floating next to it. Convert it into an
/// [`Element`] to render.
pub struct Anchored<'a, Message> {
    anchor: Element<'a, Message>,
    content: Option<Element<'a, Message>>,
    behaviour: Behaviour<'a, Message>,
}

impl<Message> std::fmt::Debug for Anchored<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let behaviour = &self.behaviour;
        f.debug_struct("Anchored")
            .field("open", &self.content.is_some())
            .field("placement", &behaviour.placement)
            .field("point", &behaviour.point)
            .field("match_width", &behaviour.match_width)
            .field("dismissable", &behaviour.on_dismiss.is_some())
            .field("dismiss_keys", &behaviour.dismiss_keys)
            .field("pass_through", &behaviour.pass_through)
            .finish_non_exhaustive()
    }
}

/// Wraps `anchor`. Nothing floats until [`content`](Anchored::content) is set.
pub fn anchored<'a, Message>(anchor: impl Into<Element<'a, Message>>) -> Anchored<'a, Message> {
    Anchored {
        anchor: anchor.into(),
        content: None,
        behaviour: Behaviour::default(),
    }
}

impl<'a, Message> Anchored<'a, Message> {
    /// The floating panel. `None` keeps it closed.
    pub fn content(mut self, content: Option<Element<'a, Message>>) -> Self {
        self.content = content;
        self
    }

    pub fn placement(mut self, placement: Placement) -> Self {
        self.behaviour.placement = placement;
        self
    }

    /// Anchors the panel to a point, relative to the anchor's top left
    /// corner, instead of to the anchor's edges.
    pub fn at(mut self, point: Point) -> Self {
        self.behaviour.point = Some(point);
        self
    }

    /// Makes the panel exactly as wide as the anchor, as a list under a
    /// field is.
    pub fn match_width(mut self, match_width: bool) -> Self {
        self.behaviour.match_width = match_width;
        self
    }

    /// Sent once when the panel is dismissed; see the module docs.
    pub fn on_dismiss(self, message: Message) -> Self {
        self.on_dismiss_maybe(Some(message))
    }

    /// Like [`on_dismiss`](Self::on_dismiss); `None` makes the panel
    /// undismissable, so presses outside it pass through.
    pub fn on_dismiss_maybe(mut self, message: Option<Message>) -> Self {
        self.behaviour.on_dismiss = message;
        self
    }

    /// The chords that dismiss the panel when the content and
    /// [`on_key`](Self::on_key) leave them alone. Defaults to Escape. Pass
    /// the chords of a component keymap's close action, so unbinding them
    /// there works.
    pub fn dismiss_keys(mut self, chords: impl IntoIterator<Item = Chord>) -> Self {
        self.behaviour.dismiss_keys = chords.into_iter().collect();
        self
    }

    /// Also dismiss on presses on the anchor itself, for anchors that do
    /// not toggle the panel, such as a context menu area. The press still
    /// reaches the anchor.
    pub fn dismiss_on_anchor_press(mut self, dismiss: bool) -> Self {
        self.behaviour.dismiss_on_anchor_press = dismiss;
        self
    }

    /// Also dismiss when a focused widget inside the anchor, such as a text
    /// field, loses focus.
    pub fn dismiss_on_blur(mut self, dismiss: bool) -> Self {
        self.behaviour.dismiss_on_blur = dismiss;
        self
    }

    /// Lets a press outside the panel reach the widgets underneath after
    /// dismissing it, for panels that are not modal, such as a popover.
    pub fn pass_through(mut self, pass_through: bool) -> Self {
        self.behaviour.pass_through = pass_through;
        self
    }

    /// Resolves key presses the content leaves, while the panel is open or
    /// something in the anchor has focus. Returning a message sends it and
    /// captures the key.
    pub fn on_key(mut self, on_key: impl Fn(&keys::Event) -> Option<Message> + 'a) -> Self {
        self.behaviour.on_key = Some(Box::new(on_key));
        self
    }

    /// Sent when the pointer is pressed on the anchor, before the anchor
    /// sees the press.
    pub fn on_anchor_press_maybe(mut self, message: Option<Message>) -> Self {
        self.behaviour.on_anchor_press = message;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Anchored<'a, Message>> for Element<'a, Message> {
    fn from(anchored: Anchored<'a, Message>) -> Self {
        Element::new(anchored)
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Anchored<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        self.anchor.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.anchor.as_widget().size_hint()
    }

    fn children(&self) -> Vec<Tree> {
        std::iter::once(&self.anchor)
            .chain(&self.content)
            .map(Tree::new)
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        match &self.content {
            Some(content) => tree.diff_children(&[&self.anchor, content]),
            None => tree.diff_children(&[&self.anchor]),
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.anchor
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
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
        self.anchor.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.anchor
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        let Some(anchor_tree) = children.first_mut() else {
            return;
        };
        let behaviour = &self.behaviour;

        if state.unfocus {
            state.unfocus = false;
            state.focused = false;
            self.anchor
                .as_widget_mut()
                .operate(anchor_tree, layout, renderer, &mut Unfocus);
            shell.request_redraw();
        }

        if state.focused
            && let Some(message) = key_message(behaviour, event)
        {
            shell.publish(message);
            shell.capture_event();
            return;
        }

        if is_press(event)
            && cursor.is_over(layout.bounds())
            && let Some(message) = &behaviour.on_anchor_press
        {
            shell.publish(message.clone());
        }

        self.anchor.as_widget_mut().update(
            anchor_tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if !behaviour.watches_focus() {
            return;
        }
        let mut focus = FindFocus::default();
        self.anchor
            .as_widget_mut()
            .operate(anchor_tree, layout, renderer, &mut focus);
        if state.focused
            && !focus.0
            && behaviour.dismiss_on_blur
            && let Some(message) = &behaviour.on_dismiss
        {
            shell.publish(message.clone());
        }
        state.focused = focus.0;
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.anchor.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let Self {
            anchor,
            content,
            behaviour,
        } = self;
        let Tree {
            state, children, ..
        } = tree;
        let mut trees = children.iter_mut();
        let anchor_tree = trees.next()?;
        let trigger = layout.bounds() + translation;

        // While the panel is open the anchor's own overlays, such as its
        // tooltip, stay hidden.
        let Some((content, tree)) = content.as_mut().zip(trees.next()) else {
            return anchor.as_widget_mut().overlay(
                anchor_tree,
                layout,
                renderer,
                viewport,
                translation,
            );
        };
        Some(floating(
            content,
            tree,
            state.downcast_mut::<State>(),
            behaviour,
            trigger,
        ))
    }
}

/// The overlay drawing a panel for `trigger`, the anchor's bounds in
/// window coordinates.
pub(crate) fn floating<'a, 'b, Message: Clone>(
    content: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    state: &'b mut State,
    behaviour: &'b Behaviour<'a, Message>,
    trigger: Rectangle,
) -> overlay::Element<'b, Message, Theme, Renderer> {
    let target = match behaviour.point {
        Some(point) => Rectangle::new(
            trigger.position() + Vector::new(point.x, point.y),
            Size::ZERO,
        ),
        None => trigger,
    };
    overlay::Element::new(Box::new(Floating {
        content,
        tree,
        state,
        behaviour,
        target,
        trigger,
    }))
}

/// The overlay drawing the panel. Its node covers the whole window so the
/// panel's shadow is never clipped; the panel is its only child.
struct Floating<'a, 'b, Message> {
    content: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    state: &'b mut State,
    behaviour: &'b Behaviour<'a, Message>,
    target: Rectangle,
    trigger: Rectangle,
}

impl<Message: Clone> Floating<'_, '_, Message> {
    fn dismiss(&mut self, shell: &mut Shell<'_, Message>) {
        if self.behaviour.closes_itself {
            self.state.open = false;
            shell.request_redraw();
        }
        if self.behaviour.dismiss_on_blur {
            self.state.unfocus = true;
        }
        if let Some(message) = &self.behaviour.on_dismiss {
            shell.publish(message.clone());
        }
    }
}

/// The message [`Behaviour::on_key`] sends for a key press, if any.
fn key_message<Message>(behaviour: &Behaviour<'_, Message>, event: &Event) -> Option<Message> {
    let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
        return None;
    };
    let on_key = behaviour.on_key.as_ref()?;
    on_key(&keys::Event {
        key: key.clone(),
        modifiers: *modifiers,
    })
}

fn is_dismiss_key<Message>(behaviour: &Behaviour<'_, Message>, event: &Event) -> bool {
    let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
        return false;
    };
    behaviour
        .dismiss_keys
        .iter()
        .any(|chord| chord.matches(key, *modifiers))
}

fn is_press(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(_))
            | Event::Touch(touch::Event::FingerPressed { .. })
    )
}

fn is_release(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. })
    )
}

/// Where a mouse or touch press landed, if the event is one.
fn press_position(event: &Event, cursor: mouse::Cursor) -> Option<Point> {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => cursor.position(),
        Event::Touch(touch::Event::FingerPressed { position, .. }) => Some(*position),
        _ => None,
    }
}

fn is_pointer_input(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(
            mouse::Event::ButtonPressed(_)
                | mouse::Event::ButtonReleased(_)
                | mouse::Event::WheelScrolled { .. }
        ) | Event::Touch(_)
    )
}

impl<Message: Clone> overlay::Overlay<Message, Theme, Renderer> for Floating<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);
        let room = viewport.shrink(MARGIN).size();
        let room = Size::new(room.width.max(0.0), room.height.max(0.0));
        let limits = if self.behaviour.match_width {
            let width = self.trigger.width.min(room.width);
            layout::Limits::new(Size::new(width, 0.0), Size::new(width, room.height))
        } else {
            layout::Limits::new(Size::ZERO, room)
        };
        let node = self
            .content
            .as_widget_mut()
            .layout(self.tree, renderer, &limits);
        let panel = place(self.target, node.size(), viewport, self.behaviour.placement);
        layout::Node::with_children(bounds, vec![node.move_to(panel.position())])
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let Some(panel) = layout.children().next() else {
            return;
        };
        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            panel,
            cursor,
            &layout.bounds(),
        );
    }

    fn operate(&mut self, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        let Some(panel) = layout.children().next() else {
            return;
        };
        self.content
            .as_widget_mut()
            .operate(self.tree, panel, renderer, operation);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(panel) = layout.children().next() else {
            return;
        };

        self.content.as_widget_mut().update(
            self.tree,
            event,
            panel,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        );
        if shell.is_event_captured() {
            // A row took the click, so the choice is made.
            if self.behaviour.closes_itself && is_release(event) {
                self.state.open = false;
                shell.request_redraw();
            }
            return;
        }

        if let Some(message) = key_message(self.behaviour, event) {
            shell.publish(message);
            shell.capture_event();
            return;
        }
        if is_dismiss_key(self.behaviour, event) && self.behaviour.can_dismiss() {
            self.dismiss(shell);
            shell.capture_event();
            return;
        }
        if !is_pointer_input(event) {
            return;
        }

        let bounds = panel.bounds();
        if cursor.is_over(bounds) {
            shell.capture_event();
            return;
        }
        let Some(position) = press_position(event, cursor) else {
            return;
        };
        if bounds.contains(position) {
            shell.capture_event();
            return;
        }
        let on_anchor = self.trigger.contains(position);
        if (on_anchor && !self.behaviour.dismiss_on_anchor_press) || !self.behaviour.can_dismiss() {
            return;
        }
        self.dismiss(shell);
        if !on_anchor && !self.behaviour.pass_through {
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let Some(panel) = layout.children().next() else {
            return mouse::Interaction::None;
        };
        if !cursor.is_over(panel.bounds()) {
            return mouse::Interaction::None;
        }
        self.content
            .as_widget()
            .mouse_interaction(self.tree, panel, cursor, &layout.bounds(), renderer)
            .max(mouse::Interaction::Idle)
    }

    fn overlay<'c>(
        &'c mut self,
        layout: Layout<'c>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'c, Message, Theme, Renderer>> {
        let panel = layout.children().next()?;
        let viewport = layout.bounds();
        self.content
            .as_widget_mut()
            .overlay(self.tree, panel, renderer, &viewport, Vector::ZERO)
    }
}

/// Whether any focusable widget in a subtree has focus.
#[derive(Default)]
struct FindFocus(bool);

impl Operation for FindFocus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        self.0 |= state.is_focused();
    }
}

/// Takes focus away from every widget in a subtree.
struct Unfocus;

impl Operation for Unfocus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        if state.is_focused() {
            state.unfocus();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};
    use iced::keyboard::{Key, Modifiers};

    const VIEWPORT: Rectangle = Rectangle {
        x: 0.0,
        y: 0.0,
        width: 400.0,
        height: 300.0,
    };

    fn anchor() -> Rectangle {
        Rectangle::new(Point::new(150.0, 100.0), Size::new(100.0, 40.0))
    }

    const PANEL: Size = Size::new(60.0, 50.0);

    fn at(side: Side, align: Align) -> Point {
        place(anchor(), PANEL, VIEWPORT, Placement::new(side, align)).position()
    }

    #[test]
    fn defaults_to_bottom_centre_with_a_small_gap() {
        let placement = Placement::default();
        assert_eq!(placement.side, Side::Bottom);
        assert_eq!(placement.align, Align::Center);
        assert_eq!(placement.gap, GAP);
        assert_eq!(placement.offset, 0.0);
    }

    #[test]
    fn each_side_sits_outside_the_anchor_with_the_gap() {
        assert_eq!(at(Side::Bottom, Align::Start), Point::new(150.0, 144.0));
        assert_eq!(at(Side::Top, Align::Start), Point::new(150.0, 46.0));
        assert_eq!(at(Side::Left, Align::Start), Point::new(86.0, 100.0));
        assert_eq!(at(Side::Right, Align::Start), Point::new(254.0, 100.0));
    }

    #[test]
    fn alignment_moves_along_the_side() {
        assert_eq!(at(Side::Bottom, Align::Center).x, 170.0);
        assert_eq!(at(Side::Bottom, Align::End).x, 190.0);
        assert_eq!(at(Side::Right, Align::Center).y, 95.0);
        assert_eq!(at(Side::Right, Align::End).y, 90.0);
    }

    #[test]
    fn offset_shifts_away_from_the_aligned_edge() {
        let placement = Placement::new(Side::Bottom, Align::Start).offset(5.0);
        let start = place(anchor(), PANEL, VIEWPORT, placement);
        assert_eq!(start.x, 155.0);
        let placement = Placement::new(Side::Bottom, Align::End).offset(5.0);
        let end = place(anchor(), PANEL, VIEWPORT, placement);
        assert_eq!(end.x, 185.0);
    }

    #[test]
    fn flips_to_the_opposite_side_when_there_is_no_room() {
        let low = Rectangle::new(Point::new(150.0, 230.0), Size::new(100.0, 40.0));
        let panel = place(
            low,
            PANEL,
            VIEWPORT,
            Placement::new(Side::Bottom, Align::Start),
        );
        assert_eq!(panel.y, 230.0 - GAP - PANEL.height);

        let left_edge = Rectangle::new(Point::new(20.0, 100.0), Size::new(40.0, 40.0));
        let panel = place(
            left_edge,
            PANEL,
            VIEWPORT,
            Placement::new(Side::Left, Align::Start),
        );
        assert_eq!(panel.x, 60.0 + GAP);
    }

    #[test]
    fn stays_put_when_neither_side_fits_and_the_requested_one_is_larger() {
        let tall = Size::new(60.0, 290.0);
        let panel = place(
            anchor(),
            tall,
            VIEWPORT,
            Placement::new(Side::Bottom, Align::Start),
        );
        assert_eq!(panel.y, MARGIN, "shifted up to fit, pinned at the margin");
    }

    #[test]
    fn mirrors_the_alignment_when_it_would_overflow() {
        let right_edge = Rectangle::new(Point::new(360.0, 100.0), Size::new(30.0, 30.0));
        let panel = place(
            right_edge,
            PANEL,
            VIEWPORT,
            Placement::new(Side::Bottom, Align::Start),
        );
        assert_eq!(panel.x, 390.0 - PANEL.width);
    }

    #[test]
    fn shifts_inside_the_margin_when_nothing_else_fits() {
        let corner = Rectangle::new(Point::new(0.0, 0.0), Size::new(10.0, 10.0));
        let panel = place(
            corner,
            PANEL,
            VIEWPORT,
            Placement::new(Side::Left, Align::Center),
        );
        assert_eq!(panel.x, 10.0 + GAP, "flipped to the right");
        assert_eq!(panel.y, MARGIN, "shifted down to the margin");
    }

    #[test]
    fn a_list_near_the_bottom_keeps_the_margin() {
        let field = Rectangle::new(Point::new(20.0, 200.0), Size::new(200.0, 36.0));
        let list = Size::new(200.0, 80.0);
        let placement = Placement::new(Side::Bottom, Align::Start);
        let panel = place(field, list, VIEWPORT, placement);
        assert_eq!(panel.y, 200.0 - GAP - list.height, "flips above the field");

        let wide = Size::new(420.0, 40.0);
        let panel = place(field, wide, VIEWPORT, placement);
        assert_eq!(panel.x, MARGIN, "clamped inside the window");
    }

    #[test]
    fn a_point_anchor_opens_below_and_right_then_flips_near_edges() {
        let point = |x, y| Rectangle::new(Point::new(x, y), Size::ZERO);
        let placement = Placement::new(Side::Bottom, Align::Start).gap(0.0);
        assert_eq!(
            place(point(100.0, 100.0), PANEL, VIEWPORT, placement).position(),
            Point::new(100.0, 100.0)
        );
        assert_eq!(
            place(point(380.0, 280.0), PANEL, VIEWPORT, placement).position(),
            Point::new(320.0, 230.0)
        );
    }

    #[test]
    fn a_panel_larger_than_the_window_is_pinned_top_left() {
        let huge = Size::new(1000.0, 1000.0);
        let panel = place(anchor(), huge, VIEWPORT, Placement::default());
        assert_eq!(panel.position(), Point::new(MARGIN, MARGIN));
        assert_eq!(panel.size(), huge);
    }

    #[test]
    fn sides_and_alignments_mirror() {
        for side in Side::ALL {
            assert_eq!(side.opposite().opposite(), side);
            assert_ne!(side.opposite(), side);
        }
        assert_eq!(Align::Center.mirrored(), Align::Center);
        assert_eq!(Align::Start.mirrored(), Align::End);
    }

    #[test]
    fn surface_uses_the_popover_colour_a_border_and_a_shadow() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let style = surface_style(&tokens);
            assert_eq!(style.background, Some(Background::Color(tokens.popover)));
            assert_eq!(style.text_color, Some(tokens.foreground));
            assert_eq!(style.border.color, tokens.border);
            assert_eq!(style.border.width, 1.0);
            assert_eq!(style.border.radius, radius::MD.into());
            assert_eq!(style.shadow, shadow(&tokens));
            assert_eq!(style.shadow.offset, Vector::new(0.0, 4.0));
            assert_eq!(style.shadow.blur_radius, 12.0);
        }
        let light = shadow(&Tokens::of(&light())).color.a;
        let dark = shadow(&Tokens::of(&dark())).color.a;
        assert!((light - 0.1).abs() < 1e-6);
        assert!((dark - 0.5).abs() < 1e-6);
    }

    #[test]
    fn builder_starts_closed_and_undismissable_with_escape_bound() {
        let anchored: Anchored<'_, ()> = anchored(iced::widget::text("Anchor"));
        let behaviour = &anchored.behaviour;
        assert!(anchored.content.is_none());
        assert!(behaviour.on_dismiss.is_none());
        assert!(behaviour.point.is_none());
        assert!(!behaviour.dismiss_on_anchor_press);
        assert!(!behaviour.dismiss_on_blur);
        assert!(!behaviour.pass_through);
        assert!(!behaviour.match_width);
        assert_eq!(behaviour.dismiss_keys, vec![Chord::named(Named::Escape)]);
        assert!(!behaviour.can_dismiss());
        assert!(!behaviour.watches_focus());
    }

    #[test]
    fn builder_sets_every_option() {
        let anchored: Anchored<'_, u8> = anchored(iced::widget::text("Anchor"))
            .at(Point::new(3.0, 4.0))
            .match_width(true)
            .on_dismiss(1)
            .dismiss_keys([Chord::character('q')])
            .dismiss_on_anchor_press(true)
            .dismiss_on_blur(true)
            .pass_through(true)
            .on_anchor_press_maybe(Some(2))
            .on_key(|_| Some(3));
        let behaviour = &anchored.behaviour;
        assert_eq!(behaviour.point, Some(Point::new(3.0, 4.0)));
        assert!(behaviour.match_width && behaviour.pass_through);
        assert!(behaviour.dismiss_on_anchor_press && behaviour.dismiss_on_blur);
        assert_eq!(behaviour.on_dismiss, Some(1));
        assert_eq!(behaviour.on_anchor_press, Some(2));
        assert_eq!(behaviour.dismiss_keys, vec![Chord::character('q')]);
        assert!(behaviour.can_dismiss() && behaviour.watches_focus());
    }

    fn key_press(key: Key, modifiers: Modifiers) -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    }

    #[test]
    fn keys_resolve_through_on_key_then_the_dismiss_chords() {
        let behaviour: Behaviour<'_, u8> = Behaviour {
            on_key: Some(Box::new(|key: &keys::Event| {
                (key.key == Key::Named(Named::ArrowDown)).then_some(7)
            })),
            ..Behaviour::default()
        };
        let down = key_press(Key::Named(Named::ArrowDown), Modifiers::empty());
        let escape = key_press(Key::Named(Named::Escape), Modifiers::empty());
        let shift_escape = key_press(Key::Named(Named::Escape), Modifiers::SHIFT);
        assert_eq!(key_message(&behaviour, &down), Some(7));
        assert_eq!(key_message(&behaviour, &escape), None);
        assert!(is_dismiss_key(&behaviour, &escape));
        assert!(!is_dismiss_key(&behaviour, &shift_escape));
        assert!(!is_dismiss_key(&behaviour, &down));

        let unbound: Behaviour<'_, u8> = Behaviour {
            dismiss_keys: Vec::new(),
            ..Behaviour::default()
        };
        assert!(!is_dismiss_key(&unbound, &escape));
    }

    #[test]
    fn press_position_only_reports_presses() {
        let cursor = mouse::Cursor::Available(Point::new(3.0, 4.0));
        let pressed = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));
        let released = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        assert_eq!(press_position(&pressed, cursor), Some(Point::new(3.0, 4.0)));
        assert_eq!(press_position(&released, cursor), None);
        assert_eq!(press_position(&pressed, mouse::Cursor::Unavailable), None);
        assert!(is_pointer_input(&released));
        assert!(is_release(&released));
        assert!(is_press(&pressed));
        assert!(!is_release(&pressed));
        assert!(!is_pointer_input(&Event::Mouse(mouse::Event::CursorLeft)));
    }
}
