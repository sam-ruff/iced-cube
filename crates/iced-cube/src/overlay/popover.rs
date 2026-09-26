//! A floating panel of any content, anchored to a trigger.
//!
//! The app owns whether the popover is open: pass it to
//! [`open`](Popover::open) and close it again when
//! [`on_dismiss`](Popover::on_dismiss) arrives, after Escape or a click
//! outside. The popover is not modal, so the rest of the window stays
//! usable while it is open.

use iced::keyboard::key::Named;
use iced::widget::container;
use iced::{Element, Length};

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
}

impl<Message> std::fmt::Debug for Popover<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Popover")
            .field("open", &self.open)
            .field("dismissable", &self.on_dismiss.is_some())
            .field("placement", &self.placement)
            .field("width", &self.width)
            .field("padding", &self.padding)
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
        } = popover;

        let panel = open.then(|| {
            container(content)
                .width(width)
                .padding(padding)
                .style(|theme| style(&Tokens::of(theme)))
                .into()
        });
        let anchored = anchored(trigger).content(panel).placement(placement);
        match on_dismiss {
            Some(message) => anchored.on_dismiss(message).into(),
            None => anchored.into(),
        }
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
}

impl Action {
    /// Whether the popover should be open after this action, or `None`
    /// when it stays as it is.
    pub fn apply(self, open: bool) -> Option<bool> {
        match self {
            Action::Toggle => Some(!open),
            Action::Close => open.then_some(false),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[Action::Toggle, Action::Close];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Toggle => "Toggle",
            Action::Close => "Close",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Toggle => "Opens the popover, or closes it when it is open.",
            Action::Close => "Closes the popover.",
        }
    }
}

/// The default popover shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `Escape` | [`Action::Close`] |
///
/// An open popover with [`on_dismiss`](Popover::on_dismiss) already takes
/// Escape itself. [`Action::Toggle`] has no default chord because a popover
/// usually belongs to one button; bind one when a popover deserves an
/// app-wide shortcut.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new().bind(Chord::named(Named::Escape), Action::Close)
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
            .padding(4.0);
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
    fn default_keymap_closes_on_escape_only() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "Escape"), Some(Action::Close));
        assert!(keymap.chords(&Action::Toggle).is_empty());
        assert_eq!(keymap.len(), 1);
        assert_eq!(Keymap::<Action>::defaults(), keymap);
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
