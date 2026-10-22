//! Named properties in a compact two-column inspector, such as the
//! properties panel of a design tool: names on the left, editors on the
//! right, grouped into categories that collapse.
//!
//! [`State`] holds the properties by id with their values, which categories
//! are collapsed, the highlight, the filter and the [`Sort`]. Its
//! [`update`](State::update) returns [`Output::Changed`] whenever a value
//! changes and [`Output::Reset`] when one goes back to its default.
//!
//! Text, numbers and colours are edited as text. A valid edit changes the
//! value as it is typed; an invalid one keeps its draft and shows the error
//! until it is fixed or cancelled, without changing the value. Cancelling
//! puts back the value from before the edit.
//!
//! The grid resolves its keys through a [`Keymap`] of [`Action`]s while it
//! has focus, which it takes when pressed; see [`default_keymap`].

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::Hash;
use std::ops::Range;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::keyboard::key::Named;
use iced::widget::{self, Space, column, container, mouse_area, responsive, row};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Rectangle, Renderer, Size,
    Task, Theme, Vector, mouse,
};

use crate::data::ellipsis::ellipsis;
use crate::data::interaction::{Look, RowMenu, pressable, row_menu, scope, search_count};
use crate::data::rows::rows;
use crate::forms::select::select;
use crate::icon::{opacity, themed};
use crate::keys::{self, Chord, Keymap};
use crate::overlay::context_menu;
use crate::overlay::dropdown_menu::item;
use crate::overlay::tooltip::tooltip;
use crate::primitives::button;
use crate::primitives::checkbox::checkbox;
use crate::primitives::icon_button::icon_button;
use crate::primitives::input::{self, input};
use crate::primitives::switch::switch;
use crate::theme::{self, Tokens, fade, radius, space, text_size};

/// Height of every row, which fits a select.
pub const ROW_HEIGHT: f32 = 36.0;
/// The widest the name column starts out, before the user drags it.
pub const NAME_WIDTH: f32 = 160.0;
/// The narrowest the name column gets.
pub const MIN_NAME_WIDTH: f32 = 64.0;
/// The room the value column keeps when the name column is dragged wide.
pub const MIN_VALUE_WIDTH: f32 = 88.0;
/// The room the value column keeps before the name column, at its default
/// width, starts to shrink: enough for a colour swatch and its hex.
pub const VALUE_WIDTH: f32 = 112.0;
/// Width of the draggable line between the columns.
pub const HANDLE_WIDTH: f32 = 9.0;
/// Width of the slot at the end of a row for the reset button.
pub const RESET_WIDTH: f32 = 24.0;
/// The least height of the description area under the rows, which grows
/// to fit a longer description.
pub const DESCRIPTION_HEIGHT: f32 = 64.0;
/// The widget id of the filter box, which [`focus_search`] focuses.
pub const SEARCH: widget::Id = widget::Id::new("iced-cube-property-grid-filter");
/// The widget id of the field being edited, to find it in tests.
pub const EDITOR_ID: widget::Id = widget::Id::new("iced-cube-property-grid-editor");

/// Shown in the description area while nothing is highlighted.
pub const NOTHING_SELECTED: &str = "Select a property to see its description.";

/// Width of the slot before a name that marks a modified property.
const MARK_WIDTH: f32 = 14.0;
const MARK_SIZE: f32 = 6.0;
const SWATCH_SIZE: f32 = 16.0;
const ICON_SIZE: f32 = 16.0;
const RESET_ICON_SIZE: f32 = 14.0;

/// A property's value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Text, a read-only value or the chosen option of a choice.
    Text(String),
    Number(f64),
    Bool(bool),
    Colour(Color),
}

impl Value {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_colour(&self) -> Option<Color> {
        match self {
            Value::Colour(value) => Some(*value),
            _ => None,
        }
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::Text(value.to_owned())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::Text(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Number(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Number(f64::from(value))
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Number(f64::from(value))
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<Color> for Value {
    fn from(value: Color) -> Self {
        Value::Colour(value)
    }
}

/// The limits and look of a number property.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Number {
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// How far Up and Down move the value while it is edited. A whole
    /// step also makes values between steps invalid.
    pub step: Option<f64>,
    /// Decimal places shown. Defaults to those of the step, or as many as
    /// the value needs.
    pub decimals: Option<usize>,
    /// Shown after the value, such as `px` or `%`, and accepted when typed.
    pub unit: Option<String>,
}

impl Number {
    /// The decimal places values are shown with, if fixed.
    pub fn precision(&self) -> Option<usize> {
        self.decimals.or_else(|| self.step.map(decimals_of))
    }

    /// The value as text, without the unit.
    pub fn format(&self, value: f64) -> String {
        let value = if value == 0.0 { 0.0 } else { value };
        match self.precision() {
            Some(decimals) => format!("{value:.decimals$}"),
            None => format!("{value}"),
        }
    }

    /// The value as text, with the unit.
    pub fn display(&self, value: f64) -> String {
        match &self.unit {
            Some(unit) => format!("{}{unit}", self.format(value)),
            None => self.format(value),
        }
    }

    /// Reads typed text, with or without the unit, and checks it.
    pub fn parse(&self, input: &str) -> Result<f64, String> {
        let mut input = input.trim();
        if let Some(unit) = self.unit.as_deref().filter(|unit| !unit.is_empty()) {
            input = input.strip_suffix(unit).unwrap_or(input).trim_end();
        }
        let value: f64 = input.parse().map_err(|_| "Enter a number".to_owned())?;
        if !value.is_finite() {
            return Err("Enter a number".to_owned());
        }
        self.check(value)
    }

    /// Checks a value against the limits and a whole step.
    pub fn check(&self, value: f64) -> Result<f64, String> {
        if let Some(min) = self.min
            && value < min
        {
            return Err(format!("Must be at least {}", self.display(min)));
        }
        if let Some(max) = self.max
            && value > max
        {
            return Err(format!("Must be at most {}", self.display(max)));
        }
        let Some(step) = self.step.filter(|step| *step > 0.0 && step.fract() == 0.0) else {
            return Ok(value);
        };
        let steps = (value - self.min.unwrap_or(0.0)) / step;
        if (steps - steps.round()).abs() > 1e-9 {
            if step == 1.0 {
                return Err("Must be a whole number".to_owned());
            }
            return Err(format!("Must be a multiple of {}", self.format(step)));
        }
        Ok(value)
    }

    /// The value one step up or down, kept within the limits.
    pub fn stepped(&self, value: f64, up: bool) -> f64 {
        let step = self.step.filter(|step| *step > 0.0).unwrap_or(1.0);
        let next = if up { value + step } else { value - step };
        let next = match self.precision() {
            Some(decimals) => {
                let scale = 10f64.powi(i32::try_from(decimals).unwrap_or(i32::MAX).min(15));
                (next * scale).round() / scale
            }
            None => next,
        };
        let next = self.min.map_or(next, |min| next.max(min));
        self.max.map_or(next, |max| next.min(max))
    }
}

/// Decimal places in a step such as 0.25.
fn decimals_of(step: f64) -> usize {
    let text = format!("{step}");
    text.split_once('.')
        .map_or(0, |(_, fraction)| fraction.len())
}

/// A colour as `#RRGGBB`, or `#RRGGBBAA` when it is not opaque.
pub fn hex(colour: Color) -> String {
    let [r, g, b, a] = colour.into_rgba8();
    if a == u8::MAX {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

/// Reads a hex colour: `#RGB`, `#RGBA`, `#RRGGBB` or `#RRGGBBAA`, with
/// or without the `#`.
pub fn parse_hex(input: &str) -> Result<Color, String> {
    let error = || "Enter a hex colour, such as #3B82F6".to_owned();
    let input = input.trim();
    let digits = input.strip_prefix('#').unwrap_or(input);
    if !digits.chars().all(|digit| digit.is_ascii_hexdigit()) {
        return Err(error());
    }
    let digits: String = match digits.len() {
        3 | 4 => digits.chars().flat_map(|digit| [digit, digit]).collect(),
        6 | 8 => digits.to_owned(),
        _ => return Err(error()),
    };
    let bytes = (0..digits.len())
        .step_by(2)
        .map(|start| {
            digits
                .get(start..start + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
        })
        .collect::<Option<Vec<u8>>>()
        .ok_or_else(error)?;
    match bytes.as_slice() {
        [r, g, b] => Ok(Color::from_rgb8(*r, *g, *b)),
        [r, g, b, a] => Ok(Color::from_rgba8(*r, *g, *b, f32::from(*a) / 255.0)),
        _ => Err(error()),
    }
}

/// Checks text typed into a text property, returning the error to show.
pub type Validate = fn(&str) -> Result<(), String>;

/// How a true or false property is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Toggle {
    #[default]
    Checkbox,
    Switch,
}

impl Toggle {
    pub const ALL: [Toggle; 2] = [Toggle::Checkbox, Toggle::Switch];
}

/// What a property holds and how it is edited.
#[derive(Debug, Clone)]
pub enum Kind {
    /// Free text, with an optional check.
    Text(Option<Validate>),
    Number(Number),
    Toggle(Toggle),
    /// One of a list of options, chosen with a select.
    Choice(Vec<String>),
    /// A colour, shown as a swatch and edited as hex.
    Colour,
    /// Text that cannot be edited.
    ReadOnly,
    /// Drawn by the app with [`PropertyGrid::custom`].
    Custom,
}

/// One named property.
#[derive(Debug, Clone)]
pub struct Property<Id> {
    pub id: Id,
    pub name: String,
    /// The group it is listed under while categorised.
    pub category: Option<String>,
    /// Shown under the rows while the property is highlighted.
    pub description: Option<String>,
    pub kind: Kind,
    pub value: Value,
    /// The value a reset goes back to. A property without one has no reset.
    pub default: Option<Value>,
    pub disabled: bool,
}

fn property<Id>(id: Id, name: impl Into<String>, kind: Kind, value: Value) -> Property<Id> {
    Property {
        id,
        name: name.into(),
        category: None,
        description: None,
        kind,
        value,
        default: None,
        disabled: false,
    }
}

/// A text property.
pub fn text<Id>(id: Id, name: impl Into<String>, value: impl Into<String>) -> Property<Id> {
    property(id, name, Kind::Text(None), Value::Text(value.into()))
}

/// A number property. Set its limits with [`min`](Property::min),
/// [`max`](Property::max), [`step`](Property::step) and its look with
/// [`decimals`](Property::decimals) and [`unit`](Property::unit).
pub fn number<Id>(id: Id, name: impl Into<String>, value: f64) -> Property<Id> {
    property(
        id,
        name,
        Kind::Number(Number::default()),
        Value::Number(value),
    )
}

/// A true or false property, shown as a checkbox, or as a switch with
/// [`switch`](Property::switch).
pub fn toggle<Id>(id: Id, name: impl Into<String>, value: bool) -> Property<Id> {
    property(id, name, Kind::Toggle(Toggle::Checkbox), Value::Bool(value))
}

/// A property chosen from `options` with a select.
pub fn choice<Id>(
    id: Id,
    name: impl Into<String>,
    options: impl IntoIterator<Item = impl Into<String>>,
    selected: impl Into<String>,
) -> Property<Id> {
    property(
        id,
        name,
        Kind::Choice(options.into_iter().map(Into::into).collect()),
        Value::Text(selected.into()),
    )
}

/// A colour property, shown as a swatch and edited as hex.
pub fn colour<Id>(id: Id, name: impl Into<String>, value: Color) -> Property<Id> {
    property(id, name, Kind::Colour, Value::Colour(value))
}

/// Text that cannot be edited, such as an id or a computed size.
pub fn read_only<Id>(id: Id, name: impl Into<String>, value: impl Into<String>) -> Property<Id> {
    property(id, name, Kind::ReadOnly, Value::Text(value.into()))
}

/// A property the app draws itself with [`PropertyGrid::custom`]. Without
/// a renderer its value is shown as text.
pub fn custom<Id>(id: Id, name: impl Into<String>, value: impl Into<Value>) -> Property<Id> {
    property(id, name, Kind::Custom, value.into())
}

impl<Id> Property<Id> {
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// The value a reset goes back to. The name shows as modified while
    /// the value differs from it.
    pub fn default(mut self, value: impl Into<Value>) -> Self {
        self.default = Some(value.into());
        self
    }

    /// A disabled property shows its value but cannot be edited or reset.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The smallest value of a number property.
    pub fn min(mut self, min: f64) -> Self {
        if let Kind::Number(number) = &mut self.kind {
            number.min = Some(min);
        }
        self
    }

    /// The largest value of a number property.
    pub fn max(mut self, max: f64) -> Self {
        if let Kind::Number(number) = &mut self.kind {
            number.max = Some(max);
        }
        self
    }

    pub fn range(self, min: f64, max: f64) -> Self {
        self.min(min).max(max)
    }

    /// How far Up and Down move a number while it is edited. A whole step
    /// also makes values between steps invalid.
    pub fn step(mut self, step: f64) -> Self {
        if let Kind::Number(number) = &mut self.kind {
            number.step = Some(step);
        }
        self
    }

    /// Decimal places a number is shown with.
    pub fn decimals(mut self, decimals: usize) -> Self {
        if let Kind::Number(number) = &mut self.kind {
            number.decimals = Some(decimals);
        }
        self
    }

    /// Shown after a number, such as `px`, and accepted when typed.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        if let Kind::Number(number) = &mut self.kind {
            number.unit = Some(unit.into());
        }
        self
    }

    /// Shows a true or false property as a switch.
    pub fn switch(mut self) -> Self {
        if let Kind::Toggle(toggle) = &mut self.kind {
            *toggle = Toggle::Switch;
        }
        self
    }

    /// Checks what is typed into a text property.
    pub fn validate(mut self, validate: Validate) -> Self {
        if let Kind::Text(check) = &mut self.kind {
            *check = Some(validate);
        }
        self
    }

    /// Whether the value differs from the default.
    pub fn is_modified(&self) -> bool {
        self.default
            .as_ref()
            .is_some_and(|default| *default != self.value)
    }

    /// Whether the value is edited by typing: text, numbers and colours.
    pub fn is_typed(&self) -> bool {
        matches!(self.kind, Kind::Text(_) | Kind::Number(_) | Kind::Colour)
    }

    /// Whether the user can change the value from the grid.
    pub fn is_editable(&self) -> bool {
        !self.disabled && !matches!(self.kind, Kind::ReadOnly | Kind::Custom)
    }

    /// The value as shown in its row.
    pub fn display(&self) -> String {
        match (&self.kind, &self.value) {
            (Kind::Number(number), Value::Number(value)) => number.display(*value),
            _ => display(&self.value),
        }
    }

    /// The value as text to start an edit with.
    pub fn edit_text(&self) -> String {
        match (&self.kind, &self.value) {
            (Kind::Number(number), Value::Number(value)) => number.format(*value),
            _ => display(&self.value),
        }
    }

    /// Reads typed text into a value of this property's kind.
    pub fn parse(&self, input: &str) -> Result<Value, String> {
        match &self.kind {
            Kind::Number(number) => number.parse(input).map(Value::Number),
            Kind::Colour => parse_hex(input).map(Value::Colour),
            Kind::Text(Some(validate)) => validate(input).map(|()| Value::Text(input.to_owned())),
            Kind::Choice(options) if !options.iter().any(|option| option == input) => {
                Err("Choose one of the options".to_owned())
            }
            _ => Ok(Value::Text(input.to_owned())),
        }
    }

    /// Whether `value` suits this property, such as an option of a choice.
    pub fn accepts(&self, value: &Value) -> bool {
        match (&self.kind, value) {
            (Kind::Text(validate), Value::Text(text)) => {
                validate.is_none_or(|validate| validate(text).is_ok())
            }
            (Kind::Number(number), Value::Number(value)) => number.check(*value).is_ok(),
            (Kind::Toggle(_), Value::Bool(_)) | (Kind::Colour, Value::Colour(_)) => true,
            (Kind::Choice(options), Value::Text(text)) => options.contains(text),
            (Kind::ReadOnly, Value::Text(_)) | (Kind::Custom, _) => true,
            _ => false,
        }
    }
}

fn display(value: &Value) -> String {
    match value {
        Value::Text(text) => text.clone(),
        Value::Number(number) => Number::default().format(*number),
        Value::Bool(true) => "On".to_owned(),
        Value::Bool(false) => "Off".to_owned(),
        Value::Colour(colour) => hex(*colour),
    }
}

/// How the properties are listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Sort {
    /// Under their categories, in the order they were given. Properties
    /// without a category come first.
    #[default]
    Categorised,
    /// One list, by name.
    Alphabetical,
}

impl Sort {
    pub const ALL: [Sort; 2] = [Sort::Categorised, Sort::Alphabetical];
}

/// One row of the grid.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Entry<Id> {
    /// A category heading, by name.
    Category(String),
    Property(Id),
}

/// Text being typed into a property, and why it is not valid yet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Draft {
    pub text: String,
    pub error: Option<String>,
}

/// Which field the grid wants focused next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Target {
    Filter,
    Editor,
}

#[derive(Debug, Clone, PartialEq)]
struct Editing<Id> {
    id: Id,
    /// The value before the edit, which a cancel puts back.
    original: Value,
}

/// Everything that changes a property grid.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<Id> {
    /// The pointer pressed a row. A category heading also opens or closes.
    Press(Entry<Id>),
    ToggleCategory(String),
    Expand(String),
    Collapse(String),
    Next,
    Previous,
    First,
    Last,
    /// Opens the highlighted category, or moves into it when it is open.
    ExpandOrEnter,
    /// Closes the highlighted category, or moves from a property to its
    /// category.
    CollapseOrLeave,
    /// Edits a property: starts typing into text, numbers and colours,
    /// flips a toggle and moves a choice to its next option.
    Edit(Id),
    /// Edits the highlighted property, or opens or closes the highlighted
    /// category.
    EditHighlighted,
    /// Text typed into a property's field.
    Input(Id, String),
    /// Enter in the field being edited. Ends the edit unless it is invalid.
    Submit,
    /// Ends the edit and puts back the value from before it. Without an
    /// edit, clears the filter.
    Cancel,
    /// Steps the number being edited up.
    Increment,
    /// Steps the number being edited down.
    Decrement,
    /// A new value from a checkbox, switch or select.
    Set(Id, Value),
    /// Puts a property back to its default.
    Reset(Id),
    ResetHighlighted,
    /// The filter text changed.
    Filter(String),
    /// Moves keyboard focus to the filter box.
    FocusSearch,
    /// Asks the app to copy a property's value, as it is shown.
    CopyValue(Id),
    Sort(Sort),
    /// The pointer pressed the line between the columns.
    ResizeStart,
    /// The pointer dragged the line: the name column's new width.
    Resize(f32),
    ResizeEnd,
    /// The grid gained or lost focus. The widget sends this itself.
    Focus(bool),
}

/// What the app may need to act on after an [`Event`].
#[derive(Debug, Clone, PartialEq)]
pub enum Output<Id> {
    /// A property has a new value.
    Changed { id: Id, value: Value },
    /// A property went back to its default, which is `value`.
    Reset { id: Id, value: Value },
    /// A value as it is shown, such as `120px` or `#3B82F6`, to write to
    /// the clipboard.
    CopyValue(String),
}

/// The properties, their values, and what is open, highlighted, filtered
/// and being edited.
#[derive(Debug, Clone)]
pub struct State<Id> {
    properties: Vec<Property<Id>>,
    index: HashMap<Id, usize>,
    categories: Vec<String>,
    collapsed: HashSet<String>,
    sort: Sort,
    filter: String,
    highlighted: Option<Entry<Id>>,
    editing: Option<Editing<Id>>,
    drafts: HashMap<Id, Draft>,
    name_width: Option<f32>,
    resizing: bool,
    focused: bool,
    /// Counts focus requests, so the widget acts on each one once.
    focus: (u64, Option<Target>),
}

impl<Id: Copy + Eq + Hash> State<Id> {
    /// Holds `properties`, categorised with every category open. Ids must
    /// be unique; a later property with the same id replaces the earlier.
    pub fn new(properties: impl IntoIterator<Item = Property<Id>>) -> Self {
        let mut list: Vec<Property<Id>> = Vec::new();
        let mut index = HashMap::new();
        for property in properties {
            match index.get(&property.id) {
                Some(&position) => {
                    if let Some(slot) = list.get_mut(position) {
                        *slot = property;
                    }
                }
                None => {
                    let _ = index.insert(property.id, list.len());
                    list.push(property);
                }
            }
        }
        let mut categories: Vec<String> = Vec::new();
        for category in list
            .iter()
            .filter_map(|property| property.category.as_ref())
        {
            if !categories.contains(category) {
                categories.push(category.clone());
            }
        }
        Self {
            properties: list,
            index,
            categories,
            collapsed: HashSet::new(),
            sort: Sort::default(),
            filter: String::new(),
            highlighted: None,
            editing: None,
            drafts: HashMap::new(),
            name_width: None,
            resizing: false,
            focused: false,
            focus: (0, None),
        }
    }

    pub fn with_sort(mut self, sort: Sort) -> Self {
        self.sort = sort;
        self
    }

    /// Starts with these categories closed.
    pub fn with_collapsed(
        mut self,
        categories: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.collapsed
            .extend(categories.into_iter().map(Into::into));
        self
    }

    /// Starts the name column at `width` pixels rather than
    /// [`NAME_WIDTH`], which a narrow grid shrinks.
    pub fn with_name_width(mut self, width: f32) -> Self {
        self.name_width = Some(width.max(MIN_NAME_WIDTH));
        self
    }

    /// Every property, in the order given.
    pub fn properties(&self) -> &[Property<Id>] {
        &self.properties
    }

    pub fn property(&self, id: Id) -> Option<&Property<Id>> {
        self.index
            .get(&id)
            .and_then(|&position| self.properties.get(position))
    }

    fn property_mut(&mut self, id: Id) -> Option<&mut Property<Id>> {
        self.index
            .get(&id)
            .and_then(|&position| self.properties.get_mut(position))
    }

    pub fn value(&self, id: Id) -> Option<&Value> {
        self.property(id).map(|property| &property.value)
    }

    /// Sets a value from the app, such as after an undo or from a custom
    /// editor, dropping any draft or edit of it. Returns whether it changed.
    pub fn set(&mut self, id: Id, value: impl Into<Value>) -> bool {
        let value = value.into();
        let _ = self.drafts.remove(&id);
        if self.editing_id() == Some(id) {
            self.editing = None;
        }
        let Some(property) = self.property_mut(id) else {
            return false;
        };
        if property.value == value {
            return false;
        }
        property.value = value;
        true
    }

    /// Category names, in the order they first appear.
    pub fn categories(&self) -> &[String] {
        &self.categories
    }

    pub fn is_expanded(&self, category: &str) -> bool {
        !self.collapsed.contains(category)
    }

    pub fn sort(&self) -> Sort {
        self.sort
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// The row the keyboard acts on, which the description area describes.
    pub fn highlighted(&self) -> Option<&Entry<Id>> {
        self.highlighted.as_ref()
    }

    /// The highlighted property, if a property rather than a category is
    /// highlighted.
    pub fn highlighted_property(&self) -> Option<&Property<Id>> {
        match self.highlighted {
            Some(Entry::Property(id)) => self.property(id),
            _ => None,
        }
    }

    /// The property being typed into.
    pub fn editing(&self) -> Option<Id> {
        self.editing_id()
    }

    fn editing_id(&self) -> Option<Id> {
        self.editing.as_ref().map(|editing| editing.id)
    }

    /// Text typed into a property that is being edited, or that was left
    /// while invalid.
    pub fn draft(&self, id: Id) -> Option<&Draft> {
        self.drafts.get(&id)
    }

    /// Why a property's draft is not valid.
    pub fn error(&self, id: Id) -> Option<&str> {
        self.drafts
            .get(&id)
            .and_then(|draft| draft.error.as_deref())
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Whether the line between the columns is being dragged.
    pub fn is_resizing(&self) -> bool {
        self.resizing
    }

    /// The name column's width once set or dragged.
    pub fn name_width(&self) -> Option<f32> {
        self.name_width
    }

    /// Whether a property passes the filter.
    pub fn matches(&self, id: Id) -> bool {
        self.property(id)
            .is_some_and(|property| self.passes(property))
    }

    fn passes(&self, property: &Property<Id>) -> bool {
        let filter = self.filter.trim();
        filter.is_empty()
            || property
                .name
                .to_lowercase()
                .contains(&filter.to_lowercase())
    }

    /// How many properties in a category pass the filter.
    pub fn count(&self, category: &str) -> usize {
        self.properties
            .iter()
            .filter(|property| property.category.as_deref() == Some(category))
            .filter(|property| self.passes(property))
            .count()
    }

    /// The rows on screen, in order: properties that pass the filter, under
    /// their categories while categorised.
    pub fn entries(&self) -> Vec<Entry<Id>> {
        let passing = self
            .properties
            .iter()
            .filter(|property| self.passes(property));
        if self.sort == Sort::Alphabetical {
            let mut sorted: Vec<&Property<Id>> = passing.collect();
            sorted.sort_by_cached_key(|property| property.name.to_lowercase());
            return sorted
                .into_iter()
                .map(|property| Entry::Property(property.id))
                .collect();
        }
        let mut entries: Vec<Entry<Id>> = passing
            .clone()
            .filter(|property| property.category.is_none())
            .map(|property| Entry::Property(property.id))
            .collect();
        for category in &self.categories {
            let members: Vec<Id> = passing
                .clone()
                .filter(|property| property.category.as_ref() == Some(category))
                .map(|property| property.id)
                .collect();
            if members.is_empty() {
                continue;
            }
            entries.push(Entry::Category(category.clone()));
            if self.is_expanded(category) {
                entries.extend(members.into_iter().map(Entry::Property));
            }
        }
        entries
    }

    /// Applies an event and returns what the app may need to act on.
    pub fn update(&mut self, event: Event<Id>) -> Option<Output<Id>> {
        match event {
            Event::Press(entry) => {
                let same = matches!(&entry, Entry::Property(id) if self.editing_id() == Some(*id));
                if !same {
                    self.finish();
                }
                if let Entry::Category(name) = &entry {
                    self.toggle(name);
                }
                self.highlighted = Some(entry);
            }
            Event::ToggleCategory(name) => self.toggle(&name),
            Event::Expand(name) => {
                let _ = self.collapsed.remove(&name);
            }
            Event::Collapse(name) => self.collapse(&name),
            Event::Next => self.step(Step::Next),
            Event::Previous => self.step(Step::Previous),
            Event::First => self.step(Step::First),
            Event::Last => self.step(Step::Last),
            Event::ExpandOrEnter => self.expand_or_enter(),
            Event::CollapseOrLeave => self.collapse_or_leave(),
            Event::Edit(id) => return self.edit(id),
            Event::EditHighlighted => match self.highlighted.clone() {
                Some(Entry::Property(id)) => return self.edit(id),
                Some(Entry::Category(name)) => self.toggle(&name),
                None => {}
            },
            Event::Input(id, input) => return self.input(id, input),
            Event::Submit => self.submit(),
            Event::Cancel => return self.cancel(),
            Event::Increment => return self.nudge(true),
            Event::Decrement => return self.nudge(false),
            Event::Set(id, value) => {
                if self.editing_id() != Some(id) {
                    self.finish();
                }
                return self.choose(id, value);
            }
            Event::Reset(id) => return self.reset(id),
            Event::ResetHighlighted => {
                if let Some(Entry::Property(id)) = self.highlighted {
                    return self.reset(id);
                }
            }
            Event::Filter(filter) => {
                self.finish();
                self.filter = filter;
                self.keep_highlight_visible();
            }
            Event::FocusSearch => {
                self.finish();
                self.request(Target::Filter);
            }
            Event::CopyValue(id) => {
                let value = self.property(id)?.display();
                if self.editing_id() != Some(id) {
                    self.finish();
                }
                self.highlighted = Some(Entry::Property(id));
                return Some(Output::CopyValue(value));
            }
            Event::Sort(sort) => {
                self.finish();
                self.sort = sort;
                self.keep_highlight_visible();
            }
            Event::ResizeStart => self.resizing = true,
            Event::Resize(width) => {
                if self.resizing {
                    self.name_width = Some(width.max(MIN_NAME_WIDTH));
                }
            }
            Event::ResizeEnd => self.resizing = false,
            Event::Focus(focused) => {
                self.focused = focused;
                if !focused {
                    self.finish();
                    self.resizing = false;
                    self.focus.1 = None;
                } else if self.highlighted.is_none() {
                    self.highlighted = self.first_property();
                }
            }
        }
        None
    }

    /// Turns a key press into an event through `keymap`.
    pub fn key_event(&self, keymap: &Keymap<Action>, key: &keys::Event) -> Option<Event<Id>> {
        keymap
            .resolve_event(key)
            .and_then(|action| action.event(self))
    }

    /// The focus request the widget has not acted on yet, by count.
    fn focus_request(&self) -> Option<(u64, Target)> {
        let (count, target) = self.focus;
        target.map(|target| (count, target))
    }

    fn request(&mut self, target: Target) {
        self.focus = (self.focus.0.wrapping_add(1), Some(target));
    }

    fn first_property(&self) -> Option<Entry<Id>> {
        let entries = self.entries();
        entries
            .iter()
            .find(|entry| matches!(entry, Entry::Property(_)))
            .or(entries.first())
            .cloned()
    }

    fn keep_highlight_visible(&mut self) {
        let Some(highlighted) = &self.highlighted else {
            return;
        };
        if !self.entries().contains(highlighted) {
            self.highlighted = self.first_property();
        }
    }

    fn toggle(&mut self, name: &str) {
        if self.collapsed.contains(name) {
            let _ = self.collapsed.remove(name);
        } else {
            self.collapse(name);
        }
    }

    fn collapse(&mut self, name: &str) {
        if !self.categories.iter().any(|category| category == name) {
            return;
        }
        let _ = self.collapsed.insert(name.to_owned());
        if self.sort == Sort::Alphabetical {
            return;
        }
        let inside = |id: Option<Id>| {
            id.and_then(|id| self.property(id))
                .is_some_and(|property| property.category.as_deref() == Some(name))
        };
        let highlighted = match self.highlighted {
            Some(Entry::Property(id)) => Some(id),
            _ => None,
        };
        let hides_highlight = inside(highlighted);
        let hides_edit = inside(self.editing_id());
        if hides_highlight {
            self.highlighted = Some(Entry::Category(name.to_owned()));
        }
        if hides_edit {
            self.finish();
        }
    }

    fn step(&mut self, step: Step) {
        self.finish();
        let entries = self.entries();
        let current = self
            .highlighted
            .as_ref()
            .and_then(|highlighted| entries.iter().position(|entry| entry == highlighted));
        let target = match (step, current) {
            (Step::First, _) | (Step::Next, None) => entries.first(),
            (Step::Last, _) | (Step::Previous, None) => entries.last(),
            (Step::Next, Some(index)) => entries.get(index + 1).or(entries.get(index)),
            (Step::Previous, Some(index)) => entries.get(index.saturating_sub(1)),
        };
        if let Some(target) = target {
            self.highlighted = Some(target.clone());
        }
    }

    fn expand_or_enter(&mut self) {
        let Some(Entry::Category(name)) = self.highlighted.clone() else {
            return;
        };
        if !self.is_expanded(&name) {
            let _ = self.collapsed.remove(&name);
            return;
        }
        let entries = self.entries();
        let first = entries
            .iter()
            .skip_while(|entry| **entry != Entry::Category(name.clone()))
            .nth(1)
            .filter(|entry| matches!(entry, Entry::Property(_)));
        if let Some(first) = first {
            self.highlighted = Some(first.clone());
        }
    }

    fn collapse_or_leave(&mut self) {
        match self.highlighted.clone() {
            Some(Entry::Category(name)) => self.collapse(&name),
            Some(Entry::Property(id)) if self.sort == Sort::Categorised => {
                let category = self
                    .property(id)
                    .and_then(|property| property.category.clone());
                if let Some(category) = category {
                    self.finish();
                    self.highlighted = Some(Entry::Category(category));
                }
            }
            _ => {}
        }
    }

    /// Ends the edit. A valid draft has already been applied and goes; an
    /// invalid one stays so the row keeps showing it and its error.
    fn finish(&mut self) {
        let Some(editing) = self.editing.take() else {
            return;
        };
        if self.error(editing.id).is_none() {
            let _ = self.drafts.remove(&editing.id);
        }
    }

    /// Sets a value, reporting it when it changed.
    fn assign(&mut self, id: Id, value: Value) -> Option<Output<Id>> {
        let property = self.property_mut(id)?;
        if property.value == value {
            return None;
        }
        property.value = value.clone();
        Some(Output::Changed { id, value })
    }

    fn edit(&mut self, id: Id) -> Option<Output<Id>> {
        let property = self.property(id)?;
        if property.disabled {
            self.highlighted = Some(Entry::Property(id));
            return None;
        }
        let kind = property.kind.clone();
        let value = property.value.clone();
        let typed = property.is_typed();
        let text = property.edit_text();
        if self.editing_id() != Some(id) {
            self.finish();
        }
        self.highlighted = Some(Entry::Property(id));
        match (kind, value) {
            (Kind::Toggle(_), Value::Bool(on)) => self.assign(id, Value::Bool(!on)),
            (Kind::Choice(options), Value::Text(current)) => {
                let next = options
                    .iter()
                    .position(|option| *option == current)
                    .map_or(0, |index| (index + 1) % options.len().max(1));
                let option = options.get(next)?.clone();
                self.assign(id, Value::Text(option))
            }
            (_, value) if typed => {
                if self.editing_id() != Some(id) {
                    let _ = self.drafts.entry(id).or_insert(Draft { text, error: None });
                    self.editing = Some(Editing {
                        id,
                        original: value,
                    });
                }
                self.request(Target::Editor);
                None
            }
            _ => None,
        }
    }

    fn input(&mut self, id: Id, input: String) -> Option<Output<Id>> {
        let property = self.property(id).filter(|property| property.is_typed())?;
        if property.disabled {
            return None;
        }
        let parsed = property.parse(&input);
        let value = property.value.clone();
        if self.editing_id() != Some(id) {
            self.finish();
            self.editing = Some(Editing {
                id,
                original: value,
            });
            self.highlighted = Some(Entry::Property(id));
        }
        match parsed {
            Ok(value) => {
                let _ = self.drafts.insert(
                    id,
                    Draft {
                        text: input,
                        error: None,
                    },
                );
                self.assign(id, value)
            }
            Err(error) => {
                let _ = self.drafts.insert(
                    id,
                    Draft {
                        text: input,
                        error: Some(error),
                    },
                );
                None
            }
        }
    }

    fn submit(&mut self) {
        let Some(id) = self.editing_id() else {
            return;
        };
        if self.error(id).is_some() {
            return;
        }
        let _ = self.drafts.remove(&id);
        self.editing = None;
    }

    fn cancel(&mut self) -> Option<Output<Id>> {
        let Some(editing) = self.editing.take() else {
            self.filter.clear();
            return None;
        };
        let _ = self.drafts.remove(&editing.id);
        self.assign(editing.id, editing.original)
    }

    fn nudge(&mut self, up: bool) -> Option<Output<Id>> {
        let id = self.editing_id()?;
        let property = self.property(id)?;
        let (Kind::Number(number), Value::Number(current)) = (&property.kind, &property.value)
        else {
            return None;
        };
        let base = self
            .drafts
            .get(&id)
            .and_then(|draft| number.parse(&draft.text).ok())
            .unwrap_or(*current);
        let next = number.stepped(base, up);
        let text = number.format(next);
        let _ = self.drafts.insert(id, Draft { text, error: None });
        self.assign(id, Value::Number(next))
    }

    fn choose(&mut self, id: Id, value: Value) -> Option<Output<Id>> {
        let property = self.property(id)?;
        if !property.is_editable() || !property.accepts(&value) {
            return None;
        }
        self.highlighted = Some(Entry::Property(id));
        self.assign(id, value)
    }

    fn reset(&mut self, id: Id) -> Option<Output<Id>> {
        let property = self.property(id)?;
        if property.disabled {
            return None;
        }
        let default = property.default.clone()?;
        let _ = self.drafts.remove(&id);
        if self.editing_id() == Some(id) {
            self.editing = None;
        }
        let property = self.property_mut(id)?;
        if property.value == default {
            return None;
        }
        property.value = default.clone();
        Some(Output::Reset { id, value: default })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Next,
    Previous,
    First,
    Last,
}

/// What a property grid keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
    First,
    Last,
    Expand,
    Collapse,
    Edit,
    Cancel,
    Reset,
    CopyValue,
    FocusSearch,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it does
    /// nothing, so the key is left for others.
    pub fn event<Id: Copy + Eq + Hash>(self, state: &State<Id>) -> Option<Event<Id>> {
        let number = state
            .editing()
            .and_then(|id| state.property(id))
            .is_some_and(|property| matches!(property.kind, Kind::Number(_)));
        let highlighted = state.highlighted();
        let property = state.highlighted_property();
        match self {
            Action::Next if number => Some(Event::Decrement),
            Action::Next => Some(Event::Next),
            Action::Previous if number => Some(Event::Increment),
            Action::Previous => Some(Event::Previous),
            Action::First => Some(Event::First),
            Action::Last => Some(Event::Last),
            Action::Expand => {
                matches!(highlighted, Some(Entry::Category(_))).then_some(Event::ExpandOrEnter)
            }
            Action::Collapse => {
                let leaves = property.is_some_and(|property| property.category.is_some())
                    && state.sort() == Sort::Categorised;
                (matches!(highlighted, Some(Entry::Category(_))) || leaves)
                    .then_some(Event::CollapseOrLeave)
            }
            Action::Edit => {
                let editable = property.is_some_and(Property::is_editable);
                (matches!(highlighted, Some(Entry::Category(_))) || editable)
                    .then_some(Event::EditHighlighted)
            }
            Action::Cancel => {
                (state.editing().is_some() || !state.filter().is_empty()).then_some(Event::Cancel)
            }
            Action::Reset => property
                .filter(|property| !property.disabled && property.default.is_some())
                .filter(|property| property.is_modified() || state.draft(property.id).is_some())
                .map(|_| Event::ResetHighlighted),
            Action::CopyValue => property.map(|property| Event::CopyValue(property.id)),
            Action::FocusSearch => Some(Event::FocusSearch),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Next,
        Action::Previous,
        Action::First,
        Action::Last,
        Action::Expand,
        Action::Collapse,
        Action::Edit,
        Action::Cancel,
        Action::Reset,
        Action::CopyValue,
        Action::FocusSearch,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Next => "Next",
            Action::Previous => "Previous",
            Action::First => "First",
            Action::Last => "Last",
            Action::Expand => "Expand",
            Action::Collapse => "Collapse",
            Action::Edit => "Edit",
            Action::Cancel => "Cancel",
            Action::Reset => "Reset",
            Action::CopyValue => "CopyValue",
            Action::FocusSearch => "FocusSearch",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Highlights the next row, or steps a number being edited down.",
            Action::Previous => "Highlights the previous row, or steps a number being edited up.",
            Action::First => "Highlights the first row.",
            Action::Last => "Highlights the last row.",
            Action::Expand => {
                "Opens the highlighted category, or moves to its first property when it is open."
            }
            Action::Collapse => {
                "Closes the highlighted category, or moves from a property to its category."
            }
            Action::Edit => {
                "Edits the highlighted property: types into text, flips a toggle or picks the next option."
            }
            Action::Cancel => "Cancels the edit and puts back the old value, or clears the filter.",
            Action::Reset => "Puts the highlighted property back to its default.",
            Action::CopyValue => "Copies the highlighted value as it is shown.",
            Action::FocusSearch => "Moves the keyboard to the filter box.",
        }
    }
}

/// The default property grid shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowDown` | [`Action::Next`] |
/// | `ArrowUp` | [`Action::Previous`] |
/// | `Home` | [`Action::First`] |
/// | `End` | [`Action::Last`] |
/// | `ArrowRight` | [`Action::Expand`] |
/// | `ArrowLeft` | [`Action::Collapse`] |
/// | `Enter`, `F2` | [`Action::Edit`] |
/// | `Escape` | [`Action::Cancel`] |
/// | `Delete` | [`Action::Reset`] |
/// | `Mod+C` (Ctrl, or Cmd on macOS) | [`Action::CopyValue`] |
/// | `Mod+F` | [`Action::FocusSearch`] |
///
/// While a field is being typed into it keeps the keys it uses, such as
/// the arrows that move its cursor; Enter ends the edit and Escape cancels
/// it. Up and Down step a number being edited. Escape in the filter box
/// clears it, and a second Escape leaves it. The grid resolves these
/// itself, only while it has focus, so they never clash with other
/// components. Pass a changed keymap with [`PropertyGrid::keymap`].
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowDown), Action::Next)
        .bind(Chord::named(Named::ArrowUp), Action::Previous)
        .bind(Chord::named(Named::Home), Action::First)
        .bind(Chord::named(Named::End), Action::Last)
        .bind(Chord::named(Named::ArrowRight), Action::Expand)
        .bind(Chord::named(Named::ArrowLeft), Action::Collapse)
        .bind(Chord::named(Named::Enter), Action::Edit)
        .bind(Chord::named(Named::F2), Action::Edit)
        .bind(Chord::named(Named::Escape), Action::Cancel)
        .bind(Chord::named(Named::Delete), Action::Reset)
        .bind(Chord::character('c').command(), Action::CopyValue)
        .bind(Chord::character('f').command(), Action::FocusSearch)
}

/// Focuses the filter box of a grid, for an app that moves the keyboard
/// there itself. [`Action::FocusSearch`] does the same from inside the
/// grid.
pub fn focus_search<Id, Message>(_state: &State<Id>) -> Task<Message> {
    widget::operation::focus(SEARCH)
}

/// The text beside the filter box: how many properties pass the filter,
/// of how many, such as "3 of 12", or "No matches".
pub fn match_label<Id: Copy + Eq + Hash>(state: &State<Id>) -> String {
    if state.filter().trim().is_empty() {
        return String::new();
    }
    let total = state.properties().len();
    match state
        .properties()
        .iter()
        .filter(|property| state.passes(property))
        .count()
    {
        0 => "No matches".to_owned(),
        shown => format!("{shown} of {total}"),
    }
}

/// An entry of the context menu from [`menu`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuItem {
    CopyValue,
    Reset,
}

impl MenuItem {
    pub const ALL: [MenuItem; 2] = [MenuItem::CopyValue, MenuItem::Reset];

    /// The event the entry sends for the property the menu opened on.
    pub fn event<Id>(self, id: Id) -> Event<Id> {
        match self {
            MenuItem::CopyValue => Event::CopyValue(id),
            MenuItem::Reset => Event::Reset(id),
        }
    }
}

/// A context menu to pass to [`PropertyGrid::context_menu`]: copy the
/// value, or reset it to its default. Turn the chosen entry into an event
/// with [`MenuItem::event`].
pub fn menu<Id: Clone + PartialEq>() -> context_menu::State<MenuItem, Id> {
    context_menu::State::new([
        item(MenuItem::CopyValue, "Copy value").icon(crate::lucide!(Copy)),
        item(MenuItem::Reset, "Reset to default").icon(crate::lucide!(RotateCcw)),
    ])
}

/// How a row is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Status {
    /// The row the keyboard acts on.
    pub highlighted: bool,
    /// The grid has keyboard focus, so the highlight carries a ring.
    pub focused: bool,
    pub hovered: bool,
    pub disabled: bool,
    /// A category heading rather than a property.
    pub heading: bool,
}

/// The colours of a row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowStyle {
    pub background: Option<Color>,
    pub text: Color,
    pub icon: Color,
    /// The ring around the highlighted row while the grid has focus.
    pub ring: Option<Color>,
}

/// Resolves a row's colours. Headings sit on a muted band. The grid selects
/// nothing, so the highlighted row only carries a ring, and only while the
/// grid has focus.
pub fn row_style(tokens: &Tokens, status: Status) -> RowStyle {
    let hovered = status.hovered && !status.disabled;
    let background = match (hovered, status.heading) {
        (true, true) => Some(tokens.muted),
        (true, false) => Some(fade(tokens.accent, 0.6)),
        (false, true) => Some(fade(tokens.muted, 0.6)),
        (false, false) => None,
    };
    let text = if status.disabled {
        tokens.muted_foreground
    } else {
        tokens.foreground
    };
    RowStyle {
        background,
        text,
        icon: tokens.muted_foreground,
        ring: (status.highlighted && status.focused).then_some(tokens.ring),
    }
}

/// The colour of a value: muted when it cannot be edited.
pub fn value_colour(tokens: &Tokens, editable: bool) -> Color {
    if editable {
        tokens.foreground
    } else {
        tokens.muted_foreground
    }
}

/// The dot before the name of a property that differs from its default.
pub fn mark_colour(tokens: &Tokens) -> Color {
    tokens.primary
}

/// The line between the name and value columns, brighter while dragged or
/// hovered.
pub fn divider_colour(tokens: &Tokens, active: bool) -> Color {
    if active {
        tokens.ring
    } else {
        fade(tokens.border, 0.7)
    }
}

/// The border around the grid.
pub fn frame_style(tokens: &Tokens) -> container::Style {
    container::Style {
        border: Border {
            color: tokens.border,
            width: 1.0,
            radius: radius::MD.into(),
        },
        ..container::Style::default()
    }
}

/// A colour swatch: the colour inside a thin border.
pub fn swatch_style(tokens: &Tokens, colour: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(colour)),
        border: Border {
            color: tokens.border,
            width: 1.0,
            radius: radius::SM.into(),
        },
        ..container::Style::default()
    }
}

/// The name column's width in a row `width` pixels wide. A dragged or set
/// width is kept between [`MIN_NAME_WIDTH`] and what leaves the value
/// column [`MIN_VALUE_WIDTH`]. Otherwise names take [`NAME_WIDTH`], and in
/// a narrow grid the value column gives up its room first, down to
/// [`VALUE_WIDTH`], so names stay readable.
pub fn name_width(requested: Option<f32>, width: f32) -> f32 {
    let room = width - HANDLE_WIDTH - RESET_WIDTH;
    let max = (room - MIN_VALUE_WIDTH).max(MIN_NAME_WIDTH);
    let wanted = requested.unwrap_or_else(|| NAME_WIDTH.min(room - VALUE_WIDTH));
    wanted.clamp(MIN_NAME_WIDTH, max)
}

type OnEvent<'a, Id, Message> = dyn Fn(Event<Id>) -> Message + 'a;
type Render<'a, Id, Message> = dyn Fn(&'a Property<Id>) -> Element<'a, Message> + 'a;
type BuildMenu<'a, Id, Message> =
    dyn FnOnce(Keymap<context_menu::Action>) -> RowMenu<'a, Id, Message> + 'a;

/// A property grid builder. Convert it into an [`Element`] to render.
pub struct PropertyGrid<'a, Id, Message> {
    state: &'a State<Id>,
    customs: Vec<(Id, Box<Render<'a, Id, Message>>)>,
    toolbar: bool,
    description: bool,
    placeholder: String,
    width: Length,
    height: Length,
    keymap: Keymap<Action>,
    menu: Option<Box<BuildMenu<'a, Id, Message>>>,
    menu_keymap: Keymap<context_menu::Action>,
    on_event: Option<Box<OnEvent<'a, Id, Message>>>,
}

/// Renders `state`. Without [`on_event`](PropertyGrid::on_event) every
/// editor renders disabled.
pub fn property_grid<Id, Message>(state: &State<Id>) -> PropertyGrid<'_, Id, Message> {
    PropertyGrid {
        state,
        customs: Vec::new(),
        toolbar: true,
        description: true,
        placeholder: "Filter".to_owned(),
        width: Length::Fill,
        height: Length::Shrink,
        keymap: default_keymap(),
        menu: None,
        menu_keymap: context_menu::default_keymap(),
        on_event: None,
    }
}

impl<Id: fmt::Debug, Message> fmt::Debug for PropertyGrid<'_, Id, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyGrid")
            .field("state", self.state)
            .field("toolbar", &self.toolbar)
            .field("description", &self.description)
            .field("placeholder", &self.placeholder)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("keymap", &self.keymap)
            .finish_non_exhaustive()
    }
}

impl<'a, Id, Message> PropertyGrid<'a, Id, Message> {
    /// Shows the filter box and the sort buttons above the rows. On by
    /// default.
    pub fn toolbar(mut self, toolbar: bool) -> Self {
        self.toolbar = toolbar;
        self
    }

    /// Shows the highlighted property's name and description under the
    /// rows. On by default. Errors show there even when it is off.
    pub fn description(mut self, description: bool) -> Self {
        self.description = description;
        self
    }

    /// The filter box placeholder.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// A fixed or filling height scrolls the rows. Defaults to shrinking to
    /// fit them.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Replaces the [`default_keymap`] the focused grid resolves keys with.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Id>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    /// Draws the value of property `id` with `render`, such as a slider.
    /// Keep it within a row's height. The app handles its messages and
    /// stores the result with [`State::set`].
    pub fn custom(
        mut self,
        id: Id,
        render: impl Fn(&'a Property<Id>) -> Element<'a, Message> + 'a,
    ) -> Self
    where
        Id: PartialEq,
    {
        self.customs.retain(|(custom, _)| *custom != id);
        self.customs.push((id, Box::new(render)));
        self
    }

    /// Gives every property row the context menu of `menu`, keyed by
    /// property, such as the one from [`menu()`]. It opens on a right-click
    /// or a long press, and on the highlighted property with its opening
    /// chord while the grid has focus.
    pub fn context_menu<MenuId>(
        mut self,
        menu: &'a context_menu::State<MenuId, Id>,
        on_event: impl Fn(context_menu::Event<MenuId, Id>) -> Message + 'a,
    ) -> Self
    where
        MenuId: Copy + PartialEq + 'a,
        Id: Clone + PartialEq + 'a,
        Message: Clone + 'a,
    {
        let on_event: Rc<dyn Fn(context_menu::Event<MenuId, Id>) -> Message + 'a> =
            Rc::new(on_event);
        self.menu = Some(Box::new(move |keymap| row_menu(menu, keymap, on_event)));
        self
    }

    /// Replaces the context menu's default keymap.
    pub fn context_menu_keymap(mut self, keymap: Keymap<context_menu::Action>) -> Self {
        self.menu_keymap = keymap;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_event.is_some()
    }
}

impl<'a, Id, Message> From<PropertyGrid<'a, Id, Message>> for Element<'a, Message>
where
    Id: Copy + Eq + Hash + 'a,
    Message: Clone + 'a,
{
    fn from(grid: PropertyGrid<'a, Id, Message>) -> Self {
        let PropertyGrid {
            state,
            customs,
            toolbar,
            description,
            placeholder,
            width,
            height,
            keymap,
            menu,
            menu_keymap,
            on_event,
        } = grid;
        let menu = menu
            .filter(|_| on_event.is_some())
            .map(|build| build(menu_keymap));
        let view = Rc::new(View {
            state,
            customs,
            toolbar,
            description,
            placeholder,
            height,
            keymap,
            menu,
            entries: state.entries(),
            on_event: on_event.map(Rc::from),
        });
        responsive(move |size| view.body(size.width))
            .width(width)
            .height(height)
            .into()
    }
}

/// Everything the grid's parts are built from.
struct View<'a, Id, Message> {
    state: &'a State<Id>,
    customs: Vec<(Id, Box<Render<'a, Id, Message>>)>,
    toolbar: bool,
    description: bool,
    placeholder: String,
    height: Length,
    keymap: Keymap<Action>,
    menu: Option<RowMenu<'a, Id, Message>>,
    entries: Vec<Entry<Id>>,
    on_event: Option<Rc<OnEvent<'a, Id, Message>>>,
}

fn muted(theme: &Theme) -> Color {
    Tokens::of(theme).muted_foreground
}

impl<'a, Id, Message> View<'a, Id, Message>
where
    Id: Copy + Eq + Hash + 'a,
    Message: Clone + 'a,
{
    fn send(&self, event: Event<Id>) -> Option<Message> {
        self.on_event.as_ref().map(|on_event| on_event(event))
    }

    fn body(self: &Rc<Self>, width: f32) -> Element<'a, Message> {
        let state = self.state;
        // The frame's border and the rows' padding on both sides.
        let inner = width - 2.0 - space::SM * 2.0;
        let names = name_width(state.name_width(), inner);

        let highlight = state
            .highlighted()
            .and_then(|highlighted| self.entries.iter().position(|entry| entry == highlighted));
        let list = {
            let view = Rc::clone(self);
            rows(self.entries.len(), ROW_HEIGHT, move |range| {
                view.lines(range, names)
            })
            .highlight(highlight)
            .height(if self.height == Length::Shrink {
                Length::Shrink
            } else {
                Length::Fill
            })
        };

        let mut inside = column![list];
        if let Some(footer) = self.footer() {
            inside = inside.push(container(Space::new()).width(Length::Fill).height(1).style(
                |theme| container::Style {
                    background: Some(Background::Color(Tokens::of(theme).border)),
                    ..container::Style::default()
                },
            ));
            inside = inside.push(footer);
        }
        let mut inside: Element<'a, Message> = inside.into();
        if state.is_resizing()
            && let Some(on_event) = self.on_event.clone()
        {
            let end = on_event(Event::ResizeEnd);
            inside = mouse_area(inside)
                .on_move(move |point| {
                    on_event(Event::Resize(point.x - space::SM - HANDLE_WIDTH / 2.0))
                })
                .on_release(end.clone())
                .on_exit(end)
                .interaction(mouse::Interaction::ResizingHorizontally)
                .into();
        }
        let frame = container(inside)
            .padding(1)
            .width(Length::Fill)
            .height(if self.height == Length::Shrink {
                Length::Shrink
            } else {
                Length::Fill
            })
            .clip(true)
            .style(|theme| frame_style(&Tokens::of(theme)));

        let mut parts = column![].spacing(space::SM);
        if self.toolbar {
            parts = parts.push(self.toolbar());
        }
        parts = parts.push(frame);
        let parts: Element<'a, Message> = parts.into();

        let Some(on_event) = self.on_event.clone() else {
            return parts;
        };
        let request = state.focus_request().map(|(count, target)| match target {
            Target::Filter => (count, SEARCH, false),
            Target::Editor => (count, EDITOR_ID, true),
        });
        let focuser = Focuser {
            content: parts,
            request,
        };
        let cancel = state.editing().map(|_| on_event(Event::Cancel));
        let clear = (!state.filter().is_empty()).then(|| on_event(Event::Filter(String::new())));
        let on_focus = on_event.clone();
        let keymap = self.keymap.clone();
        let view = Rc::clone(self);
        scope(focuser)
            .escape(EDITOR_ID, cancel)
            .escape(SEARCH, clear)
            .on_focus(move |focused| on_focus(Event::Focus(focused)))
            .on_key(move |key| {
                let opened = view
                    .menu
                    .as_ref()
                    .zip(state.highlighted_property())
                    .and_then(|(menu, property)| menu.key(key, property.id));
                opened.or_else(|| state.key_event(&keymap, key).map(|event| on_event(event)))
            })
            .into()
    }

    fn toolbar(&self) -> Element<'a, Message> {
        let state = self.state;
        let on_event = self.on_event.clone();
        let filter = input(&self.placeholder, state.filter())
            .id(SEARCH)
            .icon(crate::lucide!(Search))
            .size(input::Size::Sm)
            .width(Length::Fill)
            .on_input_maybe(on_event.map(|on_event| move |query| on_event(Event::Filter(query))));
        let sort = |sort: Sort, glyph, label| {
            icon_button(glyph)
                .label(label)
                .size(button::Size::Sm)
                .pressed(state.sort() == sort)
                .on_press_maybe(self.send(Event::Sort(sort)))
        };
        row![
            filter,
            search_count(match_label(state)),
            sort(Sort::Categorised, crate::lucide!(ListTree), "Categorised"),
            sort(
                Sort::Alphabetical,
                crate::lucide!(ArrowDownAZ),
                "Alphabetical"
            ),
        ]
        .spacing(space::XS)
        .align_y(Alignment::Center)
        .into()
    }

    /// The description area, or an error when the description is off.
    fn footer(&self) -> Option<Element<'a, Message>> {
        let state = self.state;
        let property = state.highlighted_property();
        let error = state
            .editing()
            .and_then(|id| state.error(id))
            .or_else(|| property.and_then(|property| state.error(property.id)));
        if !self.description && error.is_none() {
            return None;
        }
        let (title, body) = match state.highlighted() {
            Some(Entry::Property(_)) => (
                property
                    .map(|property| property.name.as_str())
                    .unwrap_or_default(),
                property.and_then(|property| property.description.as_deref()),
            ),
            Some(Entry::Category(name)) => (name.as_str(), None),
            None => ("", None),
        };
        let mut content = column![].spacing(2).width(Length::Fill);
        if self.description && title.is_empty() {
            content = content.push(widget::text(NOTHING_SELECTED).size(text_size::SM).style(
                |theme: &Theme| widget::text::Style {
                    color: Some(muted(theme)),
                },
            ));
        } else if self.description {
            content = content.push(ellipsis(title).font(theme::semibold()));
        }
        match (error, body) {
            (Some(error), _) => {
                content = content.push(widget::text(error.to_owned()).size(text_size::XS).style(
                    |theme: &Theme| widget::text::Style {
                        color: Some(Tokens::of(theme).destructive),
                    },
                ));
            }
            (None, Some(body)) if self.description => {
                content = content.push(widget::text(body).size(text_size::XS).style(
                    |theme: &Theme| widget::text::Style {
                        color: Some(muted(theme)),
                    },
                ));
            }
            _ => {}
        }
        let least = if self.description {
            DESCRIPTION_HEIGHT
        } else {
            DESCRIPTION_HEIGHT / 2.0
        };
        // The strut keeps the least height; longer text grows the area.
        let strut = Space::new().height(least - space::SM * 2.0);
        Some(
            container(row![content, strut])
                .padding([space::SM, space::MD])
                .width(Length::Fill)
                .into(),
        )
    }

    fn lines(&self, range: Range<usize>, names: f32) -> Element<'a, Message> {
        let entries = self.entries.get(range).unwrap_or_default();
        column(entries.iter().map(|entry| match entry {
            Entry::Category(name) => self.heading(name),
            Entry::Property(id) => {
                let row = self.property(*id, names);
                match &self.menu {
                    Some(menu) => menu.wrap(*id, row),
                    None => row,
                }
            }
        }))
        .into()
    }

    fn status(&self, entry: &Entry<Id>, disabled: bool) -> Status {
        let state = self.state;
        Status {
            highlighted: state.highlighted() == Some(entry),
            focused: state.is_focused(),
            hovered: false,
            disabled,
            heading: matches!(entry, Entry::Category(_)),
        }
    }

    fn heading(&self, name: &str) -> Element<'a, Message> {
        let state = self.state;
        let entry = Entry::Category(name.to_owned());
        let status = self.status(&entry, self.on_event.is_none());
        let enabled = self.on_event.is_some();
        let glyph = if state.is_expanded(name) {
            crate::lucide!(ChevronDown)
        } else {
            crate::lucide!(ChevronRight)
        };
        let colours = move |theme: &Theme| row_style(&Tokens::of(theme), status);
        let count = state.count(name);
        let body = row![
            themed(glyph, ICON_SIZE, opacity(enabled), move |theme| colours(
                theme
            )
            .icon),
            ellipsis(name.to_owned())
                .font(theme::semibold())
                .colour(move |theme| colours(theme).text),
            widget::text(count.to_string())
                .size(text_size::XS)
                .wrapping(widget::text::Wrapping::None)
                .style(|theme: &Theme| widget::text::Style {
                    color: Some(muted(theme)),
                }),
        ]
        .spacing(space::SM)
        .align_y(Alignment::Center)
        .padding(Padding::ZERO.left(space::SM).right(space::SM))
        .height(ROW_HEIGHT);
        let row = pressable(body, move |theme, hovered| {
            look(row_style(&Tokens::of(theme), Status { hovered, ..status }))
        });
        match self.send(Event::Press(entry)) {
            Some(message) => row.on_press(move |_| message.clone()).into(),
            None => row.into(),
        }
    }

    fn property(&self, id: Id, names: f32) -> Element<'a, Message> {
        let state = self.state;
        let Some(property) = state.property(id) else {
            return Space::new().height(ROW_HEIGHT).into();
        };
        let entry = Entry::Property(id);
        let on_event = self.on_event.clone().filter(|_| !property.disabled);
        let enabled = on_event.is_some();
        let status = self.status(&entry, !enabled);
        let colours = move |theme: &Theme| row_style(&Tokens::of(theme), status);
        let modified = property.is_modified();

        let mark: Element<'a, Message> = if modified {
            container(
                container(Space::new())
                    .width(MARK_SIZE)
                    .height(MARK_SIZE)
                    .style(|theme| container::Style {
                        background: Some(Background::Color(mark_colour(&Tokens::of(theme)))),
                        border: Border {
                            radius: radius::FULL.into(),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    }),
            )
            .width(MARK_WIDTH)
            .into()
        } else {
            Space::new().width(MARK_WIDTH).into()
        };
        let name = ellipsis(property.name.as_str())
            .font(if modified {
                theme::semibold()
            } else {
                theme::font()
            })
            .colour(move |theme| colours(theme).text);
        let name = container(row![mark, name].align_y(Alignment::Center))
            .width(names)
            .height(Length::Fill)
            .align_y(Alignment::Center);

        let handle = self.handle();
        let value = container(self.value(property))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Alignment::Center);

        let reset: Element<'a, Message> = match &on_event {
            Some(on_event)
                if modified || (state.draft(id).is_some() && property.default.is_some()) =>
            {
                let icon = themed(crate::lucide!(RotateCcw), RESET_ICON_SIZE, 1.0, muted);
                let button = mouse_area(container(icon).center(RESET_WIDTH))
                    .on_press(on_event(Event::Reset(id)))
                    .interaction(mouse::Interaction::Pointer);
                tooltip(button, "Reset to default").into()
            }
            _ => Space::new().width(RESET_WIDTH).into(),
        };

        let body = row![name, handle, value, reset]
            .align_y(Alignment::Center)
            .padding(Padding::ZERO.left(space::SM).right(space::SM))
            .height(ROW_HEIGHT);
        let row = pressable(body, move |theme, hovered| {
            look(row_style(&Tokens::of(theme), Status { hovered, ..status }))
        });
        let Some(on_event) = self.on_event.clone() else {
            return row.into();
        };
        let press = on_event(Event::Press(entry));
        let edit = property.is_editable().then(|| on_event(Event::Edit(id)));
        row.on_press(move |_| press.clone())
            .on_double_click(edit)
            .into()
    }

    /// The line between the columns, which drags to resize them.
    fn handle(&self) -> Element<'a, Message> {
        let resizing = self.state.is_resizing();
        let line = container(container(Space::new()).width(1).height(Length::Fill).style(
            move |theme| container::Style {
                background: Some(Background::Color(divider_colour(
                    &Tokens::of(theme),
                    resizing,
                ))),
                ..container::Style::default()
            },
        ))
        .center_x(HANDLE_WIDTH)
        .height(Length::Fill);
        match self.send(Event::ResizeStart) {
            Some(start) => mouse_area(line)
                .on_press(start)
                .interaction(mouse::Interaction::ResizingHorizontally)
                .into(),
            None => line.into(),
        }
    }

    fn value(&self, property: &'a Property<Id>) -> Element<'a, Message> {
        let state = self.state;
        let id = property.id;
        if let Some((_, render)) = self.customs.iter().find(|(custom, _)| *custom == id) {
            return render(property);
        }
        let on_event = self.on_event.clone().filter(|_| !property.disabled);
        let editable = on_event.is_some() && property.is_editable();

        match (&property.kind, &property.value) {
            (Kind::Toggle(toggle), Value::Bool(on)) => {
                let set =
                    on_event.map(|on_event| move |on| on_event(Event::Set(id, Value::Bool(on))));
                let control: Element<'a, Message> = match toggle {
                    Toggle::Checkbox => checkbox(*on).on_toggle_maybe(set).into(),
                    Toggle::Switch => switch(*on).on_toggle_maybe(set).into(),
                };
                container(control)
                    .padding(Padding::ZERO.left(space::SM))
                    .into()
            }
            (Kind::Choice(options), Value::Text(current)) => {
                select(options.as_slice(), Some(current.clone()))
                    .width(Length::Fill)
                    .on_select_maybe(on_event.map(|on_event| {
                        move |option: String| on_event(Event::Set(id, Value::Text(option)))
                    }))
                    .into()
            }
            _ if property.is_typed() && editable => match state.draft(id) {
                Some(draft) => self.field(property, draft),
                None => self.shown(property, true),
            },
            _ => self.shown(property, editable),
        }
    }

    /// A value shown as text, which starts an edit when pressed.
    fn shown(&self, property: &'a Property<Id>, editable: bool) -> Element<'a, Message> {
        let label = ellipsis(property.display())
            .colour(move |theme| value_colour(&Tokens::of(theme), editable));
        let content: Element<'a, Message> = match property.value {
            Value::Colour(colour) => row![swatch(colour), label]
                .spacing(space::SM)
                .align_y(Alignment::Center)
                .into(),
            _ => label.into(),
        };
        let content = container(content)
            .padding(Padding::ZERO.left(space::SM))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Alignment::Center);
        match self
            .send(Event::Edit(property.id))
            .filter(|_| editable && property.is_typed())
        {
            Some(edit) => mouse_area(content)
                .on_press(edit)
                .interaction(mouse::Interaction::Text)
                .into(),
            None => content.into(),
        }
    }

    /// The text field of a property being edited, or left while invalid.
    fn field(&self, property: &'a Property<Id>, draft: &'a Draft) -> Element<'a, Message> {
        let id = property.id;
        let mut field = input("", &draft.text)
            .size(input::Size::Sm)
            .invalid(draft.error.is_some())
            .on_input_maybe(
                self.on_event
                    .clone()
                    .map(|on_event| move |text| on_event(Event::Input(id, text))),
            )
            .on_submit_maybe(self.send(Event::Submit));
        if self.state.editing() == Some(id) {
            field = field.id(EDITOR_ID);
        }
        let Value::Colour(current) = property.value else {
            return field.into();
        };
        let colour = parse_hex(&draft.text).unwrap_or(current);
        row![swatch(colour), field]
            .spacing(space::SM)
            .align_y(Alignment::Center)
            .padding(Padding::ZERO.left(space::SM))
            .into()
    }
}

fn swatch<'a, Message: 'a>(colour: Color) -> Element<'a, Message> {
    container(Space::new())
        .width(SWATCH_SIZE)
        .height(SWATCH_SIZE)
        .style(move |theme| swatch_style(&Tokens::of(theme), colour))
        .into()
}

fn look(style: RowStyle) -> Look {
    Look {
        background: style.background,
        border: Border {
            color: style.ring.unwrap_or(Color::TRANSPARENT),
            width: if style.ring.is_some() { 1.0 } else { 0.0 },
            radius: radius::SM.into(),
        },
        divider: None,
    }
}

/// Focuses a field inside the grid once for each request the state makes,
/// selecting its text when asked.
struct Focuser<'a, Message> {
    content: Element<'a, Message>,
    request: Option<(u64, widget::Id, bool)>,
}

/// The last focus request acted on.
#[derive(Debug, Default)]
struct Handled(u64);

impl<Message: Clone> Widget<Message, Theme, Renderer> for Focuser<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Handled>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Handled::default())
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

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let Tree {
            state, children, ..
        } = tree;
        let Some(child) = children.first_mut() else {
            return layout::Node::new(Size::ZERO);
        };
        let node = self.content.as_widget_mut().layout(child, renderer, limits);
        let handled = state.downcast_mut::<Handled>();
        // The field may only exist from this layout on, so focus it here,
        // once the new rows have been built.
        if let Some((count, id, select_all)) = &self.request
            && handled.0 != *count
        {
            handled.0 = *count;
            let layout = Layout::new(&node);
            let mut focus = operation::focusable::focus(id.clone());
            self.content
                .as_widget_mut()
                .operate(child, layout, renderer, &mut focus);
            if *select_all {
                let mut select = operation::text_input::select_all(id.clone());
                self.content
                    .as_widget_mut()
                    .operate(child, layout, renderer, &mut select);
            }
        }
        node
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
        if let Some(child) = tree.children.first() {
            self.content
                .as_widget()
                .draw(child, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(child) = tree.children.first_mut() {
            self.content
                .as_widget_mut()
                .operate(child, layout, renderer, operation);
        }
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
        let Some(child) = tree.children.first_mut() else {
            return;
        };
        self.content.as_widget_mut().update(
            child, event, layout, cursor, renderer, clipboard, shell, viewport,
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
        tree.children
            .first()
            .map_or(mouse::Interaction::None, |child| {
                self.content
                    .as_widget()
                    .mouse_interaction(child, layout, cursor, viewport, renderer)
            })
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let child = tree.children.first_mut()?;
        self.content
            .as_widget_mut()
            .overlay(child, layout, renderer, viewport, translation)
    }
}

impl<'a, Message: Clone + 'a> From<Focuser<'a, Message>> for Element<'a, Message> {
    fn from(focuser: Focuser<'a, Message>) -> Self {
        Element::new(focuser)
    }
}

#[cfg(test)]
mod tests {
    use iced::keyboard::{Key, Modifiers};

    use super::*;
    use crate::theme::{dark, light};

    fn even(text: &str) -> Result<(), String> {
        if text.len().is_multiple_of(2) {
            Ok(())
        } else {
            Err("Needs an even length".to_owned())
        }
    }

    /// ```text
    /// 1 Id (read-only, no category)
    /// Layout: 2 Width, 3 Height, 4 Visible
    /// Appearance: 5 Fill, 6 Blend, 7 Label, 8 Locked (disabled)
    /// ```
    fn sample() -> State<u32> {
        State::new([
            read_only(1, "Id", "rect-1"),
            number(2, "Width", 120.0)
                .category("Layout")
                .range(0.0, 400.0)
                .step(1.0)
                .unit("px")
                .default(100.0),
            number(3, "Height", 0.5)
                .category("Layout")
                .range(0.0, 1.0)
                .step(0.25),
            toggle(4, "Visible", true).category("Layout").default(true),
            colour(5, "Fill", Color::from_rgb8(0x3b, 0x82, 0xf6))
                .category("Appearance")
                .default(Color::WHITE),
            choice(6, "Blend", ["Normal", "Multiply", "Screen"], "Normal")
                .category("Appearance")
                .description("How the layer mixes with what is under it."),
            text(7, "Label", "Box")
                .category("Appearance")
                .validate(even),
            toggle(8, "Locked", false)
                .category("Appearance")
                .disabled(true)
                .default(true),
        ])
    }

    fn property_entries(ids: &[u32]) -> Vec<Entry<u32>> {
        ids.iter().map(|&id| Entry::Property(id)).collect()
    }

    fn category(name: &str) -> Entry<u32> {
        Entry::Category(name.to_owned())
    }

    fn key(key: Key, modifiers: Modifiers) -> keys::Event {
        keys::Event { key, modifiers }
    }

    fn named(named: Named) -> keys::Event {
        key(Key::Named(named), Modifiers::empty())
    }

    fn changed(id: u32, value: impl Into<Value>) -> Option<Output<u32>> {
        Some(Output::Changed {
            id,
            value: value.into(),
        })
    }

    #[test]
    fn lists_uncategorised_properties_first_then_each_category() {
        let state = sample();
        assert_eq!(state.categories(), ["Layout", "Appearance"]);
        let mut expected = vec![Entry::Property(1), category("Layout")];
        expected.extend(property_entries(&[2, 3, 4]));
        expected.push(category("Appearance"));
        expected.extend(property_entries(&[5, 6, 7, 8]));
        assert_eq!(state.entries(), expected);
        assert_eq!(state.count("Layout"), 3);
    }

    #[test]
    fn alphabetical_lists_every_property_by_name() {
        let state = sample().with_sort(Sort::Alphabetical);
        assert_eq!(state.entries(), property_entries(&[6, 5, 3, 1, 7, 8, 4, 2]));
        assert_eq!(Sort::ALL.len(), 2);
        assert_eq!(Sort::default(), Sort::Categorised);
    }

    #[test]
    fn a_repeated_id_replaces_the_earlier_property() {
        let state = State::new([text(1, "Old", "a"), text(1, "New", "b")]);
        assert_eq!(state.properties().len(), 1);
        assert_eq!(state.property(1).map(|p| p.name.as_str()), Some("New"));
        assert!(State::<u8>::new([]).entries().is_empty());
    }

    #[test]
    fn the_filter_matches_names_and_drops_empty_categories() {
        let mut state = sample();
        let _ = state.update(Event::Press(Entry::Property(2)));
        let _ = state.update(Event::Filter("FI".into()));
        assert_eq!(
            state.entries(),
            vec![category("Appearance"), Entry::Property(5)]
        );
        assert!(state.matches(5) && !state.matches(2));
        assert_eq!(state.count("Layout"), 0);
        assert_eq!(
            state.highlighted(),
            Some(&Entry::Property(5)),
            "a hidden highlight moves to the first match"
        );
        let _ = state.update(Event::Filter("nothing".into()));
        assert!(state.entries().is_empty());
        assert_eq!(state.highlighted(), None);
    }

    #[test]
    fn pressing_a_heading_toggles_it_and_collapsing_lifts_the_highlight() {
        let mut state = sample();
        let _ = state.update(Event::Press(Entry::Property(3)));
        let _ = state.update(Event::Collapse("Layout".into()));
        assert!(!state.is_expanded("Layout"));
        assert_eq!(state.highlighted(), Some(&category("Layout")));
        assert!(!state.entries().contains(&Entry::Property(3)));
        let _ = state.update(Event::Press(category("Layout")));
        assert!(state.is_expanded("Layout"));
        let _ = state.update(Event::ToggleCategory("Layout".into()));
        assert!(!state.is_expanded("Layout"));
        let _ = state.update(Event::Expand("Layout".into()));
        assert!(state.is_expanded("Layout"));
        let _ = state.update(Event::Collapse("Missing".into()));
        assert!(
            state.is_expanded("Missing"),
            "unknown categories are ignored"
        );
    }

    #[test]
    fn with_collapsed_starts_closed() {
        let state = sample().with_collapsed(["Appearance"]);
        assert!(!state.is_expanded("Appearance"));
        assert_eq!(state.entries().last(), Some(&category("Appearance")));
    }

    #[test]
    fn arrows_walk_the_rows_without_wrapping() {
        let mut state = sample();
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(&Entry::Property(1)));
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(&category("Layout")));
        let _ = state.update(Event::Last);
        assert_eq!(state.highlighted(), Some(&Entry::Property(8)));
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(&Entry::Property(8)));
        let _ = state.update(Event::First);
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(&Entry::Property(1)));
    }

    #[test]
    fn right_opens_then_enters_and_left_closes_then_leaves() {
        let mut state = sample().with_collapsed(["Layout"]);
        let _ = state.update(Event::Press(category("Layout")));
        assert!(state.is_expanded("Layout"));
        let _ = state.update(Event::CollapseOrLeave);
        assert!(!state.is_expanded("Layout"));
        let _ = state.update(Event::ExpandOrEnter);
        assert!(state.is_expanded("Layout"));
        let _ = state.update(Event::ExpandOrEnter);
        assert_eq!(state.highlighted(), Some(&Entry::Property(2)));
        let _ = state.update(Event::CollapseOrLeave);
        assert_eq!(state.highlighted(), Some(&category("Layout")));
        assert!(state.is_expanded("Layout"), "leaving does not close");
    }

    #[test]
    fn editing_text_starts_a_draft_and_asks_for_focus() {
        let mut state = sample();
        assert!(state.update(Event::Edit(2)).is_none());
        assert_eq!(state.editing(), Some(2));
        assert_eq!(state.draft(2).map(|d| d.text.as_str()), Some("120"));
        assert_eq!(state.highlighted(), Some(&Entry::Property(2)));
        assert_eq!(state.focus_request(), Some((1, Target::Editor)));
        let _ = state.update(Event::Edit(2));
        assert_eq!(state.focus_request(), Some((2, Target::Editor)));
    }

    #[test]
    fn a_valid_edit_changes_the_value_as_it_is_typed() {
        let mut state = sample();
        let _ = state.update(Event::Edit(2));
        assert_eq!(state.update(Event::Input(2, "12".into())), changed(2, 12.0));
        assert_eq!(state.update(Event::Input(2, "12px".into())), None);
        assert_eq!(state.value(2), Some(&Value::Number(12.0)));
        let _ = state.update(Event::Submit);
        assert_eq!(state.editing(), None);
        assert!(state.draft(2).is_none());
    }

    #[test]
    fn an_invalid_edit_keeps_its_draft_and_error() {
        let mut state = sample();
        let _ = state.update(Event::Edit(2));
        assert_eq!(state.update(Event::Input(2, "9000".into())), None);
        assert_eq!(state.error(2), Some("Must be at most 400px"));
        assert_eq!(state.value(2), Some(&Value::Number(120.0)));
        let _ = state.update(Event::Submit);
        assert_eq!(
            state.editing(),
            Some(2),
            "Enter does not leave an invalid edit"
        );

        let _ = state.update(Event::Press(Entry::Property(3)));
        assert_eq!(state.editing(), None);
        assert_eq!(
            state.draft(2).map(|d| d.text.as_str()),
            Some("9000"),
            "the invalid draft stays after leaving"
        );
        assert_eq!(state.update(Event::Input(2, "40".into())), changed(2, 40.0));
        assert_eq!(state.editing(), Some(2), "typing again resumes the edit");
        assert_eq!(state.error(2), None);
    }

    #[test]
    fn leaving_a_valid_edit_drops_its_draft() {
        let mut state = sample();
        let _ = state.update(Event::Edit(7));
        let _ = state.update(Event::Input(7, "Card".into()));
        let _ = state.update(Event::Focus(false));
        assert_eq!(state.editing(), None);
        assert!(state.draft(7).is_none());
        assert_eq!(state.value(7), Some(&Value::from("Card")));
    }

    #[test]
    fn cancel_puts_back_the_value_from_before_the_edit() {
        let mut state = sample();
        let _ = state.update(Event::Edit(7));
        let _ = state.update(Event::Input(7, "Card".into()));
        let _ = state.update(Event::Input(7, "Car".into()));
        assert_eq!(state.error(7), Some("Needs an even length"));
        assert_eq!(state.update(Event::Cancel), changed(7, "Box"));
        assert_eq!(state.editing(), None);
        assert!(state.draft(7).is_none());

        let _ = state.update(Event::Edit(7));
        assert_eq!(state.update(Event::Cancel), None, "nothing changed");
    }

    #[test]
    fn cancel_without_an_edit_clears_the_filter() {
        let mut state = sample();
        let _ = state.update(Event::Filter("wid".into()));
        let _ = state.update(Event::Cancel);
        assert_eq!(state.filter(), "");
    }

    #[test]
    fn edit_flips_toggles_and_steps_choices() {
        let mut state = sample();
        assert_eq!(state.update(Event::Edit(4)), changed(4, false));
        assert_eq!(state.editing(), None);
        assert_eq!(state.update(Event::Edit(6)), changed(6, "Multiply"));
        let _ = state.update(Event::Edit(6));
        assert_eq!(
            state.update(Event::Edit(6)),
            changed(6, "Normal"),
            "wraps round"
        );
    }

    #[test]
    fn read_only_and_disabled_properties_do_not_edit() {
        let mut state = sample();
        assert_eq!(state.update(Event::Edit(1)), None);
        assert_eq!(state.editing(), None);
        assert_eq!(state.update(Event::Edit(8)), None);
        assert_eq!(state.update(Event::Set(8, Value::Bool(true))), None);
        assert_eq!(state.update(Event::Input(8, "x".into())), None);
        assert_eq!(state.update(Event::Reset(8)), None);
        assert_eq!(state.highlighted(), Some(&Entry::Property(8)));
    }

    #[test]
    fn set_accepts_only_values_that_suit_the_property() {
        let mut state = sample();
        assert_eq!(
            state.update(Event::Set(6, "Screen".into())),
            changed(6, "Screen")
        );
        assert_eq!(state.update(Event::Set(6, "Overlay".into())), None);
        assert_eq!(state.update(Event::Set(6, Value::Bool(true))), None);
        assert_eq!(
            state.update(Event::Set(4, Value::Bool(true))),
            None,
            "unchanged"
        );
        assert_eq!(state.update(Event::Set(99, Value::Bool(true))), None);
    }

    #[test]
    fn reset_puts_back_the_default() {
        let mut state = sample();
        assert!(state.property(2).is_some_and(Property::is_modified));
        assert_eq!(
            state.update(Event::Reset(2)),
            Some(Output::Reset {
                id: 2,
                value: Value::Number(100.0)
            })
        );
        assert!(!state.property(2).is_some_and(Property::is_modified));
        assert_eq!(state.update(Event::Reset(2)), None, "already the default");
        assert_eq!(state.update(Event::Reset(3)), None, "no default");
    }

    #[test]
    fn reset_drops_an_invalid_draft() {
        let mut state = sample();
        let _ = state.update(Event::Edit(2));
        let _ = state.update(Event::Input(2, "abc".into()));
        let _ = state.update(Event::Press(Entry::Property(2)));
        let _ = state.update(Event::ResetHighlighted);
        assert!(state.draft(2).is_none());
        assert_eq!(state.editing(), None);
        assert_eq!(state.value(2), Some(&Value::Number(100.0)));
    }

    #[test]
    fn up_and_down_step_the_number_being_edited() {
        let mut state = sample();
        let _ = state.update(Event::Edit(3));
        assert_eq!(state.update(Event::Increment), changed(3, 0.75));
        assert_eq!(state.update(Event::Increment), changed(3, 1.0));
        assert_eq!(state.update(Event::Increment), None, "clamped to the max");
        assert_eq!(state.draft(3).map(|d| d.text.as_str()), Some("1.00"));
        assert_eq!(state.update(Event::Decrement), changed(3, 0.75));
        let _ = state.update(Event::Submit);
        assert_eq!(state.update(Event::Increment), None, "only while editing");
    }

    #[test]
    fn numbers_parse_with_their_unit_and_limits() {
        let number = Number {
            min: Some(0.0),
            max: Some(100.0),
            step: Some(5.0),
            decimals: None,
            unit: Some("%".into()),
        };
        assert_eq!(number.parse(" 25% "), Ok(25.0));
        assert_eq!(number.parse("25"), Ok(25.0));
        assert_eq!(number.parse("-5"), Err("Must be at least 0%".into()));
        assert_eq!(number.parse("105"), Err("Must be at most 100%".into()));
        assert_eq!(number.parse("12"), Err("Must be a multiple of 5".into()));
        assert_eq!(number.parse("abc"), Err("Enter a number".into()));
        assert_eq!(number.parse("inf"), Err("Enter a number".into()));
        assert_eq!(number.display(25.0), "25%");
        assert_eq!(number.stepped(98.0, true), 100.0);

        let whole = Number {
            step: Some(1.0),
            ..Number::default()
        };
        assert_eq!(whole.parse("1.5"), Err("Must be a whole number".into()));
        let fine = Number {
            step: Some(0.1),
            ..Number::default()
        };
        assert_eq!(
            fine.parse("0.15"),
            Ok(0.15),
            "fractional steps do not validate"
        );
        assert_eq!(fine.format(0.15), "0.1");
        assert_eq!(fine.stepped(0.2, true), 0.3);
        assert_eq!(Number::default().format(-0.0), "0");
        assert_eq!(Number::default().format(2.5), "2.5");
        let fixed = Number {
            decimals: Some(2),
            ..Number::default()
        };
        assert_eq!(fixed.format(1.0), "1.00");
    }

    #[test]
    fn hex_colours_round_trip() {
        let blue = Color::from_rgb8(0x3b, 0x82, 0xf6);
        assert_eq!(hex(blue), "#3B82F6");
        assert_eq!(parse_hex("#3b82f6"), Ok(blue));
        assert_eq!(parse_hex("3B82F6"), Ok(blue));
        assert_eq!(parse_hex("#fff"), Ok(Color::WHITE));
        assert_eq!(parse_hex(" #000 "), Ok(Color::BLACK));
        let faded = parse_hex("#00000080").map(hex);
        assert_eq!(faded, Ok("#00000080".to_owned()));
        assert_eq!(parse_hex("#0008").map(hex), Ok("#00000088".to_owned()));
        for bad in ["", "#", "#12", "#12345", "#ggg", "blue", "#3B82F6F6F6"] {
            assert!(parse_hex(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn colour_edits_parse_hex() {
        let mut state = sample();
        let _ = state.update(Event::Edit(5));
        assert_eq!(state.draft(5).map(|d| d.text.as_str()), Some("#3B82F6"));
        assert_eq!(state.update(Event::Input(5, "#zz".into())), None);
        assert!(state.error(5).is_some());
        assert_eq!(
            state.update(Event::Input(5, "#fff".into())),
            changed(5, Color::WHITE)
        );
        assert!(!state.property(5).is_some_and(Property::is_modified));
    }

    #[test]
    fn property_text_and_parsing_follow_the_kind() {
        let state = sample();
        let show = |id| {
            state
                .property(id)
                .map(Property::display)
                .unwrap_or_default()
        };
        assert_eq!(show(2), "120px");
        assert_eq!(show(3), "0.50");
        assert_eq!(show(4), "On");
        assert_eq!(show(5), "#3B82F6");
        assert_eq!(
            state.property(2).map(Property::edit_text),
            Some("120".into())
        );
        let blend = state.property(6).map(|p| p.parse("Screen"));
        assert_eq!(blend, Some(Ok(Value::from("Screen"))));
        assert!(state.property(6).is_some_and(|p| p.parse("Nope").is_err()));
        assert!(state.property(1).is_some_and(|p| !p.is_editable()));
        assert!(state.property(8).is_some_and(|p| !p.is_editable()));
        assert!(state.property(4).is_some_and(|p| !p.is_typed()));
        let shown = custom::<u8>(1, "Opacity", 0.5);
        assert_eq!(shown.display(), "0.5");
        assert!(shown.accepts(&Value::Bool(true)));
        assert_eq!(Toggle::ALL.len(), 2);
    }

    #[test]
    fn number_settings_only_apply_to_numbers() {
        let label = text::<u8>(1, "Label", "a").min(1.0).unit("px").switch();
        assert!(matches!(label.kind, Kind::Text(None)));
        let flag = toggle::<u8>(2, "Flag", true).switch();
        assert!(matches!(flag.kind, Kind::Toggle(Toggle::Switch)));
        let size = number::<u8>(3, "Size", 1.0).decimals(1).validate(even);
        assert!(matches!(
            size.kind,
            Kind::Number(Number {
                decimals: Some(1),
                ..
            })
        ));
    }

    #[test]
    fn values_convert_and_read_back() {
        assert_eq!(Value::from(2).as_number(), Some(2.0));
        assert_eq!(Value::from(0.5f32).as_number(), Some(0.5));
        assert_eq!(Value::from("a").as_text(), Some("a"));
        assert_eq!(Value::from(String::from("b")).as_text(), Some("b"));
        assert_eq!(Value::from(true).as_bool(), Some(true));
        assert_eq!(Value::from(Color::BLACK).as_colour(), Some(Color::BLACK));
        assert_eq!(Value::from(true).as_text(), None);
        assert_eq!(Value::from("a").as_number(), None);
        assert_eq!(Value::from("a").as_bool(), None);
        assert_eq!(Value::from("a").as_colour(), None);
    }

    #[test]
    fn set_from_the_app_drops_the_edit() {
        let mut state = sample();
        let _ = state.update(Event::Edit(7));
        assert!(state.set(7, "Card"));
        assert_eq!(state.editing(), None);
        assert!(state.draft(7).is_none());
        assert!(!state.set(7, "Card"));
        assert!(!state.set(99, "Card"));
    }

    #[test]
    fn the_name_column_drags_only_while_resizing() {
        let mut state = sample().with_name_width(10.0);
        assert_eq!(state.name_width(), Some(MIN_NAME_WIDTH));
        let _ = state.update(Event::Resize(150.0));
        assert_eq!(state.name_width(), Some(MIN_NAME_WIDTH));
        let _ = state.update(Event::ResizeStart);
        assert!(state.is_resizing());
        let _ = state.update(Event::Resize(150.0));
        assert_eq!(state.name_width(), Some(150.0));
        let _ = state.update(Event::Resize(2.0));
        assert_eq!(state.name_width(), Some(MIN_NAME_WIDTH));
        let _ = state.update(Event::ResizeEnd);
        assert!(!state.is_resizing());
    }

    #[test]
    fn the_name_column_keeps_room_for_values() {
        assert_eq!(name_width(None, 600.0), NAME_WIDTH);
        let narrow = name_width(None, 260.0);
        let room = 260.0 - HANDLE_WIDTH - RESET_WIDTH;
        assert_eq!(narrow, room - VALUE_WIDTH, "the values give way first");
        assert!(narrow > room * 0.45, "names keep a fair share");
        assert_eq!(
            name_width(None, 300.0 + NAME_WIDTH),
            NAME_WIDTH,
            "values only give way when room runs short"
        );
        assert_eq!(
            name_width(Some(500.0), 300.0),
            300.0 - HANDLE_WIDTH - RESET_WIDTH - MIN_VALUE_WIDTH
        );
        assert_eq!(name_width(Some(10.0), 300.0), MIN_NAME_WIDTH);
        assert_eq!(
            name_width(None, 100.0),
            MIN_NAME_WIDTH,
            "never below the minimum"
        );
    }

    #[test]
    fn focus_highlights_the_first_property_and_losing_it_ends_edits() {
        let mut state = sample();
        let _ = state.update(Event::Focus(true));
        assert!(state.is_focused());
        assert_eq!(state.highlighted(), Some(&Entry::Property(1)));
        let _ = state.update(Event::Edit(2));
        let _ = state.update(Event::ResizeStart);
        let _ = state.update(Event::Focus(false));
        assert_eq!(state.editing(), None);
        assert!(!state.is_resizing());
    }

    #[test]
    fn focus_search_asks_for_the_filter_box() {
        let mut state = sample();
        let _ = state.update(Event::Edit(7));
        let _ = state.update(Event::FocusSearch);
        assert_eq!(state.editing(), None);
        assert_eq!(state.focus_request(), Some((2, Target::Filter)));
        let _ = state.update(Event::Sort(Sort::Alphabetical));
        assert_eq!(state.sort(), Sort::Alphabetical);
    }

    #[test]
    fn copying_gives_the_value_as_shown_and_highlights_it() {
        let mut state = sample();
        assert_eq!(
            state.update(Event::CopyValue(2)),
            Some(Output::CopyValue("120px".into()))
        );
        assert_eq!(state.highlighted(), Some(&Entry::Property(2)));
        assert_eq!(
            state.update(Event::CopyValue(5)),
            Some(Output::CopyValue("#3B82F6".into()))
        );
        assert_eq!(state.update(Event::CopyValue(99)), None);
    }

    #[test]
    fn menu_items_map_to_events() {
        assert_eq!(MenuItem::ALL.len(), 2);
        assert_eq!(MenuItem::CopyValue.event(4), Event::CopyValue(4));
        assert_eq!(MenuItem::Reset.event(4), Event::Reset(4));
        assert!(!menu::<u32>().is_open());
    }

    #[test]
    fn the_match_label_counts_what_passes_the_filter() {
        let mut state = sample();
        assert_eq!(match_label(&state), "");
        let _ = state.update(Event::Filter("i".into()));
        assert_eq!(
            match_label(&state),
            "5 of 8",
            "Id, Width, Height, Visible, Fill"
        );
        let _ = state.update(Event::Filter("zebra".into()));
        assert_eq!(match_label(&state), "No matches");
    }

    #[test]
    fn default_keymap_resolves_each_action() {
        let keymap = default_keymap();
        let cases = [
            (named(Named::ArrowDown), Action::Next),
            (named(Named::ArrowUp), Action::Previous),
            (named(Named::Home), Action::First),
            (named(Named::End), Action::Last),
            (named(Named::ArrowRight), Action::Expand),
            (named(Named::ArrowLeft), Action::Collapse),
            (named(Named::Enter), Action::Edit),
            (named(Named::F2), Action::Edit),
            (named(Named::Escape), Action::Cancel),
            (named(Named::Delete), Action::Reset),
            (
                key(Key::Character("c".into()), Modifiers::COMMAND),
                Action::CopyValue,
            ),
            (
                key(Key::Character("f".into()), Modifiers::COMMAND),
                Action::FocusSearch,
            ),
        ];
        for (press, action) in cases {
            assert_eq!(keymap.resolve_event(&press), Some(action), "{press:?}");
        }
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?} unbound");
            assert!(keys::Action::description(*action).ends_with('.'));
            assert!(!keys::Action::name(*action).is_empty());
        }
    }

    #[test]
    fn keymaps_can_be_changed() {
        let keymap = default_keymap()
            .bind(Chord::character('j'), Action::Next)
            .unbind(&Chord::named(Named::Delete));
        let mut state = sample();
        let _ = state.update(Event::Focus(true));
        let j = key(Key::Character("j".into()), Modifiers::empty());
        assert_eq!(state.key_event(&keymap, &j), Some(Event::Next));
        let _ = state.update(Event::Press(Entry::Property(2)));
        assert_eq!(state.key_event(&keymap, &named(Named::Delete)), None);
        assert_eq!(
            state.key_event(&default_keymap(), &named(Named::Delete)),
            Some(Event::ResetHighlighted)
        );
    }

    #[test]
    fn actions_leave_keys_they_cannot_use() {
        let mut state = sample();
        assert_eq!(Action::Edit.event(&state), None, "nothing highlighted");
        assert_eq!(Action::Cancel.event(&state), None);
        assert_eq!(Action::FocusSearch.event(&state), Some(Event::FocusSearch));
        assert_eq!(Action::CopyValue.event(&state), None);

        let _ = state.update(Event::Press(Entry::Property(1)));
        assert_eq!(Action::CopyValue.event(&state), Some(Event::CopyValue(1)));
        assert_eq!(Action::Edit.event(&state), None, "read-only");
        assert_eq!(Action::Expand.event(&state), None);
        assert_eq!(Action::Collapse.event(&state), None, "no category");
        assert_eq!(Action::Reset.event(&state), None);

        let _ = state.update(Event::Press(Entry::Property(3)));
        assert_eq!(Action::Collapse.event(&state), Some(Event::CollapseOrLeave));
        assert_eq!(Action::Reset.event(&state), None, "no default");
        assert_eq!(Action::Edit.event(&state), Some(Event::EditHighlighted));
        let _ = state.update(Event::EditHighlighted);
        assert_eq!(Action::Next.event(&state), Some(Event::Decrement));
        assert_eq!(Action::Previous.event(&state), Some(Event::Increment));
        assert_eq!(Action::Cancel.event(&state), Some(Event::Cancel));

        let _ = state.update(Event::Press(Entry::Property(2)));
        assert_eq!(Action::Next.event(&state), Some(Event::Next));
        assert_eq!(Action::Reset.event(&state), Some(Event::ResetHighlighted));

        let _ = state.update(Event::Press(category("Layout")));
        assert_eq!(Action::Expand.event(&state), Some(Event::ExpandOrEnter));
        assert_eq!(Action::Edit.event(&state), Some(Event::EditHighlighted));
        let _ = state.update(Event::EditHighlighted);
        assert!(state.is_expanded("Layout"), "Enter toggles a heading");

        let mut alphabetical = sample().with_sort(Sort::Alphabetical);
        let _ = alphabetical.update(Event::Press(Entry::Property(3)));
        assert_eq!(Action::Collapse.event(&alphabetical), None);
    }

    #[test]
    fn rows_resolve_their_colours_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let idle = row_style(&tokens, Status::default());
            assert_eq!(idle.background, None);
            assert_eq!(idle.text, tokens.foreground);
            assert_eq!(idle.ring, None);

            let highlighted = Status {
                highlighted: true,
                ..Status::default()
            };
            assert_eq!(
                row_style(&tokens, highlighted).background,
                None,
                "nothing is selected, so the highlight has no fill"
            );
            assert_eq!(
                row_style(&tokens, highlighted).ring,
                None,
                "no ring unfocused"
            );
            let focused = Status {
                focused: true,
                ..highlighted
            };
            assert_eq!(row_style(&tokens, focused).ring, Some(tokens.ring));
            let focused_heading = Status {
                heading: true,
                ..focused
            };
            assert_eq!(
                row_style(&tokens, focused_heading).background,
                Some(fade(tokens.muted, 0.6)),
                "a highlighted heading keeps its band and adds the ring"
            );
            assert_eq!(row_style(&tokens, focused_heading).ring, Some(tokens.ring));

            let hovered = Status {
                hovered: true,
                ..Status::default()
            };
            assert_eq!(
                row_style(&tokens, hovered).background,
                Some(fade(tokens.accent, 0.6))
            );
            let disabled = Status {
                disabled: true,
                ..hovered
            };
            assert_eq!(row_style(&tokens, disabled).background, None);
            assert_eq!(row_style(&tokens, disabled).text, tokens.muted_foreground);

            let heading = Status {
                heading: true,
                ..Status::default()
            };
            assert_eq!(
                row_style(&tokens, heading).background,
                Some(fade(tokens.muted, 0.6))
            );
            assert_eq!(
                row_style(
                    &tokens,
                    Status {
                        hovered: true,
                        ..heading
                    }
                )
                .background,
                Some(tokens.muted)
            );

            assert_eq!(value_colour(&tokens, true), tokens.foreground);
            assert_eq!(value_colour(&tokens, false), tokens.muted_foreground);
            assert_eq!(mark_colour(&tokens), tokens.primary);
            assert_eq!(divider_colour(&tokens, true), tokens.ring);
            assert_ne!(divider_colour(&tokens, false), tokens.ring);
            assert_eq!(frame_style(&tokens).border.color, tokens.border);
            let swatch = swatch_style(&tokens, Color::WHITE);
            assert_eq!(swatch.background, Some(Background::Color(Color::WHITE)));
            assert_eq!(swatch.border.color, tokens.border);
        }
    }

    #[test]
    fn the_builder_starts_with_the_toolbar_and_description() {
        let state = sample();
        let grid: PropertyGrid<'_, u32, ()> = property_grid(&state);
        assert!(grid.toolbar && grid.description);
        assert!(!grid.is_enabled());
        assert_eq!(grid.height, Length::Shrink);
        let grid = grid
            .toolbar(false)
            .description(false)
            .placeholder("Search")
            .height(200)
            .keymap(Keymap::new())
            .custom(1, |_| Space::new().into())
            .custom(1, |_| Space::new().into())
            .on_event(|_| ());
        assert!(grid.is_enabled());
        assert_eq!(
            grid.customs.len(),
            1,
            "a second renderer replaces the first"
        );
        assert!(format!("{grid:?}").contains("Search"));
    }
}
