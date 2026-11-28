//! Two images of the same scene compared in one view, such as a screen
//! before and after a change: side by side, one blended over the other, or
//! split by a divider that is dragged across. Outlines can mark regions of
//! either image, given in the image's own pixels.
//!
//! The app owns the mode, the divider position and the overlay opacity.
//! Pointer positions come back in image pixels too, so an app can hit test
//! what it drew.
//!
//! Images are drawn from an [`image::Handle`]. Handles built from pixels
//! with [`image::Handle::from_rgba`] work as they are; to show PNG or JPEG
//! files, enable iced's `image` feature in your app.

use iced::advanced::image::{self as core_image, Renderer as _};
use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Renderer as _, Shell, renderer};
use iced::keyboard::{self, key::Named};
use iced::widget::image;
use iced::{
    Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, mouse, touch,
};

use crate::focus;
use crate::keys::{self, Chord, Keymap};
use crate::theme::{Tokens, fade, mix, space};

/// How close to the divider a mouse press grabs it, in logical pixels.
pub const GRAB: f32 = 8.0;
/// How close to the divider a finger grabs it.
pub const TOUCH_GRAB: f32 = 20.0;
/// Radius of the divider's handle.
pub const HANDLE_RADIUS: f32 = 8.0;
/// How far a step action moves the divider, as a fraction of the width.
pub const STEP: f32 = 0.05;
/// The shape used while no image can be measured yet.
const FALLBACK: Size = Size::new(16.0, 9.0);

/// How the two images are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    /// The before image left of a divider, the after image right of it.
    #[default]
    Slider,
    /// The before image on the left, the after image on the right.
    SideBySide,
    /// The before image blended over the after image.
    Overlay,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Slider, Mode::SideBySide, Mode::Overlay];

    /// A short label, such as `"Side by side"`.
    pub fn label(self) -> &'static str {
        match self {
            Mode::Slider => "Slider",
            Mode::SideBySide => "Side by side",
            Mode::Overlay => "Overlay",
        }
    }
}

/// The colour of an outline.
pub use crate::theme::Tone;

/// A rectangle drawn over the images, in image pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outline {
    pub rect: Rectangle,
    pub tone: Tone,
}

impl Outline {
    pub fn new(rect: Rectangle) -> Self {
        Self {
            rect,
            tone: Tone::default(),
        }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

/// An image compare builder. Convert it into an [`Element`] to render.
///
/// The divider can only be dragged when the compare has an
/// [`on_divider`](ImageCompare::on_divider) message.
pub struct ImageCompare<'a, Message> {
    before: Option<image::Handle>,
    after: image::Handle,
    mode: Mode,
    divider: f32,
    opacity: f32,
    outlines: Vec<Outline>,
    width: Length,
    height: Length,
    filter: image::FilterMethod,
    label: iced::widget::text::Fragment<'a>,
    keymap: Keymap<Action>,
    on_divider: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    on_hover: Option<Box<dyn Fn(Option<Point>) -> Message + 'a>>,
    on_press: Option<Box<dyn Fn(Point) -> Message + 'a>>,
}

impl<Message> std::fmt::Debug for ImageCompare<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageCompare")
            .field("before", &self.before)
            .field("after", &self.after)
            .field("mode", &self.mode)
            .field("divider", &self.divider)
            .field("opacity", &self.opacity)
            .field("outlines", &self.outlines)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("label", &self.label)
            .field("keymap", &self.keymap)
            .field("draggable", &self.on_divider.is_some())
            .finish_non_exhaustive()
    }
}

/// Compares `before` with `after`, in [`Mode::Slider`] with the divider in
/// the middle.
pub fn image_compare<'a, Message>(
    before: impl Into<image::Handle>,
    after: impl Into<image::Handle>,
) -> ImageCompare<'a, Message> {
    ImageCompare {
        before: Some(before.into()),
        ..single(after)
    }
}

/// Shows one image, with outlines and pointer reports but no divider.
pub fn single<'a, Message>(image: impl Into<image::Handle>) -> ImageCompare<'a, Message> {
    ImageCompare {
        before: None,
        after: image.into(),
        mode: Mode::default(),
        divider: 0.5,
        opacity: 0.5,
        outlines: Vec::new(),
        width: Length::Fill,
        height: Length::Shrink,
        filter: image::FilterMethod::Linear,
        label: "Image compare".into(),
        keymap: default_keymap(),
        on_divider: None,
        on_hover: None,
        on_press: None,
    }
}

impl<'a, Message> ImageCompare<'a, Message> {
    pub fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Where the divider sits in [`Mode::Slider`], from 0 (all after) to 1
    /// (all before).
    pub fn divider(mut self, divider: f32) -> Self {
        self.divider = clamp_unit(divider);
        self
    }

    /// How opaque the before image is in [`Mode::Overlay`], from 0 to 1.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = clamp_unit(opacity);
        self
    }

    /// Adds an outline over the images.
    pub fn outline(mut self, outline: Outline) -> Self {
        self.outlines.push(outline);
        self
    }

    /// Adds several outlines over the images.
    pub fn outlines(mut self, outlines: impl IntoIterator<Item = Outline>) -> Self {
        self.outlines.extend(outlines);
        self
    }

    /// Fills the width by default.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Follows the images' shape by default. A fixed height letterboxes
    /// them, centred.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Draws pixels as hard squares rather than smoothing them, for
    /// screenshots compared at a large zoom.
    pub fn pixelated(mut self, pixelated: bool) -> Self {
        self.filter = if pixelated {
            image::FilterMethod::Nearest
        } else {
            image::FilterMethod::Linear
        };
        self
    }

    /// The name tests and assistive tools know it by. "Image compare" by
    /// default.
    pub fn label(mut self, label: impl iced::widget::text::IntoFragment<'a>) -> Self {
        self.label = label.into_fragment();
        self
    }

    /// Sent with the divider's new position while it is dragged, or where
    /// a shortcut moves it.
    pub fn on_divider(mut self, on_divider: impl Fn(f32) -> Message + 'a) -> Self {
        self.on_divider = Some(Box::new(on_divider));
        self
    }

    /// Replaces the [`default_keymap`] the compare resolves keys with while
    /// it has keyboard focus or was the last thing pressed.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    /// Sent with the image pixel under the pointer whenever it changes, and
    /// `None` when the pointer leaves the images.
    pub fn on_hover(mut self, on_hover: impl Fn(Option<Point>) -> Message + 'a) -> Self {
        self.on_hover = Some(Box::new(on_hover));
        self
    }

    /// Sent with the image pixel under a press away from the divider.
    pub fn on_press(mut self, on_press: impl Fn(Point) -> Message + 'a) -> Self {
        self.on_press = Some(Box::new(on_press));
        self
    }

    pub fn current_mode(&self) -> Mode {
        self.mode
    }

    pub fn divider_position(&self) -> f32 {
        self.divider
    }

    pub fn is_single(&self) -> bool {
        self.before.is_none()
    }

    /// The mode the images are drawn in: a single image ignores the mode.
    fn shown(&self) -> Option<Mode> {
        self.before.as_ref().map(|_| self.mode)
    }

    fn image_size(&self, renderer: &Renderer) -> Size {
        measure(renderer, &self.after)
            .or_else(|| self.before.as_ref().and_then(|b| measure(renderer, b)))
            .unwrap_or(FALLBACK)
    }

    fn divider_x(&self, frames: &Frames) -> Option<f32> {
        (self.shown() == Some(Mode::Slider))
            .then_some(frames.after.x + frames.after.width * self.divider)
    }
}

fn clamp_unit(value: f32) -> f32 {
    if value.is_nan() {
        return 0.5;
    }
    value.clamp(0.0, 1.0)
}

fn measure(renderer: &Renderer, handle: &image::Handle) -> Option<Size> {
    if let Some(size) = renderer.measure_image(handle) {
        return Some(Size::new(size.width as f32, size.height as f32));
    }
    match handle {
        image::Handle::Rgba { width, height, .. } => Some(Size::new(*width as f32, *height as f32)),
        _ => None,
    }
    .filter(|size| size.width > 0.0 && size.height > 0.0)
}

/// Where an image of `image` size is drawn inside `area`: as large as fits,
/// keeping its shape, centred.
pub fn fit(image: Size, area: Rectangle) -> Rectangle {
    if image.width <= 0.0 || image.height <= 0.0 || area.width <= 0.0 || area.height <= 0.0 {
        return Rectangle {
            x: area.center_x(),
            y: area.center_y(),
            width: 0.0,
            height: 0.0,
        };
    }
    let scale = (area.width / image.width).min(area.height / image.height);
    let size = Size::new(image.width * scale, image.height * scale);
    Rectangle {
        x: area.x + (area.width - size.width) / 2.0,
        y: area.y + (area.height - size.height) / 2.0,
        width: size.width,
        height: size.height,
    }
}

/// Where each image is drawn. `before` is `None` when both images share
/// the `after` frame or there is no before image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frames {
    pub after: Rectangle,
    pub before: Option<Rectangle>,
}

impl Frames {
    /// Every frame, before first.
    pub fn all(&self) -> impl Iterator<Item = Rectangle> + '_ {
        self.before.into_iter().chain(std::iter::once(self.after))
    }
}

/// The frames for `mode` inside `bounds`. Side by side, the images split the
/// width with `gap` between them; `None` is a single image.
pub fn frames(mode: Option<Mode>, image: Size, bounds: Rectangle, gap: f32) -> Frames {
    if mode != Some(Mode::SideBySide) {
        return Frames {
            after: fit(image, bounds),
            before: None,
        };
    }
    let half = ((bounds.width - gap) / 2.0).max(0.0);
    let left = Rectangle {
        width: half,
        ..bounds
    };
    let right = Rectangle {
        x: bounds.x + half + gap,
        width: half,
        ..bounds
    };
    Frames {
        after: fit(image, right),
        before: Some(fit(image, left)),
    }
}

/// The height the frames need at `width`, keeping the images' shape.
pub fn natural_height(mode: Option<Mode>, image: Size, width: f32, gap: f32) -> f32 {
    if image.width <= 0.0 {
        return 0.0;
    }
    let each = if mode == Some(Mode::SideBySide) {
        ((width - gap) / 2.0).max(0.0)
    } else {
        width
    };
    each * image.height / image.width
}

/// The image pixel under `position`, or `None` outside the frame.
pub fn to_image(position: Point, frame: Rectangle, image: Size) -> Option<Point> {
    if frame.width <= 0.0 || image.width <= 0.0 || !frame.contains(position) {
        return None;
    }
    let scale = image.width / frame.width;
    Some(Point::new(
        ((position.x - frame.x) * scale).min(image.width),
        ((position.y - frame.y) * scale).min(image.height),
    ))
}

/// A rectangle in image pixels, on screen inside `frame`.
pub fn from_image(rect: Rectangle, frame: Rectangle, image: Size) -> Rectangle {
    let scale = if image.width > 0.0 {
        frame.width / image.width
    } else {
        0.0
    };
    Rectangle {
        x: frame.x + rect.x * scale,
        y: frame.y + rect.y * scale,
        width: rect.width * scale,
        height: rect.height * scale,
    }
}

/// The divider position for a pointer at `x` over `frame`.
pub fn divider_at(x: f32, frame: Rectangle) -> f32 {
    if frame.width <= 0.0 {
        return 0.5;
    }
    ((x - frame.x) / frame.width).clamp(0.0, 1.0)
}

/// What an image compare shortcut does to the divider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Left,
    Right,
    Start,
    End,
}

impl Action {
    /// The divider position after this action, to send through the
    /// compare's `on_divider` message.
    pub fn apply(self, divider: f32) -> f32 {
        let divider = clamp_unit(divider);
        match self {
            Action::Left => (divider - STEP).max(0.0),
            Action::Right => (divider + STEP).min(1.0),
            Action::Start => 0.0,
            Action::End => 1.0,
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[Action::Left, Action::Right, Action::Start, Action::End];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Left => "Left",
            Action::Right => "Right",
            Action::Start => "Start",
            Action::End => "End",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Left => "Moves the divider left, showing more of the after image.",
            Action::Right => "Moves the divider right, showing more of the before image.",
            Action::Start => "Moves the divider to the left edge.",
            Action::End => "Moves the divider to the right edge.",
        }
    }
}

/// The default image compare shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowLeft` | [`Action::Left`] |
/// | `ArrowRight` | [`Action::Right`] |
/// | `Home` | [`Action::Start`] |
/// | `End` | [`Action::End`] |
///
/// The compare resolves them itself while its divider can be dragged and it
/// has keyboard focus or was the last thing pressed. Pass a changed keymap
/// with [`ImageCompare::keymap`].
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowLeft), Action::Left)
        .bind(Chord::named(Named::ArrowRight), Action::Right)
        .bind(Chord::named(Named::Home), Action::Start)
        .bind(Chord::named(Named::End), Action::End)
}

/// How the pointer is interacting with the divider.
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

/// Colours of an image compare.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The line around each frame.
    pub frame: Color,
    pub divider: Color,
    /// The edge either side of the divider, so it shows over any image.
    pub divider_edge: Color,
    pub handle: Color,
    pub handle_border: Color,
    pub handle_border_width: f32,
}

/// Resolves the colours of an image compare for the divider's status.
pub fn style(tokens: &Tokens, status: Status) -> Style {
    let border = match status {
        Status::Active => tokens.primary,
        Status::Hovered => mix(tokens.primary, tokens.background, 0.15),
        Status::Dragged => mix(tokens.primary, tokens.background, 0.3),
        Status::Disabled => fade(tokens.primary, 0.5),
    };
    Style {
        frame: tokens.border,
        divider: border,
        divider_edge: tokens.background,
        handle: tokens.background,
        handle_border: border,
        handle_border_width: match status {
            Status::Hovered | Status::Dragged => 2.5,
            Status::Active | Status::Disabled => 2.0,
        },
    }
}

#[derive(Debug, Default)]
struct State {
    dragging: bool,
    finger: Option<touch::Finger>,
    hovered: Option<Point>,
    near: bool,
    focus: focus::State,
    /// The last press landed on the compare, so its keys apply.
    active: bool,
}

impl<Message> ImageCompare<'_, Message> {
    /// Whether shortcuts move the divider: it can be dragged, and the
    /// compare has keyboard focus or was the last thing pressed.
    fn keys_apply(&self, state: &State, frames: &Frames) -> bool {
        self.on_divider.is_some()
            && self.divider_x(frames).is_some()
            && (state.active || state.focus.is_focused())
    }

    fn frames(&self, renderer: &Renderer, bounds: Rectangle) -> (Frames, Size) {
        let image = self.image_size(renderer);
        (frames(self.shown(), image, bounds, space::SM), image)
    }

    fn status(&self, state: &State, frames: &Frames, cursor: mouse::Cursor) -> Status {
        if self.on_divider.is_none() {
            return Status::Disabled;
        }
        if state.dragging {
            return Status::Dragged;
        }
        let near = cursor
            .position_over(frames.after)
            .zip(self.divider_x(frames))
            .is_some_and(|(position, x)| (position.x - x).abs() <= GRAB);
        if near {
            Status::Hovered
        } else {
            Status::Active
        }
    }

    fn hover(&self, state: &mut State, point: Option<Point>, shell: &mut Shell<'_, Message>) {
        if point == state.hovered {
            return;
        }
        state.hovered = point;
        if let Some(on_hover) = &self.on_hover {
            shell.publish(on_hover(point));
        }
    }

    /// Starts a drag if `position` is near the divider. Returns whether it did.
    fn grab(&self, state: &mut State, frames: &Frames, position: Point, reach: f32) -> bool {
        let near = self
            .divider_x(frames)
            .is_some_and(|x| (position.x - x).abs() <= reach && frames.after.contains(position));
        if !near || self.on_divider.is_none() {
            return false;
        }
        state.dragging = true;
        true
    }

    fn drag(&self, frames: &Frames, x: f32, shell: &mut Shell<'_, Message>) {
        if let Some(on_divider) = &self.on_divider {
            shell.publish(on_divider(divider_at(x, frames.after)));
        }
    }

    fn press(&self, frames: &Frames, image: Size, position: Point, shell: &mut Shell<'_, Message>) {
        let Some(on_press) = &self.on_press else {
            return;
        };
        let Some(point) = frames
            .all()
            .find_map(|frame| to_image(position, frame, image))
        else {
            return;
        };
        shell.publish(on_press(point));
        shell.capture_event();
    }
}

fn draw(
    renderer: &mut Renderer,
    handle: &image::Handle,
    filter: image::FilterMethod,
    opacity: f32,
    frame: Rectangle,
    clip: Rectangle,
) {
    let image = core_image::Image {
        opacity,
        ..core_image::Image::new(handle.clone()).filter_method(filter)
    };
    // A layer clips on every renderer; the CPU one ignores an image's own clip.
    renderer.with_layer(clip, |renderer| renderer.draw_image(image, frame, clip));
}

fn quad(renderer: &mut Renderer, bounds: Rectangle, background: Color, border: Border) {
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border,
            ..renderer::Quad::default()
        },
        background,
    );
}

impl<Message> Widget<Message, Theme, Renderer> for ImageCompare<'_, Message> {
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
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
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let image = self.image_size(renderer);
        let natural_width = if self.shown() == Some(Mode::SideBySide) {
            image.width * 2.0 + space::SM
        } else {
            image.width
        };
        let width = limits
            .resolve(self.width, Length::Shrink, Size::new(natural_width, 0.0))
            .width;
        let height = natural_height(self.shown(), image, width, space::SM);
        layout::Node::new(limits.resolve(self.width, self.height, Size::new(width, height)))
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
        let bounds = layout.bounds();
        let (frames, image) = self.frames(renderer, bounds);
        let tokens = Tokens::of(theme);
        let state = tree.state.downcast_ref::<State>();
        let style = style(&tokens, self.status(state, &frames, cursor));

        for frame in frames.all() {
            quad(
                renderer,
                frame.expand(1.0),
                Color::TRANSPARENT,
                Border {
                    color: style.frame,
                    width: 1.0,
                    radius: 0.0.into(),
                },
            );
        }

        let filter = self.filter;
        match (self.shown(), &self.before) {
            (Some(Mode::SideBySide), Some(before)) => {
                let left = frames.before.unwrap_or(frames.after);
                draw(renderer, before, filter, 1.0, left, left);
                draw(
                    renderer,
                    &self.after,
                    filter,
                    1.0,
                    frames.after,
                    frames.after,
                );
            }
            (Some(Mode::Overlay), Some(before)) => {
                draw(
                    renderer,
                    &self.after,
                    filter,
                    1.0,
                    frames.after,
                    frames.after,
                );
                draw(
                    renderer,
                    before,
                    filter,
                    self.opacity,
                    frames.after,
                    frames.after,
                );
            }
            (Some(Mode::Slider), Some(before)) => {
                draw(
                    renderer,
                    &self.after,
                    filter,
                    1.0,
                    frames.after,
                    frames.after,
                );
                let clip = Rectangle {
                    width: frames.after.width * self.divider,
                    ..frames.after
                };
                draw(renderer, before, filter, 1.0, frames.after, clip);
            }
            _ => draw(
                renderer,
                &self.after,
                filter,
                1.0,
                frames.after,
                frames.after,
            ),
        }

        // Images draw above quads within a layer, so marks go in their own.
        renderer.with_layer(bounds, |renderer| {
            if state.focus.is_focused() {
                // Drawn on the frame's edge, so the layer never clips it.
                let target = frames.after.shrink(focus::RING_OUTSET);
                focus::draw_ring(renderer, theme, target, 0.0);
            }
            for frame in frames.all() {
                for outline in &self.outlines {
                    let rect = from_image(outline.rect, frame, image);
                    let Some(clipped) = rect.intersection(&frame.expand(2.0)) else {
                        continue;
                    };
                    quad(
                        renderer,
                        clipped,
                        Color::TRANSPARENT,
                        Border {
                            color: outline.tone.colour(&tokens),
                            width: 2.0,
                            radius: 2.0.into(),
                        },
                    );
                }
            }
            let Some(x) = self.divider_x(&frames) else {
                return;
            };
            quad(
                renderer,
                Rectangle {
                    x: x - 2.0,
                    y: frames.after.y,
                    width: 4.0,
                    height: frames.after.height,
                },
                style.divider_edge,
                Border::default(),
            );
            quad(
                renderer,
                Rectangle {
                    x: x - 1.0,
                    y: frames.after.y,
                    width: 2.0,
                    height: frames.after.height,
                },
                style.divider,
                Border::default(),
            );
            quad(
                renderer,
                Rectangle {
                    x: x - HANDLE_RADIUS,
                    y: frames.after.center_y() - HANDLE_RADIUS,
                    width: HANDLE_RADIUS * 2.0,
                    height: HANDLE_RADIUS * 2.0,
                },
                style.handle,
                Border {
                    color: style.handle_border,
                    width: style.handle_border_width,
                    radius: HANDLE_RADIUS.into(),
                },
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let (frames, image) = self.frames(renderer, layout.bounds());
        let before = state.dragging;
        let keyed = (state.focus.is_focused(), state.active);
        let _ = state.focus.update(event);
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(_)) => {
                state.active = cursor.is_over(layout.bounds());
            }
            Event::Touch(touch::Event::FingerPressed { position, .. }) => {
                state.active = layout.bounds().contains(*position);
            }
            _ => {}
        }
        if (state.focus.is_focused(), state.active) != keyed {
            shell.request_redraw();
        }
        match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })
                if self.keys_apply(state, &frames) =>
            {
                if let Some(action) = self.keymap.resolve(key, *modifiers) {
                    let divider = action.apply(self.divider);
                    if let Some(on_divider) = &self.on_divider
                        && divider != self.divider
                    {
                        shell.publish(on_divider(divider));
                    }
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if state.dragging && state.finger.is_none() {
                    self.drag(&frames, position.x, shell);
                    return;
                }
                let point = frames
                    .all()
                    .find_map(|frame| to_image(*position, frame, image));
                self.hover(state, point, shell);
                // The handle thickens while the pointer is near it.
                let near = self.on_divider.is_some()
                    && frames.after.contains(*position)
                    && self
                        .divider_x(&frames)
                        .is_some_and(|x| (position.x - x).abs() <= GRAB);
                if near != state.near {
                    state.near = near;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorLeft) => self.hover(state, None, shell),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor.position_over(layout.bounds()) else {
                    return;
                };
                if self.grab(state, &frames, position, GRAB) {
                    shell.capture_event();
                } else {
                    self.press(&frames, image, position, shell);
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.dragging && state.finger.is_none() =>
            {
                state.dragging = false;
                shell.capture_event();
            }
            Event::Touch(touch::Event::FingerPressed { id, position }) => {
                if state.dragging || !layout.bounds().contains(*position) {
                    return;
                }
                if self.grab(state, &frames, *position, TOUCH_GRAB) {
                    state.finger = Some(*id);
                    shell.capture_event();
                } else {
                    self.press(&frames, image, *position, shell);
                }
            }
            Event::Touch(touch::Event::FingerMoved { id, position })
                if state.finger == Some(*id) =>
            {
                self.drag(&frames, position.x, shell);
                shell.capture_event();
            }
            Event::Touch(
                touch::Event::FingerLifted { id, .. } | touch::Event::FingerLost { id, .. },
            ) if state.finger == Some(*id) => {
                state.dragging = false;
                state.finger = None;
                shell.capture_event();
            }
            _ => {}
        }
        if state.dragging != before {
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        if state.dragging {
            return mouse::Interaction::ResizingHorizontally;
        }
        let (frames, _) = self.frames(renderer, layout.bounds());
        let Some(position) = frames.all().find_map(|frame| cursor.position_over(frame)) else {
            return mouse::Interaction::None;
        };
        match self.divider_x(&frames) {
            Some(x) if self.on_divider.is_some() && (position.x - x).abs() <= GRAB => {
                mouse::Interaction::ResizingHorizontally
            }
            _ if self.on_press.is_some() => mouse::Interaction::Crosshair,
            _ => mouse::Interaction::None,
        }
    }

    #[cfg(not(feature = "icedwright"))]
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        let (frames, _) = self.frames(renderer, layout.bounds());
        if self.on_divider.is_some() && self.divider_x(&frames).is_some() {
            let state = tree.state.downcast_mut::<State>();
            focus::report(operation, None, frames.after, &mut state.focus);
        }
    }

    #[cfg(feature = "icedwright")]
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        use icedwright_probe::{Annotation, report, role};

        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        let (frames, _) = self.frames(renderer, bounds);
        const KIND: &str = "iced_cube::image_compare";
        let mode = match self.shown() {
            None => "single",
            Some(Mode::Slider) => "slider",
            Some(Mode::SideBySide) => "side-by-side",
            Some(Mode::Overlay) => "overlay",
        };
        report(operation, bounds, || {
            Annotation::new()
                .role(role::Group)
                .name(self.label.to_string())
                .prop("mode", mode)
                .component(KIND)
        });
        operation.traverse(&mut |operation| {
            let before = match self.shown() {
                Some(Mode::SideBySide) => frames.before,
                Some(_) => Some(frames.after),
                None => None,
            };
            if let Some(frame) = before {
                report(operation, frame, || {
                    Annotation::new()
                        .role(role::Image)
                        .name("Before")
                        .component(KIND)
                });
            }
            report(operation, frames.after, || {
                Annotation::new()
                    .role(role::Image)
                    .name("After")
                    .component(KIND)
            });
            if self.divider_x(&frames).is_none() {
                return;
            }
            let enabled = self.on_divider.is_some();
            report(operation, frames.after, || {
                let divider = Annotation::new()
                    .role(role::Splitter)
                    .name("Divider")
                    .component(KIND)
                    .range(f64::from(self.divider * 100.0).round(), 0.0..=100.0);
                if enabled {
                    divider
                } else {
                    divider.disabled(true)
                }
            });
            // After the divider, at the same bounds, so they merge.
            if enabled {
                focus::report(operation, None, frames.after, &mut state.focus);
            }
        });
    }
}

impl<'a, Message: 'a> From<ImageCompare<'a, Message>> for Element<'a, Message> {
    fn from(compare: ImageCompare<'a, Message>) -> Self {
        Element::new(compare)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::Action as _;
    use crate::theme::{dark, light};

    fn area() -> Rectangle {
        Rectangle {
            x: 100.0,
            y: 50.0,
            width: 400.0,
            height: 400.0,
        }
    }

    fn pixel() -> image::Handle {
        image::Handle::from_rgba(1, 1, vec![0, 0, 0, 255])
    }

    #[test]
    fn default_builder_is_a_centred_slider_filling_the_width() {
        let c: ImageCompare<'_, ()> = image_compare(pixel(), pixel());
        assert_eq!(c.current_mode(), Mode::Slider);
        assert_eq!(c.divider_position(), 0.5);
        assert_eq!(c.opacity, 0.5);
        assert_eq!(c.width, Length::Fill);
        assert_eq!(c.height, Length::Shrink);
        assert_eq!(c.label, "Image compare");
        assert!(!c.is_single());
        assert!(c.on_divider.is_none());
        let s: ImageCompare<'_, ()> = single(pixel()).mode(Mode::Overlay);
        assert!(s.is_single());
        assert_eq!(s.shown(), None);
    }

    #[test]
    fn divider_and_opacity_are_clamped() {
        let c: ImageCompare<'_, ()> = image_compare(pixel(), pixel()).divider(3.0).opacity(-1.0);
        assert_eq!((c.divider, c.opacity), (1.0, 0.0));
        let c: ImageCompare<'_, ()> = image_compare(pixel(), pixel()).divider(f32::NAN);
        assert_eq!(c.divider, 0.5);
    }

    #[test]
    fn a_wide_image_fits_the_width_centred() {
        let fitted = fit(Size::new(800.0, 400.0), area());
        assert_eq!(
            fitted,
            Rectangle::new(Point::new(100.0, 150.0), Size::new(400.0, 200.0))
        );
        let tall = fit(Size::new(100.0, 400.0), area());
        assert_eq!(
            tall,
            Rectangle::new(Point::new(250.0, 50.0), Size::new(100.0, 400.0))
        );
    }

    #[test]
    fn an_empty_image_fits_nowhere() {
        let fitted = fit(Size::new(0.0, 0.0), area());
        assert_eq!(fitted.width, 0.0);
        assert_eq!(to_image(Point::new(300.0, 250.0), fitted, Size::ZERO), None);
    }

    #[test]
    fn side_by_side_splits_the_width_around_the_gap() {
        let frames = frames(Some(Mode::SideBySide), Size::new(100.0, 100.0), area(), 8.0);
        let Some(before) = frames.before else {
            panic!("a before frame");
        };
        assert_eq!(before.width, 196.0);
        assert_eq!(before.x, 100.0);
        assert_eq!(frames.after.x, 100.0 + 196.0 + 8.0);
        assert_eq!(frames.all().count(), 2);
        for mode in [None, Some(Mode::Slider), Some(Mode::Overlay)] {
            let shared = super::frames(mode, Size::new(100.0, 100.0), area(), 8.0);
            assert_eq!(shared.before, None);
            assert_eq!(shared.after, fit(Size::new(100.0, 100.0), area()));
        }
    }

    #[test]
    fn natural_height_keeps_the_shape() {
        let image = Size::new(200.0, 100.0);
        assert_eq!(natural_height(Some(Mode::Slider), image, 400.0, 8.0), 200.0);
        assert_eq!(
            natural_height(Some(Mode::SideBySide), image, 408.0, 8.0),
            100.0
        );
        assert_eq!(natural_height(None, Size::ZERO, 400.0, 8.0), 0.0);
    }

    #[test]
    fn screen_and_image_points_round_trip() {
        let image = Size::new(800.0, 400.0);
        let fitted = fit(image, area());
        let point = to_image(Point::new(300.0, 250.0), fitted, image);
        assert_eq!(point, Some(Point::new(400.0, 200.0)));
        let rect = from_image(
            Rectangle::new(Point::new(400.0, 200.0), Size::new(80.0, 40.0)),
            fitted,
            image,
        );
        assert_eq!(
            rect,
            Rectangle::new(Point::new(300.0, 250.0), Size::new(40.0, 20.0))
        );
        assert_eq!(to_image(Point::new(300.0, 60.0), fitted, image), None);
    }

    #[test]
    fn the_divider_follows_the_pointer_across_the_frame() {
        let frame = Rectangle::new(Point::new(100.0, 0.0), Size::new(200.0, 100.0));
        assert_eq!(divider_at(150.0, frame), 0.25);
        assert_eq!(divider_at(0.0, frame), 0.0);
        assert_eq!(divider_at(900.0, frame), 1.0);
        assert_eq!(divider_at(10.0, Rectangle::default()), 0.5);
    }

    #[test]
    fn actions_move_the_divider_within_the_frame() {
        assert!((Action::Left.apply(0.5) - 0.45).abs() < 1e-6);
        assert!((Action::Right.apply(0.5) - 0.55).abs() < 1e-6);
        assert_eq!(Action::Left.apply(0.02), 0.0);
        assert_eq!(Action::Right.apply(0.99), 1.0);
        assert_eq!(Action::Start.apply(0.7), 0.0);
        assert_eq!(Action::End.apply(0.2), 1.0);
    }

    #[test]
    fn default_keymap_and_overrides() {
        let keymap = default_keymap();
        let resolve =
            |keymap: &Keymap<Action>, chord: Chord| keymap.resolve(chord.key(), chord.modifiers());
        assert_eq!(
            resolve(&keymap, Chord::named(Named::ArrowLeft)),
            Some(Action::Left)
        );
        assert_eq!(
            resolve(&keymap, Chord::named(Named::End)),
            Some(Action::End)
        );
        assert_eq!(Keymap::<Action>::defaults(), keymap);
        let keymap = keymap
            .bind(Chord::character('h'), Action::Left)
            .unbind(&Chord::named(Named::Home));
        assert_eq!(resolve(&keymap, Chord::character('h')), Some(Action::Left));
        assert_eq!(resolve(&keymap, Chord::named(Named::Home)), None);
        for action in Action::ALL {
            assert!(action.description().ends_with('.'));
        }
    }

    #[test]
    fn every_mode_has_a_label() {
        let labels: Vec<_> = Mode::ALL.iter().map(|m| m.label()).collect();
        assert_eq!(labels, ["Slider", "Side by side", "Overlay"]);
    }

    #[test]
    fn style_follows_the_status_in_light_and_dark() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let active = style(&tokens, Status::Active);
            assert_eq!(active.divider, tokens.primary);
            assert_eq!(active.frame, tokens.border);
            let hovered = style(&tokens, Status::Hovered);
            assert_ne!(hovered.divider, active.divider);
            assert!(hovered.handle_border_width > active.handle_border_width);
            let dragged = style(&tokens, Status::Dragged);
            assert_ne!(dragged.divider, hovered.divider);
            let disabled = style(&tokens, Status::Disabled);
            assert!(disabled.divider.a < active.divider.a);
            assert_eq!(Status::ALL.len(), 4);
        }
    }

    #[test]
    fn outlines_take_a_tone_and_default_to_primary() {
        let outline = Outline::new(Rectangle::default()).tone(Tone::Warning);
        assert_eq!(outline.tone, Tone::Warning);
        assert_eq!(Outline::new(Rectangle::default()).tone, Tone::Primary);
    }
}
