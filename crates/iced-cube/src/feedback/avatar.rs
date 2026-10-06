//! A small picture of a person or team, with initials when there is no
//! image.
//!
//! [`avatar`] takes a name and shows its [`initials`] on a muted
//! background. Add an [`image`](Avatar::image) to show a picture instead,
//! and a [`presence`](Avatar::presence) dot for online status.
//! [`avatar_group`] overlaps several avatars and counts the ones left over.
//!
//! Images are drawn from an [`image::Handle`]. Handles built from pixels
//! (`Handle::from_rgba`) work as they are; to load PNG or JPEG files, enable
//! iced's `image` feature in your app. iced's GPU renderer crops images to
//! the avatar's shape; its software fallback draws them with square corners.

use iced::widget::{Stack, container, image, space, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};

use crate::icon::{Glyph, themed};
use crate::lucide;
use crate::natural::natural;
use crate::theme::{Tokens, radius, semibold};

/// Avatar dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
    Xl,
}

impl Size {
    pub const ALL: [Size; 4] = [Size::Sm, Size::Md, Size::Lg, Size::Xl];

    /// Diameter, initials text size, icon size and presence dot diameter.
    pub const fn metrics(self) -> Metrics {
        match self {
            Size::Sm => Metrics {
                diameter: 24.0,
                text: 10.0,
                icon: 14.0,
                dot: 8.0,
            },
            Size::Md => Metrics {
                diameter: 32.0,
                text: 12.0,
                icon: 16.0,
                dot: 10.0,
            },
            Size::Lg => Metrics {
                diameter: 40.0,
                text: 14.0,
                icon: 20.0,
                dot: 12.0,
            },
            Size::Xl => Metrics {
                diameter: 56.0,
                text: 18.0,
                icon: 28.0,
                dot: 14.0,
            },
        }
    }
}

/// Resolved dimensions for a [`Size`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub diameter: f32,
    pub text: f32,
    pub icon: f32,
    pub dot: f32,
}

/// The outline of an avatar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Shape {
    /// For people.
    #[default]
    Circle,
    /// Rounded corners, for teams, organisations and bots.
    Square,
}

impl Shape {
    pub const ALL: [Shape; 2] = [Shape::Circle, Shape::Square];

    /// The corner radius at a diameter.
    pub fn radius(self, diameter: f32) -> f32 {
        match self {
            Shape::Circle => diameter / 2.0,
            Shape::Square => (diameter / 5.0).clamp(radius::SM, radius::LG),
        }
    }
}

/// Whether someone is available, drawn as a dot on the avatar's corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Presence {
    Online,
    Away,
    Busy,
    Offline,
}

impl Presence {
    pub const ALL: [Presence; 4] = [
        Presence::Online,
        Presence::Away,
        Presence::Busy,
        Presence::Offline,
    ];
}

/// Up to two initials for a name: the first letters of its first and last
/// words, in upper case. One word gives one letter, and an empty name none.
pub fn initials(name: &str) -> String {
    let mut words = name
        .split_whitespace()
        .filter_map(|word| word.chars().find(|c| c.is_alphanumeric()));
    let Some(first) = words.next() else {
        return String::new();
    };
    let mut initials: String = first.to_uppercase().collect();
    if let Some(last) = words.next_back() {
        initials.extend(last.to_uppercase());
    }
    initials
}

/// An avatar builder. Convert it into an [`Element`] to render.
#[derive(Debug, Clone)]
pub struct Avatar {
    initials: String,
    image: Option<image::Handle>,
    icon: Option<Glyph>,
    size: Size,
    shape: Shape,
    presence: Option<Presence>,
}

/// Creates an avatar showing the [`initials`] of `name`. A name with no
/// letters shows a person icon instead.
pub fn avatar(name: impl AsRef<str>) -> Avatar {
    Avatar {
        initials: initials(name.as_ref()),
        image: None,
        icon: None,
        size: Size::default(),
        shape: Shape::default(),
        presence: None,
    }
}

impl Avatar {
    /// Shows a picture instead of the initials, cropped to fill the shape.
    pub fn image(mut self, handle: impl Into<image::Handle>) -> Self {
        self.image = Some(handle.into());
        self
    }

    /// Shows an icon instead of the initials, such as a bot for an
    /// automated account. An image still takes precedence.
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn shape(mut self, shape: Shape) -> Self {
        self.shape = shape;
        self
    }

    pub fn presence(mut self, presence: Presence) -> Self {
        self.presence = Some(presence);
        self
    }

    pub fn initials(&self) -> &str {
        &self.initials
    }

    /// What the avatar shows.
    pub fn content(&self) -> Content<'_> {
        if let Some(handle) = &self.image {
            return Content::Image(handle);
        }
        if let Some(glyph) = self.icon {
            return Content::Icon(glyph);
        }
        if self.initials.is_empty() {
            return Content::Icon(lucide!(User));
        }
        Content::Initials(&self.initials)
    }
}

/// What an [`Avatar`] shows, in order of preference.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Content<'a> {
    Image(&'a image::Handle),
    Icon(Glyph),
    Initials(&'a str),
}

impl<'a, Message: 'a> From<Avatar> for Element<'a, Message> {
    fn from(avatar: Avatar) -> Self {
        render(avatar, false)
    }
}

/// Draws an avatar, with a ring in the background colour when it overlaps
/// others in a group.
fn render<'a, Message: 'a>(avatar: Avatar, ring: bool) -> Element<'a, Message> {
    let metrics = avatar.size.metrics();
    let shape = avatar.shape;
    let diameter = metrics.diameter;
    let corner = shape.radius(diameter);

    let face: Element<'a, Message> = match avatar.content() {
        Content::Image(handle) => image(handle.clone())
            .width(diameter)
            .height(diameter)
            .content_fit(iced::ContentFit::Cover)
            .border_radius(corner)
            .into(),
        Content::Icon(glyph) => themed(glyph, metrics.icon, 1.0, |theme| {
            Tokens::of(theme).muted_foreground
        })
        .into(),
        Content::Initials(initials) => text(initials.to_owned())
            .size(metrics.text)
            .font(semibold())
            .wrapping(text::Wrapping::None)
            .into(),
    };

    let ring_width = if ring { RING } else { 0.0 };
    let side = diameter + 2.0 * ring_width;
    let face = container(face)
        .center(Length::Fixed(side))
        .padding(ring_width)
        .style(move |theme: &Theme| style(&Tokens::of(theme), shape, diameter, ring));

    // The outer container shrinks to its fixed-size content, so `natural`
    // keeps the avatar whole in a narrower parent.
    let Some(presence) = avatar.presence else {
        return natural(container(face));
    };
    natural(container(
        Stack::with_children([
            face.into(),
            container(dot(presence, metrics.dot))
                .align_right(Length::Fixed(side))
                .align_bottom(Length::Fixed(side))
                .into(),
        ])
        .width(side)
        .height(side),
    ))
}

fn dot<'a, Message: 'a>(presence: Presence, diameter: f32) -> Element<'a, Message> {
    container(space())
        .width(diameter)
        .height(diameter)
        .style(move |theme: &Theme| dot_style(&Tokens::of(theme), presence))
        .into()
}

/// Width of the ring around an avatar in a group, in the background colour,
/// so overlapping avatars stay apart. The ring and the presence dot's ring
/// are drawn in the page background, so they suit avatars on the page
/// rather than on a raised card.
pub const RING: f32 = 2.0;

/// The container style of an avatar: muted behind initials and icons, with
/// a ring in the background colour when `ring` is set.
pub fn style(tokens: &Tokens, shape: Shape, diameter: f32, ring: bool) -> container::Style {
    let ring_width = if ring { RING } else { 0.0 };
    container::Style {
        text_color: Some(tokens.muted_foreground),
        background: Some(Background::Color(tokens.muted)),
        border: Border {
            color: if ring {
                tokens.background
            } else {
                Color::TRANSPARENT
            },
            width: ring_width,
            radius: shape.radius(diameter + 2.0 * ring_width).into(),
        },
        ..container::Style::default()
    }
}

/// The colour of a presence dot.
pub fn presence_colour(tokens: &Tokens, presence: Presence) -> Color {
    match presence {
        Presence::Online => tokens.success,
        Presence::Away => tokens.warning,
        Presence::Busy => tokens.destructive,
        Presence::Offline => tokens.muted_foreground,
    }
}

/// The presence dot, ringed in the background colour so it stands off the
/// picture behind it.
pub fn dot_style(tokens: &Tokens, presence: Presence) -> container::Style {
    container::Style {
        background: Some(Background::Color(presence_colour(tokens, presence))),
        border: Border {
            color: tokens.background,
            width: RING,
            radius: radius::FULL.into(),
        },
        ..container::Style::default()
    }
}

/// A row of overlapping avatars. Convert it into an [`Element`] to render.
#[derive(Debug, Clone)]
pub struct AvatarGroup {
    avatars: Vec<Avatar>,
    size: Size,
    max: Option<usize>,
}

/// Overlaps `avatars` from left to right, each one ringed in the
/// background colour.
pub fn avatar_group(avatars: impl IntoIterator<Item = Avatar>) -> AvatarGroup {
    AvatarGroup {
        avatars: avatars.into_iter().collect(),
        size: Size::default(),
        max: None,
    }
}

impl AvatarGroup {
    /// The size of every avatar in the group, replacing their own.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Shows at most `max` avatars. When there are more, the last place
    /// shows a count of the rest, such as "+3", so the group never takes
    /// more than `max` places.
    pub fn max_items(mut self, max: usize) -> Self {
        self.max = Some(max);
        self
    }

    /// How many avatars are shown, and how many the count stands in for.
    pub fn visible(&self) -> (usize, usize) {
        overflow(self.avatars.len(), self.max)
    }
}

/// Splits `len` avatars into the number shown and the number counted, for
/// at most `max` places. The count takes one of the places, and at least
/// one avatar is always shown.
pub fn overflow(len: usize, max: Option<usize>) -> (usize, usize) {
    let Some(max) = max else {
        return (len, 0);
    };
    let max = max.max(2);
    if len <= max {
        return (len, 0);
    }
    (max - 1, len - (max - 1))
}

/// How far each avatar in a group sits from the one before it: a fifth of
/// the face is covered, plus the ring, so initials stay readable.
pub fn step(size: Size) -> f32 {
    (size.metrics().diameter * 0.8 + RING).round()
}

impl<'a, Message: 'a> From<AvatarGroup> for Element<'a, Message> {
    fn from(group: AvatarGroup) -> Self {
        let (shown, rest) = group.visible();
        let size = group.size;
        let metrics = size.metrics();
        let side = metrics.diameter + 2.0 * RING;
        let step = step(size);

        let mut faces: Vec<Element<'a, Message>> = group
            .avatars
            .into_iter()
            .take(shown)
            .map(|avatar| render(avatar.size(size), true))
            .collect();
        if rest > 0 {
            faces.push(count(rest, size));
        }

        let places = faces.len();
        let width = if places == 0 {
            0.0
        } else {
            side + step * (places - 1) as f32
        };
        let layers = faces.into_iter().enumerate().map(|(index, face)| {
            container(face)
                .padding(Padding::ZERO.left(step * index as f32))
                .into()
        });
        natural(container(
            Stack::with_children(layers).width(width).height(side),
        ))
    }
}

/// The "+N" place at the end of a group.
fn count<'a, Message: 'a>(rest: usize, size: Size) -> Element<'a, Message> {
    let metrics = size.metrics();
    let side = metrics.diameter + 2.0 * RING;
    container(
        text(format!("+{rest}"))
            .size(metrics.text)
            .font(semibold())
            .wrapping(text::Wrapping::None)
            .align_x(Alignment::Center),
    )
    .center(Length::Fixed(side))
    .padding(RING)
    .style(move |theme: &Theme| style(&Tokens::of(theme), Shape::Circle, metrics.diameter, true))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn initials_take_the_first_and_last_words() {
        assert_eq!(initials("Ada Lovelace"), "AL");
        assert_eq!(initials("grace brewster murray hopper"), "GH");
        assert_eq!(initials("  Linus  "), "L");
        assert_eq!(initials("émile zola"), "ÉZ");
        assert_eq!(initials("(Ops) Team"), "OT");
        assert_eq!(initials(""), "");
        assert_eq!(initials("   "), "");
        assert_eq!(initials("- -"), "");
    }

    #[test]
    fn builder_defaults() {
        let a = avatar("Ada Lovelace");
        assert_eq!(a.initials(), "AL");
        assert_eq!(a.size, Size::Md);
        assert_eq!(a.shape, Shape::Circle);
        assert_eq!(a.presence, None);
        assert_eq!(a.content(), Content::Initials("AL"));
    }

    #[test]
    fn content_prefers_image_then_icon_then_initials() {
        let handle = image::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]);
        let bot = lucide!(Bot);
        let a = avatar("Build bot").icon(bot);
        assert_eq!(a.content(), Content::Icon(bot));
        let a = a.image(handle.clone());
        assert_eq!(a.content(), Content::Image(&handle));
        assert_eq!(avatar("").content(), Content::Icon(lucide!(User)));
    }

    #[test]
    fn builder_sets_size_shape_and_presence() {
        let a = avatar("Ops")
            .size(Size::Xl)
            .shape(Shape::Square)
            .presence(Presence::Busy);
        assert_eq!(a.size, Size::Xl);
        assert_eq!(a.shape, Shape::Square);
        assert_eq!(a.presence, Some(Presence::Busy));
    }

    #[test]
    fn sizes_grow_monotonically() {
        let metrics = Size::ALL.map(Size::metrics);
        for pair in metrics.windows(2) {
            assert!(pair[0].diameter < pair[1].diameter);
            assert!(pair[0].text < pair[1].text);
            assert!(pair[0].dot < pair[1].dot);
        }
    }

    #[test]
    fn circles_are_round_and_squares_have_modest_corners() {
        for size in Size::ALL {
            let diameter = size.metrics().diameter;
            assert_eq!(Shape::Circle.radius(diameter), diameter / 2.0);
            let square = Shape::Square.radius(diameter);
            assert!((radius::SM..=radius::LG).contains(&square), "{size:?}");
        }
    }

    #[test]
    fn avatars_sit_on_the_muted_surface_and_ring_only_in_groups() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for shape in Shape::ALL {
                let alone = style(&tokens, shape, 32.0, false);
                let ringed = style(&tokens, shape, 32.0, true);
                assert_eq!(alone.background, Some(Background::Color(tokens.muted)));
                assert_eq!(alone.text_color, Some(tokens.muted_foreground));
                assert_eq!(alone.border.width, 0.0);
                assert_eq!(ringed.border.width, RING);
                assert_eq!(ringed.border.color, tokens.background);
            }
        }
    }

    #[test]
    fn presence_uses_status_tokens() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(presence_colour(&tokens, Presence::Online), tokens.success);
            assert_eq!(presence_colour(&tokens, Presence::Away), tokens.warning);
            assert_eq!(presence_colour(&tokens, Presence::Busy), tokens.destructive);
            assert_eq!(
                presence_colour(&tokens, Presence::Offline),
                tokens.muted_foreground
            );
            for presence in Presence::ALL {
                let style = dot_style(&tokens, presence);
                assert_eq!(style.border.color, tokens.background);
                assert_eq!(style.border.radius, radius::FULL.into());
            }
        }
    }

    #[test]
    fn overflow_keeps_the_group_within_its_places() {
        assert_eq!(overflow(3, None), (3, 0));
        assert_eq!(overflow(3, Some(3)), (3, 0));
        assert_eq!(overflow(5, Some(3)), (2, 3));
        assert_eq!(overflow(5, Some(0)), (1, 4));
        assert_eq!(overflow(0, Some(3)), (0, 0));
        for len in 0..10 {
            for max in 2..6 {
                let (shown, rest) = overflow(len, Some(max));
                assert_eq!(shown + rest, len);
                assert!(shown + usize::from(rest > 0) <= max);
            }
        }
    }

    #[test]
    fn group_avatars_overlap_by_a_fifth_of_their_face() {
        for size in Size::ALL {
            let diameter = size.metrics().diameter;
            let step = step(size);
            assert!(step < diameter + 2.0 * RING, "{size:?} overlaps");
            assert!(step >= diameter * 0.8, "{size:?} keeps initials clear");
        }
    }

    #[test]
    fn group_builder() {
        let group = avatar_group(["A", "B", "C", "D"].map(avatar))
            .size(Size::Sm)
            .max_items(3);
        assert_eq!(group.size, Size::Sm);
        assert_eq!(group.visible(), (2, 2));
        assert_eq!(avatar_group([avatar("A")]).visible(), (1, 0));
    }
}
