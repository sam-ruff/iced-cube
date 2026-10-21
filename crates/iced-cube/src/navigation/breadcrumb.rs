//! Shows where the current view sits in a hierarchy, with a link to each
//! level above it.
//!
//! The last crumb is the current view and never responds to clicks. Crumbs
//! before it are links when they have a message. A long trail collapses its
//! middle into an ellipsis with [`max_items`](Breadcrumb::max_items); see
//! [`slots`].
//!
//! Keyboard shortcuts resolve through a [`Keymap`] of [`Action`]s; see
//! [`default_keymap`] and [`crate::keys`].

use iced::keyboard::key::Named;
use iced::widget::{self, button::Status, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Shadow, Theme};

use crate::icon::{Glyph, themed};
use crate::keys::{self, Chord, Keymap};
use crate::lucide;
use crate::theme::{Tokens, mix, radius, text_size};

const ICON_SIZE: f32 = 14.0;

/// What goes between two crumbs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Separator {
    #[default]
    Chevron,
    Slash,
}

impl Separator {
    pub const ALL: [Separator; 2] = [Separator::Chevron, Separator::Slash];
}

/// One place in a rendered trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    /// The crumb at this index.
    Crumb(usize),
    /// Crumbs left out of the middle.
    Ellipsis,
}

/// The crumbs to show for a trail of `len`, keeping at most `max` of them.
///
/// The first crumb and the last `max - 1` stay, with an ellipsis for the
/// rest, so the root and the levels nearest the current view remain one
/// click away. `max` is at least 2. `None` shows every crumb.
pub fn slots(len: usize, max: Option<usize>) -> Vec<Slot> {
    let max = max.map_or(len, |max| max.max(2));
    if len <= max {
        return (0..len).map(Slot::Crumb).collect();
    }
    let tail = max - 1;
    let mut slots = vec![Slot::Crumb(0), Slot::Ellipsis];
    slots.extend((len - tail..len).map(Slot::Crumb));
    slots
}

/// One level in a [`Breadcrumb`].
#[derive(Debug, Clone)]
pub struct Crumb<'a, Message> {
    label: text::Fragment<'a>,
    icon: Option<Glyph>,
    on_press: Option<Message>,
}

/// Creates a crumb with a text label. Add a message with
/// [`on_press`](Crumb::on_press) to make it a link.
pub fn crumb<'a, Message>(label: impl text::IntoFragment<'a>) -> Crumb<'a, Message> {
    Crumb {
        label: label.into_fragment(),
        icon: None,
        on_press: None,
    }
}

impl<Message> Crumb<'_, Message> {
    /// Adds an icon before the label, such as a house for the root.
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    pub fn is_link(&self) -> bool {
        self.on_press.is_some()
    }
}

/// A breadcrumb trail builder. Convert it into an [`Element`] to render.
#[derive(Debug)]
pub struct Breadcrumb<'a, Message> {
    crumbs: Vec<Crumb<'a, Message>>,
    separator: Separator,
    max_items: Option<usize>,
    on_expand: Option<Message>,
}

/// Creates a trail from the root to the current view, in that order.
pub fn breadcrumb<'a, Message>(
    crumbs: impl IntoIterator<Item = Crumb<'a, Message>>,
) -> Breadcrumb<'a, Message> {
    Breadcrumb {
        crumbs: crumbs.into_iter().collect(),
        separator: Separator::default(),
        max_items: None,
        on_expand: None,
    }
}

impl<Message> Breadcrumb<'_, Message> {
    pub fn separator(mut self, separator: Separator) -> Self {
        self.separator = separator;
        self
    }

    /// Shows at most `max` crumbs, collapsing the middle into an ellipsis.
    pub fn max_items(mut self, max: usize) -> Self {
        self.max_items = Some(max);
        self
    }

    /// Makes the ellipsis a button that sends `message`, such as one that
    /// lifts the limit to show the whole trail.
    pub fn on_expand(mut self, message: Message) -> Self {
        self.on_expand = Some(message);
        self
    }

    pub fn len(&self) -> usize {
        self.crumbs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.crumbs.is_empty()
    }
}

impl<'a, Message: Clone + 'a> From<Breadcrumb<'a, Message>> for Element<'a, Message> {
    fn from(breadcrumb: Breadcrumb<'a, Message>) -> Self {
        let Breadcrumb {
            crumbs,
            separator,
            max_items,
            on_expand,
        } = breadcrumb;
        let len = crumbs.len();
        let slots = slots(len, max_items);
        let mut crumbs: Vec<_> = crumbs.into_iter().map(Some).collect();

        let mut trail = row![].spacing(6).align_y(Alignment::Center);
        let mut first = true;
        for slot in slots {
            let element = match slot {
                Slot::Crumb(at) => {
                    let Some(crumb) = crumbs.get_mut(at).and_then(Option::take) else {
                        continue;
                    };
                    item(crumb, at + 1 == len)
                }
                Slot::Ellipsis => ellipsis(on_expand.clone()),
            };
            // A divider travels with the crumb after it, so a wrapped trail
            // never ends a line on a separator.
            if first {
                trail = trail.push(element);
            } else {
                trail = trail.push(
                    row![divider(separator), element]
                        .spacing(6)
                        .align_y(Alignment::Center),
                );
            }
            first = false;
        }
        trail.wrap().into()
    }
}

fn item<'a, Message: Clone + 'a>(crumb: Crumb<'a, Message>, current: bool) -> Element<'a, Message> {
    let Crumb {
        label,
        icon,
        on_press,
    } = crumb;
    let message = on_press.filter(|_| !current);

    let link = message.is_some();
    let content = |status: Status| {
        let mut content = row![].spacing(6).align_y(Alignment::Center);
        if let Some(glyph) = icon {
            content = content.push(themed(glyph, ICON_SIZE, 1.0, move |theme| {
                foreground(&Tokens::of(theme), current, link, status)
            }));
        }
        content.push(
            text(label.clone())
                .size(text_size::SM)
                .wrapping(text::Wrapping::None),
        )
    };

    let Some(message) = message else {
        return container(content(Status::Active))
            .padding(Padding::from([2.0, 0.0]))
            .style(move |theme: &Theme| container::Style {
                text_color: Some(foreground(
                    &Tokens::of(theme),
                    current,
                    false,
                    Status::Active,
                )),
                ..container::Style::default()
            })
            .into();
    };
    // The icon is drawn twice so it follows the label's hover colour: iced
    // tints an svg by its own hover state, not its button's.
    widget::button(widget::hover(
        content(Status::Active),
        content(Status::Hovered),
    ))
    .padding(Padding::from([2.0, 0.0]))
    .on_press(message)
    .style(|theme, status| style(&Tokens::of(theme), status))
    .into()
}

fn divider<'a, Message: 'a>(separator: Separator) -> Element<'a, Message> {
    let colour = |theme: &Theme| Tokens::of(theme).muted_foreground;
    match separator {
        Separator::Chevron => themed(lucide!(ChevronRight), ICON_SIZE, 1.0, colour).into(),
        Separator::Slash => text("/")
            .size(text_size::SM)
            .style(move |theme: &Theme| text::Style {
                color: Some(colour(theme)),
            })
            .into(),
    }
}

fn ellipsis<'a, Message: Clone + 'a>(on_expand: Option<Message>) -> Element<'a, Message> {
    let glyph = |status| {
        themed(lucide!(Ellipsis), 16.0, 1.0, move |theme| {
            foreground(&Tokens::of(theme), false, true, status)
        })
    };
    let Some(message) = on_expand else {
        return container(glyph(Status::Active))
            .center_y(Length::Fixed(24.0))
            .into();
    };
    widget::button(glyph(Status::Active))
        .padding(Padding::from([4.0, 2.0]))
        .on_press(message)
        .style(|theme, status| ellipsis_style(&Tokens::of(theme), status))
        .into()
}

/// The text colour of a crumb: the current one in the foreground colour,
/// the levels above it muted until a link is hovered.
pub fn foreground(tokens: &Tokens, current: bool, link: bool, status: Status) -> Color {
    let hover = matches!(status, Status::Hovered | Status::Pressed);
    if current || (link && hover) {
        tokens.foreground
    } else {
        tokens.muted_foreground
    }
}

/// The style of a crumb that is a link.
pub fn style(tokens: &Tokens, status: Status) -> widget::button::Style {
    widget::button::Style {
        background: None,
        text_color: foreground(tokens, false, true, status),
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

/// The style of the ellipsis when it expands the trail: a small ghost
/// button, so it reads as clickable.
pub fn ellipsis_style(tokens: &Tokens, status: Status) -> widget::button::Style {
    let hover = matches!(status, Status::Hovered | Status::Pressed);
    widget::button::Style {
        background: hover.then(|| Background::Color(mix(tokens.background, tokens.muted, 0.9))),
        text_color: foreground(tokens, false, true, status),
        border: Border {
            radius: radius::SM.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// What a breadcrumb keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Parent,
    Root,
}

impl Action {
    /// The index of the crumb to go to in a trail of `len`, where the last
    /// crumb is the current view, or `None` when already at the top.
    pub fn target(self, len: usize) -> Option<usize> {
        match self {
            Action::Parent => len.checked_sub(2),
            Action::Root => (len > 1).then_some(0),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[Action::Parent, Action::Root];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Parent => "Parent",
            Action::Root => "Root",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Parent => "Goes up one level, to the crumb before the current one.",
            Action::Root => "Goes to the first crumb.",
        }
    }
}

/// The default breadcrumb shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `Alt+ArrowUp` | [`Action::Parent`] |
/// | `Alt+Shift+ArrowUp` | [`Action::Root`] |
///
/// `Alt+ArrowUp` goes to the parent folder in desktop file managers.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowUp).alt(), Action::Parent)
        .bind(Chord::named(Named::ArrowUp).alt().shift(), Action::Root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn every_crumb_shows_without_a_limit() {
        assert_eq!(slots(0, None), vec![]);
        assert_eq!(slots(3, None), (0..3).map(Slot::Crumb).collect::<Vec<_>>());
        assert_eq!(slots(3, Some(3)), slots(3, None));
        assert_eq!(slots(3, Some(10)), slots(3, None));
    }

    #[test]
    fn a_long_trail_keeps_the_root_and_the_nearest_levels() {
        assert_eq!(
            slots(6, Some(3)),
            vec![
                Slot::Crumb(0),
                Slot::Ellipsis,
                Slot::Crumb(4),
                Slot::Crumb(5)
            ]
        );
        assert_eq!(
            slots(6, Some(4)),
            vec![
                Slot::Crumb(0),
                Slot::Ellipsis,
                Slot::Crumb(3),
                Slot::Crumb(4),
                Slot::Crumb(5)
            ]
        );
    }

    #[test]
    fn the_limit_never_drops_below_root_and_current() {
        let expected = vec![Slot::Crumb(0), Slot::Ellipsis, Slot::Crumb(4)];
        assert_eq!(slots(5, Some(0)), expected);
        assert_eq!(slots(5, Some(1)), expected);
        assert_eq!(slots(5, Some(2)), expected);
    }

    #[test]
    fn builder_defaults_and_options() {
        let trail: Breadcrumb<'_, u8> = breadcrumb([crumb("Home").on_press(0), crumb("Docs")]);
        assert_eq!(trail.separator, Separator::Chevron);
        assert_eq!(trail.max_items, None);
        assert!(trail.on_expand.is_none());
        assert_eq!(trail.len(), 2);
        assert!(!trail.is_empty());

        let trail = trail.separator(Separator::Slash).max_items(3).on_expand(9);
        assert_eq!(trail.separator, Separator::Slash);
        assert_eq!(trail.max_items, Some(3));
        assert_eq!(trail.on_expand, Some(9));
        assert!(breadcrumb::<u8>([]).is_empty());
    }

    #[test]
    fn crumbs_are_links_only_with_a_message() {
        let home = crumb::<u8>("Home").icon(lucide!(House));
        assert!(!home.is_link());
        assert_eq!(home.icon.map(Glyph::name), Some("house"));
        assert!(home.clone().on_press(1).is_link());
        assert!(!home.on_press(1).on_press_maybe(None).is_link());
    }

    #[test]
    fn current_crumb_is_foreground_and_links_are_muted_until_hovered() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in [Status::Active, Status::Hovered] {
                assert_eq!(foreground(&tokens, true, false, status), tokens.foreground);
                assert_eq!(
                    foreground(&tokens, false, false, status),
                    tokens.muted_foreground
                );
            }
            assert_eq!(
                foreground(&tokens, false, true, Status::Active),
                tokens.muted_foreground
            );
            assert_eq!(
                foreground(&tokens, false, true, Status::Hovered),
                tokens.foreground
            );
            assert_eq!(
                foreground(&tokens, false, true, Status::Pressed),
                tokens.foreground
            );
        }
    }

    #[test]
    fn links_have_no_background_or_border() {
        let tokens = Tokens::of(&light());
        for status in [Status::Active, Status::Hovered, Status::Pressed] {
            let style = style(&tokens, status);
            assert!(style.background.is_none());
            assert_eq!(style.border.width, 0.0);
        }
    }

    #[test]
    fn the_ellipsis_fills_only_on_hover() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert!(ellipsis_style(&tokens, Status::Active).background.is_none());
            assert!(
                ellipsis_style(&tokens, Status::Hovered)
                    .background
                    .is_some()
            );
        }
    }

    #[test]
    fn actions_target_the_parent_and_the_root() {
        assert_eq!(Action::Parent.target(4), Some(2));
        assert_eq!(Action::Root.target(4), Some(0));
        assert_eq!(Action::Parent.target(2), Some(0));
        assert_eq!(Action::Parent.target(1), None);
        assert_eq!(Action::Root.target(1), None);
        assert_eq!(Action::Parent.target(0), None);
        assert_eq!(Action::Root.target(0), None);
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "Alt+ArrowUp"), Some(Action::Parent));
        assert_eq!(press(&keymap, "Alt+Shift+ArrowUp"), Some(Action::Root));
        assert_eq!(press(&keymap, "ArrowUp"), None);
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?}");
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&Chord::named(Named::ArrowUp).alt())
            .bind("Backspace".parse().unwrap(), Action::Parent);
        assert_eq!(press(&keymap, "Alt+ArrowUp"), None);
        assert_eq!(press(&keymap, "Backspace"), Some(Action::Parent));
        assert_eq!(press(&keymap, "Alt+Shift+ArrowUp"), Some(Action::Root));
    }
}
