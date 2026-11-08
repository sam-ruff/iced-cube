//! The editor widget: draws the laid out blocks and turns input into
//! [`Action`]s for the app.

use iced::advanced::clipboard::Kind;
use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse::click::{self, Click};
use iced::advanced::text::{self, Paragraph as _, Renderer as _, Text};
use iced::advanced::widget::{Operation, Tree, Widget, operation, tree};
use iced::advanced::{Clipboard, InputMethod, Shell, input_method, renderer};
use iced::alignment;
use iced::keyboard::{self, Modifiers};
use iced::time::{Duration, Instant};
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::widget::text_editor::{self, Binding, KeyPress};
use iced::widget::{Id, text_input};
use iced::{
    Background, Border, Event, Length, Padding, Pixels, Point, Rectangle, Renderer, Size, Theme,
    mouse, touch, window,
};

use super::content::{Action, Content, Edit, Motion};
use super::document::{BlockKind, Document, Position};
use super::layout::{LINE_HEIGHT, Paragraph, Stack};
use super::{RichTextEditor, Shortcut, Shortcuts, shortcut_action};
use crate::editing;
use crate::keys::Keymap;
use crate::theme::{self, Tokens, radius};

const BLINK: u128 = 500;
/// How far a finger moves before a touch scrolls rather than taps.
const TOUCH_SLOP: f32 = 10.0;
const SCROLLBAR_WIDTH: f32 = 4.0;

#[derive(Debug, Clone, Copy)]
struct Focus {
    updated_at: Instant,
    now: Instant,
    window: bool,
}

impl Focus {
    fn now() -> Self {
        let now = Instant::now();
        Self {
            updated_at: now,
            now,
            window: true,
        }
    }

    fn caret_visible(&self) -> bool {
        self.window && ((self.now - self.updated_at).as_millis() / BLINK).is_multiple_of(2)
    }
}

#[derive(Debug, Clone, Copy)]
struct Touch {
    finger: touch::Finger,
    start: Point,
    last: Point,
    scrolling: bool,
}

/// What the editor keeps between frames.
#[derive(Debug, Default)]
pub(super) struct State {
    stack: Stack,
    focus: Option<Focus>,
    scroll: f32,
    /// The revision and selection the scroll last followed.
    seen: Option<(u64, super::Selection)>,
    preedit: Option<input_method::Preedit>,
    dragging: bool,
    last_click: Option<Click>,
    touch: Option<Touch>,
    /// The last copy, so pasting it back keeps its formatting.
    copied: Option<(String, Document)>,
    /// Where up and down keep the caret, across short lines.
    preferred_x: Option<f32>,
    modifiers: Modifiers,
    last_status: Option<text_input::Status>,
    /// The height of the text area.
    viewport: f32,
    /// The placeholder laid out at a width.
    placeholder: Option<(String, f32, Paragraph)>,
}

impl State {
    fn max_scroll(&self) -> f32 {
        (self.stack.height() - self.viewport).max(0.0)
    }

    fn scroll_by(&mut self, delta: f32) -> bool {
        let before = self.scroll;
        self.scroll = (self.scroll - delta).clamp(0.0, self.max_scroll());
        (self.scroll - before).abs() > f32::EPSILON
    }
}

impl operation::Focusable for State {
    fn is_focused(&self) -> bool {
        self.focus.is_some()
    }

    fn focus(&mut self) {
        self.focus = Some(Focus::now());
    }

    fn unfocus(&mut self) {
        self.focus = None;
        self.preedit = None;
    }
}

pub(super) struct Editor<'a, Message> {
    content: &'a Content,
    placeholder: Option<String>,
    id: Option<Id>,
    on_action: Option<Box<dyn Fn(Action) -> Message + 'a>>,
    on_link: Option<Box<dyn Fn(String) -> Message + 'a>>,
    invalid: bool,
    width: Length,
    height: Length,
    min_height: f32,
    max_height: f32,
    text_size: f32,
    padding: Padding,
    shortcuts: Shortcuts,
    keymap: Keymap<Shortcut>,
}

impl<'a, Message> Editor<'a, Message> {
    pub fn new(editor: RichTextEditor<'a, Message>) -> Self {
        let RichTextEditor {
            content,
            placeholder,
            id,
            on_action,
            on_link,
            invalid,
            width,
            height,
            min_height,
            max_height,
            text_size,
            padding,
            shortcuts,
            keymap,
        } = editor;
        Self {
            content,
            placeholder,
            id,
            on_action,
            on_link,
            invalid,
            width,
            height,
            min_height,
            max_height,
            text_size,
            padding,
            shortcuts,
            keymap,
        }
    }

    fn status(
        &self,
        state: &State,
        cursor: mouse::Cursor,
        bounds: Rectangle,
    ) -> text_input::Status {
        let hovered = cursor.is_over(bounds);
        if self.on_action.is_none() {
            text_input::Status::Disabled
        } else if state.focus.is_some() {
            text_input::Status::Focused {
                is_hovered: hovered,
            }
        } else if hovered {
            text_input::Status::Hovered
        } else {
            text_input::Status::Active
        }
    }

    fn text_bounds(&self, bounds: Rectangle) -> Rectangle {
        bounds.shrink(self.padding)
    }

    /// A window point in content coordinates.
    fn to_content(&self, state: &State, bounds: Rectangle, point: Point) -> Point {
        let text = self.text_bounds(bounds);
        Point::new(point.x - text.x, point.y - text.y + state.scroll)
    }

    fn line_height(&self) -> f32 {
        (self.text_size * LINE_HEIGHT).round()
    }

    /// Keeps the caret in view after the content or selection changed.
    fn follow_caret(&self, state: &mut State) {
        let key = (self.content.revision(), self.content.selection());
        if state.seen.is_some_and(|seen| seen != key) {
            if let Some((_, top, height)) = state.stack.caret(self.content.selection().head) {
                if top < state.scroll {
                    state.scroll = top;
                } else if top + height > state.scroll + state.viewport {
                    state.scroll = top + height - state.viewport;
                }
            }
            if let Some(focus) = &mut state.focus {
                focus.updated_at = focus.now;
            }
        }
        state.seen = Some(key);
        state.scroll = state.scroll.clamp(0.0, state.max_scroll());
    }

    fn input_method<'b>(&self, state: &'b State, bounds: Rectangle) -> InputMethod<&'b str> {
        let Some(Focus { window: true, .. }) = state.focus else {
            return InputMethod::Disabled;
        };
        let text = self.text_bounds(bounds);
        let (x, top, height) = state.stack.caret(self.content.selection().head).unwrap_or((
            0.0,
            0.0,
            self.line_height(),
        ));
        InputMethod::Enabled {
            cursor: Rectangle::new(
                Point::new(text.x + x, text.y + top - state.scroll),
                Size::new(1.0, height),
            ),
            purpose: input_method::Purpose::Normal,
            preedit: state.preedit.as_ref().map(input_method::Preedit::as_ref),
        }
    }

    fn publish(&self, shell: &mut Shell<'_, Message>, action: Action) {
        if let Some(on_action) = &self.on_action {
            shell.publish(on_action(action));
        }
    }

    /// Turns a motion from iced's key bindings into one the content
    /// understands, working out line moves from the layout.
    fn motion(&self, state: &mut State, motion: text_editor::Motion) -> Motion {
        use text_editor::Motion as Key;
        let head = self.content.selection().head;
        let vertical = |state: &mut State, lines: f32| {
            let x = state
                .preferred_x
                .or_else(|| state.stack.caret(head).map(|(x, _, _)| x))
                .unwrap_or(0.0);
            state.preferred_x = Some(x);
            Motion::To(state.stack.vertical(head, x, lines))
        };
        let page = (state.viewport / self.line_height()).floor().max(1.0);
        match motion {
            Key::Left => Motion::Left,
            Key::Right => Motion::Right,
            Key::WordLeft => Motion::WordLeft,
            Key::WordRight => Motion::WordRight,
            Key::DocumentStart => Motion::DocumentStart,
            Key::DocumentEnd => Motion::DocumentEnd,
            Key::Up => vertical(state, -1.0),
            Key::Down => vertical(state, 1.0),
            Key::PageUp => vertical(state, -page),
            Key::PageDown => vertical(state, page),
            Key::Home | Key::End => {
                let Some(laid) = state.stack.blocks.get(head.block) else {
                    return if motion == Key::Home {
                        Motion::BlockStart
                    } else {
                        Motion::BlockEnd
                    };
                };
                let (start, end) = laid.line_bounds(head.offset);
                let offset = if motion == Key::Home { start } else { end };
                Motion::To(Position::new(head.block, offset))
            }
        }
    }

    fn shortcut(&self, state: &mut State, shortcut: Shortcut, shell: &mut Shell<'_, Message>) {
        match shortcut_action(shortcut) {
            Some(action) => self.publish(shell, action),
            None => {
                operation::Focusable::unfocus(state);
                shell.request_redraw();
            }
        }
    }

    fn key(
        &self,
        state: &mut State,
        event: &Event,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            physical_key,
            modifiers,
            ..
        }) = event
        else {
            return;
        };
        if let Some(shortcut) = editing::resolve(&self.keymap, key, *physical_key, *modifiers) {
            // A shortcut that is off is left to the app: the editor does
            // not act on its keys either.
            if self.shortcuts.contains(shortcut) {
                state.preferred_x = None;
                self.shortcut(state, shortcut, shell);
                shell.capture_event();
            }
            return;
        }
        let stripped = editing::without_shortcut_text(event);
        let Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modified_key,
            physical_key,
            modifiers,
            text,
            ..
        }) = stripped.as_ref().unwrap_or(event)
        else {
            return;
        };
        let Some(binding) = Binding::<()>::from_key_press(KeyPress {
            key: key.clone(),
            modified_key: modified_key.clone(),
            physical_key: *physical_key,
            modifiers: *modifiers,
            text: text.clone(),
            status: text_editor::Status::Focused { is_hovered: false },
        }) else {
            return;
        };
        if !matches!(
            binding,
            Binding::Move(
                text_editor::Motion::Up
                    | text_editor::Motion::Down
                    | text_editor::Motion::PageUp
                    | text_editor::Motion::PageDown
            ) | Binding::Select(
                text_editor::Motion::Up
                    | text_editor::Motion::Down
                    | text_editor::Motion::PageUp
                    | text_editor::Motion::PageDown
            )
        ) {
            state.preferred_x = None;
        }
        let selection = self.content.selection();
        let action = match binding {
            Binding::Copy | Binding::Cut => {
                if selection.is_collapsed() {
                    shell.capture_event();
                    return;
                }
                let fragment = self.content.selected();
                let copied = fragment.text();
                clipboard.write(Kind::Standard, copied.clone());
                state.copied = Some((copied, fragment));
                if matches!(binding, Binding::Copy) {
                    shell.capture_event();
                    return;
                }
                Action::Edit(Edit::Delete)
            }
            Binding::Paste => {
                let Some(pasted) = clipboard.read(Kind::Standard) else {
                    shell.capture_event();
                    return;
                };
                let pasted = pasted.replace("\r\n", "\n");
                match &state.copied {
                    Some((copied, fragment)) if *copied == pasted => {
                        Action::Edit(Edit::PasteFormatted(fragment.clone()))
                    }
                    _ => Action::Edit(Edit::Paste(pasted)),
                }
            }
            Binding::SelectAll => Action::SelectAll,
            Binding::Move(motion) => Action::Move(self.motion(state, motion)),
            Binding::Select(motion) => Action::Select(self.motion(state, motion)),
            Binding::Enter => Action::Edit(Edit::Enter),
            Binding::Backspace => Action::Edit(Edit::Backspace),
            Binding::Delete => Action::Edit(Edit::Delete),
            Binding::Insert(_) => {
                let typed: String = text
                    .as_deref()
                    .unwrap_or_default()
                    .chars()
                    .filter(|c| !c.is_control())
                    .collect();
                if typed.is_empty() {
                    return;
                }
                Action::Edit(Edit::Insert(typed))
            }
            // Escape with its shortcut off, and bindings the defaults never produce.
            _ => return,
        };
        self.publish(shell, action);
        shell.capture_event();
    }

    fn press(
        &self,
        state: &mut State,
        bounds: Rectangle,
        position: Point,
        shell: &mut Shell<'_, Message>,
    ) {
        let point = self.to_content(state, bounds, position);
        let at = state.stack.hit(point);
        if state.modifiers.command()
            && let Some(on_link) = &self.on_link
            && let Some((_, _, url)) = self.content.document().link_at(at)
        {
            shell.publish(on_link(url.to_string()));
            shell.capture_event();
            return;
        }
        let click = Click::new(position, mouse::Button::Left, state.last_click);
        let action = match click.kind() {
            click::Kind::Single if state.modifiers.shift() => Action::Select(Motion::To(at)),
            click::Kind::Single => Action::Move(Motion::To(at)),
            click::Kind::Double => Action::SelectWord(at),
            click::Kind::Triple => Action::SelectBlock(at),
        };
        if state.focus.is_none() {
            state.focus = Some(Focus::now());
        }
        state.last_click = Some(click);
        state.dragging = matches!(click.kind(), click::Kind::Single);
        state.preferred_x = None;
        self.publish(shell, action);
        shell.capture_event();
    }

    fn touch(
        &self,
        state: &mut State,
        bounds: Rectangle,
        event: &touch::Event,
        shell: &mut Shell<'_, Message>,
    ) {
        match *event {
            touch::Event::FingerPressed { id, position } => {
                if !bounds.contains(position) {
                    operation::Focusable::unfocus(state);
                    return;
                }
                state.touch = Some(Touch {
                    finger: id,
                    start: position,
                    last: position,
                    scrolling: false,
                });
                shell.capture_event();
            }
            touch::Event::FingerMoved { id, position } => {
                let Some(touch) = state.touch.as_mut().filter(|touch| touch.finger == id) else {
                    return;
                };
                if !touch.scrolling && touch.start.distance(position) > TOUCH_SLOP {
                    touch.scrolling = true;
                }
                if !touch.scrolling {
                    return;
                }
                let delta = position.y - touch.last.y;
                touch.last = position;
                if state.scroll_by(delta) {
                    shell.request_redraw();
                }
                shell.capture_event();
            }
            touch::Event::FingerLifted { id, position } => {
                let Some(touch) = state.touch.take().filter(|touch| touch.finger == id) else {
                    return;
                };
                if !touch.scrolling {
                    let at = state.stack.hit(self.to_content(state, bounds, position));
                    if state.focus.is_none() {
                        state.focus = Some(Focus::now());
                    }
                    self.publish(shell, Action::Move(Motion::To(at)));
                }
                shell.capture_event();
            }
            touch::Event::FingerLost { id, .. } => {
                if state.touch.is_some_and(|touch| touch.finger == id) {
                    state.touch = None;
                }
            }
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Editor<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        let limits = limits.width(self.width).height(self.height);
        let available = limits.max().width;
        let width = if available.is_finite() {
            available
        } else {
            limits.min().width.max(320.0)
        };
        let text_width = (width - self.padding.x()).max(1.0);
        state.stack.update(
            self.content.document(),
            self.content.revision(),
            text_width,
            self.text_size,
        );
        let natural = (state.stack.height() + self.padding.y())
            .max(self.min_height)
            .min(self.max_height.max(self.min_height));
        let size = limits.resolve(self.width, self.height, Size::new(width, natural));
        state.viewport = (size.height - self.padding.y()).max(0.0);
        state.placeholder =
            self.placeholder
                .as_ref()
                .map(|placeholder| match state.placeholder.take() {
                    Some(laid) if laid.0 == *placeholder && laid.1 == text_width => laid,
                    _ => (
                        placeholder.clone(),
                        text_width,
                        Paragraph::with_text(Text {
                            content: placeholder.as_str(),
                            bounds: Size::new(text_width, f32::INFINITY),
                            size: Pixels(self.text_size),
                            line_height: LineHeight::Relative(LINE_HEIGHT),
                            font: theme::font(),
                            align_x: text::Alignment::Left,
                            align_y: alignment::Vertical::Top,
                            shaping: Shaping::Advanced,
                            wrapping: Wrapping::WordOrGlyph,
                        }),
                    ),
                });
        self.follow_caret(state);
        layout::Node::new(size)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let status = self.status(state, cursor, bounds);
        let look = super::style(&Tokens::of(theme), status, self.invalid);
        <Renderer as iced::advanced::Renderer>::fill_quad(
            renderer,
            renderer::Quad {
                bounds,
                border: look.field.border,
                ..renderer::Quad::default()
            },
            look.field.background,
        );
        let text_bounds = self.text_bounds(bounds);
        let Some(clip) = text_bounds.intersection(viewport) else {
            return;
        };
        let origin = Point::new(text_bounds.x, text_bounds.y - state.scroll);
        let selection = self.content.selection();
        // Kept while a toolbar has focus, but quieter, so the editor does
        // not look active.
        let selection_colour = if state.focus.is_some() {
            look.field.selection
        } else {
            theme::fade(look.field.selection, 0.5)
        };
        let composing = state
            .preedit
            .as_ref()
            .is_some_and(|preedit| !preedit.content.is_empty());
        let quad =
            |renderer: &mut Renderer, [x, y, width, height]: [f32; 4], offset: Point, colour| {
                <Renderer as iced::advanced::Renderer>::fill_quad(
                    renderer,
                    renderer::Quad {
                        bounds: Rectangle::new(
                            Point::new(offset.x + x, offset.y + y),
                            Size::new(width, height),
                        ),
                        ..renderer::Quad::default()
                    },
                    Background::Color(colour),
                );
            };

        <Renderer as iced::advanced::Renderer>::with_layer(renderer, clip, |renderer| {
            let (start, end) = (selection.start(), selection.end());
            for (index, laid) in state.stack.blocks.iter().enumerate() {
                let top = origin.y + laid.y;
                if top > clip.y + clip.height {
                    break;
                }
                if top + laid.height < clip.y {
                    continue;
                }
                let at = Point::new(origin.x + laid.indent, top);
                if !selection.is_collapsed() && (start.block..=end.block).contains(&index) {
                    let from = if index == start.block {
                        start.offset
                    } else {
                        0
                    };
                    let to = if index == end.block {
                        end.offset
                    } else {
                        laid.len()
                    };
                    for rect in laid.selection(from, to, index < end.block) {
                        quad(renderer, rect, at, selection_colour);
                    }
                }
                let kind = laid.block.kind();
                let colour = if kind == BlockKind::Quote {
                    look.quote_text
                } else {
                    look.field.value
                };
                if kind == BlockKind::Quote {
                    quad(
                        renderer,
                        [1.0, 0.0, 2.0, laid.height],
                        Point::new(origin.x, top),
                        look.quote,
                    );
                }
                renderer.fill_paragraph(&laid.paragraph, at, colour, clip);
                if let Some(links) = &laid.links {
                    renderer.fill_paragraph(links, at, look.link, clip);
                }
                if let Some(marker) = &laid.marker {
                    renderer.fill_paragraph(
                        marker,
                        Point::new(origin.x, top + laid.marker_y),
                        colour,
                        clip,
                    );
                }
                for (rect, link) in laid.lines() {
                    quad(renderer, rect, at, if link { look.link } else { colour });
                }
            }

            if self.content.is_empty()
                && !composing
                && let Some((_, _, placeholder)) = &state.placeholder
            {
                renderer.fill_paragraph(placeholder, origin, look.field.placeholder, clip);
            }

            let caret = state
                .focus
                .filter(|focus| focus.caret_visible() && selection.is_collapsed() && !composing)
                .and_then(|_| state.stack.caret(selection.head));
            if let Some((x, y, height)) = caret {
                quad(renderer, [x, y, 1.0, height], origin, look.caret);
            }
        });

        let total = state.stack.height();
        if total > state.viewport + 0.5 && state.viewport > 0.0 {
            let track = text_bounds.height;
            let thumb = (track * state.viewport / total).max(16.0).min(track);
            let y = text_bounds.y + (track - thumb) * state.scroll / state.max_scroll().max(1.0);
            <Renderer as iced::advanced::Renderer>::fill_quad(
                renderer,
                renderer::Quad {
                    bounds: Rectangle::new(
                        Point::new(bounds.x + bounds.width - SCROLLBAR_WIDTH - 2.0, y),
                        Size::new(SCROLLBAR_WIDTH, thumb),
                    ),
                    border: Border {
                        radius: radius::FULL.into(),
                        ..Border::default()
                    },
                    ..renderer::Quad::default()
                },
                Background::Color(look.scrollbar),
            );
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        operation.focusable(self.id.as_ref(), layout.bounds(), state);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        if self.on_action.is_none() {
            state.focus = None;
            return;
        }
        let is_redraw = matches!(event, Event::Window(window::Event::RedrawRequested(_)));

        match event {
            Event::Window(window::Event::Unfocused) => {
                if let Some(focus) = &mut state.focus {
                    focus.window = false;
                }
            }
            Event::Window(window::Event::Focused) => {
                if let Some(focus) = &mut state.focus {
                    focus.window = true;
                    focus.updated_at = focus.now;
                    shell.request_redraw();
                }
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                if let Some(focus) = &mut state.focus
                    && focus.window
                {
                    focus.now = *now;
                    let wait = BLINK - (focus.now - focus.updated_at).as_millis() % BLINK;
                    shell.request_redraw_at(focus.now + Duration::from_millis(wait as u64));
                }
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
            }
            Event::Keyboard(keyboard::Event::KeyPressed { .. })
                if state.focus.is_some() && !shell.is_event_captured() =>
            {
                if let Some(focus) = &mut state.focus {
                    focus.updated_at = focus.now;
                }
                self.key(state, event, clipboard, shell);
            }
            Event::InputMethod(event) if state.focus.is_some() => match event {
                input_method::Event::Opened => {
                    state.preedit = Some(input_method::Preedit::new());
                    shell.request_redraw();
                }
                input_method::Event::Preedit(content, selection) => {
                    state.preedit = Some(input_method::Preedit {
                        content: content.clone(),
                        selection: selection.clone(),
                        text_size: Some(Pixels(self.text_size)),
                    });
                    shell.request_redraw();
                }
                input_method::Event::Commit(text) => {
                    state.preedit = None;
                    self.publish(shell, Action::Edit(Edit::Insert(text.clone())));
                    shell.capture_event();
                }
                input_method::Event::Closed => {
                    state.preedit = None;
                    shell.request_redraw();
                }
            },
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                match cursor.position_over(bounds) {
                    Some(position) if !shell.is_event_captured() => {
                        self.press(state, bounds, position, shell);
                    }
                    Some(_) => {}
                    None => operation::Focusable::unfocus(state),
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if state.dragging => {
                let at = state.stack.hit(self.to_content(state, bounds, *position));
                if at != self.content.selection().head {
                    self.publish(shell, Action::Select(Motion::To(at)));
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragging = false;
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                let pixels = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * self.line_height(),
                    mouse::ScrollDelta::Pixels { y, .. } => *y,
                };
                if state.scroll_by(pixels) {
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Touch(event) => self.touch(state, bounds, event, shell),
            _ => {}
        }

        let status = self.status(state, cursor, bounds);
        if is_redraw {
            state.last_status = Some(status);
            shell.request_input_method(&self.input_method(state, bounds));
        } else if state.last_status.is_some_and(|last| last != status) {
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
        let Some(position) = cursor.position_over(layout.bounds()) else {
            return mouse::Interaction::default();
        };
        if self.on_action.is_none() {
            return mouse::Interaction::NotAllowed;
        }
        let state = tree.state.downcast_ref::<State>();
        let over_link = self.on_link.is_some()
            && state.modifiers.command()
            && self
                .content
                .document()
                .link_at(
                    state
                        .stack
                        .hit(self.to_content(state, layout.bounds(), position)),
                )
                .is_some();
        if over_link {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::Text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_caret_blinks_every_half_second() {
        let start = Instant::now();
        let mut focus = Focus {
            updated_at: start,
            now: start,
            window: true,
        };
        assert!(focus.caret_visible());
        focus.now = start + Duration::from_millis(600);
        assert!(!focus.caret_visible());
        focus.now = start + Duration::from_millis(1100);
        assert!(focus.caret_visible());
        focus.window = false;
        assert!(!focus.caret_visible());
    }

    #[test]
    fn scrolling_stays_within_the_content() {
        let mut state = State {
            viewport: 100.0,
            ..State::default()
        };
        assert!(!state.scroll_by(-50.0));
        let document = Document::from_text(&"line\n".repeat(20));
        state.stack.update(&document, 0, 200.0, 14.0);
        assert!(state.scroll_by(-50.0));
        assert_eq!(state.scroll, 50.0);
        state.scroll_by(-1e6);
        assert_eq!(state.scroll, state.max_scroll());
        state.scroll_by(1e6);
        assert_eq!(state.scroll, 0.0);
    }

    #[test]
    fn unfocusing_drops_the_composition() {
        let mut state = State::default();
        operation::Focusable::focus(&mut state);
        state.preedit = Some(input_method::Preedit::new());
        assert!(operation::Focusable::is_focused(&state));
        operation::Focusable::unfocus(&mut state);
        assert!(state.preedit.is_none() && state.focus.is_none());
    }
}
