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
//! subscription for it: Escape dismisses, Enter sends the
//! [`on_confirm`](Dialog::on_confirm) message, and Tab and Shift+Tab move
//! focus through everything focusable inside the dialog without leaving
//! it, scrolling the body to show it. That is every text field and this
//! library's buttons, icon buttons, switches, checkboxes, radios, toggles
//! and selects, including the footer actions and the close button, in the
//! order they are laid out. A focused control rings and Space or Enter
//! works it; a focused select opens on Space, Enter or ArrowDown. Opening
//! the dialog focuses its first text field, if it has one.
//!
//! Escape first closes whatever is open inside the dialog, such as a
//! select's list, a combobox's suggestions or a menu. The next Escape
//! dismisses the dialog, even while a text field has focus.
//!
//! While it is open the dialog captures every key press, after the content
//! inside it has had its turn, so app-wide shortcuts from
//! [`keys::subscription`] never reach the components underneath. Let
//! chosen chords through with [`Dialog::pass_through`].
//!
//! The dialog never grows taller than the window. The title, description
//! and actions stay in view and the body scrolls when it does not fit.
//! Group a long body with [`section`] and lay sections side by side with
//! [`columns`].

use std::any::Any;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::scrollable::AbsoluteOffset;
use iced::advanced::widget::operation::{Focusable, Scrollable};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::keyboard::key::Named;
use iced::widget::{self, center, column, container, mouse_area, row, stack, text};
use iced::{
    Alignment, Background, Border, Color, Element, Event, Length, Padding, Point, Rectangle,
    Renderer, Size as Bounds, Theme, Vector, keyboard, mouse,
};

use crate::focus;
use crate::keys::{self, Chord, Keymap};
use crate::overlay::anchored;
use crate::primitives::button::{self, Size as ButtonSize};
use crate::primitives::icon_button::icon_button;
use crate::primitives::scroll_area::scroll_area;
use crate::solid::solid;
use crate::theme::{Tokens, fade, heading, radius, space, supporting, text_size};

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
    Confirm,
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
    /// Emit the `on_confirm` message.
    Confirm,
    /// Move focus to another field or control inside the dialog.
    Focus(Direction),
}

impl Action {
    /// The effect of this action on an open dialog, or `None` when Close
    /// has nothing to do because Escape does not dismiss the dialog.
    pub fn effect(self, dismiss_on_escape: bool) -> Option<Effect> {
        match self {
            Action::Close => dismiss_on_escape.then_some(Effect::Dismiss),
            Action::Confirm => Some(Effect::Confirm),
            Action::FocusNext => Some(Effect::Focus(Direction::Next)),
            Action::FocusPrevious => Some(Effect::Focus(Direction::Previous)),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Close,
        Action::Confirm,
        Action::FocusNext,
        Action::FocusPrevious,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Close => "Close",
            Action::Confirm => "Confirm",
            Action::FocusNext => "FocusNext",
            Action::FocusPrevious => "FocusPrevious",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Close => "Dismisses the dialog, unless Escape dismissal is turned off.",
            Action::Confirm => {
                "Sends the dialog's confirm message. An alert dialog's destructive action only runs on Enter when it opts in."
            }
            Action::FocusNext => {
                "Focuses the next field, button or control in the dialog, wrapping from the last to the first."
            }
            Action::FocusPrevious => {
                "Focuses the previous field, button or control in the dialog, wrapping from the first to the last."
            }
        }
    }
}

/// The default dialog shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `Escape` | [`Action::Close`] |
/// | `Enter` | [`Action::Confirm`] |
/// | `Tab` | [`Action::FocusNext`] |
/// | `Shift+Tab` | [`Action::FocusPrevious`] |
///
/// The dialog resolves these itself while it is open, after the content
/// inside it, so a menu, select or combobox list in the dialog closes on
/// Escape before the dialog does, and a field that submits on Enter or a
/// focused button keeps it. A focused text field does not hold up Close:
/// the press that takes focus off it also dismisses the dialog. Enter does
/// nothing in a dialog without [`Dialog::on_confirm`], or in an alert
/// dialog without [`AlertDialog::confirm_on_enter`].
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::Escape), Action::Close)
        .bind(Chord::named(Named::Enter), Action::Confirm)
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
    on_confirm: Option<Message>,
    dismiss_on_escape: bool,
    dismiss_on_scrim: bool,
    close_button: bool,
    width: f32,
    max_height: f32,
    id: widget::Id,
    keymap: Keymap<Action>,
    pass_through: Vec<Chord>,
    /// Built from an [`AlertDialog`], so icedwright reports an alert dialog.
    #[cfg(feature = "icedwright")]
    alert: bool,
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
            .field("max_height", &self.max_height)
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
        on_confirm: None,
        dismiss_on_escape: true,
        dismiss_on_scrim: true,
        close_button: true,
        width: Size::default().width(),
        max_height: f32::INFINITY,
        id: DEFAULT_ID,
        keymap: default_keymap(),
        pass_through: Vec::new(),
        #[cfg(feature = "icedwright")]
        alert: false,
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

    /// Content between the header and the footer, such as a form. It
    /// scrolls when the dialog would not otherwise fit the window.
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

    /// The message [`Action::Confirm`] (Enter) emits, usually the same as
    /// the main footer action. Without it Enter does nothing. A field
    /// inside the dialog that submits on Enter keeps the key.
    pub fn on_confirm(mut self, message: Message) -> Self {
        self.on_confirm = Some(message);
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

    /// Sets the maximum height in logical pixels. The dialog is never
    /// taller than the window less the scrim's padding, whatever this is.
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = height;
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
            on_confirm,
            dismiss_on_escape,
            dismiss_on_scrim,
            close_button,
            width,
            max_height,
            id,
            keymap,
            pass_through,
            #[cfg(feature = "icedwright")]
            alert,
        } = dialog;

        let guarded = Element::new(Guard {
            content: base,
            blocked: open,
        });

        // The base is always the first child, so it keeps its widget state
        // when the dialog opens and closes.
        let mut layers = stack![guarded].width(Length::Fill).height(Length::Fill);

        if open {
            #[cfg(feature = "icedwright")]
            let name = title.clone();
            #[cfg(feature = "icedwright")]
            let about = description.clone();
            let surface = surface(
                title,
                description,
                body,
                actions,
                close_button.then(|| on_dismiss.clone()),
                Bounds::new(width, max_height),
                id.clone(),
            );
            #[cfg(feature = "icedwright")]
            let surface = annotate_surface(surface, alert, name, about);
            let scrim = center(solid(surface))
                .padding(space::LG)
                .style(|theme| scrim_style(&Tokens::of(theme)));
            let layer: Element<'a, Message> = match on_dismiss.clone().filter(|_| dismiss_on_scrim)
            {
                Some(message) => solid(mouse_area(scrim).on_press(message)),
                None => solid(scrim),
            };
            #[cfg(feature = "icedwright")]
            let layer: Element<'a, Message> = icedwright_probe::semantic(layer)
                .role(icedwright_probe::role::Scrim)
                .component("iced_cube::dialog")
                .into();
            layers = layers.push(layer);
        }

        Element::new(Scope {
            content: layers.into(),
            open,
            surface: id,
            keymap,
            pass_through,
            on_escape: on_dismiss.filter(|_| dismiss_on_escape),
            on_confirm,
        })
    }
}

/// Reports the surface as a modal dialog named by its title.
#[cfg(feature = "icedwright")]
fn annotate_surface<'a, Message: 'a>(
    surface: Element<'a, Message>,
    alert: bool,
    title: Option<text::Fragment<'a>>,
    description: Option<text::Fragment<'a>>,
) -> Element<'a, Message> {
    use icedwright_probe::{Semantic, role, semantic};

    fn finish<'a, Message: 'a, R: role::RoleKind>(
        mut annotated: Semantic<'a, Message, Theme, iced::Renderer, R>,
        title: Option<text::Fragment<'a>>,
        description: Option<text::Fragment<'a>>,
    ) -> Element<'a, Message> {
        annotated = annotated.modal(true).component("iced_cube::dialog");
        if let Some(title) = title {
            annotated = annotated.name(title);
        }
        if let Some(description) = description {
            annotated = annotated.description(description);
        }
        annotated.into()
    }

    if alert {
        finish(
            semantic(surface).role(role::AlertDialog),
            title,
            description,
        )
    } else {
        finish(semantic(surface).role(role::Dialog), title, description)
    }
}

fn surface<'a, Message: Clone + 'a>(
    title: Option<text::Fragment<'a>>,
    description: Option<text::Fragment<'a>>,
    body: Option<Element<'a, Message>>,
    actions: Vec<Element<'a, Message>>,
    close: Option<Option<Message>>,
    max: Bounds,
    id: widget::Id,
) -> Element<'a, Message> {
    let mut header = column![].spacing(space::XS);
    if let Some(title) = title {
        header = header.push(heading(title, text_size::LG));
    }
    if let Some(description) = description {
        header = header.push(supporting(description));
    }
    // Keeps long titles clear of the close button.
    if close.is_some() {
        header = header.padding(Padding::ZERO.right(space::XL));
    }

    let mut parts: Vec<Element<'a, Message>> = vec![header.into()];
    let mut flexible = None;
    if let Some(body) = body {
        flexible = Some(parts.len());
        parts.push(scroll_area(body).width(Length::Fill).into());
    }
    if !actions.is_empty() {
        parts.push(
            container(row(actions).spacing(space::SM))
                .width(Length::Fill)
                .align_x(Alignment::End)
                .into(),
        );
    }
    let content = Element::new(Pinned {
        children: parts,
        flexible,
        spacing: space::LG,
    });

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
        .max_width(max.width)
        .max_height(max.height)
        .style(|theme| surface_style(&Tokens::of(theme)))
        .into()
}

/// The height left for the flexible child of a [`Pinned`] layout once the
/// other children and the gaps between `count` children are placed.
fn room(available: f32, fixed: f32, spacing: f32, count: usize) -> f32 {
    let gaps = spacing * count.saturating_sub(1) as f32;
    (available - fixed - gaps).max(0.0)
}

/// Lays its children out top to bottom like a column, but the flexible
/// child only gets the height the others leave. The dialog's header and
/// footer stay in view while its body, a scroll area, shrinks.
struct Pinned<'a, Message> {
    children: Vec<Element<'a, Message>>,
    flexible: Option<usize>,
    spacing: f32,
}

impl<Message> Widget<Message, Theme, Renderer> for Pinned<'_, Message> {
    fn size(&self) -> Bounds<Length> {
        Bounds::new(Length::Fill, Length::Shrink)
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let max = limits.max();
        let loose = layout::Limits::new(Bounds::ZERO, max);
        let mut nodes: Vec<Option<layout::Node>> = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .enumerate()
            .map(|(index, (child, tree))| {
                (self.flexible != Some(index))
                    .then(|| child.as_widget_mut().layout(tree, renderer, &loose))
            })
            .collect();

        if let Some(index) = self.flexible {
            let fixed = nodes.iter().flatten().map(|node| node.size().height).sum();
            let height = room(max.height, fixed, self.spacing, self.children.len());
            let limits = layout::Limits::new(Bounds::ZERO, Bounds::new(max.width, height));
            if let (Some(child), Some(tree), Some(slot)) = (
                self.children.get_mut(index),
                tree.children.get_mut(index),
                nodes.get_mut(index),
            ) {
                *slot = Some(child.as_widget_mut().layout(tree, renderer, &limits));
            }
        }

        let (nodes, size) = stack_vertically(nodes.into_iter().flatten(), self.spacing);
        layout::Node::with_children(limits.resolve(Length::Fill, Length::Shrink, size), nodes)
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
        for ((child, tree), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operate_all(&mut self.children, tree, layout, renderer, operation);
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
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
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
        interaction_of_all(&self.children, tree, layout, cursor, viewport, renderer)
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
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

/// Places `nodes` one under another with `spacing` between them, and
/// returns them with the size they cover.
fn stack_vertically(
    nodes: impl IntoIterator<Item = layout::Node>,
    spacing: f32,
) -> (Vec<layout::Node>, Bounds) {
    let mut y = 0.0;
    let mut width: f32 = 0.0;
    let placed: Vec<layout::Node> = nodes
        .into_iter()
        .map(|node| {
            let size = node.size();
            let node = node.move_to(Point::new(0.0, y));
            y += size.height + spacing;
            width = width.max(size.width);
            node
        })
        .collect();
    let height = if placed.is_empty() { 0.0 } else { y - spacing };
    (placed, Bounds::new(width, height))
}

fn operate_all<Message>(
    children: &mut [Element<'_, Message>],
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &Renderer,
    operation: &mut dyn Operation,
) {
    operation.container(None, layout.bounds());
    operation.traverse(&mut |operation| {
        for ((child, tree), layout) in children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child
                .as_widget_mut()
                .operate(tree, layout, renderer, operation);
        }
    });
}

fn interaction_of_all<Message>(
    children: &[Element<'_, Message>],
    tree: &Tree,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    viewport: &Rectangle,
    renderer: &Renderer,
) -> mouse::Interaction {
    children
        .iter()
        .zip(&tree.children)
        .zip(layout.children())
        .map(|((child, tree), layout)| {
            child
                .as_widget()
                .mouse_interaction(tree, layout, cursor, viewport, renderer)
        })
        .max()
        .unwrap_or_default()
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
    on_confirm: Option<Message>,
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

/// Whether a Close press the content captured still dismisses the dialog:
/// it does when all the content did was take focus off a field inside.
fn dismisses_anyway(focused_before: bool, focused_after: bool) -> bool {
    focused_before && !focused_after
}

impl<Message> Scope<'_, Message> {
    /// Whether `event` is a press of a Close chord that would dismiss.
    fn is_dismissal(&self, event: &Event) -> bool {
        let Some((key, modifiers, true)) = key_of(event) else {
            return false;
        };
        self.open
            && self.on_escape.is_some()
            && !self
                .pass_through
                .iter()
                .any(|chord| chord.matches(key, modifiers))
            && self.keymap.resolve(key, modifiers) == Some(Action::Close)
    }

    fn has_focus_inside(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> bool {
        let mut survey = Survey::new(self.surface.clone());
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, &mut survey);
        survey.has_focus_inside()
    }

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
            focused: None,
        };
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, &mut apply);

        let Some(focused) = apply.focused else {
            return;
        };
        let mut reveal = Reveal {
            scope: Survey::new(self.surface.clone()),
            target: focused,
        };
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, &mut reveal);
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

        // A field drops focus on Escape and keeps the key, which would
        // otherwise take a second press to dismiss. Open lists and menus
        // float in overlays, which see the key first and never pass it on.
        let dismissal = self.is_dismissal(event);
        let focused_before = dismissal && self.has_focus_inside(content, layout, renderer);

        self.content.as_widget_mut().update(
            content, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
        if !self.open {
            return;
        }
        if shell.is_event_captured() {
            if !dismissal {
                return;
            }
            let focused_after = self.has_focus_inside(content, layout, renderer);
            if let Some(message) = self
                .on_escape
                .clone()
                .filter(|_| dismisses_anyway(focused_before, focused_after))
            {
                shell.publish(message);
            }
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
            Effect::Confirm => {
                let Some(message) = self.on_confirm.clone() else {
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
    /// A button or other control, which Tab reaches but opening does not.
    control: bool,
}

/// The index of the slot to focus. Only slots inside the dialog are
/// candidates, and focus wraps at either end. Opening focuses the first
/// text field, never a control. Every other slot loses focus.
fn target(slots: &[Slot], movement: Movement) -> Option<usize> {
    let inside: Vec<usize> = (0..slots.len()).filter(|&i| slots[i].inside).collect();
    if movement == Movement::First {
        return inside.into_iter().find(|&i| !slots[i].control);
    }
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
    /// The next focusable is a control, marked just before it.
    control: bool,
    slots: Vec<Slot>,
}

impl Survey {
    fn new(scope: widget::Id) -> Self {
        Self {
            scope,
            entering: false,
            depth: 0,
            control: false,
            slots: Vec::new(),
        }
    }

    fn has_focus_inside(&self) -> bool {
        self.slots.iter().any(|slot| slot.inside && slot.focused)
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

    fn custom(&mut self, _id: Option<&widget::Id>, _bounds: Rectangle, state: &mut dyn Any) {
        if state.is::<focus::Control>() {
            self.control = true;
        }
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
            control: std::mem::take(&mut self.control),
        });
    }
}

/// Focuses the target slot and unfocuses every other one, and records the
/// bounds of the one it focused.
struct Apply {
    target: Option<usize>,
    index: usize,
    focused: Option<Rectangle>,
}

impl Operation for Apply {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&widget::Id>,
        bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        let is_target = self.target == Some(self.index);
        if is_target {
            self.focused = Some(bounds);
        }
        if is_target && !state.is_focused() {
            state.focus();
        } else if !is_target && state.is_focused() {
            state.unfocus();
        }
        self.index += 1;
    }
}

/// Room kept above or below a field scrolled into view, so its focus ring
/// shows too.
const REVEAL_MARGIN: f32 = space::XS;

/// The scroll offset that brings `target` fully into a scroll area's
/// `viewport`, or `None` when it is already in view or not inside the
/// area's `content`. Bounds are in the content's unscrolled coordinates,
/// as operations report them.
fn reveal(viewport: Rectangle, content: Rectangle, offset: f32, target: Rectangle) -> Option<f32> {
    if !content.intersects(&target) {
        return None;
    }
    let top = (target.y - content.y - REVEAL_MARGIN).max(0.0);
    let bottom = target.y + target.height - content.y + REVEAL_MARGIN;
    if top < offset {
        return Some(top);
    }
    if bottom > offset + viewport.height {
        return Some((bottom - viewport.height).min(top));
    }
    None
}

/// Scrolls the scroll areas inside the dialog so the focused field shows.
struct Reveal {
    scope: Survey,
    target: Rectangle,
}

impl Operation for Reveal {
    fn container(&mut self, id: Option<&widget::Id>, bounds: Rectangle) {
        self.scope.container(id, bounds);
    }

    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        let entered = std::mem::take(&mut self.scope.entering);
        self.scope.depth += usize::from(entered);
        operate(self);
        self.scope.depth -= usize::from(entered);
    }

    fn scrollable(
        &mut self,
        _id: Option<&widget::Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn Scrollable,
    ) {
        if self.scope.depth == 0 {
            return;
        }
        let Some(y) = reveal(bounds, content_bounds, translation.y, self.target) else {
            return;
        };
        state.scroll_to(AbsoluteOffset {
            x: None,
            y: Some(y),
        });
    }
}

/// A confirmation that needs an explicit answer: a clicked scrim does
/// nothing unless [`dismiss_on_scrim`](Self::dismiss_on_scrim) is on,
/// there is no close button, and the footer holds Cancel and a
/// destructive action.
///
/// Keyboard users reach both buttons with Tab and Shift+Tab, which ring
/// the focused one, and press it with Enter or Space. Nothing is focused
/// when the dialog opens, so a stray Enter does nothing unless
/// [`confirm_on_enter`](Self::confirm_on_enter) is on; Escape cancels.
pub struct AlertDialog<'a, Message> {
    base: Element<'a, Message>,
    open: bool,
    title: text::Fragment<'a>,
    description: text::Fragment<'a>,
    cancel: text::Fragment<'a>,
    confirm: text::Fragment<'a>,
    on_cancel: Option<Message>,
    on_confirm: Option<Message>,
    confirm_on_enter: bool,
    dismiss_on_escape: bool,
    dismiss_on_scrim: bool,
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
            .field("confirm_on_enter", &self.confirm_on_enter)
            .field("dismiss_on_escape", &self.dismiss_on_escape)
            .field("dismiss_on_scrim", &self.dismiss_on_scrim)
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
        confirm_on_enter: false,
        dismiss_on_escape: true,
        dismiss_on_scrim: false,
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

    /// Emitted by the destructive button, and by Enter when
    /// [`confirm_on_enter`](Self::confirm_on_enter) is on.
    pub fn on_confirm(mut self, message: Message) -> Self {
        self.on_confirm = Some(message);
        self
    }

    /// Whether Enter ([`Action::Confirm`]) runs the destructive action
    /// while no button is focused. Defaults to `false`, so a stray Enter
    /// never destroys anything; keyboard users Tab to the button and press
    /// Enter or Space instead.
    pub fn confirm_on_enter(mut self, confirm: bool) -> Self {
        self.confirm_on_enter = confirm;
        self
    }

    /// Whether Escape cancels. Defaults to `true`.
    pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
        self.dismiss_on_escape = dismiss;
        self
    }

    /// Whether a click on the scrim cancels, sending the
    /// [`on_cancel`](Self::on_cancel) message. Defaults to `false`.
    pub fn dismiss_on_scrim(mut self, dismiss: bool) -> Self {
        self.dismiss_on_scrim = dismiss;
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
                    .on_press_maybe(alert.on_confirm.clone()),
            )
            .dismiss_on_escape(alert.dismiss_on_escape)
            .dismiss_on_scrim(alert.dismiss_on_scrim)
            .close_button(false)
            .size(alert.size)
            .keymap(alert.keymap);
        #[cfg(feature = "icedwright")]
        {
            dialog.alert = true;
        }
        if let Some(message) = alert.on_cancel {
            dialog = dialog.on_dismiss(message);
        }
        let Some(message) = alert.on_confirm.filter(|_| alert.confirm_on_enter) else {
            return dialog;
        };
        dialog.on_confirm(message)
    }
}

impl<'a, Message: Clone + 'a> From<AlertDialog<'a, Message>> for Element<'a, Message> {
    fn from(alert: AlertDialog<'a, Message>) -> Self {
        Dialog::from(alert).into()
    }
}

/// A headed group of content in a dialog body. Convert it into an
/// [`Element`] to render.
pub struct Section<'a, Message> {
    title: text::Fragment<'a>,
    description: Option<text::Fragment<'a>>,
    body: Element<'a, Message>,
}

impl<Message> std::fmt::Debug for Section<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Section")
            .field("title", &self.title)
            .field("description", &self.description)
            .finish_non_exhaustive()
    }
}

/// Puts `title` over `body`, in the dialog's heading style one step
/// smaller than the dialog's own title.
pub fn section<'a, Message>(
    title: impl text::IntoFragment<'a>,
    body: impl Into<Element<'a, Message>>,
) -> Section<'a, Message> {
    Section {
        title: title.into_fragment(),
        description: None,
        body: body.into(),
    }
}

impl<'a, Message> Section<'a, Message> {
    /// A line of muted text under the title.
    pub fn description(mut self, description: impl text::IntoFragment<'a>) -> Self {
        self.description = Some(description.into_fragment());
        self
    }
}

/// Text size of a section title.
pub const SECTION_TITLE_SIZE: f32 = text_size::MD;

impl<'a, Message: 'a> From<Section<'a, Message>> for Element<'a, Message> {
    fn from(section: Section<'a, Message>) -> Self {
        let mut header = column![heading(section.title, SECTION_TITLE_SIZE)].spacing(space::XS);
        if let Some(description) = section.description {
            header = header.push(supporting(description));
        }
        column![header, section.body]
            .spacing(space::MD)
            .width(Length::Fill)
            .into()
    }
}

/// The width below which [`columns`] stacks its children in one column.
pub const BREAKPOINT: f32 = 400.0;

/// How many columns fit in `width`, and how wide each one is.
pub fn cells(width: f32, breakpoint: f32, spacing: f32) -> (usize, f32) {
    if !width.is_finite() || width < breakpoint {
        return (1, width);
    }
    (2, ((width - spacing) / 2.0).max(0.0))
}

/// Two columns that fall back to one in a narrow space. Convert it into an
/// [`Element`] to render.
pub struct Columns<'a, Message> {
    children: Vec<Element<'a, Message>>,
    breakpoint: f32,
    spacing: f32,
}

impl<Message> std::fmt::Debug for Columns<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Columns")
            .field("children", &self.children.len())
            .field("breakpoint", &self.breakpoint)
            .field("spacing", &self.spacing)
            .finish()
    }
}

/// Lays `children` out in two equal columns, filling each row left to
/// right. Where there is less room than the [`breakpoint`](Columns::breakpoint)
/// they stack in a single column instead.
pub fn columns<'a, Message>(
    children: impl IntoIterator<Item = Element<'a, Message>>,
) -> Columns<'a, Message> {
    Columns {
        children: children.into_iter().collect(),
        breakpoint: BREAKPOINT,
        spacing: space::XL,
    }
}

impl<'a, Message> Columns<'a, Message> {
    /// Adds a child.
    pub fn push(mut self, child: impl Into<Element<'a, Message>>) -> Self {
        self.children.push(child.into());
        self
    }

    /// The narrowest width, in logical pixels, that still gets two columns.
    /// Defaults to [`BREAKPOINT`].
    pub fn breakpoint(mut self, width: f32) -> Self {
        self.breakpoint = width;
        self
    }

    /// The gap between the columns and between the rows. Defaults to 24.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

impl<'a, Message: 'a> From<Columns<'a, Message>> for Element<'a, Message> {
    fn from(columns: Columns<'a, Message>) -> Self {
        Element::new(columns)
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Columns<'_, Message> {
    fn size(&self) -> Bounds<Length> {
        Bounds::new(Length::Fill, Length::Shrink)
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let max = limits.max();
        let (count, cell) = cells(max.width, self.breakpoint, self.spacing);
        let cell_limits = layout::Limits::new(Bounds::ZERO, Bounds::new(cell, max.height));

        let mut nodes = Vec::with_capacity(self.children.len());
        let mut y = 0.0;
        let mut width: f32 = 0.0;
        for (children, trees) in self
            .children
            .chunks_mut(count)
            .zip(tree.children.chunks_mut(count))
        {
            let mut height: f32 = 0.0;
            for (column, (child, tree)) in children.iter_mut().zip(trees).enumerate() {
                let node = child.as_widget_mut().layout(tree, renderer, &cell_limits);
                let x = column as f32 * (cell + self.spacing);
                let size = node.size();
                height = height.max(size.height);
                width = width.max(x + size.width);
                nodes.push(node.move_to(Point::new(x, y)));
            }
            y += height + self.spacing;
        }
        let height = if nodes.is_empty() {
            0.0
        } else {
            y - self.spacing
        };

        let intrinsic = Bounds::new(width, height);
        let size = if max.width.is_finite() {
            limits.resolve(Length::Fill, Length::Shrink, intrinsic)
        } else {
            intrinsic
        };
        layout::Node::with_children(size, nodes)
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
        for ((child, tree), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operate_all(&mut self.children, tree, layout, renderer, operation);
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
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
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
        interaction_of_all(&self.children, tree, layout, cursor, viewport, renderer)
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
            &mut self.children,
            tree,
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

    fn slots(layout: &[(bool, bool)]) -> Vec<Slot> {
        layout
            .iter()
            .map(|&(inside, focused)| Slot {
                inside,
                focused,
                control: false,
            })
            .collect()
    }

    #[test]
    fn a_captured_close_still_dismisses_when_it_only_unfocused_a_field() {
        assert!(dismisses_anyway(true, false));
        assert!(!dismisses_anyway(true, true), "something else used the key");
        assert!(!dismisses_anyway(false, false));
        assert!(!dismisses_anyway(false, true));
    }

    #[test]
    fn the_survey_marks_controls_and_finds_focus_inside() {
        let mut survey = Survey::new(DEFAULT_ID);
        let mut field = focus::State::default();
        let mut control = focus::State::default();
        let bounds = Rectangle::with_size(Bounds::new(10.0, 10.0));
        survey.container(Some(&DEFAULT_ID), bounds);
        survey.traverse(&mut |operation| {
            operation.focusable(None, bounds, &mut field);
            focus::report(operation, None, bounds, &mut control);
        });
        assert_eq!(
            survey
                .slots
                .iter()
                .map(|slot| slot.control)
                .collect::<Vec<_>>(),
            vec![false, true]
        );
        assert!(survey.slots.iter().all(|slot| slot.inside));
        assert!(!survey.has_focus_inside());

        let mut outside = Survey::new(DEFAULT_ID);
        Focusable::focus(&mut field);
        outside.focusable(None, bounds, &mut field);
        assert!(!outside.has_focus_inside(), "focus outside the surface");
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
        assert_eq!(d.max_height, f32::INFINITY);
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
            .max_height(420.0)
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
        assert_eq!(d.max_height, 420.0);
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
        assert_eq!(d.on_confirm, None, "Enter does not destroy by default");
        assert_eq!(d.actions.len(), 2);
    }

    #[test]
    fn alert_dialog_can_cancel_on_the_scrim() {
        let alert = alert_dialog::<u8>(text("Base"), "Discard draft?", "It has not been saved.");
        assert!(!alert.dismiss_on_scrim);

        let d = Dialog::from(alert.on_cancel(1).dismiss_on_scrim(true));
        assert!(d.dismiss_on_scrim);
        assert_eq!(d.on_dismiss, Some(1), "the scrim sends on_cancel");
    }

    #[test]
    fn the_body_gets_what_the_header_footer_and_gaps_leave() {
        assert_eq!(room(600.0, 100.0, 16.0, 3), 600.0 - 100.0 - 32.0);
        assert_eq!(room(600.0, 100.0, 16.0, 1), 500.0);
        assert_eq!(room(100.0, 200.0, 16.0, 3), 0.0, "never negative");
        assert_eq!(room(f32::INFINITY, 50.0, 16.0, 2), f32::INFINITY);
        assert_eq!(room(100.0, 0.0, 16.0, 0), 100.0);
    }

    #[test]
    fn stacking_places_nodes_under_each_other() {
        let nodes = [Bounds::new(40.0, 10.0), Bounds::new(80.0, 20.0)]
            .map(layout::Node::new)
            .to_vec();
        let (placed, size) = stack_vertically(nodes, 5.0);
        assert_eq!(size, Bounds::new(80.0, 35.0));
        assert_eq!(placed[1].bounds().y, 15.0);
        assert_eq!(stack_vertically([], 5.0).1, Bounds::ZERO);
    }

    #[test]
    fn reveal_scrolls_a_hidden_field_into_view() {
        let viewport = Rectangle::new(Point::new(0.0, 100.0), Bounds::new(300.0, 200.0));
        let content = Rectangle::new(Point::new(0.0, 100.0), Bounds::new(300.0, 800.0));
        let field = |y: f32| Rectangle::new(Point::new(0.0, 100.0 + y), Bounds::new(300.0, 40.0));

        assert_eq!(reveal(viewport, content, 0.0, field(50.0)), None, "in view");
        assert_eq!(
            reveal(viewport, content, 0.0, field(400.0)),
            Some(400.0 + 40.0 + REVEAL_MARGIN - 200.0),
            "below: its bottom lines up with the bottom"
        );
        assert_eq!(
            reveal(viewport, content, 300.0, field(100.0)),
            Some(100.0 - REVEAL_MARGIN),
            "above: its top lines up with the top"
        );
        let tall = Rectangle::new(Point::new(0.0, 500.0), Bounds::new(300.0, 300.0));
        assert_eq!(
            reveal(viewport, content, 0.0, tall),
            Some(400.0 - REVEAL_MARGIN),
            "a field taller than the view shows its top"
        );
        let elsewhere = Rectangle::new(Point::new(0.0, 1000.0), Bounds::new(300.0, 40.0));
        assert_eq!(reveal(viewport, content, 0.0, elsewhere), None);
    }

    #[test]
    fn columns_fall_back_to_one_below_the_breakpoint() {
        assert_eq!(cells(464.0, BREAKPOINT, 24.0), (2, 220.0));
        assert_eq!(cells(BREAKPOINT, BREAKPOINT, 24.0), (2, 188.0));
        assert_eq!(
            cells(BREAKPOINT - 1.0, BREAKPOINT, 24.0),
            (1, BREAKPOINT - 1.0)
        );
        assert_eq!(cells(280.0, 200.0, 0.0), (2, 140.0));
        assert_eq!(cells(f32::INFINITY, BREAKPOINT, 24.0).0, 1);
    }

    #[test]
    fn columns_and_sections_builders() {
        let c = columns::<()>([text("A").into()])
            .push(text("B"))
            .breakpoint(320.0)
            .spacing(8.0);
        assert_eq!(c.children.len(), 2);
        assert_eq!(c.breakpoint, 320.0);
        assert_eq!(c.spacing, 8.0);
        let d = columns::<()>([]);
        assert_eq!((d.breakpoint, d.spacing), (BREAKPOINT, space::XL));

        let s = section::<()>("Event", text("Body")).description("When and where");
        assert_eq!(s.title.as_ref(), "Event");
        assert_eq!(s.description.as_deref(), Some("When and where"));
        const { assert!(SECTION_TITLE_SIZE < text_size::LG) };
    }

    #[test]
    fn alert_dialog_confirms_on_enter_only_when_asked() {
        let alert = alert_dialog::<u8>(text("Base"), "Delete file?", "This cannot be undone.")
            .on_confirm(2)
            .confirm_on_enter(true);
        assert_eq!(Dialog::from(alert).on_confirm, Some(2));
    }

    #[test]
    fn alert_dialog_without_cancel_has_no_dismiss() {
        let d = Dialog::from(
            alert_dialog::<u8>(text("Base"), "Title", "Description").dismiss_on_escape(false),
        );
        assert!(d.on_dismiss.is_none());
        assert!(d.on_confirm.is_none());
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
        assert_eq!(press(&keymap, "Enter"), Some(Action::Confirm));
        assert_eq!(press(&keymap, "Shift+Enter"), None);
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
            assert_eq!(Action::Confirm.effect(allowed), Some(Effect::Confirm));
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
    fn opening_skips_controls_but_tab_reaches_them() {
        let mut layout = slots(&[(true, false), (true, false)]);
        for slot in &mut layout {
            slot.control = true;
        }
        assert_eq!(target(&layout, Movement::First), None);
        assert_eq!(target(&layout, Movement::Next), Some(0));
        assert_eq!(target(&layout, Movement::Previous), Some(1));

        layout[1].control = false;
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
