//! A modal window over a dimmed scrim, with a title, a description, body
//! content and a footer of actions.
//!
//! The app owns whether the dialog is open. [`dialog`] wraps the app's
//! content (the base) and, while open, draws a scrim over it that blocks
//! every click and key press from reaching the base.
//!
//! ```no_run
//! use iced::Element;
//! use iced::widget::center;
//! use iced_cube::button;
//! use iced_cube::overlay::dialog::dialog;
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Open,
//!     Close,
//! }
//!
//! struct App {
//!     open: bool,
//! }
//!
//! impl App {
//!     fn update(&mut self, message: Message) {
//!         self.open = matches!(message, Message::Open);
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         dialog(center(button("Open").on_press(Message::Open)))
//!             .open(self.open)
//!             .title("Share link")
//!             .description("Anyone with the link can view this document.")
//!             .action(button("Done").on_press(Message::Close))
//!             .on_dismiss(Message::Close)
//!             .into()
//!     }
//! }
//! ```
//!
//! The dialog handles its own keys through a [`Keymap`], so an app needs no
//! subscription for it: Escape dismisses, and Tab and Shift+Tab move focus
//! between the text fields inside the dialog without leaving it.
//!
//! While it is open the dialog captures every key press, after the content
//! inside it has had its turn, so app-wide shortcuts from
//! [`keys::subscription`] never reach the components underneath. Let
//! chosen chords through with [`Dialog::pass_through`].

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::keyboard::key::Named;
use iced::widget::{self, center, column, container, mouse_area, opaque, row, stack, text};
use iced::{
    Alignment, Background, Border, Color, Element, Event, Length, Padding, Rectangle, Renderer,
    Size as Bounds, Theme, Vector, keyboard, mouse,
};

use crate::keys::{self, Chord, Keymap};
use crate::overlay::anchored;
use crate::primitives::button::{self, Size as ButtonSize};
use crate::primitives::icon_button::icon_button;
use crate::theme::{Tokens, fade, radius, space, text_size};

/// The default widget id of the dialog surface, which scopes focus.
pub const DEFAULT_ID: widget::Id = widget::Id::new("iced-cube-dialog");

/// The widget id of the close button, for clicking it in tests.
pub const CLOSE_BUTTON_ID: widget::Id = widget::Id::new("iced-cube-dialog-close");

/// Maximum width of the dialog surface. It shrinks to fit smaller windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
}

impl Size {
    pub const ALL: [Size; 3] = [Size::Sm, Size::Md, Size::Lg];

    /// The maximum width in logical pixels.
    pub const fn width(self) -> f32 {
        match self {
            Size::Sm => 384.0,
            Size::Md => 512.0,
            Size::Lg => 640.0,
        }
    }
}

/// What a dialog keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Close,
    FocusNext,
    FocusPrevious,
}

/// The direction focus moves in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Next,
    Previous,
}

/// What an [`Action`] does to an open dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Emit the `on_dismiss` message.
    Dismiss,
    /// Move focus to another text field inside the dialog.
    Focus(Direction),
}

impl Action {
    /// The effect of this action on an open dialog, or `None` when Close
    /// has nothing to do because Escape does not dismiss the dialog.
    pub fn effect(self, dismiss_on_escape: bool) -> Option<Effect> {
        match self {
            Action::Close => dismiss_on_escape.then_some(Effect::Dismiss),
            Action::FocusNext => Some(Effect::Focus(Direction::Next)),
            Action::FocusPrevious => Some(Effect::Focus(Direction::Previous)),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[Action::Close, Action::FocusNext, Action::FocusPrevious];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Close => "Close",
            Action::FocusNext => "FocusNext",
            Action::FocusPrevious => "FocusPrevious",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Close => "Dismisses the dialog, unless Escape dismissal is turned off.",
            Action::FocusNext => {
                "Focuses the next text field in the dialog, wrapping from the last to the first."
            }
            Action::FocusPrevious => {
                "Focuses the previous text field in the dialog, wrapping from the first to the last."
            }
        }
    }
}

/// The default dialog shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `Escape` | [`Action::Close`] |
/// | `Tab` | [`Action::FocusNext`] |
/// | `Shift+Tab` | [`Action::FocusPrevious`] |
///
/// The dialog resolves these itself while it is open, after the content
/// inside it, so a menu or combobox in the dialog closes on Escape before
/// the dialog does. A focused text field takes the first Escape to lose
/// focus, so it takes a second Escape to dismiss the dialog.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::Escape), Action::Close)
        .bind(Chord::named(Named::Tab), Action::FocusNext)
        .bind(Chord::named(Named::Tab).shift(), Action::FocusPrevious)
}

/// A dialog builder. Convert it into an [`Element`] to render.
pub struct Dialog<'a, Message> {
    base: Element<'a, Message>,
    open: bool,
    title: Option<text::Fragment<'a>>,
    description: Option<text::Fragment<'a>>,
    body: Option<Element<'a, Message>>,
    actions: Vec<Element<'a, Message>>,
    on_dismiss: Option<Message>,
    dismiss_on_escape: bool,
    dismiss_on_scrim: bool,
    close_button: bool,
    width: f32,
    id: widget::Id,
    keymap: Keymap<Action>,
    pass_through: Vec<Chord>,
}

impl<Message> std::fmt::Debug for Dialog<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dialog")
            .field("open", &self.open)
            .field("title", &self.title)
            .field("description", &self.description)
            .field("actions", &self.actions.len())
            .field("dismiss_on_escape", &self.dismiss_on_escape)
            .field("dismiss_on_scrim", &self.dismiss_on_scrim)
            .field("close_button", &self.close_button)
            .field("width", &self.width)
            .field("id", &self.id)
            .field("pass_through", &self.pass_through)
            .finish_non_exhaustive()
    }
}

/// Wraps `base`, the content the dialog opens over. The dialog is closed
/// until [`open`](Dialog::open) is `true`.
pub fn dialog<'a, Message>(base: impl Into<Element<'a, Message>>) -> Dialog<'a, Message> {
    Dialog {
        base: base.into(),
        open: false,
        title: None,
        description: None,
        body: None,
        actions: Vec::new(),
        on_dismiss: None,
        dismiss_on_escape: true,
        dismiss_on_scrim: true,
        close_button: true,
        width: Size::default().width(),
        id: DEFAULT_ID,
        keymap: default_keymap(),
        pass_through: Vec::new(),
    }
}

impl<'a, Message> Dialog<'a, Message> {
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn title(mut self, title: impl text::IntoFragment<'a>) -> Self {
        self.title = Some(title.into_fragment());
        self
    }

    /// A line of muted text under the title.
    pub fn description(mut self, description: impl text::IntoFragment<'a>) -> Self {
        self.description = Some(description.into_fragment());
        self
    }

    /// Content between the header and the footer, such as a form.
    pub fn body(mut self, body: impl Into<Element<'a, Message>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Adds a button or other element to the footer, which lines them up
    /// on the right. Add the main action last.
    pub fn action(mut self, action: impl Into<Element<'a, Message>>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// The message emitted when Escape, a click on the scrim or the close
    /// button dismisses the dialog. Without it, only the footer actions
    /// can close the dialog and the close button renders disabled.
    pub fn on_dismiss(mut self, message: Message) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Whether Escape dismisses the dialog. Defaults to `true`.
    pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
        self.dismiss_on_escape = dismiss;
        self
    }

    /// Whether a click on the scrim dismisses the dialog. Defaults to
    /// `true`; turn it off for confirmations that need an explicit answer.
    pub fn dismiss_on_scrim(mut self, dismiss: bool) -> Self {
        self.dismiss_on_scrim = dismiss;
        self
    }

    /// Whether to show a close button in the top right corner. Defaults to
    /// `true`.
    pub fn close_button(mut self, show: bool) -> Self {
        self.close_button = show;
        self
    }

    /// Sets the maximum width from a [`Size`].
    pub fn size(mut self, size: Size) -> Self {
        self.width = size.width();
        self
    }

    /// Sets the maximum width in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the surface's widget id, which scopes focus. Give nested
    /// dialogs different ids.
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = id.into();
        self
    }

    /// Replaces the default shortcuts.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    /// Chords the open dialog lets through to the app, such as the shortcut
    /// that toggles it. Every other key press stops at the dialog.
    pub fn pass_through(mut self, chords: impl IntoIterator<Item = Chord>) -> Self {
        self.pass_through = chords.into_iter().collect();
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }
}

impl<'a, Message: Clone + 'a> From<Dialog<'a, Message>> for Element<'a, Message> {
    fn from(dialog: Dialog<'a, Message>) -> Self {
        let Dialog {
            base,
            open,
            title,
            description,
            body,
            actions,
            on_dismiss,
            dismiss_on_escape,
            dismiss_on_scrim,
            close_button,
            width,
            id,
            keymap,
            pass_through,
        } = dialog;

        let guarded = Element::new(Guard {
            content: base,
            blocked: open,
        });

        // The base is always the first child, so it keeps its widget state
        // when the dialog opens and closes.
        let mut layers = stack![guarded].width(Length::Fill).height(Length::Fill);

        if open {
            let surface = surface(
                title,
                description,
                body,
                actions,
                close_button.then(|| on_dismiss.clone()),
                width,
                id.clone(),
            );
            let scrim = center(opaque(surface))
                .padding(space::LG)
                .style(|theme| scrim_style(&Tokens::of(theme)));
            let layer: Element<'a, Message> = match on_dismiss.clone().filter(|_| dismiss_on_scrim)
            {
                Some(message) => opaque(mouse_area(scrim).on_press(message)),
                None => opaque(scrim),
            };
            layers = layers.push(layer);
        }

        Element::new(Scope {
            content: layers.into(),
            open,
            surface: id,
            keymap,
            pass_through,
            on_escape: on_dismiss.filter(|_| dismiss_on_escape),
        })
    }
}

fn surface<'a, Message: Clone + 'a>(
    title: Option<text::Fragment<'a>>,
    description: Option<text::Fragment<'a>>,
    body: Option<Element<'a, Message>>,
    actions: Vec<Element<'a, Message>>,
    close: Option<Option<Message>>,
    width: f32,
    id: widget::Id,
) -> Element<'a, Message> {
    let mut header = column![].spacing(space::XS);
    if let Some(title) = title {
        header = header.push(
            text(title)
                .size(text_size::LG)
                .font(crate::theme::semibold()),
        );
    }
    if let Some(description) = description {
        header = header.push(
            text(description)
                .size(text_size::SM)
                .style(|theme: &Theme| text::Style {
                    color: Some(Tokens::of(theme).muted_foreground),
                }),
        );
    }
    // Keeps long titles clear of the close button.
    if close.is_some() {
        header = header.padding(Padding::ZERO.right(space::XL));
    }

    let mut content = column![header].spacing(space::LG).width(Length::Fill);
    if let Some(body) = body {
        content = content.push(body);
    }
    if !actions.is_empty() {
        content = content.push(
            container(row(actions).spacing(space::SM))
                .width(Length::Fill)
                .align_x(Alignment::End),
        );
    }

    let mut layers = stack![container(content).padding(space::XL)];
    if let Some(on_press) = close {
        layers = layers.push(
            container(
                icon_button(crate::lucide!(X))
                    .label("Close")
                    .tooltip(None)
                    .size(ButtonSize::Sm)
                    .id(CLOSE_BUTTON_ID)
                    .on_press_maybe(on_press),
            )
            .padding(space::MD)
            .width(Length::Fill)
            .align_x(Alignment::End),
        );
    }

    container(layers)
        .id(id)
        .width(Length::Fill)
        .max_width(width)
        .style(|theme| surface_style(&Tokens::of(theme)))
        .into()
}

/// The dialog surface: the floating surface every overlay shares, with a
/// larger radius.
pub fn surface_style(tokens: &Tokens) -> container::Style {
    let style = anchored::surface_style(tokens);
    container::Style {
        border: Border {
            radius: radius::LG.into(),
            ..style.border
        },
        ..style
    }
}

/// The scrim: black at half opacity in light themes and more in dark ones,
/// where half would barely show.
pub fn scrim_style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(fade(
            Color::BLACK,
            if tokens.is_dark { 0.7 } else { 0.5 },
        ))),
        ..container::Style::default()
    }
}

/// Draws the base content, and while `blocked` keeps input and hover state
/// away from it.
struct Guard<'a, Message> {
    content: Element<'a, Message>,
    blocked: bool,
}

impl<Message> Widget<Message, Theme, Renderer> for Guard<'_, Message> {
    fn size(&self) -> Bounds<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Bounds<Length> {
        self.content.as_widget().size_hint()
    }

    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
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
        let cursor = if self.blocked {
            mouse::Cursor::Unavailable
        } else {
            cursor
        };
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
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
            .operate(tree, layout, renderer, operation);
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
        if !self.blocked {
            self.content.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
            return;
        }
        // Window events still arrive so animations underneath keep running.
        if !matches!(event, Event::Window(_)) {
            return;
        }
        self.content.as_widget_mut().update(
            tree,
            event,
            layout,
            mouse::Cursor::Unavailable,
            renderer,
            clipboard,
            shell,
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
        if self.blocked {
            return mouse::Interaction::None;
        }
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        if self.blocked {
            return None;
        }
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

/// Resolves the dialog's keymap, keeps focus inside the surface and
/// captures every other key press while the dialog is open.
struct Scope<'a, Message> {
    content: Element<'a, Message>,
    open: bool,
    surface: widget::Id,
    keymap: Keymap<Action>,
    pass_through: Vec<Chord>,
    on_escape: Option<Message>,
}

/// The key and modifiers of a key press or release.
fn key_of(event: &Event) -> Option<(&keyboard::Key, keyboard::Modifiers, bool)> {
    match event {
        Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
            Some((key, *modifiers, true))
        }
        Event::Keyboard(keyboard::Event::KeyReleased { key, modifiers, .. }) => {
            Some((key, *modifiers, false))
        }
        _ => None,
    }
}

/// Whether focus has moved into the dialog since it opened.
#[derive(Debug, Default)]
struct ScopeState {
    entered: bool,
}

impl<Message> Scope<'_, Message> {
    fn move_focus(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        movement: Movement,
    ) {
        let mut survey = Survey::new(self.surface.clone());
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, &mut survey);

        let mut apply = Apply {
            target: target(&survey.slots, movement),
            index: 0,
        };
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, &mut apply);
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Scope<'_, Message> {
    fn size(&self) -> Bounds<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Bounds<Length> {
        self.content.as_widget().size_hint()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ScopeState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ScopeState::default())
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
        let state = tree.state.downcast_mut::<ScopeState>();
        let entering = self.open && !state.entered;
        state.entered = self.open;

        let content = &mut tree.children[0];
        if entering {
            self.move_focus(content, layout, renderer, Movement::First);
            shell.request_redraw();
        }

        self.content.as_widget_mut().update(
            content, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
        if !self.open || shell.is_event_captured() {
            return;
        }

        let Some((key, modifiers, pressed)) = key_of(event) else {
            return;
        };
        if self
            .pass_through
            .iter()
            .any(|chord| chord.matches(key, modifiers))
        {
            return;
        }
        shell.capture_event();
        if !pressed {
            return;
        }
        let Some(effect) = self
            .keymap
            .resolve(key, modifiers)
            .and_then(|action| action.effect(self.on_escape.is_some()))
        else {
            return;
        };

        match effect {
            Effect::Dismiss => {
                let Some(message) = self.on_escape.clone() else {
                    return;
                };
                shell.publish(message);
            }
            Effect::Focus(direction) => {
                self.move_focus(content, layout, renderer, Movement::from(direction));
                shell.request_redraw();
            }
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

/// Where focus goes inside the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Movement {
    First,
    Next,
    Previous,
}

impl From<Direction> for Movement {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::Next => Movement::Next,
            Direction::Previous => Movement::Previous,
        }
    }
}

/// A focusable widget, in tree order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Slot {
    inside: bool,
    focused: bool,
}

/// The index of the slot to focus. Only slots inside the dialog are
/// candidates, and focus wraps at either end. Every other slot loses focus.
fn target(slots: &[Slot], movement: Movement) -> Option<usize> {
    let inside: Vec<usize> = (0..slots.len()).filter(|&i| slots[i].inside).collect();
    let (first, last) = (inside.first().copied()?, inside.last().copied()?);
    let current = inside.iter().position(|&i| slots[i].focused);
    let count = inside.len();

    match (movement, current) {
        (Movement::First, _) | (Movement::Next, None) => Some(first),
        (Movement::Previous, None) => Some(last),
        (Movement::Next, Some(at)) => inside.get((at + 1) % count).copied(),
        (Movement::Previous, Some(at)) => inside.get((at + count - 1) % count).copied(),
    }
}

/// Records every focusable widget and whether it sits inside the surface.
struct Survey {
    scope: widget::Id,
    entering: bool,
    depth: usize,
    slots: Vec<Slot>,
}

impl Survey {
    fn new(scope: widget::Id) -> Self {
        Self {
            scope,
            entering: false,
            depth: 0,
            slots: Vec::new(),
        }
    }
}

impl Operation for Survey {
    fn container(&mut self, id: Option<&widget::Id>, _bounds: Rectangle) {
        if id == Some(&self.scope) {
            self.entering = true;
        }
    }

    // A container reports itself and then traverses its children, so the
    // traversal right after the scope's container is the inside of it.
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        let entered = std::mem::take(&mut self.entering);
        self.depth += usize::from(entered);
        operate(self);
        self.depth -= usize::from(entered);
    }

    fn focusable(
        &mut self,
        _id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        self.slots.push(Slot {
            inside: self.depth > 0,
            focused: state.is_focused(),
        });
    }
}

/// Focuses the target slot and unfocuses every other one.
struct Apply {
    target: Option<usize>,
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
        let is_target = self.target == Some(self.index);
        if is_target && !state.is_focused() {
            state.focus();
        } else if !is_target && state.is_focused() {
            state.unfocus();
        }
        self.index += 1;
    }
}

/// A confirmation that needs an explicit answer: a clicked scrim does
/// nothing, there is no close button, and the footer holds Cancel and a
/// destructive action.
pub struct AlertDialog<'a, Message> {
    base: Element<'a, Message>,
    open: bool,
    title: text::Fragment<'a>,
    description: text::Fragment<'a>,
    cancel: text::Fragment<'a>,
    confirm: text::Fragment<'a>,
    on_cancel: Option<Message>,
    on_confirm: Option<Message>,
    dismiss_on_escape: bool,
    size: Size,
    keymap: Keymap<Action>,
}

impl<Message> std::fmt::Debug for AlertDialog<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlertDialog")
            .field("open", &self.open)
            .field("title", &self.title)
            .field("description", &self.description)
            .field("cancel", &self.cancel)
            .field("confirm", &self.confirm)
            .field("dismiss_on_escape", &self.dismiss_on_escape)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// Wraps `base` in a confirmation dialog. The buttons read "Cancel" and
/// "Continue" until you change them.
pub fn alert_dialog<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    title: impl text::IntoFragment<'a>,
    description: impl text::IntoFragment<'a>,
) -> AlertDialog<'a, Message> {
    AlertDialog {
        base: base.into(),
        open: false,
        title: title.into_fragment(),
        description: description.into_fragment(),
        cancel: "Cancel".into(),
        confirm: "Continue".into(),
        on_cancel: None,
        on_confirm: None,
        dismiss_on_escape: true,
        size: Size::default(),
        keymap: default_keymap(),
    }
}

impl<'a, Message> AlertDialog<'a, Message> {
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// The label of the cancel button.
    pub fn cancel(mut self, label: impl text::IntoFragment<'a>) -> Self {
        self.cancel = label.into_fragment();
        self
    }

    /// The label of the destructive button, such as "Delete".
    pub fn confirm(mut self, label: impl text::IntoFragment<'a>) -> Self {
        self.confirm = label.into_fragment();
        self
    }

    /// Emitted by the cancel button and by Escape.
    pub fn on_cancel(mut self, message: Message) -> Self {
        self.on_cancel = Some(message);
        self
    }

    /// Emitted by the destructive button.
    pub fn on_confirm(mut self, message: Message) -> Self {
        self.on_confirm = Some(message);
        self
    }

    /// Whether Escape cancels. Defaults to `true`.
    pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
        self.dismiss_on_escape = dismiss;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Replaces the default shortcuts.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }
}

impl<'a, Message: Clone + 'a> From<AlertDialog<'a, Message>> for Dialog<'a, Message> {
    fn from(alert: AlertDialog<'a, Message>) -> Self {
        let mut dialog = dialog(alert.base)
            .open(alert.open)
            .title(alert.title)
            .description(alert.description)
            .action(
                button::button(alert.cancel)
                    .variant(button::Variant::Outline)
                    .on_press_maybe(alert.on_cancel.clone()),
            )
            .action(
                button::button(alert.confirm)
                    .variant(button::Variant::Destructive)
                    .on_press_maybe(alert.on_confirm),
            )
            .dismiss_on_escape(alert.dismiss_on_escape)
            .dismiss_on_scrim(false)
            .close_button(false)
            .size(alert.size)
            .keymap(alert.keymap);
        if let Some(message) = alert.on_cancel {
            dialog = dialog.on_dismiss(message);
        }
        dialog
    }
}

impl<'a, Message: Clone + 'a> From<AlertDialog<'a, Message>> for Element<'a, Message> {
    fn from(alert: AlertDialog<'a, Message>) -> Self {
        Dialog::from(alert).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    fn slots(layout: &[(bool, bool)]) -> Vec<Slot> {
        layout
            .iter()
            .map(|&(inside, focused)| Slot { inside, focused })
            .collect()
    }

    #[test]
    fn builder_defaults() {
        let d: Dialog<'_, ()> = dialog(text("Base"));
        assert!(!d.is_open());
        assert!(d.dismiss_on_escape);
        assert!(d.dismiss_on_scrim);
        assert!(d.close_button);
        assert!(d.on_dismiss.is_none());
        assert_eq!(d.width, Size::Md.width());
        assert_eq!(d.id, DEFAULT_ID);
        assert_eq!(d.keymap, default_keymap());
        assert!(d.pass_through.is_empty());
    }

    #[test]
    fn builder_sets_every_option() {
        let d = dialog::<u8>(text("Base"))
            .open(true)
            .title("Title")
            .description("Description")
            .body(text("Body"))
            .action(text("One"))
            .action(text("Two"))
            .on_dismiss(1)
            .dismiss_on_escape(false)
            .dismiss_on_scrim(false)
            .close_button(false)
            .size(Size::Lg)
            .id("custom")
            .keymap(Keymap::new())
            .pass_through([Chord::character('k').command()]);
        assert_eq!(d.pass_through, vec![Chord::character('k').command()]);
        assert!(d.is_open());
        assert_eq!(d.title.as_deref(), Some("Title"));
        assert_eq!(d.description.as_deref(), Some("Description"));
        assert!(d.body.is_some());
        assert_eq!(d.actions.len(), 2);
        assert_eq!(d.on_dismiss, Some(1));
        assert!(!d.dismiss_on_escape && !d.dismiss_on_scrim && !d.close_button);
        assert_eq!(d.width, 640.0);
        assert_eq!(d.id, widget::Id::new("custom"));
        assert!(d.keymap.is_empty());
        assert_eq!(dialog::<u8>(text("Base")).width(300.0).width, 300.0);
    }

    #[test]
    fn sizes_grow_monotonically() {
        let widths = Size::ALL.map(Size::width);
        assert!(widths.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(Size::default(), Size::Md);
    }

    #[test]
    fn alert_dialog_needs_an_explicit_answer() {
        let alert = alert_dialog::<u8>(text("Base"), "Delete file?", "This cannot be undone.")
            .open(true)
            .confirm("Delete")
            .on_cancel(1)
            .on_confirm(2);
        assert_eq!(alert.cancel.as_ref(), "Cancel");
        assert_eq!(alert.confirm.as_ref(), "Delete");

        let d = Dialog::from(alert);
        assert!(d.is_open());
        assert!(!d.dismiss_on_scrim);
        assert!(!d.close_button);
        assert!(d.dismiss_on_escape);
        assert_eq!(d.on_dismiss, Some(1));
        assert_eq!(d.actions.len(), 2);
    }

    #[test]
    fn alert_dialog_without_cancel_has_no_dismiss() {
        let d = Dialog::from(
            alert_dialog::<u8>(text("Base"), "Title", "Description").dismiss_on_escape(false),
        );
        assert!(d.on_dismiss.is_none());
        assert!(!d.dismiss_on_escape);
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().ok()?;
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_and_overrides() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "Escape"), Some(Action::Close));
        assert_eq!(press(&keymap, "Tab"), Some(Action::FocusNext));
        assert_eq!(press(&keymap, "Shift+Tab"), Some(Action::FocusPrevious));
        assert_eq!(press(&keymap, "Ctrl+Tab"), None);

        let custom = keymap
            .unbind(&Chord::named(Named::Escape))
            .bind(Chord::character('w').command(), Action::Close);
        assert_eq!(press(&custom, "Escape"), None);
        assert_eq!(press(&custom, "Mod+W"), Some(Action::Close));
        assert!(
            custom
                .unbind_action(&Action::Close)
                .chords(&Action::Close)
                .is_empty()
        );
    }

    #[test]
    fn close_only_dismisses_when_escape_is_allowed() {
        assert_eq!(Action::Close.effect(true), Some(Effect::Dismiss));
        assert_eq!(Action::Close.effect(false), None);
        for allowed in [true, false] {
            assert_eq!(
                Action::FocusNext.effect(allowed),
                Some(Effect::Focus(Direction::Next))
            );
            assert_eq!(
                Action::FocusPrevious.effect(allowed),
                Some(Effect::Focus(Direction::Previous))
            );
        }
    }

    #[test]
    fn every_action_has_a_name_and_a_sentence() {
        use keys::Action as _;
        for action in Action::ALL {
            assert!(!action.name().is_empty());
            assert!(action.description().ends_with('.'));
        }
    }

    #[test]
    fn first_focuses_the_first_field_inside() {
        let layout = slots(&[(false, true), (true, false), (true, false)]);
        assert_eq!(target(&layout, Movement::First), Some(1));
    }

    #[test]
    fn next_and_previous_wrap_inside_the_dialog() {
        let last = slots(&[(false, false), (true, false), (true, true), (false, false)]);
        assert_eq!(target(&last, Movement::Next), Some(1));
        assert_eq!(target(&last, Movement::Previous), Some(1));

        let first = slots(&[(true, true), (true, false), (true, false)]);
        assert_eq!(target(&first, Movement::Previous), Some(2));
        assert_eq!(target(&first, Movement::Next), Some(1));
    }

    #[test]
    fn focus_outside_moves_to_either_end_inside() {
        let layout = slots(&[(false, true), (true, false), (true, false), (false, false)]);
        assert_eq!(target(&layout, Movement::Next), Some(1));
        assert_eq!(target(&layout, Movement::Previous), Some(2));
    }

    #[test]
    fn nothing_to_focus_without_fields_inside() {
        let layout = slots(&[(false, true), (false, false)]);
        for movement in [Movement::First, Movement::Next, Movement::Previous] {
            assert_eq!(target(&layout, movement), None);
        }
        assert_eq!(target(&[], Movement::Next), None);
    }

    #[test]
    fn a_single_field_keeps_focus() {
        let layout = slots(&[(true, true)]);
        assert_eq!(target(&layout, Movement::Next), Some(0));
        assert_eq!(target(&layout, Movement::Previous), Some(0));
    }

    #[test]
    fn surface_uses_the_shared_floating_style() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let style = surface_style(&tokens);
            let shared = anchored::surface_style(&tokens);
            assert_eq!(style.background, Some(Background::Color(tokens.popover)));
            assert_eq!(style.text_color, Some(tokens.foreground));
            assert_eq!(style.border.color, tokens.border);
            assert_eq!(style.border.width, 1.0);
            assert_eq!(style.border.radius, radius::LG.into());
            assert_eq!(style.shadow, shared.shadow);
        }
    }

    #[test]
    fn dark_theme_has_a_stronger_shadow_and_scrim() {
        let light = Tokens::of(&light());
        let dark = Tokens::of(&dark());
        assert!(surface_style(&dark).shadow.color.a > surface_style(&light).shadow.color.a);

        let alpha = |tokens: &Tokens| match scrim_style(tokens).background {
            Some(Background::Color(colour)) => colour.a,
            _ => 0.0,
        };
        assert!((alpha(&light) - 0.5).abs() < 1e-6);
        assert!((alpha(&dark) - 0.7).abs() < 1e-6);
    }
}
