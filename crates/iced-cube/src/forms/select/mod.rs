//! A dropdown for choosing one value from a list.
//!
//! The select opens and closes its own list, so the app only handles the
//! chosen value. The list floats on the shared
//! [anchored layer](crate::overlay::anchored) in the
//! [menu look](crate::overlay::menu), with a check mark on the selected
//! option. Escape or a click outside closes it.
//!
//! A long list shows [`MAX_VISIBLE_ROWS`] rows and scrolls, with a
//! scrollbar, and is never taller than the room the window has. It opens
//! with the selected option in the middle, and the highlight stays in view
//! as the keys move it. A list that fits looks as it always has.
//!
//! An enabled select takes keyboard focus from iced's focus operations,
//! such as a dialog's Tab order, and rings while focused. Space, Enter or
//! ArrowDown then opens the list with the selected option highlighted.
//! In the open list the arrow keys, Page Up, Page Down, Home and End move
//! the highlight, Space or Enter chooses it and closes the list, and Tab
//! closes it and moves on. Typing letters moves the highlight to the
//! first option starting with them.
//!
//! [`searchable`](Select::searchable) adds a search field that filters the
//! options as you type, and [`on_custom`](Select::on_custom) lets the user
//! enter a value that is not in the list.

use std::borrow::Cow;

use iced::advanced::layout;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Layout, Shell, overlay, renderer};
use iced::keyboard::{self, key::Named};
use iced::time::Duration;
use iced::widget::pick_list::{self, Status};
use iced::widget::{container, row};
use iced::{
    Alignment, Background, Border, Color, Element, Event, Length, Rectangle, Renderer, Size, Theme,
    Vector, mouse, touch,
};

use crate::data::ellipsis::ellipsis;
use crate::focus;
use crate::forms::matching;
use crate::icon::{opacity, themed};
use crate::keys::{self, Chord, Keymap};
use crate::overlay::anchored::{self, Align, Behaviour, Placement, Side};
use crate::overlay::menu::{self, Parts, ROW_PADDING, RowStatus, Trailing};
pub use crate::primitives::input::{Shortcut, Shortcuts};
use crate::theme::{Tokens, fade, mix, radius, space};

mod list;
mod navigate;

use list::{List, Search};

/// Height of the closed select in logical pixels.
pub const HEIGHT: f32 = 36.0;

/// Rows the open list shows before it scrolls, unless
/// [`max_visible_rows`](Select::max_visible_rows) says otherwise.
pub const MAX_VISIBLE_ROWS: usize = 8;

/// How long a pause in typing starts a new type-ahead search.
pub const TYPE_AHEAD_PAUSE: Duration = Duration::from_secs(1);

/// Shown in the open list of a select with no options.
pub const NO_OPTIONS: &str = "No options";

/// Shown when the search matches no option.
pub const DEFAULT_EMPTY: &str = "No results found.";

/// Shown when the app turns a typed value down.
pub const DEFAULT_NOT_VALID: &str = "Not valid";

const CHEVRON_SIZE: f32 = 16.0;
const LINE_HEIGHT: f32 = 20.0;

/// A select builder. Convert it into an [`Element`] to render.
///
/// A select without a message is rendered disabled.
pub struct Select<'a, T: Clone, Message> {
    options: Cow<'a, [T]>,
    selected: Option<T>,
    placeholder: Option<String>,
    width: Length,
    on_select: Option<Box<dyn Fn(T) -> Message + 'a>>,
    leading: Option<Leading<'a, T, Message>>,
    max_rows: usize,
    scrollable: bool,
    searchable: bool,
    on_custom: Option<OnCustom<'a, Message>>,
    search_placeholder: Option<String>,
    shortcuts: Shortcuts,
    empty: String,
    not_valid: String,
    type_ahead_pause: Duration,
}

type Leading<'a, T, Message> = Box<dyn Fn(&T) -> Option<Element<'a, Message>> + 'a>;
type OnCustom<'a, Message> = Box<dyn Fn(String) -> Option<Message> + 'a>;

impl<T: Clone + std::fmt::Debug, Message> std::fmt::Debug for Select<'_, T, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Select")
            .field("options", &self.options)
            .field("selected", &self.selected)
            .field("placeholder", &self.placeholder)
            .field("enabled", &self.on_select.is_some())
            .field("leading", &self.leading.is_some())
            .field("max_rows", &self.max_rows)
            .field("scrollable", &self.scrollable)
            .field("searchable", &self.searchable)
            .field("shortcuts", &self.shortcuts)
            .field("custom", &self.on_custom.is_some())
            .finish_non_exhaustive()
    }
}

/// Creates a select over `options`, showing `selected` when it is set.
/// Options are labelled with their `Display` text.
pub fn select<'a, T, Message>(
    options: impl Into<Cow<'a, [T]>>,
    selected: Option<T>,
) -> Select<'a, T, Message>
where
    T: ToString + PartialEq + Clone,
{
    Select {
        options: options.into(),
        selected,
        placeholder: None,
        width: Length::Fixed(200.0),
        on_select: None,
        leading: None,
        max_rows: MAX_VISIBLE_ROWS,
        scrollable: true,
        searchable: false,
        on_custom: None,
        search_placeholder: None,
        shortcuts: Shortcuts::all(),
        empty: DEFAULT_EMPTY.to_owned(),
        not_valid: DEFAULT_NOT_VALID.to_owned(),
        type_ahead_pause: TYPE_AHEAD_PAUSE,
    }
}

impl<'a, T: Clone, Message> Select<'a, T, Message> {
    /// Text shown while nothing is selected.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the message emitted with the chosen option.
    pub fn on_select(mut self, on_select: impl Fn(T) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// Sets the message emitted with the chosen option. `None` disables the
    /// select.
    pub fn on_select_maybe(mut self, on_select: Option<impl Fn(T) -> Message + 'a>) -> Self {
        self.on_select = on_select.map(|f| Box::new(f) as Box<dyn Fn(T) -> Message + 'a>);
        self
    }

    /// Draws an element before an option's label, such as a colour
    /// [swatch](crate::theme::swatch) or an icon, both in the open list and
    /// in the field while that option is selected. Options it returns
    /// `None` for show their label alone.
    pub fn leading(mut self, leading: impl Fn(&T) -> Option<Element<'a, Message>> + 'a) -> Self {
        self.leading = Some(Box::new(leading));
        self
    }

    /// The most rows the open list shows before it scrolls. Defaults to
    /// [`MAX_VISIBLE_ROWS`]; at least one row always shows.
    pub fn max_visible_rows(mut self, rows: usize) -> Self {
        self.max_rows = rows.max(1);
        self
    }

    /// Whether a long list scrolls. On by default. Turned off, the list is
    /// as tall as its options, however many there are.
    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }

    /// Adds a search field to the open list: typing filters the options by
    /// their labels, ignoring case, and highlights the first match. The
    /// query is cleared when the list closes. Off by default.
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    /// Lets the user type a value that is not in the list. Enter with text
    /// that matches no option sends the text, trimmed, to `on_custom`,
    /// which returns the message for a valid value or `None` to turn it
    /// down. A value turned down stays in the search field, in red with
    /// the [`not_valid`](Self::not_valid) text below, until it is edited.
    ///
    /// With [`searchable`](Self::searchable) on as well, Enter chooses the
    /// highlighted match and only sends the text when nothing matches.
    /// Without it, the list is not filtered and the text only chooses an
    /// option whose whole label it is.
    pub fn on_custom(mut self, on_custom: impl Fn(String) -> Option<Message> + 'a) -> Self {
        self.on_custom = Some(Box::new(on_custom));
        self
    }

    /// Text in the empty search field. Defaults to "Search", or "Type a
    /// value" when the list only takes custom values.
    pub fn search_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.search_placeholder = Some(placeholder.into());
        self
    }

    /// The editing shortcuts of the search field. Defaults to
    /// [`Shortcuts::all`]; Escape always closes the list instead of
    /// leaving the field.
    pub fn shortcuts(mut self, shortcuts: Shortcuts) -> Self {
        self.shortcuts = shortcuts;
        self
    }

    /// Shown in place of rows when the search matches nothing. Defaults
    /// to [`DEFAULT_EMPTY`].
    pub fn empty(mut self, empty: impl Into<String>) -> Self {
        self.empty = empty.into();
        self
    }

    /// Shown in place of rows when [`on_custom`](Self::on_custom) turns the
    /// typed value down, such as the format expected. Defaults to
    /// [`DEFAULT_NOT_VALID`].
    pub fn not_valid(mut self, text: impl Into<String>) -> Self {
        self.not_valid = text.into();
        self
    }

    /// How long a pause in typing starts a new type-ahead search. Defaults
    /// to [`TYPE_AHEAD_PAUSE`].
    pub fn type_ahead_pause(mut self, pause: Duration) -> Self {
        self.type_ahead_pause = pause;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_select.is_some()
    }
}

impl<'a, T, Message> From<Select<'a, T, Message>> for Element<'a, Message>
where
    T: ToString + PartialEq + Clone + 'a,
    Message: Clone + 'a,
{
    fn from(select: Select<'a, T, Message>) -> Self {
        let Select {
            options,
            selected,
            placeholder,
            width,
            on_select,
            leading,
            max_rows,
            scrollable,
            searchable,
            on_custom,
            search_placeholder,
            shortcuts,
            empty,
            not_valid,
            type_ahead_pause,
        } = select;
        let enabled = on_select.is_some();
        let leading_of = |option: &T| leading.as_ref().and_then(|leading| leading(option));
        let label = selected.as_ref().map(ToString::to_string);
        let has_value = label.is_some();

        let text_colour = move |theme: &Theme| {
            let style = style(&Tokens::of(theme), Status::Active, enabled);
            if has_value {
                style.text_color
            } else {
                style.placeholder_color
            }
        };
        let chevron = themed(
            crate::lucide!(ChevronDown),
            CHEVRON_SIZE,
            opacity(enabled),
            move |theme| style(&Tokens::of(theme), Status::Active, enabled).handle_color,
        );
        let selected_leading = selected.as_ref().and_then(leading_of);
        // A long label ends in an ellipsis rather than running under the chevron.
        let field = row![]
            .push(selected_leading)
            .push(ellipsis(label.or(placeholder).unwrap_or_default()).colour(text_colour))
            .push(chevron)
            .spacing(space::SM)
            .align_y(Alignment::Center)
            .padding([(HEIGHT - LINE_HEIGHT) / 2.0, space::MD])
            .width(width)
            .height(HEIGHT);

        let rows = options
            .iter()
            .map(|option| {
                let is_selected = selected.as_ref() == Some(option);
                option_row(&option.to_string(), leading_of(option), is_selected)
            })
            .collect();
        let choices = options
            .iter()
            .map(|option| {
                on_select
                    .as_deref()
                    .map(|on_select| on_select(option.clone()))
            })
            .collect();
        let search = (searchable || on_custom.is_some()).then(|| {
            let placeholder = search_placeholder
                .unwrap_or_else(|| if searchable { "Search" } else { "Type a value" }.to_owned());
            Search {
                filters: searchable,
                on_custom,
                placeholder,
                shortcuts,
            }
        });
        let max_rows = scrollable.then_some(max_rows);
        let list = List {
            rows,
            labels: matching::lowercase(&options),
            choices,
            selected: selected
                .as_ref()
                .and_then(|value| options.iter().position(|option| option == value)),
            max_rows,
            search,
            pause: type_ahead_pause,
            empty: NO_OPTIONS.to_owned(),
            no_results: empty,
            not_valid,
        };
        let list = menu::surface(Element::new(list)).width(Length::Fill).into();

        Element::new(Picker {
            field: field.into(),
            list,
            enabled,
            behaviour: Behaviour {
                placement: Placement::new(Side::Bottom, Align::Start),
                match_width: true,
                fit_height: max_rows.is_some(),
                closes_itself: true,
                ..Behaviour::default()
            },
        })
    }
}

/// One option in the open list. The list draws the highlight behind it
/// and sets the text colour.
fn option_row<'a, Message: 'a>(
    label: &str,
    leading: Option<Element<'a, Message>>,
    selected: bool,
) -> Element<'a, Message> {
    let trailing = if selected {
        Trailing::Check
    } else {
        Trailing::None
    };
    let parts = Parts {
        trailing,
        ..Parts::label(label)
    };
    let content = menu::content(parts, RowStatus::Idle, false);
    let content = match leading {
        Some(leading) => row![leading, content]
            .spacing(space::SM)
            .align_y(Alignment::Center),
        None => content,
    };
    menu::inset(container(content).padding(ROW_PADDING).width(Length::Fill))
}

/// The select widget: draws the field and floats the list while open.
struct Picker<'a, Message> {
    field: Element<'a, Message>,
    list: Element<'a, Message>,
    enabled: bool,
    behaviour: Behaviour<'a, Message>,
}

#[derive(Debug, Default)]
struct PickerState {
    panel: anchored::State,
    hovered: bool,
    focus: focus::State,
}

/// Whether a key press opens the list of a focused, closed select.
fn opens_list(key: Named) -> bool {
    matches!(key, Named::Space | Named::Enter | Named::ArrowDown)
}

/// Whether a key press chooses the highlighted option of an open list.
fn chooses(key: Named) -> bool {
    matches!(key, Named::Space | Named::Enter)
}

impl<Message> Picker<'_, Message> {
    /// Opens the list with a fresh highlight on the selected option.
    fn open(&self, tree: &mut Tree) {
        if let Some(list) = tree.children.get_mut(1) {
            *list = Tree::new(&self.list);
        }
        let panel = &mut tree.state.downcast_mut::<PickerState>().panel;
        panel.open = true;
        panel.side = None;
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Picker<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<PickerState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(PickerState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.field), Tree::new(&self.list)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.field, &self.list]);
    }

    fn size(&self) -> Size<Length> {
        self.field.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.field
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
        let state = tree.state.downcast_ref::<PickerState>();
        let bounds = layout.bounds();
        let hovered = self.enabled && cursor.is_over(bounds);
        let status = field_status(state.panel.open, hovered);
        let field = self::style(&Tokens::of(theme), status, self.enabled);
        <Renderer as renderer::Renderer>::fill_quad(
            renderer,
            renderer::Quad {
                bounds,
                border: field.border,
                ..renderer::Quad::default()
            },
            field.background,
        );
        self.field.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        if self.enabled && state.focus.is_focused() {
            focus::draw_ring(renderer, theme, bounds, radius::MD);
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if self.enabled {
            let state = tree.state.downcast_mut::<PickerState>();
            focus::report(operation, None, layout.bounds(), &mut state.focus);
        }
        self.field
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        _viewport: &Rectangle,
    ) {
        if !self.enabled {
            return;
        }
        let state = tree.state.downcast_mut::<PickerState>();
        if state.focus.update(event) {
            shell.request_redraw();
        }
        // An open list keeps every key press its panel leaves, so keymaps
        // behind it do not act while it is open. The list sends the
        // highlighted option on Space or Enter and leaves the key here,
        // where it closes the list. Tab closes it and goes on to move focus.
        if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event
            && state.panel.open
        {
            if *key == keyboard::Key::Named(Named::Tab) {
                state.panel.open = false;
                shell.request_redraw();
                return;
            }
            if focus::named_press(event).is_some_and(chooses) {
                state.panel.open = false;
                shell.request_redraw();
            }
            shell.capture_event();
            return;
        }
        if state.focus.is_focused() && focus::named_press(event).is_some_and(opens_list) {
            self.open(tree);
            shell.capture_event();
            shell.request_redraw();
            return;
        }
        let over = cursor.is_over(layout.bounds());
        if over != state.hovered {
            state.hovered = over;
            shell.request_redraw();
        }
        let pressed = matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerPressed { .. })
        );
        if !pressed || !over {
            return;
        }
        if state.panel.open {
            state.panel.open = false;
        } else {
            self.open(tree);
        }
        shell.capture_event();
        shell.request_redraw();
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.enabled && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<PickerState>();
        if !self.enabled || !state.panel.open {
            return None;
        }
        let list_tree = children.get_mut(1)?;
        Some(anchored::floating(
            &mut self.list,
            list_tree,
            &mut state.panel,
            &self.behaviour,
            layout.bounds() + translation,
        ))
    }
}

/// The field's status: opened while the list shows, hovered under the
/// pointer, active otherwise.
fn field_status(open: bool, hovered: bool) -> Status {
    match (open, hovered) {
        (true, is_hovered) => Status::Opened { is_hovered },
        (false, true) => Status::Hovered,
        (false, false) => Status::Active,
    }
}

/// The iced pick list style for a status.
pub fn style(tokens: &Tokens, status: Status, enabled: bool) -> pick_list::Style {
    let border = match status {
        Status::Active => tokens.border,
        Status::Hovered => mix(tokens.border, tokens.foreground, 0.3),
        Status::Opened { .. } => mix(tokens.border, tokens.foreground, 0.6),
    };
    let style = pick_list::Style {
        text_color: tokens.foreground,
        placeholder_color: tokens.muted_foreground,
        handle_color: tokens.muted_foreground,
        // Transparent so the trigger takes the colour of its surface.
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: border,
            width: 1.0,
            radius: radius::MD.into(),
        },
    };

    if enabled {
        return style;
    }
    pick_list::Style {
        text_color: fade(tokens.foreground, 0.5),
        placeholder_color: fade(tokens.muted_foreground, 0.5),
        handle_color: fade(tokens.muted_foreground, 0.5),
        background: Background::Color(tokens.disabled_field()),
        border: Border {
            color: fade(tokens.border, 0.5),
            ..style.border
        },
    }
}

/// What a select keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
}

impl Action {
    /// The option to select after this action, to send through the
    /// select's `on_select` message. Stops at the ends of the list, and
    /// returns `None` when the selection would not change. With nothing
    /// selected, `Next` picks the first option and `Previous` the last.
    pub fn apply<T: PartialEq + Clone>(self, options: &[T], selected: Option<&T>) -> Option<T> {
        let current = selected.and_then(|value| options.iter().position(|option| option == value));
        let target = match (self, current) {
            (Action::Next, None) => 0,
            (Action::Previous, None) => options.len().checked_sub(1)?,
            (Action::Next, Some(index)) => index + 1,
            (Action::Previous, Some(index)) => index.checked_sub(1)?,
        };
        options.get(target).cloned()
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[Action::Next, Action::Previous];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Next => "Next",
            Action::Previous => "Previous",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Selects the next option, stopping at the last.",
            Action::Previous => "Selects the previous option, stopping at the first.",
        }
    }
}

/// The default select shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowDown` | [`Action::Next`] |
/// | `ArrowUp` | [`Action::Previous`] |
///
/// These change the value without opening the menu. Shortcuts are
/// app-wide, so an app with several selects routes them to the one it
/// considers current. A select with keyboard focus keeps ArrowDown, which
/// opens its list instead.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowDown), Action::Next)
        .bind(Chord::named(Named::ArrowUp), Action::Previous)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_and_overrides() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "ArrowDown"), Some(Action::Next));
        assert_eq!(press(&keymap, "ArrowUp"), Some(Action::Previous));
        let custom = keymap
            .bind("J".parse().unwrap(), Action::Next)
            .unbind(&"ArrowDown".parse().unwrap());
        assert_eq!(press(&custom, "j"), Some(Action::Next));
        assert_eq!(press(&custom, "ArrowDown"), None);
    }

    #[test]
    fn next_and_previous_stop_at_the_ends() {
        let options = ["a", "b", "c"];
        assert_eq!(Action::Next.apply(&options, Some(&"a")), Some("b"));
        assert_eq!(Action::Next.apply(&options, Some(&"c")), None);
        assert_eq!(Action::Previous.apply(&options, Some(&"b")), Some("a"));
        assert_eq!(Action::Previous.apply(&options, Some(&"a")), None);
    }

    #[test]
    fn without_a_selection_next_and_previous_pick_an_end() {
        let options = ["a", "b", "c"];
        assert_eq!(Action::Next.apply(&options, None), Some("a"));
        assert_eq!(Action::Previous.apply(&options, None), Some("c"));
        assert_eq!(Action::Next.apply(&options, Some(&"z")), Some("a"));
        let empty: [&str; 0] = [];
        assert_eq!(Action::Next.apply(&empty, None), None);
        assert_eq!(Action::Previous.apply(&empty, None), None);
    }

    const STATES: [Status; 4] = [
        Status::Active,
        Status::Hovered,
        Status::Opened { is_hovered: false },
        Status::Opened { is_hovered: true },
    ];

    #[test]
    fn default_builder_has_no_selection_and_is_disabled() {
        let s: Select<'_, &str, ()> = select(vec!["a", "b"], None);
        assert_eq!(s.options.len(), 2);
        assert!(s.selected.is_none());
        assert!(s.placeholder.is_none());
        assert_eq!(s.width, Length::Fixed(200.0));
        assert!(!s.is_enabled());
        assert!(s.leading.is_none());
    }

    #[test]
    fn leading_is_asked_per_option() {
        let s: Select<'_, &str, ()> = select(vec!["red", "plain"], None).leading(|option| {
            (*option == "red")
                .then(|| crate::theme::swatch_color(Color::from_rgb(1.0, 0.0, 0.0)).into())
        });
        let Some(leading) = s.leading.as_ref() else {
            panic!("leading is set");
        };
        assert!(leading(&"red").is_some());
        assert!(leading(&"plain").is_none());
    }

    #[test]
    fn builder_sets_placeholder_and_message() {
        const OPTIONS: &[u8] = &[1, 2, 3];
        let s: Select<'_, u8, u8> = select(OPTIONS, Some(2))
            .placeholder("Pick a number")
            .on_select(|v| v);
        assert_eq!(s.placeholder.as_deref(), Some("Pick a number"));
        assert!(s.is_enabled());
        assert!(matches!(s.options, Cow::Borrowed(_)));
    }

    #[test]
    fn on_select_maybe_enables_only_with_a_message() {
        let enabled: Select<'_, u8, u8> = select(vec![1, 2], None).on_select_maybe(Some(|v| v));
        assert!(enabled.is_enabled());
        let disabled: Select<'_, u8, u8> =
            select(vec![1, 2], None).on_select_maybe(None::<fn(u8) -> u8>);
        assert!(!disabled.is_enabled());
    }

    #[test]
    fn hover_and_open_strengthen_the_border() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let borders = STATES.map(|status| style(&tokens, status, true).border.color);
            assert_ne!(borders[0], borders[1]);
            assert_ne!(borders[1], borders[2]);
            assert_eq!(borders[2], borders[3]);
        }
    }

    #[test]
    fn placeholder_is_muted() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let style = style(&tokens, Status::Active, true);
            assert_eq!(style.placeholder_color, tokens.muted_foreground);
            assert_ne!(style.placeholder_color, style.text_color);
        }
    }

    #[test]
    fn disabled_fades_text_and_uses_the_disabled_field_background() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in STATES {
                let disabled = style(&tokens, status, false);
                assert!((disabled.text_color.a - tokens.foreground.a * 0.5).abs() < 1e-6);
                assert_eq!(
                    disabled.background,
                    Background::Color(tokens.disabled_field())
                );
            }
        }
    }

    #[test]
    fn an_enabled_trigger_takes_the_colour_of_its_surface() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in STATES {
                let enabled = style(&tokens, status, true);
                assert_eq!(enabled.background, Background::Color(Color::TRANSPARENT));
            }
        }
    }

    #[test]
    fn long_lists_scroll_by_default_and_search_and_custom_values_are_off() {
        let s: Select<'_, &str, ()> = select(vec!["a", "b"], None);
        assert_eq!(s.max_rows, MAX_VISIBLE_ROWS);
        assert!(s.scrollable);
        assert!(!s.searchable);
        assert!(s.on_custom.is_none());
        assert_eq!(s.shortcuts, Shortcuts::all());
        assert_eq!(s.empty, DEFAULT_EMPTY);
        assert_eq!(s.not_valid, DEFAULT_NOT_VALID);
        assert_eq!(s.type_ahead_pause, TYPE_AHEAD_PAUSE);
    }

    #[test]
    fn the_row_limit_and_scrolling_are_set_in_any_order() {
        let s: Select<'_, &str, ()> = select(vec!["a"], None).max_visible_rows(0);
        assert_eq!(s.max_rows, 1, "at least one row");
        let s: Select<'_, &str, ()> = select(vec!["a"], None)
            .scrollable(false)
            .max_visible_rows(5);
        assert!(!s.scrollable, "a row limit does not turn scrolling back on");
        let s: Select<'_, &str, ()> = select(vec!["a"], None)
            .max_visible_rows(5)
            .scrollable(false)
            .scrollable(true);
        assert_eq!(s.max_rows, 5, "the limit is kept");
        assert!(s.scrollable);
    }

    #[test]
    fn search_and_custom_values_are_set_by_the_builder() {
        let s: Select<'_, &str, String> = select(vec!["a"], None)
            .searchable(true)
            .on_custom(|text| (!text.is_empty()).then_some(text))
            .search_placeholder("Find a time")
            .empty("Nothing")
            .not_valid("Use hh:mm")
            .shortcuts(Shortcuts::none())
            .type_ahead_pause(Duration::from_millis(500));
        assert!(s.searchable);
        assert_eq!(s.search_placeholder.as_deref(), Some("Find a time"));
        assert_eq!(s.empty, "Nothing");
        assert_eq!(s.not_valid, "Use hh:mm");
        assert_eq!(s.shortcuts, Shortcuts::none());
        assert_eq!(s.type_ahead_pause, Duration::from_millis(500));
        let Some(on_custom) = s.on_custom.as_ref() else {
            panic!("on_custom is set");
        };
        assert_eq!(on_custom("x".to_owned()), Some("x".to_owned()));
        assert_eq!(on_custom(String::new()), None);
    }

    #[test]
    fn keys_that_open_and_choose() {
        for key in [Named::Space, Named::Enter, Named::ArrowDown] {
            assert!(opens_list(key), "{key:?}");
        }
        assert!(!opens_list(Named::ArrowUp));
        assert!(!opens_list(Named::Escape));
        assert!(chooses(Named::Space) && chooses(Named::Enter));
        assert!(!chooses(Named::ArrowDown));
    }

    #[test]
    fn field_status_follows_the_list_and_the_pointer() {
        assert_eq!(field_status(false, false), Status::Active);
        assert_eq!(field_status(false, true), Status::Hovered);
        assert_eq!(
            field_status(true, false),
            Status::Opened { is_hovered: false }
        );
        assert_eq!(
            field_status(true, true),
            Status::Opened { is_hovered: true }
        );
    }
}
