//! A toolbar for the [rich text editor](mod@crate::forms::rich_text_editor):
//! text size, bold, italic, underline, strikethrough, lists, link and clear
//! formatting, each showing whether it is on at the caret.
//!
//! It is its own component so the app can put it anywhere: above the
//! editor, in a window's toolbar row or below a compose form. The buttons
//! are a [toolbar](mod@crate::application::toolbar), so whatever does not
//! fit moves into its More menu on a narrow window, and the text size is a
//! [dropdown menu](mod@crate::overlay::dropdown_menu).
//!
//! The app owns a [`State`]. After every change to the editor's
//! [`Content`], call [`State::sync`] so the buttons follow the caret. A
//! press returns an [`Output`]: apply [`Output::Format`] to the content and
//! give the editor focus back, and focus [`LINK_FIELD`] on
//! [`Output::EditLink`], when the address field opens.
//!
//! ```no_run
//! use iced::widget::{column, operation};
//! use iced::{Element, Task};
//! use iced_cube::formatting_toolbar::{self, Output, formatting_toolbar};
//! use iced_cube::rich_text_editor::{self, Action, Content, rich_text_editor};
//!
//! const BODY: &str = "body";
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Edit(Action),
//!     Format(formatting_toolbar::Event),
//! }
//!
//! #[derive(Default)]
//! struct Compose {
//!     body: Content,
//!     formatting: formatting_toolbar::State,
//! }
//!
//! impl Compose {
//!     fn update(&mut self, message: Message) -> Task<Message> {
//!         let output = match message {
//!             Message::Edit(action) => {
//!                 self.body.perform(action);
//!                 self.formatting.sync(&self.body)
//!             }
//!             Message::Format(event) => self.formatting.update(event, &self.body),
//!         };
//!         match output {
//!             Some(Output::Format(action)) => {
//!                 self.body.perform(action);
//!                 self.formatting.sync(&self.body);
//!                 operation::focus(BODY)
//!             }
//!             Some(Output::EditLink) => operation::focus(formatting_toolbar::LINK_FIELD),
//!             Some(Output::Closed) => operation::focus(BODY),
//!             Some(Output::Unfocus) => formatting_toolbar::unfocus(),
//!             None => Task::none(),
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         column![
//!             formatting_toolbar(&self.formatting).on_event(Message::Format),
//!             rich_text_editor(&self.body).id(BODY).on_action(Message::Edit),
//!         ]
//!         .into()
//!     }
//! }
//! ```

use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::keyboard;
use iced::widget::{Id, column, container, row};
use iced::{Alignment, Element, Length, Rectangle, Renderer, Theme, mouse};

use crate::forms::rich_text_editor::{
    Action, BlockKind, Content, Format, Mark, Shortcut, Size, default_keymap,
};
use crate::keys::{self, Keymap};
use crate::lucide;
use crate::overlay::dropdown_menu::{self, dropdown_menu};
use crate::overlay::tooltip::Position;
use crate::primitives::button::{self, Variant, button};
use crate::primitives::icon_button::icon_button;
use crate::primitives::input::{Shortcuts, input};
use crate::primitives::separator::vertical_separator;
use crate::theme::{Tokens, space};

use super::toolbar::{self, PADDING, SEPARATOR_HEIGHT, separator, toggle, toolbar};

/// The id of the link address field, to focus it on [`Output::EditLink`].
pub const LINK_FIELD: &str = "iced-cube-formatting-link";
/// Width of the text size button, wide enough for the longest size name.
pub const SIZE_WIDTH: f32 = 96.0;

/// A button in the strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Bullets,
    Numbers,
    Link,
    Clear,
}

impl Tool {
    pub const ALL: [Tool; 8] = [
        Tool::Bold,
        Tool::Italic,
        Tool::Underline,
        Tool::Strikethrough,
        Tool::Bullets,
        Tool::Numbers,
        Tool::Link,
        Tool::Clear,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Tool::Bold => "Bold",
            Tool::Italic => "Italic",
            Tool::Underline => "Underline",
            Tool::Strikethrough => "Strikethrough",
            Tool::Bullets => "Bulleted list",
            Tool::Numbers => "Numbered list",
            Tool::Link => "Link",
            Tool::Clear => "Clear formatting",
        }
    }

    /// The editor shortcut that does the same.
    pub const fn shortcut(self) -> Shortcut {
        match self {
            Tool::Bold => Shortcut::Bold,
            Tool::Italic => Shortcut::Italic,
            Tool::Underline => Shortcut::Underline,
            Tool::Strikethrough => Shortcut::Strikethrough,
            Tool::Bullets => Shortcut::BulletedList,
            Tool::Numbers => Shortcut::NumberedList,
            Tool::Link => Shortcut::Link,
            Tool::Clear => Shortcut::ClearFormatting,
        }
    }

    /// The format a press applies. The link opens the address field instead.
    pub fn format(self) -> Option<Format> {
        Some(match self {
            Tool::Bold => Format::Toggle(Mark::Bold),
            Tool::Italic => Format::Toggle(Mark::Italic),
            Tool::Underline => Format::Toggle(Mark::Underline),
            Tool::Strikethrough => Format::Toggle(Mark::Strikethrough),
            Tool::Bullets => Format::Block(BlockKind::Bullet),
            Tool::Numbers => Format::Block(BlockKind::Numbered),
            Tool::Clear => Format::Clear,
            Tool::Link => return None,
        })
    }
}

fn entries(keymap: &Keymap<Shortcut>) -> Vec<toolbar::Entry<Tool>> {
    let entry = |tool: Tool| {
        let glyph = match tool {
            Tool::Bold => lucide!(Bold),
            Tool::Italic => lucide!(Italic),
            Tool::Underline => lucide!(Underline),
            Tool::Strikethrough => lucide!(Strikethrough),
            Tool::Bullets => lucide!(List),
            Tool::Numbers => lucide!(ListOrdered),
            Tool::Link => lucide!(Link),
            Tool::Clear => lucide!(RemoveFormatting),
        };
        let entry = if tool == Tool::Clear {
            toolbar::button(tool, glyph, tool.label())
        } else {
            toggle(tool, glyph, tool.label(), false)
        };
        match keymap.chords(&tool.shortcut()).first() {
            Some(chord) => entry.shortcut(chord.to_string()),
            None => entry,
        }
    };
    vec![
        entry(Tool::Bold),
        entry(Tool::Italic),
        entry(Tool::Underline),
        entry(Tool::Strikethrough),
        separator(),
        entry(Tool::Bullets),
        entry(Tool::Numbers),
        separator(),
        entry(Tool::Link),
        entry(Tool::Clear),
    ]
}

fn sizes(selected: Option<Size>) -> dropdown_menu::State<Size> {
    dropdown_menu::State::new(
        Size::ALL.map(|size| dropdown_menu::radio_item(size, size.label(), Some(size) == selected)),
    )
}

/// Changes to the toolbar.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Toolbar(toolbar::Event<Tool>),
    Sizes(dropdown_menu::Event<Size>),
    /// The address in the link field changed.
    LinkChanged(String),
    /// Applies the address, or removes the link when it is empty.
    LinkSubmit,
    LinkRemove,
    LinkCancel,
}

/// What a press asks of the app.
#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    /// Apply this to the content, then give the editor focus back.
    Format(Action),
    /// The link field opened: focus [`LINK_FIELD`].
    EditLink,
    /// The link field, the size menu or the keyboard focus on the buttons
    /// closed with nothing to apply: give the editor focus back.
    Closed,
    /// The keyboard moved onto the buttons: take focus from the editor so
    /// the arrow keys reach the toolbar, for example with [`unfocus`].
    Unfocus,
}

/// A task that takes focus from whichever widget has it, for
/// [`Output::Unfocus`].
pub fn unfocus<T: Send + 'static>() -> iced::Task<T> {
    iced::advanced::widget::operate(iced::advanced::widget::operation::focusable::unfocus())
}

/// The buttons, the size menu and the link field.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    toolbar: toolbar::State<Tool>,
    sizes: dropdown_menu::State<Size>,
    size: Option<Size>,
    /// The address being edited while the link field is open.
    link: Option<String>,
    link_invalid: bool,
    has_link: bool,
    link_requests: Option<u64>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    /// A toolbar whose tooltips show the editor's
    /// [default shortcuts](crate::forms::rich_text_editor::default_keymap).
    pub fn new() -> Self {
        Self::with_keymap(&default_keymap())
    }

    /// A toolbar whose tooltips show the chords of `keymap`, for an editor
    /// given the same keymap.
    pub fn with_keymap(keymap: &Keymap<Shortcut>) -> Self {
        Self {
            toolbar: toolbar::State::new(entries(keymap)),
            sizes: sizes(Some(Size::Normal)),
            size: Some(Size::Normal),
            link: None,
            link_invalid: false,
            has_link: false,
            link_requests: None,
        }
    }

    /// Whether a button shows as on.
    pub fn is_on(&self, tool: Tool) -> bool {
        self.toolbar.is_on(tool)
    }

    /// The size at the caret, or `None` when the selection mixes sizes.
    pub fn size(&self) -> Option<Size> {
        self.size
    }

    /// The address in the link field, while it is open.
    pub fn link(&self) -> Option<&str> {
        self.link.as_deref()
    }

    pub fn toolbar(&self) -> &toolbar::State<Tool> {
        &self.toolbar
    }

    /// Matches the buttons to the formatting at the caret. Returns
    /// [`Output::EditLink`] when the editor's link shortcut was pressed
    /// since the last call, having opened the link field.
    pub fn sync(&mut self, content: &Content) -> Option<Output> {
        let formatting = content.formatting();
        for (tool, on) in [
            (Tool::Bold, formatting.bold),
            (Tool::Italic, formatting.italic),
            (Tool::Underline, formatting.underline),
            (Tool::Strikethrough, formatting.strikethrough),
            (Tool::Bullets, formatting.block == Some(BlockKind::Bullet)),
            (Tool::Numbers, formatting.block == Some(BlockKind::Numbered)),
            (Tool::Link, formatting.link.is_some()),
        ] {
            if self.toolbar.is_on(tool) != on {
                self.toolbar.set_on(tool, on);
            }
        }
        if self.size != formatting.size {
            self.size = formatting.size;
            let open = self.sizes.is_open();
            self.sizes = sizes(formatting.size);
            if open {
                let _ = self.sizes.update(dropdown_menu::Event::Open);
            }
        }
        self.has_link = formatting.link.is_some();
        let requests = content.link_requests();
        let seen = self.link_requests.replace(requests);
        if seen.is_some_and(|seen| seen != requests) {
            self.open_link(content);
            return Some(Output::EditLink);
        }
        None
    }

    fn open_link(&mut self, content: &Content) {
        let current = content.formatting().link;
        self.has_link = current.is_some();
        self.link_invalid = false;
        self.link = Some(current.map(|url| url.to_string()).unwrap_or_default());
    }

    /// Applies an event. `content` is read for the link at the caret.
    pub fn update(&mut self, event: Event, content: &Content) -> Option<Output> {
        let format = |format| Some(Output::Format(Action::Format(format)));
        match event {
            Event::Toolbar(event) => {
                let focused = self.toolbar.is_focused();
                let output = self.toolbar.update(event);
                let Some(output) = output else {
                    return match (focused, self.toolbar.is_focused()) {
                        (false, true) => Some(Output::Unfocus),
                        (true, false) => Some(Output::Closed),
                        _ => None,
                    };
                };
                let tool = match output {
                    dropdown_menu::Output::Activated(tool)
                    | dropdown_menu::Output::Toggled(tool, _)
                    | dropdown_menu::Output::Selected(tool) => tool,
                };
                // A press from the keyboard ends its focus on the buttons,
                // as the editor or link field takes focus next.
                for _ in 0..2 {
                    if self.toolbar.is_focused() {
                        let _ = self.toolbar.update(toolbar::Event::Close);
                    }
                }
                if let Some(change) = tool.format() {
                    return format(change);
                }
                if self.link.is_some() {
                    self.link = None;
                    self.sync_link_button();
                    return Some(Output::Closed);
                }
                self.open_link(content);
                self.sync_link_button();
                Some(Output::EditLink)
            }
            Event::Sizes(event) => {
                let open = self.sizes.is_open();
                match self.sizes.update(event) {
                    Some(
                        dropdown_menu::Output::Selected(size)
                        | dropdown_menu::Output::Activated(size)
                        | dropdown_menu::Output::Toggled(size, _),
                    ) => format(Format::Size(size)),
                    None if open && !self.sizes.is_open() => Some(Output::Closed),
                    None => None,
                }
            }
            Event::LinkChanged(address) => {
                self.link_invalid = false;
                if let Some(link) = &mut self.link {
                    *link = address;
                }
                None
            }
            Event::LinkSubmit => {
                let address = self.link.as_deref()?.trim().to_owned();
                if address.is_empty() {
                    self.link = None;
                    return if self.has_link {
                        format(Format::Unlink)
                    } else {
                        Some(Output::Closed)
                    };
                }
                if crate::forms::rich_text_editor::normalise_link(&address).is_none() {
                    self.link_invalid = true;
                    return None;
                }
                self.link = None;
                format(Format::Link(address))
            }
            Event::LinkRemove => {
                self.link = None;
                format(Format::Unlink)
            }
            Event::LinkCancel => {
                self.link = None;
                Some(Output::Closed)
            }
        }
    }

    /// A press flips a toggle straight away; the link button shows the
    /// caret's link rather than the press.
    fn sync_link_button(&mut self) {
        if self.toolbar.is_on(Tool::Link) != self.has_link {
            self.toolbar.set_on(Tool::Link, self.has_link);
        }
    }

    /// Resolves a key press from [`keys::subscription`] for the strip,
    /// as [`toolbar::State::key_event`] does.
    pub fn key_event(&self, keymap: &Keymap<toolbar::Action>, key: &keys::Event) -> Option<Event> {
        self.toolbar.key_event(keymap, key).map(Event::Toolbar)
    }
}

/// A formatting toolbar builder. Convert it into an [`Element`] to render.
pub struct FormattingToolbar<'a, Message> {
    state: &'a State,
    tooltip: Option<Position>,
    on_event: Option<Box<dyn Fn(Event) -> Message + 'a>>,
}

impl<Message> std::fmt::Debug for FormattingToolbar<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormattingToolbar")
            .field("state", self.state)
            .field("tooltip", &self.tooltip)
            .finish_non_exhaustive()
    }
}

/// Renders the size menu, the buttons that fit and, while it is open, the
/// link field. Without [`on_event`](FormattingToolbar::on_event) every
/// control renders disabled.
pub fn formatting_toolbar<'a, Message>(state: &'a State) -> FormattingToolbar<'a, Message> {
    FormattingToolbar {
        state,
        tooltip: Some(Position::Bottom),
        on_event: None,
    }
}

impl<'a, Message> FormattingToolbar<'a, Message> {
    /// Where the button tooltips appear, or `None` for none. Defaults to
    /// below, as for the toolbar: at the top of a window there is no room
    /// above, and a tooltip squeezed over its button would catch the next
    /// click.
    pub fn tooltip(mut self, position: Option<Position>) -> Self {
        self.tooltip = position;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

impl<'a, Message: Clone + 'a> From<FormattingToolbar<'a, Message>> for Element<'a, Message> {
    fn from(bar: FormattingToolbar<'a, Message>) -> Self {
        let FormattingToolbar {
            state,
            tooltip,
            on_event,
        } = bar;
        let on_event: Option<Rc<dyn Fn(Event) -> Message + 'a>> = on_event.map(Rc::from);
        let send = |event: Event| on_event.as_ref().map(|on_event| on_event(event));

        let label = state.size.map_or("Mixed", Size::label);
        let trigger = button(label)
            .variant(Variant::Ghost)
            .size(button::Size::Sm)
            .trailing_icon(lucide!(ChevronDown))
            .width(SIZE_WIDTH)
            .on_press_maybe(send(Event::Sizes(dropdown_menu::Event::Toggle)));
        let size_menu: Element<'a, Message> = match on_event.clone() {
            Some(on_event) => dropdown_menu(&state.sizes, trigger)
                .width(160.0)
                .on_event(move |event| on_event(Event::Sizes(event)))
                .into(),
            None => trigger.into(),
        };

        let mut strip = toolbar(&state.toolbar).tooltip(tooltip).edge(false);
        if let Some(on_event) = on_event.clone() {
            strip = strip.on_event(move |event| on_event(Event::Toolbar(event)));
        }

        let bar = container(
            row![
                container(size_menu).padding(PADDING),
                container(vertical_separator()).height(SEPARATOR_HEIGHT),
                strip,
            ]
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .style(|theme| toolbar::bar_style(&Tokens::of(theme)));

        let Some(address) = &state.link else {
            return bar.into();
        };
        // Escape closes the field rather than only leaving it.
        let field = input("Paste or type a link", address)
            .id(Id::new(LINK_FIELD))
            .shortcuts(Shortcuts::all().without(Shortcut::EscapeUnfocus))
            .invalid(state.link_invalid)
            .on_input_maybe(
                on_event
                    .clone()
                    .map(|on_event| move |text| on_event(Event::LinkChanged(text))),
            )
            .on_submit_maybe(send(Event::LinkSubmit));
        let apply = button("Apply")
            .size(button::Size::Sm)
            .on_press_maybe(send(Event::LinkSubmit));
        let mut controls = row![field, apply]
            .spacing(space::SM)
            .align_y(Alignment::Center);
        if state.has_link {
            controls = controls.push(
                button("Remove")
                    .variant(Variant::Ghost)
                    .size(button::Size::Sm)
                    .on_press_maybe(send(Event::LinkRemove)),
            );
        }
        let close = icon_button(lucide!(X))
            .label("Close")
            .tooltip(None)
            .size(button::Size::Sm)
            .on_press_maybe(send(Event::LinkCancel));
        controls = controls.push(close);
        let controls: Element<'a, Message> = container(controls)
            .padding([space::XS, space::SM])
            .width(Length::Fill)
            .style(|theme| toolbar::bar_style(&Tokens::of(theme)))
            .into();
        let controls = match send(Event::LinkCancel) {
            Some(cancel) => Element::new(OnEscape {
                content: controls,
                message: cancel,
            }),
            None => controls,
        };
        column![bar, controls].into()
    }
}

/// Sends `message` on an Escape press that the content leaves, so Escape
/// closes the open link field.
struct OnEscape<'a, Message> {
    content: Element<'a, Message>,
    message: Message,
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for OnEscape<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> iced::Size<Length> {
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
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
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
        if shell.is_event_captured() {
            return;
        }
        if let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event
            && *key == keyboard::Key::Named(keyboard::key::Named::Escape)
            && modifiers.is_empty()
        {
            shell.publish(self.message.clone());
            shell.capture_event();
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
        translation: iced::Vector,
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
    use crate::forms::rich_text_editor::{Motion, Position};

    fn press(state: &mut State, content: &Content, tool: Tool) -> Option<Output> {
        state.update(Event::Toolbar(toolbar::Event::Press(tool)), content)
    }

    #[test]
    fn every_tool_but_link_formats() {
        for tool in Tool::ALL {
            assert_eq!(tool.format().is_none(), tool == Tool::Link, "{tool:?}");
        }
        let mut state = State::new();
        let content = Content::with_text("text");
        assert_eq!(
            press(&mut state, &content, Tool::Bold),
            Some(Output::Format(Action::Format(Format::Toggle(Mark::Bold))))
        );
        assert_eq!(
            press(&mut state, &content, Tool::Numbers),
            Some(Output::Format(Action::Format(Format::Block(
                BlockKind::Numbered
            ))))
        );
    }

    #[test]
    fn tooltips_show_the_shortcuts() {
        let state = State::new();
        let bold = state
            .toolbar()
            .item(Tool::Bold)
            .and_then(|item| item.shortcut.clone());
        let expected = if cfg!(target_os = "macos") {
            "Cmd+B"
        } else {
            "Ctrl+B"
        };
        assert_eq!(bold.as_deref(), Some(expected));
        let bare = State::with_keymap(&Keymap::new());
        assert!(
            bare.toolbar()
                .item(Tool::Bold)
                .is_some_and(|item| item.shortcut.is_none())
        );
    }

    #[test]
    fn sync_follows_the_caret() {
        let mut content = Content::with_text("plain");
        let mut state = State::new();
        assert_eq!(state.sync(&content), None);
        content.perform(Action::SelectAll);
        for format in [
            Format::Toggle(Mark::Bold),
            Format::Toggle(Mark::Underline),
            Format::Size(Size::Large),
            Format::Block(BlockKind::Bullet),
        ] {
            content.perform(Action::Format(format));
        }
        state.sync(&content);
        assert!(state.is_on(Tool::Bold) && state.is_on(Tool::Underline));
        assert!(!state.is_on(Tool::Italic) && !state.is_on(Tool::Numbers));
        assert!(state.is_on(Tool::Bullets));
        assert_eq!(state.size(), Some(Size::Large));
        assert!(state.sizes.is_checked(Size::Large));
        content.perform(Action::Move(Motion::DocumentEnd));
        content.perform(Action::Edit(crate::forms::rich_text_editor::Edit::Insert(
            " x".into(),
        )));
        content.perform(Action::Select(Motion::DocumentStart));
        content.perform(Action::Format(Format::Size(Size::Small)));
        content.perform(Action::Move(Motion::To(Position::new(0, 0))));
        content.perform(Action::Select(Motion::DocumentEnd));
        state.sync(&content);
        assert_eq!(state.size(), Some(Size::Small));
    }

    #[test]
    fn sizes_come_from_the_menu() {
        let mut state = State::new();
        let content = Content::new();
        assert_eq!(
            state.update(
                Event::Sizes(dropdown_menu::Event::Activate(Size::Huge)),
                &content
            ),
            Some(Output::Format(Action::Format(Format::Size(Size::Huge))))
        );
    }

    #[test]
    fn the_link_field_opens_applies_and_cancels() {
        let mut content = Content::with_text("see here");
        let mut state = State::new();
        state.sync(&content);
        assert_eq!(
            press(&mut state, &content, Tool::Link),
            Some(Output::EditLink)
        );
        assert_eq!(state.link(), Some(""));
        assert!(!state.is_on(Tool::Link));
        state.update(Event::LinkChanged("javascript:alert(1)".into()), &content);
        assert_eq!(state.update(Event::LinkSubmit, &content), None);
        assert!(state.link_invalid);
        state.update(Event::LinkChanged("example.com".into()), &content);
        let Some(Output::Format(action)) = state.update(Event::LinkSubmit, &content) else {
            panic!("a link to apply");
        };
        assert_eq!(action, Action::Format(Format::Link("example.com".into())));
        assert_eq!(state.link(), None);
        content.perform(action);
        state.sync(&content);
        assert!(state.is_on(Tool::Link));

        assert_eq!(
            press(&mut state, &content, Tool::Link),
            Some(Output::EditLink)
        );
        assert_eq!(state.link(), Some("https://example.com"));
        assert!(state.is_on(Tool::Link));
        assert_eq!(
            state.update(Event::LinkRemove, &content),
            Some(Output::Format(Action::Format(Format::Unlink)))
        );
        press(&mut state, &content, Tool::Link);
        assert_eq!(
            state.update(Event::LinkCancel, &content),
            Some(Output::Closed)
        );
        press(&mut state, &content, Tool::Link);
        state.update(Event::LinkChanged("  ".into()), &content);
        assert_eq!(
            state.update(Event::LinkSubmit, &content),
            Some(Output::Format(Action::Format(Format::Unlink)))
        );
    }

    #[test]
    fn keyboard_focus_on_the_buttons_takes_and_returns_focus() {
        let mut state = State::new();
        let content = Content::with_text("x");
        assert_eq!(
            state.update(Event::Toolbar(toolbar::Event::Focus), &content),
            Some(Output::Unfocus)
        );
        assert_eq!(
            state.update(Event::Toolbar(toolbar::Event::Next), &content),
            None
        );
        assert_eq!(
            state.update(Event::Toolbar(toolbar::Event::Close), &content),
            Some(Output::Closed)
        );
    }

    #[test]
    fn closing_the_size_menu_returns_focus() {
        let mut state = State::new();
        let content = Content::new();
        assert_eq!(
            state.update(Event::Sizes(dropdown_menu::Event::Toggle), &content),
            None
        );
        assert_eq!(
            state.update(Event::Sizes(dropdown_menu::Event::Close), &content),
            Some(Output::Closed)
        );
        assert_eq!(
            state.update(Event::Sizes(dropdown_menu::Event::Close), &content),
            None
        );
    }

    #[test]
    fn pressing_link_again_closes_the_field() {
        let mut state = State::new();
        let content = Content::with_text("x");
        assert_eq!(
            press(&mut state, &content, Tool::Link),
            Some(Output::EditLink)
        );
        assert_eq!(
            press(&mut state, &content, Tool::Link),
            Some(Output::Closed)
        );
        assert_eq!(state.link(), None);
        assert!(!state.is_on(Tool::Link));
    }

    #[test]
    fn the_link_shortcut_opens_the_field_once() {
        let mut content = Content::with_text("x");
        let mut state = State::new();
        state.sync(&content);
        content.perform(Action::RequestLink);
        assert_eq!(state.sync(&content), Some(Output::EditLink));
        assert_eq!(state.link(), Some(""));
        assert_eq!(state.sync(&content), None);
    }
}
