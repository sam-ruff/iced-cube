//! Rich text that can be selected with the mouse and copied.
//!
//! [`selectable_text`] takes the spans iced's rich text takes and draws its
//! own part of a selection. A [`selection_area`] around any number of them
//! owns the selection, so a drag can run from one paragraph into the next:
//! drag to select, double click for a word, triple click for a block.
//! Links still open on a plain click.
//!
//! The area sends the selected text through `on_select` each time a
//! selection is finished and through `on_copy` when the Copy shortcut is
//! pressed; the app writes it to the clipboard. Keyboard shortcuts resolve
//! through a [`Keymap`] of [`Action`]s; see [`default_keymap`]. Typing, Tab
//! or another Ctrl or Cmd chord drops the selection, as the keys then
//! belong to something else. A field given focus without a key or a press,
//! by a task, does not: clear the selection then if the two share chords.
//!
//! Put the scrollable inside the area, not around it, and a drag past its
//! top or bottom edge scrolls it.
//!
//! For a context menu, give the area an id and wrap it in a context menu
//! whose items run [`selected`] and [`select_all`].
//!
//! ```no_run
//! use iced::widget::{column, span};
//! use iced::{Element, Task};
//! use iced_cube::data::selectable_text::{selectable_text, selection_area};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Copy(String),
//!     Open(String),
//! }
//!
//! fn view<'a>() -> Element<'a, Message> {
//!     selection_area(column![
//!         selectable_text(vec![span("Read the "), span("guide").link("guide".to_owned())])
//!             .on_link_click(Message::Open),
//!         selectable_text::<String, _>(vec![span("A second paragraph.")]),
//!     ])
//!     .on_copy(Message::Copy)
//!     .into()
//! }
//!
//! fn update(message: Message) -> Task<Message> {
//!     match message {
//!         Message::Copy(text) => iced::clipboard::write(text),
//!         Message::Open(_) => Task::none(),
//!     }
//! }
//! ```

use std::any::Any;
use std::fmt;

use iced::advanced::graphics::text::cosmic_text;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::text::{self, Paragraph as _, Span};
use iced::advanced::widget::operation::{Operation, Outcome, Scrollable};
use iced::advanced::widget::{self, Id, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget, mouse, overlay};
use iced::keyboard::{self, key::Named};
use iced::time::{Duration, Instant};
use iced::widget::scrollable::AbsoluteOffset;
use iced::{
    Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Renderer, Size, Task, Theme,
    Vector, window,
};

use crate::keys::{self, Chord, Keymap};
use crate::shaped::{Paragraph, Shaped};
use crate::theme::{Tokens, fade};

/// How fast a drag just past the edge of a scrollable scrolls it, in pixels
/// a second.
pub const SCROLL_SPEED: f32 = 200.0;
/// How much faster it scrolls for each pixel the pointer is past the edge.
pub const SCROLL_GAIN: f32 = 12.0;
/// The fastest a drag scrolls, in pixels a second.
pub const MAX_SCROLL_SPEED: f32 = 2400.0;
/// The longest gap between frames that still counts as scrolling time.
const MAX_FRAME: Duration = Duration::from_millis(50);
/// How soon a press must follow the last to count as a double or triple
/// click.
pub const DOUBLE_CLICK: Duration = Duration::from_millis(400);
/// How far apart the presses of a double click may be, in pixels.
const CLICK_SLOP: f32 = 6.0;

/// A place in one text: the line, then the byte within it.
type Caret = (usize, usize);
/// A place in an area: which text, counted in drawing order, then where in
/// it.
type Position = (usize, Caret);

const START: Caret = (0, 0);
const END: Caret = (usize::MAX, usize::MAX);

/// How selected text looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// Drawn over the selected text.
    pub selection: Color,
}

/// The selection colour, matching a text field's.
pub fn style(tokens: &Tokens) -> Style {
    Style {
        selection: fade(tokens.primary, 0.25),
    }
}

/// How fast a drag `distance` pixels past the edge of a scrollable scrolls
/// it, in pixels a second.
pub fn scroll_speed(distance: f32) -> f32 {
    (SCROLL_SPEED + SCROLL_GAIN * distance.max(0.0)).min(MAX_SCROLL_SPEED)
}

/// What a selection area's keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Sends the selected text through `on_copy`.
    Copy,
    /// Selects every text in the area.
    SelectAll,
    /// Drops the selection.
    Clear,
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[Action::Copy, Action::SelectAll, Action::Clear];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Copy => "Copy",
            Action::SelectAll => "Select all",
            Action::Clear => "Clear",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Copy => "Copies the selected text.",
            Action::SelectAll => "Selects all the text in the area that was last clicked.",
            Action::Clear => "Drops the selection.",
        }
    }
}

/// The default selection shortcuts. Mod is Cmd on macOS and Ctrl elsewhere.
///
/// | Keys | Action |
/// | --- | --- |
/// | `Mod+C` | [`Action::Copy`] |
/// | `Mod+A` | [`Action::SelectAll`] |
/// | `Escape` | [`Action::Clear`] |
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::character('c').command(), Action::Copy)
        .bind(Chord::character('a').command(), Action::SelectAll)
        .bind(Chord::named(Named::Escape), Action::Clear)
}

/// What a [`SelectionArea`] reads and writes in each text under it.
struct Body {
    paragraph: Paragraph,
    /// Where the pointer was at the last event, relative to the text's
    /// corner.
    pointer: Option<Point>,
    /// Whether the pointer was inside every scrollable around the text.
    direct: bool,
    selection: Option<(Caret, Caret)>,
}

impl Body {
    fn hit(&self, point: Point) -> Caret {
        let cursor = self
            .paragraph
            .shaped()
            .and_then(|buffer| buffer.hit(point.x, point.y));
        match cursor {
            Some(cursor) => (cursor.line, cursor.index),
            None if point.y < 0.0 => START,
            None => END,
        }
    }

    fn rectangles(&self, from: Caret, to: Caret) -> Vec<Rectangle> {
        let Some(buffer) = self.paragraph.shaped() else {
            return Vec::new();
        };
        let start = cosmic_text::Cursor::new(from.0, from.1);
        let end = cosmic_text::Cursor::new(to.0, to.1);
        buffer
            .layout_runs()
            .filter_map(|run| {
                let (x, width) = run.highlight(start, end)?;
                (width > 0.0).then(|| {
                    Rectangle::new(
                        Point::new(x, run.line_top),
                        Size::new(width, run.line_height),
                    )
                })
            })
            .collect()
    }

    fn text(&self, from: Caret, to: Caret) -> String {
        let Some(buffer) = self.paragraph.shaped() else {
            return String::new();
        };
        let mut lines = Vec::new();
        for (index, line) in buffer.lines.iter().enumerate() {
            if index < from.0 || index > to.0 {
                continue;
            }
            let text = line.text();
            let start = if index == from.0 {
                from.1.min(text.len())
            } else {
                0
            };
            let end = if index == to.0 {
                to.1.min(text.len())
            } else {
                text.len()
            };
            lines.push(text.get(start..end).unwrap_or_default());
        }
        lines.join("\n")
    }

    fn selected(&self) -> Option<String> {
        let (from, to) = self.selection?;
        let text = self.text(from, to);
        (!text.is_empty()).then_some(text)
    }

    /// The word around `at`, or the single character there when it is not
    /// part of one.
    fn word(&self, at: Caret) -> Option<(Caret, Caret)> {
        let buffer = self.paragraph.shaped()?;
        let line = at.0.min(buffer.lines.len().checked_sub(1)?);
        let (start, end) = word_bounds(buffer.lines.get(line)?.text(), at.1)?;
        Some(((line, start), (line, end)))
    }
}

/// The point of `bounds` nearest to `point`.
fn clamp(point: Point, bounds: Rectangle) -> Point {
    Point::new(
        point.x.clamp(bounds.x, bounds.x + bounds.width.max(0.0)),
        point.y.clamp(bounds.y, bounds.y + bounds.height.max(0.0)),
    )
}

/// Finds the text the pointer is on from the texts in drawing order: the
/// one it is over, else the one it is beside, else the next one below it.
#[derive(Default)]
struct Search {
    found: Option<Position>,
    /// The last text level with the pointer, for when the pointer is beside
    /// it.
    level: Option<Position>,
}

impl Search {
    fn feed(&mut self, index: usize, size: Size, pointer: Point, hit: impl FnOnce() -> Caret) {
        if self.found.is_some() {
            return;
        }
        if pointer.y < 0.0 {
            self.found = self.level.or(Some((index, START)));
            return;
        }
        if pointer.y >= size.height {
            return;
        }
        let here = Some((index, hit()));
        if pointer.x < size.width {
            self.found = here;
        } else {
            self.level = here;
        }
    }

    fn result(&self) -> Option<Position> {
        self.found.or(self.level)
    }
}

/// How many presses in a row this one makes: one, two for a double click or
/// three for a triple click. A fourth press starts again.
fn click_count(previous: Option<Click>, position: Point, now: Instant, interval: Duration) -> u8 {
    let Some(previous) = previous else {
        return 1;
    };
    let repeated = previous.count < 3
        && position.distance(previous.position) <= CLICK_SLOP
        && now.saturating_duration_since(previous.at) <= interval;
    if repeated { previous.count + 1 } else { 1 }
}

#[derive(Debug, Clone, Copy)]
struct Click {
    position: Point,
    at: Instant,
    count: u8,
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// The byte range of the word at `index`, or of the character there when it
/// is not a letter, digit or underscore.
fn word_bounds(text: &str, index: usize) -> Option<(usize, usize)> {
    let (at, character) = text
        .char_indices()
        .take_while(|(start, _)| *start <= index)
        .last()?;
    if !is_word(character) {
        return Some((at, at + character.len_utf8()));
    }
    let start = text[..at]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word(*c))
        .last()
        .map_or(at, |(start, _)| start);
    let end = text[at..]
        .char_indices()
        .find(|(_, c)| !is_word(*c))
        .map_or(text.len(), |(offset, _)| at + offset);
    Some((start, end))
}

struct State<Link> {
    spans: Vec<Span<'static, Link>>,
    /// The spans' text in one piece, for operations that look for text.
    text: String,
    hovered: Option<usize>,
    pressed: Option<usize>,
    body: Body,
}

/// Rich text whose links can be clicked and whose text can be selected
/// inside a [`SelectionArea`]. Convert it into an [`Element`] to render.
pub struct SelectableText<'a, Link, Message> {
    spans: Box<dyn AsRef<[Span<'a, Link>]> + 'a>,
    size: Option<Pixels>,
    line_height: text::LineHeight,
    font: Option<Font>,
    colour: Option<Color>,
    width: Length,
    on_link_click: Option<Box<dyn Fn(Link) -> Message + 'a>>,
    style: Option<StyleFn<'a>>,
}

type StyleFn<'a> = Box<dyn Fn(&Theme) -> Style + 'a>;

impl<Link, Message> fmt::Debug for SelectableText<'_, Link, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectableText")
            .field("spans", &self.spans.as_ref().as_ref().len())
            .field("size", &self.size)
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}

/// Selectable rich text made of `spans`, as iced's rich text takes them.
/// Text without links needs the link type named:
/// `selectable_text::<String, _>(...)`.
pub fn selectable_text<'a, Link, Message>(
    spans: impl AsRef<[Span<'a, Link>]> + 'a,
) -> SelectableText<'a, Link, Message> {
    SelectableText {
        spans: Box::new(spans),
        size: None,
        line_height: text::LineHeight::default(),
        font: None,
        colour: None,
        width: Length::Shrink,
        on_link_click: None,
        style: None,
    }
}

impl<'a, Link, Message> SelectableText<'a, Link, Message> {
    /// The text size of spans that set none.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    pub fn line_height(mut self, line_height: impl Into<text::LineHeight>) -> Self {
        self.line_height = line_height.into();
        self
    }

    /// The font of spans that set none.
    pub fn font(mut self, font: impl Into<Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// The colour of spans that set none. Defaults to the text colour around
    /// the widget.
    pub fn color(mut self, colour: impl Into<Color>) -> Self {
        self.colour = Some(colour.into());
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// The message for a link that is clicked. Without it links are drawn
    /// but do nothing.
    pub fn on_link_click(mut self, on_link_click: impl Fn(Link) -> Message + 'a) -> Self {
        self.on_link_click = Some(Box::new(on_link_click));
        self
    }

    /// Replaces the default look, which is [`style`].
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    fn spans(&self) -> &[Span<'a, Link>] {
        self.spans.as_ref().as_ref()
    }

    fn link(&self, span: usize) -> Option<&Link> {
        self.spans().get(span)?.link.as_ref()
    }

    fn draw_decorations(
        &self,
        state: &State<Link>,
        renderer: &mut Renderer,
        defaults: &renderer::Style,
        origin: Vector,
    ) {
        for (index, span) in self.spans().iter().enumerate() {
            let hovered_link = self.on_link_click.is_some() && state.hovered == Some(index);
            let lined = span.underline || span.strikethrough || hovered_link;
            if span.highlight.is_none() && !lined {
                continue;
            }
            let regions = state.body.paragraph.span_bounds(index);

            if let Some(highlight) = span.highlight {
                for bounds in &regions {
                    let padded = Rectangle::new(
                        bounds.position() - Vector::new(span.padding.left, span.padding.top),
                        bounds.size() + Size::new(span.padding.x(), span.padding.y()),
                    );
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: padded + origin,
                            border: highlight.border,
                            ..renderer::Quad::default()
                        },
                        highlight.background,
                    );
                }
            }
            if !lined {
                continue;
            }

            let size = span
                .size
                .or(self.size)
                .unwrap_or(text::Renderer::default_size(renderer));
            let line_height = span
                .line_height
                .unwrap_or(self.line_height)
                .to_absolute(size);
            let colour = span.color.or(self.colour).unwrap_or(defaults.text_color);
            let baseline = origin + Vector::new(0.0, size.0 + (line_height.0 - size.0) / 2.0);
            let line = |renderer: &mut Renderer, rise: f32| {
                for bounds in &regions {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle::new(
                                bounds.position() + baseline - Vector::new(0.0, rise),
                                Size::new(bounds.width, 1.0),
                            ),
                            ..renderer::Quad::default()
                        },
                        colour,
                    );
                }
            };
            if span.underline || hovered_link {
                line(renderer, size.0 * 0.08);
            }
            if span.strikethrough {
                line(renderer, size.0 / 2.0);
            }
        }
    }
}

impl<Link, Message> Widget<Message, Theme, Renderer> for SelectableText<'_, Link, Message>
where
    Link: Clone + PartialEq + 'static,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Link>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Link> {
            spans: Vec::new(),
            text: String::new(),
            hovered: None,
            pressed: None,
            body: Body {
                paragraph: Paragraph::default(),
                pointer: None,
                direct: false,
                selection: None,
            },
        })
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State<Link>>();
        let spans = self.spans.as_ref().as_ref();
        let size = self
            .size
            .unwrap_or_else(|| text::Renderer::default_size(renderer));
        let font = self
            .font
            .unwrap_or_else(|| text::Renderer::default_font(renderer));
        let line_height = self.line_height;

        layout::sized(limits, self.width, Length::Shrink, |limits| {
            let bounds = limits.max();
            let shape = text::Text {
                content: (),
                bounds,
                size,
                line_height,
                font,
                align_x: text::Alignment::Default,
                align_y: iced::alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::default(),
            };
            let shaped = || Paragraph::with_spans(shape.with_content(spans));

            if state.spans != spans {
                state.body.paragraph = shaped();
                state.spans = spans.iter().cloned().map(Span::to_static).collect();
                state.text = spans.iter().map(|span| &*span.text).collect();
                state.body.selection = None;
            } else {
                match state.body.paragraph.compare(shape) {
                    text::Difference::None => {}
                    text::Difference::Bounds => state.body.paragraph.resize(bounds),
                    text::Difference::Shape => state.body.paragraph = shaped(),
                }
            }
            state.body.paragraph.min_bounds()
        })
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State<Link>>();
        operation.text(None, layout.bounds(), &state.text);
        operation.custom(None, layout.bounds(), &mut state.body);
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
        viewport: &Rectangle,
    ) {
        let event = match event {
            Event::Mouse(event) => Some(event),
            // A frame drawn while a drag scrolls the page moves the text
            // under a pointer that has not moved.
            Event::Window(window::Event::RedrawRequested(_)) => None,
            _ => return,
        };
        let state = tree.state.downcast_mut::<State<Link>>();
        let bounds = layout.bounds();
        // A cursor outside a scrolled parent still says where a drag has
        // reached, but only as far as the edge of what the parent shows:
        // text scrolled out of view is never under the pointer.
        state.body.direct = cursor.position().is_some();
        state.body.pointer = cursor.land().position().map(|at| {
            let at = if state.body.direct {
                at
            } else {
                clamp(at, *viewport)
            };
            Point::new(at.x - bounds.x, at.y - bounds.y)
        });
        let Some(event) = event else {
            return;
        };

        let hovered = cursor
            .position_in(bounds)
            .and_then(|at| state.body.paragraph.hit_span(at))
            .filter(|span| self.link(*span).is_some());
        if hovered != state.hovered {
            state.hovered = hovered;
            shell.request_redraw();
        }

        match event {
            mouse::Event::ButtonPressed(mouse::Button::Left) => state.pressed = state.hovered,
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                let pressed = state.pressed.take();
                // A drag that ends on the link it began on is a selection,
                // not a click.
                if pressed.is_none() || pressed != state.hovered || state.body.selection.is_some() {
                    return;
                }
                let link = pressed.and_then(|span| self.link(span));
                if let (Some(link), Some(on_link_click)) = (link, &self.on_link_click) {
                    shell.publish(on_link_click(link.clone()));
                }
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if !bounds.intersects(viewport) {
            return;
        }
        let state = tree.state.downcast_ref::<State<Link>>();
        let origin = bounds.position() - Point::ORIGIN;

        // Inline code has an opaque background, so the selection goes over
        // it.
        self.draw_decorations(state, renderer, defaults, origin);
        if let Some((from, to)) = state.body.selection {
            let colour = match &self.style {
                Some(style) => style(theme),
                None => style(&Tokens::of(theme)),
            }
            .selection;
            for rectangle in state.body.rectangles(from, to) {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: rectangle + origin,
                        ..renderer::Quad::default()
                    },
                    colour,
                );
            }
        }
        widget::text::draw(
            renderer,
            defaults,
            bounds,
            &state.body.paragraph,
            widget::text::Style { color: self.colour },
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State<Link>>();
        if state.hovered.is_some() && self.on_link_click.is_some() {
            return mouse::Interaction::Pointer;
        }
        if cursor.is_over(layout.bounds()) {
            return mouse::Interaction::Text;
        }
        mouse::Interaction::None
    }
}

impl<'a, Link, Message> From<SelectableText<'a, Link, Message>> for Element<'a, Message>
where
    Link: Clone + PartialEq + 'static,
    Message: 'a,
{
    fn from(text: SelectableText<'a, Link, Message>) -> Self {
        Element::new(text)
    }
}

/// A drag past the edge of a scrollable: where the pointer is and how long
/// the frame took.
#[derive(Clone, Copy)]
struct Pull {
    pointer: Point,
    elapsed: Duration,
}

/// Calls `each` with every [`Body`] it meets, numbered in drawing order, and
/// scrolls the scrollables a [`Pull`] is past the edge of.
struct Visit<'f> {
    index: usize,
    each: &'f mut (dyn FnMut(usize, Rectangle, &mut Body) + Send),
    pull: Option<Pull>,
    /// Whether the pointer is past the edge of a scrollable.
    pulling: bool,
    /// How far the scrollables around the widgets being visited have
    /// scrolled, which is how far their layout is from the window's.
    scrolled: Vector,
    /// The scroll of the scrollable whose content is visited next.
    entering: Option<Vector>,
}

impl Operation for Visit<'_> {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        // A scrollable announces itself just before its content is visited.
        let entered = self.entering.take().unwrap_or(Vector::ZERO);
        self.scrolled += entered;
        operate(self);
        self.scrolled -= entered;
    }

    fn custom(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Any) {
        let Some(body) = state.downcast_mut::<Body>() else {
            return;
        };
        (self.each)(self.index, bounds, body);
        self.index += 1;
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn Scrollable,
    ) {
        self.entering = Some(translation);
        let Some(pull) = self.pull else {
            return;
        };
        if content_bounds.height <= bounds.height {
            return;
        }
        // The pointer in the space this scrollable is laid out in.
        let pointer = pull.pointer + self.scrolled;
        let above = bounds.y - pointer.y;
        let below = pointer.y - (bounds.y + bounds.height);
        let (distance, direction) = if above > 0.0 {
            (above, -1.0)
        } else if below > 0.0 {
            (below, 1.0)
        } else {
            return;
        };
        self.pulling = true;
        let y = direction * scroll_speed(distance) * pull.elapsed.as_secs_f32();
        state.scroll_by(AbsoluteOffset { x: 0.0, y }, bounds, content_bounds);
    }
}

/// What a task asks of the area with its id.
#[derive(Default)]
struct Request {
    select_all: bool,
    read: bool,
}

/// The selected text, handed back to the task that asked.
struct Reply(String);

struct Command {
    target: Id,
    select_all: bool,
    text: Option<String>,
}

impl Operation<String> for Command {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<String>)) {
        operate(self);
    }

    fn custom(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Any) {
        if id != Some(&self.target) {
            return;
        }
        if let Some(request) = state.downcast_mut::<Request>() {
            request.select_all = self.select_all;
            request.read = true;
        }
        if let Some(Reply(text)) = state.downcast_mut::<Reply>() {
            self.text = Some(std::mem::take(text));
        }
    }

    fn finish(&self) -> Outcome<String> {
        self.text.clone().map_or(Outcome::None, Outcome::Some)
    }
}

/// The widget operations behind [`selected`] and [`select_all`], for apps
/// that run operations themselves.
pub mod operation {
    use super::{Command, Id, Operation};

    /// Produces the text selected in the area with this id.
    pub fn selected(id: impl Into<Id>) -> impl Operation<String> {
        Command {
            target: id.into(),
            select_all: false,
            text: None,
        }
    }

    /// Selects every text in the area with this id and produces it.
    pub fn select_all(id: impl Into<Id>) -> impl Operation<String> {
        Command {
            target: id.into(),
            select_all: true,
            text: None,
        }
    }
}

/// Produces the text selected in the area with this id, which is empty when
/// nothing is. Produces nothing when no such area is on screen.
pub fn selected(id: impl Into<Id>) -> Task<String> {
    widget::operate(operation::selected(id.into()))
}

/// Selects every text in the area with this id and produces it.
pub fn select_all(id: impl Into<Id>) -> Task<String> {
    widget::operate(operation::select_all(id.into()))
}

#[derive(Default)]
struct AreaState {
    anchor: Option<Position>,
    dragging: bool,
    /// Whether the last press landed in the area, so Select all is meant
    /// for it.
    active: bool,
    last_click: Option<Click>,
    /// When the drag last scrolled a scrollable.
    last_pull: Option<Instant>,
}

/// Lets the [`SelectableText`]s inside it be selected together. Convert it
/// into an [`Element`] to render.
pub struct SelectionArea<'a, Message> {
    content: Element<'a, Message>,
    id: Option<Id>,
    keymap: Keymap<Action>,
    double_click: Duration,
    on_copy: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_select: Option<Box<dyn Fn(String) -> Message + 'a>>,
}

impl<Message> fmt::Debug for SelectionArea<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectionArea")
            .field("id", &self.id)
            .field("keymap", &self.keymap)
            .finish_non_exhaustive()
    }
}

/// An area in which the selectable texts of `content` are selected
/// together.
pub fn selection_area<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> SelectionArea<'a, Message> {
    SelectionArea {
        content: content.into(),
        id: None,
        keymap: default_keymap(),
        double_click: DOUBLE_CLICK,
        on_copy: None,
        on_select: None,
    }
}

impl<'a, Message> SelectionArea<'a, Message> {
    /// The id [`selected`] and [`select_all`] find the area by.
    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Replaces the default shortcuts.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    /// How soon a press must follow the last to count as a double or triple
    /// click. Defaults to [`DOUBLE_CLICK`].
    pub fn double_click(mut self, interval: Duration) -> Self {
        self.double_click = interval;
        self
    }

    /// The message for the selected text when the Copy shortcut is pressed.
    pub fn on_copy(mut self, on_copy: impl Fn(String) -> Message + 'a) -> Self {
        self.on_copy = Some(Box::new(on_copy));
        self
    }

    /// The message for the selected text each time a selection is finished:
    /// a drag is released, a word or block is clicked or everything is
    /// selected from the keyboard.
    pub fn on_select(mut self, on_select: impl Fn(String) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    fn visit(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        pull: Option<Pull>,
        each: &mut (dyn FnMut(usize, Rectangle, &mut Body) + Send),
    ) -> bool {
        let mut visit = Visit {
            index: 0,
            each,
            pull,
            pulling: false,
            scrolled: Vector::ZERO,
            entering: None,
        };
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, &mut visit);
        visit.pulling
    }

    /// Where the pointer is among the texts, or `None` when no text has
    /// seen it.
    fn locate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<Position> {
        let mut seen = false;
        let mut count = 0;
        // Texts the pointer can really be over come first. The rest sit in
        // a scrollable the pointer is outside, and count only when a drag
        // has left every text behind.
        let mut direct = Search::default();
        let mut any = Search::default();
        self.visit(tree, layout, renderer, None, &mut |index, bounds, body| {
            count = index + 1;
            let Some(pointer) = body.pointer else {
                return;
            };
            seen = true;
            let body = &*body;
            if body.direct {
                direct.feed(index, bounds.size(), pointer, || body.hit(pointer));
            }
            any.feed(index, bounds.size(), pointer, || body.hit(pointer));
        });
        seen.then(|| direct.result().or(any.result()).unwrap_or((count, START)))
    }

    fn select(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        from: Position,
        to: Position,
    ) -> bool {
        let (start, end) = if from <= to { (from, to) } else { (to, from) };
        let mut changed = false;
        self.visit(tree, layout, renderer, None, &mut |index, _, body| {
            let first = if index == start.0 { start.1 } else { START };
            let last = if index == end.0 { end.1 } else { END };
            let inside = index >= start.0 && index <= end.0 && first < last;
            let selection = inside.then_some((first, last));
            changed |= body.selection != selection;
            body.selection = selection;
        });
        changed
    }

    fn select_everything(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer) {
        let _ = self.select(tree, layout, renderer, (0, START), (usize::MAX, END));
    }

    /// Drops the selection and says whether there was one.
    fn clear(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer) -> bool {
        let state = tree.state.downcast_mut::<AreaState>();
        state.active = false;
        state.dragging = false;
        state.last_pull = None;
        self.select(tree, layout, renderer, (0, START), (0, START))
    }

    fn select_word(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        at: Position,
    ) {
        self.visit(tree, layout, renderer, None, &mut |index, _, body| {
            body.selection = (index == at.0).then(|| body.word(at.1)).flatten();
        });
    }

    fn selected(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer) -> String {
        let mut parts = Vec::new();
        self.visit(tree, layout, renderer, None, &mut |_, _, body| {
            parts.extend(body.selected());
        });
        parts.join("\n")
    }

    fn announce(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
    ) {
        if self.on_select.is_none() {
            return;
        }
        let text = self.selected(tree, layout, renderer);
        if text.is_empty() {
            return;
        }
        if let Some(on_select) = &self.on_select {
            shell.publish(on_select(text));
        }
    }

    fn pressed(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(position) = cursor.position_over(layout.bounds()) else {
            if tree.state.downcast_ref::<AreaState>().active && self.clear(tree, layout, renderer) {
                shell.request_redraw();
            }
            return;
        };
        // A button inside the area took the press.
        if shell.is_event_captured() {
            return;
        }
        let Some(at) = self.locate(tree, layout, renderer) else {
            return;
        };
        let state = tree.state.downcast_mut::<AreaState>();
        let now = Instant::now();
        let count = click_count(state.last_click, position, now, self.double_click);
        state.last_click = Some(Click {
            position,
            at: now,
            count,
        });
        state.active = true;
        state.anchor = Some(at);
        state.dragging = count == 1;
        state.last_pull = None;

        match count {
            1 => {
                let _ = self.select(tree, layout, renderer, at, at);
            }
            2 => self.select_word(tree, layout, renderer, at),
            _ => {
                let _ = self.select(tree, layout, renderer, (at.0, START), (at.0, END));
            }
        }
        if count > 1 {
            self.announce(tree, layout, renderer, shell);
        }
        shell.request_redraw();
    }

    /// Extends the selection to the pointer and scrolls a scrollable the
    /// pointer has been dragged past the edge of. `now` is set on a frame,
    /// which is when the scrolling happens.
    fn dragged(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        now: Option<Instant>,
    ) {
        let anchor = tree.state.downcast_ref::<AreaState>().anchor;
        let focus = self.locate(tree, layout, renderer);
        if let (Some(anchor), Some(focus)) = (anchor, focus)
            && self.select(tree, layout, renderer, anchor, focus)
        {
            shell.request_redraw();
        }

        let Some(pointer) = cursor.land().position() else {
            return;
        };
        let state = tree.state.downcast_mut::<AreaState>();
        let elapsed = match (now, state.last_pull) {
            (Some(now), Some(last)) => now.saturating_duration_since(last).min(MAX_FRAME),
            _ => Duration::ZERO,
        };
        let pull = Pull { pointer, elapsed };
        let pulling = self.visit(tree, layout, renderer, Some(pull), &mut |_, _, _| {});
        let state = tree.state.downcast_mut::<AreaState>();
        if !pulling {
            state.last_pull = None;
            return;
        }
        if now.is_some() || state.last_pull.is_none() {
            state.last_pull = now.or_else(|| Some(Instant::now()));
        }
        // The next frame scrolls further and moves the selection with it.
        shell.request_redraw();
    }

    fn key_pressed(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        action: Option<Action>,
        elsewhere: bool,
    ) {
        // A focused field that used the key keeps it.
        if shell.is_event_captured() {
            return;
        }
        match action {
            Some(Action::Copy) => {
                let text = self.selected(tree, layout, renderer);
                if text.is_empty() {
                    return;
                }
                if let Some(on_copy) = &self.on_copy {
                    shell.publish(on_copy(text));
                    shell.capture_event();
                }
            }
            Some(Action::SelectAll) => {
                if !tree.state.downcast_ref::<AreaState>().active {
                    return;
                }
                self.select_everything(tree, layout, renderer);
                self.announce(tree, layout, renderer, shell);
                shell.capture_event();
                shell.request_redraw();
            }
            Some(Action::Clear) => {
                if !tree.state.downcast_ref::<AreaState>().active {
                    return;
                }
                // With nothing selected the key is left for whatever is
                // around the area, such as a dialog.
                if self.clear(tree, layout, renderer) {
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            // The key belongs to something else, so the selection steps
            // aside.
            None if elsewhere
                && tree.state.downcast_ref::<AreaState>().active
                && self.clear(tree, layout, renderer) =>
            {
                shell.request_redraw();
            }
            None => {}
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for SelectionArea<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<AreaState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(AreaState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
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

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if self.id.is_some() {
            let mut request = Request::default();
            operation.custom(self.id.as_ref(), layout.bounds(), &mut request);
            if request.select_all {
                tree.state.downcast_mut::<AreaState>().active = true;
                self.select_everything(tree, layout, renderer);
            }
            if request.read {
                let mut reply = Reply(self.selected(tree, layout, renderer));
                operation.custom(self.id.as_ref(), layout.bounds(), &mut reply);
            }
        }
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
        let dragging = tree.state.downcast_ref::<AreaState>().dragging;
        let frame = match event {
            Event::Window(window::Event::RedrawRequested(now)) => Some(*now),
            _ => None,
        };
        let left_pressed = matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        );
        let moved = matches!(event, Event::Mouse(mouse::Event::CursorMoved { .. }));
        let dragged = dragging && (moved || frame.is_some());
        if left_pressed || dragged {
            // Only texts this event reaches may say where the pointer is.
            self.visit(tree, layout, renderer, None, &mut |_, _, body| {
                body.pointer = None;
            });
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                self.pressed(tree, layout, cursor, renderer, shell);
            }
            _ if dragged => self.dragged(tree, layout, cursor, renderer, shell, frame),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if dragging => {
                let state = tree.state.downcast_mut::<AreaState>();
                state.dragging = false;
                state.last_pull = None;
                self.announce(tree, layout, renderer, shell);
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                text,
                ..
            }) => {
                // Typing, Tab and an app's own chords are meant for something
                // other than the text being read.
                let elsewhere = text.is_some()
                    || modifiers.command()
                    || *key == keyboard::Key::Named(Named::Tab);
                let action = self.keymap.resolve(key, *modifiers);
                self.key_pressed(tree, layout, renderer, shell, action, elsewhere);
            }
            // A release that never arrives must not leave a drag running.
            Event::Mouse(mouse::Event::CursorLeft) if dragging => {
                let state = tree.state.downcast_mut::<AreaState>();
                state.dragging = false;
                state.last_pull = None;
            }
            _ => {}
        }
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

impl<'a, Message: 'a> From<SelectionArea<'a, Message>> for Element<'a, Message> {
    fn from(area: SelectionArea<'a, Message>) -> Self {
        Element::new(area)
    }
}

#[cfg(test)]
mod tests {
    use iced::keyboard::Key;

    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn word_bounds_find_the_word_under_the_caret() {
        assert_eq!(word_bounds("alpha beta", 7), Some((6, 10)));
        assert_eq!(word_bounds("alpha beta", 0), Some((0, 5)));
        assert_eq!(word_bounds("snake_case here", 3), Some((0, 10)));
    }

    #[test]
    fn word_bounds_take_one_character_between_words() {
        assert_eq!(word_bounds("alpha beta", 5), Some((5, 6)));
        assert_eq!(word_bounds("a, b", 1), Some((1, 2)));
    }

    #[test]
    fn word_bounds_respect_characters_of_several_bytes() {
        assert_eq!(word_bounds("caf\u{e9} noir", 4), Some((0, 5)));
        assert_eq!(word_bounds("caf\u{e9} noir", 100), Some((6, 10)));
        assert_eq!(word_bounds("", 0), None);
    }

    #[test]
    fn presses_in_a_row_count_up_to_three_then_start_again() {
        let interval = Duration::from_millis(400);
        let start = Instant::now();
        let at = Point::new(10.0, 10.0);
        let click = |count, after: u64| {
            Some(Click {
                position: at,
                at: start + Duration::from_millis(after),
                count,
            })
        };
        let soon = start + Duration::from_millis(100);

        assert_eq!(click_count(None, at, start, interval), 1);
        assert_eq!(click_count(click(1, 0), at, soon, interval), 2);
        assert_eq!(click_count(click(2, 0), at, soon, interval), 3);
        assert_eq!(click_count(click(3, 0), at, soon, interval), 1);
    }

    #[test]
    fn a_late_or_distant_press_is_a_single_click() {
        let interval = Duration::from_millis(400);
        let start = Instant::now();
        let at = Point::new(10.0, 10.0);
        let previous = Some(Click {
            position: at,
            at: start,
            count: 1,
        });

        let late = start + Duration::from_millis(401);
        assert_eq!(click_count(previous, at, late, interval), 1);
        let on_time = start + Duration::from_millis(400);
        assert_eq!(click_count(previous, at, on_time, interval), 2);
        let far = Point::new(10.0 + CLICK_SLOP + 1.0, 10.0);
        assert_eq!(click_count(previous, far, start, interval), 1);
    }

    #[test]
    fn a_point_is_clamped_to_the_nearest_point_of_a_rectangle() {
        let bounds = Rectangle::new(Point::new(10.0, 20.0), Size::new(100.0, 50.0));
        assert_eq!(
            clamp(Point::new(50.0, 40.0), bounds),
            Point::new(50.0, 40.0)
        );
        assert_eq!(clamp(Point::new(0.0, 0.0), bounds), Point::new(10.0, 20.0));
        assert_eq!(
            clamp(Point::new(500.0, 500.0), bounds),
            Point::new(110.0, 70.0)
        );
    }

    /// Feeds `pointer`, given in window coordinates, to texts laid out at
    /// `rows`, and returns the text found and the caret column asked for.
    fn search(rows: &[Rectangle], pointer: Point) -> Option<Position> {
        let mut search = Search::default();
        for (index, row) in rows.iter().enumerate() {
            let local = Point::new(pointer.x - row.x, pointer.y - row.y);
            search.feed(index, row.size(), local, || (0, local.x as usize));
        }
        search.result()
    }

    #[test]
    fn the_pointer_finds_the_text_it_is_over_beside_or_above() {
        let row = |y: f32, width: f32| Rectangle::new(Point::new(0.0, y), Size::new(width, 20.0));
        let rows = [row(0.0, 100.0), row(30.0, 40.0), row(60.0, 100.0)];

        assert_eq!(search(&rows, Point::new(50.0, 10.0)), Some((0, (0, 50))));
        // Beside a short line: that line, not the one below.
        assert_eq!(search(&rows, Point::new(80.0, 40.0)), Some((1, (0, 80))));
        // In the gap between lines: the start of the next one.
        assert_eq!(search(&rows, Point::new(50.0, 55.0)), Some((2, START)));
        // Below everything: nothing, which the area reads as the end.
        assert_eq!(search(&rows, Point::new(50.0, 500.0)), None);
    }

    #[test]
    fn cells_side_by_side_are_read_in_order() {
        let cell = |x: f32, y: f32| Rectangle::new(Point::new(x, y), Size::new(100.0, 20.0));
        let cells = [
            cell(0.0, 0.0),
            cell(120.0, 0.0),
            cell(0.0, 30.0),
            cell(120.0, 30.0),
        ];

        assert_eq!(search(&cells, Point::new(130.0, 10.0)), Some((1, (0, 10))));
        assert_eq!(search(&cells, Point::new(10.0, 40.0)), Some((2, (0, 10))));
        // Between two cells: the start of the one on the right.
        assert_eq!(search(&cells, Point::new(110.0, 40.0)), Some((3, START)));
        // Right of the last cell in a row: the end of that cell.
        assert_eq!(search(&cells, Point::new(300.0, 10.0)), Some((1, (0, 180))));
    }

    #[test]
    fn the_selection_is_a_faded_primary_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let selection = style(&tokens).selection;
            assert_eq!(selection, fade(tokens.primary, 0.25));
            assert!(selection.a < 1.0, "the text must show through");
        }
    }

    #[test]
    fn a_drag_scrolls_faster_the_further_past_the_edge_it_is() {
        assert_eq!(scroll_speed(0.0), SCROLL_SPEED);
        assert_eq!(scroll_speed(-20.0), SCROLL_SPEED);
        assert!(scroll_speed(10.0) > scroll_speed(1.0));
        assert_eq!(scroll_speed(10_000.0), MAX_SCROLL_SPEED);
    }

    #[test]
    fn default_keymap_and_overrides() {
        let c = Key::Character("c".into());
        let a = Key::Character("a".into());
        let escape = Key::Named(Named::Escape);
        let command = keyboard::Modifiers::COMMAND;
        let keymap = default_keymap();
        assert_eq!(keymap.resolve(&c, command), Some(Action::Copy));
        assert_eq!(keymap.resolve(&a, command), Some(Action::SelectAll));
        assert_eq!(
            keymap.resolve(&escape, keyboard::Modifiers::empty()),
            Some(Action::Clear)
        );
        assert_eq!(keymap.resolve(&c, keyboard::Modifiers::empty()), None);

        let custom = keymap
            .unbind(&Chord::character('a').command())
            .bind(Chord::character('e').command(), Action::SelectAll);
        assert_eq!(custom.resolve(&a, command), None);
        assert_eq!(
            custom.resolve(&Key::Character("e".into()), command),
            Some(Action::SelectAll)
        );
    }

    #[test]
    fn every_action_is_documented_and_bound() {
        use keys::Action as _;
        let keymap = Action::defaults();
        for action in Action::ALL {
            assert!(!action.name().is_empty());
            assert!(action.description().ends_with('.'));
            assert!(!keymap.chords(action).is_empty(), "{action:?} has no chord");
        }
    }

    #[test]
    fn a_task_reads_only_the_area_with_its_id() {
        let mut command = Command {
            target: Id::new("notes"),
            select_all: true,
            text: None,
        };
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(10.0, 10.0));

        let mut other = Request::default();
        command.custom(Some(&Id::new("other")), bounds, &mut other);
        assert!(!other.read && !other.select_all);

        let mut request = Request::default();
        command.custom(Some(&Id::new("notes")), bounds, &mut request);
        assert!(request.read && request.select_all);

        assert!(matches!(command.finish(), Outcome::None));
        command.custom(
            Some(&Id::new("notes")),
            bounds,
            &mut Reply("text".to_owned()),
        );
        assert!(matches!(command.finish(), Outcome::Some(text) if text == "text"));
    }
}
