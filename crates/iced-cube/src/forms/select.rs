//! A dropdown for choosing one value from a list.
//!
//! The select opens and closes its own list, so the app only handles the
//! chosen value. The list floats on the shared
//! [anchored layer](crate::overlay::anchored) in the
//! [menu look](crate::overlay::menu), with a check mark on the selected
//! option. Escape or a click outside closes it.

use std::borrow::Cow;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::keyboard::key::Named;
use iced::widget::pick_list::{self, Status};
use iced::widget::text::LineHeight;
use iced::widget::{self, Column, row, text};
use iced::{
    Alignment, Background, Border, Element, Event, Length, Rectangle, Renderer, Size, Theme,
    Vector, mouse, touch,
};

use crate::icon::{opacity, themed};
use crate::keys::{self, Chord, Keymap};
use crate::overlay::anchored::{self, Align, Behaviour, Placement, Side};
use crate::overlay::menu::{self, Parts, ROW_PADDING, RowStatus, Trailing};
use crate::theme::{Tokens, fade, mix, radius, space, text_size};

/// Height of the closed select in logical pixels.
pub const HEIGHT: f32 = 36.0;

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
}

impl<T: Clone + std::fmt::Debug, Message> std::fmt::Debug for Select<'_, T, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Select")
            .field("options", &self.options)
            .field("selected", &self.selected)
            .field("placeholder", &self.placeholder)
            .field("enabled", &self.on_select.is_some())
            .finish()
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
        } = select;
        let enabled = on_select.is_some();
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
        let field = row![
            text(label.or(placeholder).unwrap_or_default())
                .size(text_size::SM)
                .line_height(LineHeight::Absolute(LINE_HEIGHT.into()))
                .wrapping(text::Wrapping::None)
                .width(Length::Fill)
                .style(move |theme| text::Style {
                    color: Some(text_colour(theme)),
                }),
            chevron,
        ]
        .spacing(space::SM)
        .align_y(Alignment::Center)
        .padding([(HEIGHT - LINE_HEIGHT) / 2.0, space::MD])
        .width(width)
        .height(HEIGHT);

        let rows = options.iter().map(|option| {
            let is_selected = selected.as_ref() == Some(option);
            option_row(option, is_selected, on_select.as_deref())
        });
        let list = menu::surface(Column::with_children(rows).width(Length::Fill))
            .width(Length::Fill)
            .into();

        Element::new(Picker {
            field: field.into(),
            list,
            enabled,
            behaviour: Behaviour {
                placement: Placement::new(Side::Bottom, Align::Start),
                match_width: true,
                closes_itself: true,
                ..Behaviour::default()
            },
        })
    }
}

/// One option in the open list, highlighted under the pointer.
fn option_row<'a, T: ToString + Clone, Message: Clone + 'a>(
    option: &T,
    selected: bool,
    on_select: Option<&(dyn Fn(T) -> Message + 'a)>,
) -> Element<'a, Message> {
    let label = option.to_string();
    let trailing = if selected {
        Trailing::Check
    } else {
        Trailing::None
    };
    let parts = Parts {
        trailing,
        ..Parts::label(&label)
    };
    let row = widget::button(menu::content(parts, RowStatus::Idle, false))
        .padding(ROW_PADDING)
        .width(Length::Fill)
        .on_press_maybe(on_select.map(|on_select| on_select(option.clone())))
        .style(|theme, status| option_style(&Tokens::of(theme), status));
    menu::inset(row)
}

/// An option row: the shared menu row, highlighted while hovered or pressed.
pub fn option_style(tokens: &Tokens, status: widget::button::Status) -> widget::button::Style {
    use widget::button::Status as Button;
    let status = match status {
        Button::Hovered | Button::Pressed => RowStatus::Highlighted,
        Button::Active => RowStatus::Idle,
        Button::Disabled => RowStatus::Disabled,
    };
    let row = menu::row_style(tokens, status, false);
    widget::button::Style {
        background: row.background.map(Background::Color),
        text_color: row.text,
        border: Border {
            radius: radius::SM.into(),
            ..Border::default()
        },
        ..widget::button::Style::default()
    }
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
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
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
        if pressed && over {
            state.panel.open = !state.panel.open;
            shell.capture_event();
            shell.request_redraw();
        }
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
        background: Background::Color(tokens.background),
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
/// considers current.
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
    fn options_highlight_with_the_menu_row_style() {
        use widget::button::Status as Button;
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let idle = option_style(&tokens, Button::Active);
            assert_eq!(idle.background, None);
            assert_eq!(idle.text_color, tokens.foreground);
            for status in [Button::Hovered, Button::Pressed] {
                let highlighted = option_style(&tokens, status);
                assert_eq!(
                    highlighted.background,
                    Some(Background::Color(tokens.accent))
                );
                assert_eq!(highlighted.text_color, tokens.accent_foreground);
                assert_eq!(highlighted.border.radius, radius::SM.into());
            }
            let disabled = option_style(&tokens, Button::Disabled);
            assert_eq!(disabled.text_color, tokens.muted_foreground);
        }
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
