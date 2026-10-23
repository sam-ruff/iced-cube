//! A floating panel of any content, anchored to a trigger.
//!
//! The app owns whether the popover is open: pass it to
//! [`open`](Popover::open) and close it again when
//! [`on_dismiss`](Popover::on_dismiss) arrives, after Escape or a click
//! outside. The popover is not modal: the rest of the window stays usable
//! while it is open, and the click that dismisses it still reaches what it
//! landed on.
//!
//! Escape reaches the content first, so a menu, combobox or command list
//! inside the popover closes before the popover does.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::keyboard::key::Named;
use iced::widget::container;
use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, keyboard, mouse, widget,
};

use crate::keys::{self, Chord, Keymap};
use crate::overlay::anchored::{Placement, anchored, surface_style};
use crate::theme::{Tokens, space};

pub use crate::overlay::anchored::{Align, Side};

/// Default panel width in logical pixels.
pub const WIDTH: f32 = 288.0;

/// A popover builder. Convert it into an [`Element`] to render.
pub struct Popover<'a, Message> {
    trigger: Element<'a, Message>,
    content: Element<'a, Message>,
    open: bool,
    on_dismiss: Option<Message>,
    placement: Placement,
    width: Length,
    padding: f32,
    keymap: Keymap<Action>,
}

impl<Message> std::fmt::Debug for Popover<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Popover")
            .field("open", &self.open)
            .field("dismissable", &self.on_dismiss.is_some())
            .field("placement", &self.placement)
            .field("width", &self.width)
            .field("padding", &self.padding)
            .field("keymap", &self.keymap)
            .finish_non_exhaustive()
    }
}

/// Shows `content` in a panel next to `trigger` while
/// [`open`](Popover::open) is true. The trigger opens it itself, usually a
/// button whose message sets the app's flag.
pub fn popover<'a, Message>(
    trigger: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
) -> Popover<'a, Message> {
    Popover {
        trigger: trigger.into(),
        content: content.into(),
        open: false,
        on_dismiss: None,
        placement: Placement::default(),
        width: Length::Fixed(WIDTH),
        padding: space::LG,
        keymap: default_keymap(),
    }
}

impl<'a, Message> Popover<'a, Message> {
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Sent on Escape and on a press outside the panel and the trigger.
    /// Without it the popover only closes when the app says so.
    pub fn on_dismiss(mut self, message: Message) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Which side of the trigger the panel appears on. Defaults to the bottom.
    pub fn side(mut self, side: Side) -> Self {
        self.placement.side = side;
        self
    }

    /// How the panel lines up with the trigger. Defaults to the centre.
    pub fn align(mut self, align: Align) -> Self {
        self.placement.align = align;
        self
    }

    /// Space between the trigger and the panel.
    pub fn gap(mut self, gap: f32) -> Self {
        self.placement.gap = gap;
        self
    }

    /// Defaults to [`WIDTH`]. `Length::Shrink` fits the content.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Space inside the panel's border. Defaults to `space::LG`.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Replaces the [`default_keymap`]. The open popover dismisses itself
    /// on the chords bound to [`Action::Close`].
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Popover<'a, Message>> for Element<'a, Message> {
    fn from(popover: Popover<'a, Message>) -> Self {
        let Popover {
            trigger,
            content,
            open,
            on_dismiss,
            placement,
            width,
            padding,
            keymap,
        } = popover;

        let cycle = Cycle {
            content,
            keymap: keymap.clone(),
        };
        let panel = open.then(|| {
            container(Element::new(cycle))
                .width(width)
                .padding(padding)
                .style(|theme| style(&Tokens::of(theme)))
                .into()
        });
        anchored(trigger)
            .content(panel)
            .placement(placement)
            .pass_through(true)
            .dismiss_keys(keymap.chords(&Action::Close).into_iter().cloned())
            .on_dismiss_maybe(on_dismiss)
            .into()
    }
}

/// The panel style: the floating surface shared with menus.
pub fn style(tokens: &Tokens) -> container::Style {
    surface_style(tokens)
}

/// What a popover keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Opens a closed popover and closes an open one. Not bound by default.
    Toggle,
    Close,
    /// Moves focus from a field in the panel to the next one, wrapping.
    FocusNext,
    /// Moves focus from a field in the panel to the previous one, wrapping.
    FocusPrevious,
}

impl Action {
    /// Whether the popover should be open after this action, or `None`
    /// when it stays as it is.
    pub fn apply(self, open: bool) -> Option<bool> {
        match self {
            Action::Toggle => Some(!open),
            Action::Close => open.then_some(false),
            Action::FocusNext | Action::FocusPrevious => None,
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Toggle,
        Action::Close,
        Action::FocusNext,
        Action::FocusPrevious,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Toggle => "Toggle",
            Action::Close => "Close",
            Action::FocusNext => "FocusNext",
            Action::FocusPrevious => "FocusPrevious",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Toggle => "Opens the popover, or closes it when it is open.",
            Action::Close => "Closes the popover.",
            Action::FocusNext => {
                "Moves from a field in the popover to its next field, wrapping at the end."
            }
            Action::FocusPrevious => {
                "Moves from a field in the popover to its previous field, wrapping at the start."
            }
        }
    }
}

/// The default popover shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `Escape` | [`Action::Close`] |
/// | `Tab` | [`Action::FocusNext`] |
/// | `Shift+Tab` | [`Action::FocusPrevious`] |
///
/// The focus actions only apply while a field inside the panel has focus,
/// so Tab elsewhere moves on as usual.
///
/// An open popover with [`on_dismiss`](Popover::on_dismiss) resolves
/// [`Action::Close`] itself, after its content has had the key, so pass a
/// changed keymap with [`Popover::keymap`]. [`Action::Toggle`] has no
/// default chord because a popover usually belongs to one button; bind one
/// when a popover deserves an app-wide shortcut, and route it from
/// [`keys::subscription`] with [`Action::apply`].
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::Escape), Action::Close)
        .bind(Chord::named(Named::Tab), Action::FocusNext)
        .bind(Chord::named(Named::Tab).shift(), Action::FocusPrevious)
}

/// The field to focus after `forward` or backward movement from the
/// focused one, wrapping at either end, given whether each field in the
/// panel has focus. `None` when no field in the panel has focus.
fn cycle_target(focused: &[bool], forward: bool) -> Option<usize> {
    let current = focused.iter().position(|focused| *focused)?;
    let count = focused.len();
    Some(if forward {
        (current + 1) % count
    } else {
        (current + count - 1) % count
    })
}

/// The panel's content, with Tab and Shift+Tab moving between its fields
/// while one of them has focus.
struct Cycle<'a, Message> {
    content: Element<'a, Message>,
    keymap: Keymap<Action>,
}

/// Whether each focusable widget has focus, in tree order.
#[derive(Default)]
struct Survey {
    focused: Vec<bool>,
}

impl Operation for Survey {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        self.focused.push(state.is_focused());
    }
}

/// Focuses the focusable widget at `target` and unfocuses the others.
struct Apply {
    target: usize,
    index: usize,
}

impl Operation for Apply {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        if self.index == self.target {
            state.focus();
        } else {
            state.unfocus();
        }
        self.index += 1;
    }
}

impl<Message> Cycle<'_, Message> {
    /// The movement a key press asks for, if it is a focus chord.
    fn movement(&self, event: &Event) -> Option<bool> {
        let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
            return None;
        };
        match self.keymap.resolve(key, *modifiers)? {
            Action::FocusNext => Some(true),
            Action::FocusPrevious => Some(false),
            Action::Toggle | Action::Close => None,
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Cycle<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
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
        self.content.as_widget().draw(
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
        self.content
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
        let content = &mut tree.children[0];
        self.content.as_widget_mut().update(
            content, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
        if shell.is_event_captured() {
            return;
        }
        let Some(forward) = self.movement(event) else {
            return;
        };
        let mut survey = Survey::default();
        self.content
            .as_widget_mut()
            .operate(content, layout, renderer, &mut survey);
        let Some(target) = cycle_target(&survey.focused, forward) else {
            return;
        };
        self.content.as_widget_mut().operate(
            content,
            layout,
            renderer,
            &mut Apply { target, index: 0 },
        );
        shell.capture_event();
        shell.request_redraw();
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
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
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};
    use iced::widget::text;

    #[test]
    fn builder_defaults_to_closed_below_and_centred() {
        let popover: Popover<'_, ()> = popover(text("Open"), text("Content"));
        assert!(!popover.open);
        assert!(popover.on_dismiss.is_none());
        assert_eq!(popover.placement, Placement::default());
        assert_eq!(popover.placement.side, Side::Bottom);
        assert_eq!(popover.placement.align, Align::Center);
        assert_eq!(popover.width, Length::Fixed(WIDTH));
        assert_eq!(popover.padding, space::LG);
        assert_eq!(popover.keymap, default_keymap());
    }

    #[test]
    fn builder_sets_every_option() {
        let popover: Popover<'_, u8> = popover(text("Open"), text("Content"))
            .open(true)
            .on_dismiss(1)
            .side(Side::Right)
            .align(Align::End)
            .gap(10.0)
            .width(Length::Shrink)
            .padding(4.0)
            .keymap(Keymap::new());
        assert!(popover.keymap.is_empty());
        assert!(popover.open);
        assert_eq!(popover.on_dismiss, Some(1));
        assert_eq!(popover.placement.side, Side::Right);
        assert_eq!(popover.placement.align, Align::End);
        assert_eq!(popover.placement.gap, 10.0);
        assert_eq!(popover.width, Length::Shrink);
        assert_eq!(popover.padding, 4.0);
    }

    #[test]
    fn panel_uses_the_shared_surface_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(style(&tokens), surface_style(&tokens));
        }
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_closes_on_escape_and_moves_focus_on_tab() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "Escape"), Some(Action::Close));
        assert_eq!(press(&keymap, "Tab"), Some(Action::FocusNext));
        assert_eq!(press(&keymap, "Shift+Tab"), Some(Action::FocusPrevious));
        assert!(keymap.chords(&Action::Toggle).is_empty());
        assert_eq!(keymap.len(), 3);
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn focus_moves_between_fields_and_wraps() {
        assert_eq!(cycle_target(&[true, false, false], true), Some(1));
        assert_eq!(cycle_target(&[false, false, true], true), Some(0));
        assert_eq!(cycle_target(&[true, false, false], false), Some(2));
        assert_eq!(cycle_target(&[false, true], false), Some(0));
        assert_eq!(
            cycle_target(&[true], true),
            Some(0),
            "a lone field keeps focus"
        );
        assert_eq!(
            cycle_target(&[false, false], true),
            None,
            "no field in the panel has focus"
        );
        assert_eq!(cycle_target(&[], true), None);
    }

    #[test]
    fn focus_actions_leave_the_open_state_alone() {
        assert_eq!(Action::FocusNext.apply(true), None);
        assert_eq!(Action::FocusPrevious.apply(false), None);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .bind("I".parse().unwrap(), Action::Toggle)
            .unbind(&"Escape".parse().unwrap());
        assert_eq!(press(&keymap, "i"), Some(Action::Toggle));
        assert_eq!(press(&keymap, "Escape"), None);
    }

    #[test]
    fn actions_map_to_the_next_open_state() {
        assert_eq!(Action::Toggle.apply(false), Some(true));
        assert_eq!(Action::Toggle.apply(true), Some(false));
        assert_eq!(Action::Close.apply(true), Some(false));
        assert_eq!(Action::Close.apply(false), None);
    }
}
