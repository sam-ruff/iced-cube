//! Records in rows and columns, with sorting, filtering, selection and
//! pages.
//!
//! [`State`] owns the rows, the [`Column`]s that read them, and everything
//! the reader changes: the sort, the search and column filters, the page,
//! the selection and which columns are hidden. Its
//! [`update`](State::update) is pure, so it is tested without a renderer.
//! The view only builds the rows of the current page that are in view, and
//! the header stays put while they scroll.
//!
//! Narrower than its [breakpoint](DataTable::breakpoint), the table turns
//! into a stack of cards, one per row, with each column as a labelled line,
//! so it never overflows a phone screen.
//!
//! The table resolves its keys through a [`Keymap`] of [`Action`]s while it
//! has focus, which it takes when pressed; see [`default_keymap`].

use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;
use std::ops::Range;
use std::rc::Rc;

use iced::alignment::Horizontal;
use iced::keyboard::key::Named;
use iced::widget::{self, Space, container, responsive, row, space as spacer, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};

use crate::data::ellipsis::ellipsis;
use crate::data::interaction::{Look, RowMenu, pressable, row_menu, scope};
use crate::data::rows::rows;
use crate::forms::select::select;
use crate::icon::{Glyph, themed};
use crate::keys::{self, Chord, Keymap};
use crate::overlay::context_menu;
use crate::overlay::dropdown_menu::{self, checkbox_item, dropdown_menu};
use crate::primitives::button::{self, button};
use crate::primitives::checkbox::{CheckState, checkbox};
use crate::primitives::icon_button::icon_button;
use crate::primitives::input::input;
use crate::theme::{Tokens, fade, radius, semibold, space, text_size};

/// Height of the header row.
pub const HEADER_HEIGHT: f32 = 40.0;
/// Height of every body row.
pub const ROW_HEIGHT: f32 = 44.0;
/// Below this width, in logical pixels, rows become cards.
pub const BREAKPOINT: f32 = 560.0;
/// The page sizes offered unless the state sets its own.
pub const PAGE_SIZES: [usize; 4] = [10, 20, 50, 100];
/// Placeholder rows shown while loading.
pub const SKELETON_ROWS: usize = 5;
/// Height of one labelled line in a card.
pub const CARD_LINE: f32 = 28.0;
const CARD_PADDING: f32 = 12.0;
const CARD_GAP: f32 = 8.0;
const CHECK_WIDTH: f32 = 40.0;
const ACTIONS_WIDTH: f32 = 44.0;

/// Names a column. Sorting, filters and hiding refer to columns by it.
pub type ColumnId = &'static str;

/// A cell's value, which sorting, searching and filtering read.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    #[default]
    Empty,
    Text(String),
    Number(f64),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Empty => Ok(()),
            Value::Text(value) => f.write_str(value),
            Value::Number(value) if value.fract() == 0.0 && value.abs() < 1e15 => {
                write!(f, "{}", *value as i64)
            }
            Value::Number(value) => write!(f, "{value}"),
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

macro_rules! number {
    ($($ty:ty),*) => {$(
        impl From<$ty> for Value {
            fn from(value: $ty) -> Self {
                Value::Number(value as f64)
            }
        }
    )*};
}

number!(f32, i32, i64, u32, u64, usize);

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        value.map_or(Value::Empty, Into::into)
    }
}

/// Orders two values: numbers by size, text alphabetically ignoring case,
/// and numbers before text.
pub fn compare(a: &Value, b: &Value) -> Ordering {
    match (a, b) {
        (Value::Empty, Value::Empty) => Ordering::Equal,
        (Value::Empty, _) => Ordering::Greater,
        (_, Value::Empty) => Ordering::Less,
        (Value::Number(a), Value::Number(b)) => a.total_cmp(b),
        (Value::Number(_), Value::Text(_)) => Ordering::Less,
        (Value::Text(_), Value::Number(_)) => Ordering::Greater,
        (Value::Text(a), Value::Text(b)) => a
            .to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b)),
    }
}

/// Which way a column is sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Ascending,
    Descending,
}

impl Direction {
    pub const ALL: [Direction; 2] = [Direction::Ascending, Direction::Descending];
}

/// How a column's header and cells line up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Align {
    #[default]
    Start,
    Center,
    /// For numbers, such as amounts.
    End,
}

impl Align {
    pub const ALL: [Align; 3] = [Align::Start, Align::Center, Align::End];

    fn horizontal(self) -> Horizontal {
        match self {
            Align::Start => Horizontal::Left,
            Align::Center => Horizontal::Center,
            Align::End => Horizontal::Right,
        }
    }
}

/// One column: its header, how it reads a row, and how it is shown.
pub struct Column<Row> {
    pub id: ColumnId,
    pub header: String,
    pub width: Length,
    pub align: Align,
    pub sortable: bool,
    pub hideable: bool,
    /// Offers a filter of the column's distinct values in the toolbar.
    pub filterable: bool,
    /// Whether the search box looks in this column.
    pub searchable: bool,
    value: fn(&Row) -> Value,
    format: Option<fn(&Row) -> String>,
}

/// A sortable, searchable and hideable column that fills an equal share
/// of the width. `value` reads the cell from a row.
pub fn column<Row>(
    id: ColumnId,
    header: impl Into<String>,
    value: fn(&Row) -> Value,
) -> Column<Row> {
    Column {
        id,
        header: header.into(),
        width: Length::FillPortion(1),
        align: Align::Start,
        sortable: true,
        hideable: true,
        filterable: false,
        searchable: true,
        value,
        format: None,
    }
}

impl<Row> Clone for Column<Row> {
    fn clone(&self) -> Self {
        Self {
            header: self.header.clone(),
            ..*self
        }
    }
}

impl<Row> fmt::Debug for Column<Row> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Column")
            .field("id", &self.id)
            .field("header", &self.header)
            .field("width", &self.width)
            .field("align", &self.align)
            .field("sortable", &self.sortable)
            .field("hideable", &self.hideable)
            .field("filterable", &self.filterable)
            .field("searchable", &self.searchable)
            .finish_non_exhaustive()
    }
}

impl<Row> Column<Row> {
    /// A fixed width in pixels, or a share of the room with
    /// `Length::FillPortion`.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    pub fn hideable(mut self, hideable: bool) -> Self {
        self.hideable = hideable;
        self
    }

    pub fn filterable(mut self, filterable: bool) -> Self {
        self.filterable = filterable;
        self
    }

    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    /// The text shown, searched and filtered on, in place of the value's
    /// own, such as an amount with its currency. Sorting still uses the
    /// value.
    pub fn format(mut self, format: fn(&Row) -> String) -> Self {
        self.format = Some(format);
        self
    }

    /// The cell's value in `row`.
    pub fn value(&self, row: &Row) -> Value {
        (self.value)(row)
    }

    /// The cell's text in `row`.
    pub fn text(&self, row: &Row) -> String {
        match self.format {
            Some(format) => format(row),
            None => (self.value)(row).to_string(),
        }
    }
}

/// Everything that changes a table.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<Key> {
    /// A header was pressed: unsorted, then ascending, then descending.
    Sort(ColumnId),
    /// Sorts by a column in a direction, or restores the row order.
    SortBy(Option<(ColumnId, Direction)>),
    Search(String),
    /// Keeps only rows whose cell in the column shows this text, or clears
    /// the column's filter.
    Filter(ColumnId, Option<String>),
    /// Clears the search and every filter.
    ClearFilters,
    PageSize(usize),
    /// Goes to a page, counting from zero.
    Page(usize),
    NextPage,
    PreviousPage,
    /// A row's checkbox changed.
    Select(Key, bool),
    /// The header checkbox changed: selects or clears the rows on the page.
    SelectPage(bool),
    ClearSelection,
    /// The pointer pressed a row.
    Press(Key),
    /// A row was double clicked.
    Activate(Key),
    /// Highlights the next row, moving to the next page after the last.
    Next,
    /// Highlights the previous row, moving to the previous page before the
    /// first.
    Previous,
    First,
    Last,
    ToggleHighlighted,
    ActivateHighlighted,
    /// Navigation and choices in the Columns menu.
    Columns(dropdown_menu::Event<ColumnId>),
    /// The table gained or lost focus. The widget sends this itself.
    Focus(bool),
}

/// What the app may need to act on after an [`Event`].
#[derive(Debug, Clone, PartialEq)]
pub enum Output<Key> {
    /// A row was activated with Enter or a double click.
    Activated(Key),
    /// The selection changed. Holds every selected key, in row order.
    Selected(Vec<Key>),
}

/// The rows, the columns and how the reader is looking at them.
#[derive(Debug, Clone)]
pub struct State<Row, Key> {
    rows: Vec<Row>,
    columns: Vec<Column<Row>>,
    key: fn(&Row) -> Key,
    hidden: HashSet<ColumnId>,
    columns_menu: dropdown_menu::State<ColumnId>,
    sort: Option<(ColumnId, Direction)>,
    search: String,
    filters: Vec<(ColumnId, String)>,
    options: Vec<(ColumnId, Vec<String>)>,
    /// Indices into `rows` that pass the filters, in sorted order.
    order: Vec<usize>,
    page: usize,
    page_size: usize,
    page_sizes: Vec<usize>,
    selectable: bool,
    selected: HashSet<Key>,
    highlighted: Option<Key>,
    focused: bool,
}

impl<Row, Key: Clone + Eq + Hash> State<Row, Key> {
    /// A table over `rows`, showing `columns` in order. `key` gives each
    /// row an identity that survives sorting and new rows, such as an id.
    pub fn new(
        columns: impl IntoIterator<Item = Column<Row>>,
        rows: impl IntoIterator<Item = Row>,
        key: fn(&Row) -> Key,
    ) -> Self {
        let mut state = Self {
            rows: rows.into_iter().collect(),
            columns: columns.into_iter().collect(),
            key,
            hidden: HashSet::new(),
            columns_menu: dropdown_menu::State::new([]),
            sort: None,
            search: String::new(),
            filters: Vec::new(),
            options: Vec::new(),
            order: Vec::new(),
            page: 0,
            page_size: PAGE_SIZES[0],
            page_sizes: PAGE_SIZES.to_vec(),
            selectable: false,
            selected: HashSet::new(),
            highlighted: None,
            focused: false,
        };
        state.rebuild_menu();
        state.refresh();
        state
    }

    /// Adds a checkbox to every row and a select-all box to the header.
    pub fn with_selection(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// The sizes offered in the footer. The first is used to start with.
    pub fn with_page_sizes(mut self, sizes: impl IntoIterator<Item = usize>) -> Self {
        let sizes: Vec<usize> = sizes.into_iter().filter(|&size| size > 0).collect();
        if let Some(&first) = sizes.first() {
            self.page_size = first;
            self.page_sizes = sizes;
        }
        self.refresh();
        self
    }

    pub fn with_sort(mut self, column: ColumnId, direction: Direction) -> Self {
        let _ = self.update(Event::SortBy(Some((column, direction))));
        self
    }

    /// Starts with these columns hidden.
    pub fn with_hidden(mut self, columns: impl IntoIterator<Item = ColumnId>) -> Self {
        self.hidden = columns.into_iter().collect();
        self.rebuild_menu();
        self.refresh();
        self
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn columns(&self) -> &[Column<Row>] {
        &self.columns
    }

    /// The columns on screen, in order.
    pub fn visible_columns(&self) -> Vec<&Column<Row>> {
        self.columns
            .iter()
            .filter(|column| !self.hidden.contains(column.id))
            .collect()
    }

    pub fn column(&self, id: ColumnId) -> Option<&Column<Row>> {
        self.columns.iter().find(|column| column.id == id)
    }

    pub fn is_hidden(&self, id: ColumnId) -> bool {
        self.hidden.contains(id)
    }

    /// The Columns menu, with a check for each hideable column.
    pub fn columns_menu(&self) -> &dropdown_menu::State<ColumnId> {
        &self.columns_menu
    }

    pub fn key(&self, row: &Row) -> Key {
        (self.key)(row)
    }

    pub fn row(&self, key: &Key) -> Option<&Row> {
        self.rows.iter().find(|row| (self.key)(row) == *key)
    }

    /// Replaces every row. The sort, filters and selection of rows that
    /// remain are kept.
    pub fn set_rows(&mut self, rows: impl IntoIterator<Item = Row>) {
        self.rows = rows.into_iter().collect();
        self.refresh();
    }

    pub fn push(&mut self, row: Row) {
        self.rows.push(row);
        self.refresh();
    }

    /// Changes the row with this key, if there is one.
    pub fn update_row(&mut self, key: &Key, change: impl FnOnce(&mut Row)) {
        let Some(row) = self.rows.iter_mut().find(|row| (self.key)(row) == *key) else {
            return;
        };
        change(row);
        self.refresh();
    }

    /// Keeps only the rows for which `keep` returns true.
    pub fn retain(&mut self, keep: impl FnMut(&Row) -> bool) {
        self.rows.retain(keep);
        self.refresh();
    }

    pub fn sort(&self) -> Option<(ColumnId, Direction)> {
        self.sort
    }

    pub fn search(&self) -> &str {
        &self.search
    }

    /// The text a column is filtered to, if any.
    pub fn filter(&self, column: ColumnId) -> Option<&str> {
        self.filters
            .iter()
            .find(|(id, _)| *id == column)
            .map(|(_, value)| value.as_str())
    }

    /// Whether the search or any filter hides rows.
    pub fn is_filtered(&self) -> bool {
        !self.search.trim().is_empty() || !self.filters.is_empty()
    }

    /// The distinct texts of a filterable column, to choose from.
    pub fn options(&self, column: ColumnId) -> &[String] {
        self.options
            .iter()
            .find(|(id, _)| *id == column)
            .map_or(&[], |(_, options)| options.as_slice())
    }

    /// How many rows pass the search and filters.
    pub fn filtered_len(&self) -> usize {
        self.order.len()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The current page, counting from zero.
    pub fn page(&self) -> usize {
        self.page
    }

    pub fn page_size(&self) -> usize {
        self.page_size
    }

    pub fn page_sizes(&self) -> &[usize] {
        &self.page_sizes
    }

    /// How many pages the filtered rows fill; at least one.
    pub fn page_count(&self) -> usize {
        self.order.len().div_ceil(self.page_size).max(1)
    }

    /// Positions of the current page's rows among the filtered rows.
    pub fn page_range(&self) -> Range<usize> {
        let start = (self.page * self.page_size).min(self.order.len());
        start..(start + self.page_size).min(self.order.len())
    }

    /// The rows on the current page, in order.
    pub fn page_rows(&self) -> Vec<&Row> {
        self.page_indices()
            .into_iter()
            .filter_map(|index| self.rows.get(index))
            .collect()
    }

    /// The keys of the rows on the current page, in order.
    pub fn page_keys(&self) -> Vec<Key> {
        self.page_rows().into_iter().map(self.key).collect()
    }

    /// "Showing 1-10 of 243", for the footer.
    pub fn showing(&self) -> String {
        let range = self.page_range();
        if range.is_empty() {
            return format!("Showing 0 of {}", self.order.len());
        }
        format!(
            "Showing {}-{} of {}",
            range.start + 1,
            range.end,
            self.order.len()
        )
    }

    pub fn is_selectable(&self) -> bool {
        self.selectable
    }

    pub fn is_selected(&self, key: &Key) -> bool {
        self.selected.contains(key)
    }

    /// Every selected key, on any page, in row order.
    pub fn selected(&self) -> Vec<Key> {
        self.rows
            .iter()
            .map(self.key)
            .filter(|key| self.selected.contains(key))
            .collect()
    }

    /// The header checkbox: checked when every row on the page is
    /// selected, indeterminate when some are.
    pub fn page_check_state(&self) -> CheckState {
        let keys = self.page_keys();
        let count = keys
            .iter()
            .filter(|key| self.selected.contains(key))
            .count();
        match count {
            0 => CheckState::Unchecked,
            n if n == keys.len() => CheckState::Checked,
            _ => CheckState::Indeterminate,
        }
    }

    /// The row the keyboard acts on.
    pub fn highlighted(&self) -> Option<&Key> {
        self.highlighted.as_ref()
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Applies an event and returns what the app may need to act on.
    pub fn update(&mut self, event: Event<Key>) -> Option<Output<Key>> {
        match event {
            Event::Sort(id) => {
                let next = match self.sort {
                    Some((sorted, Direction::Ascending)) if sorted == id => {
                        Some((id, Direction::Descending))
                    }
                    Some((sorted, Direction::Descending)) if sorted == id => None,
                    _ => Some((id, Direction::Ascending)),
                };
                self.sort_by(next);
            }
            Event::SortBy(sort) => self.sort_by(sort),
            Event::Search(search) => {
                self.search = search;
                self.page = 0;
                self.refresh();
            }
            Event::Filter(id, value) => {
                self.filters.retain(|(column, _)| *column != id);
                if let Some(value) = value.filter(|_| self.column(id).is_some()) {
                    self.filters.push((id, value));
                }
                self.page = 0;
                self.refresh();
            }
            Event::ClearFilters => {
                self.search.clear();
                self.filters.clear();
                self.page = 0;
                self.refresh();
            }
            Event::PageSize(size) => {
                let first = self.page_range().start;
                self.page_size = size.max(1);
                self.page = first / self.page_size;
                self.refresh();
            }
            Event::Page(page) => self.go_to(page),
            Event::NextPage => self.go_to(self.page + 1),
            Event::PreviousPage => self.go_to(self.page.saturating_sub(1)),
            Event::Select(key, selected) => return self.select([key], selected),
            Event::SelectPage(selected) => return self.select(self.page_keys(), selected),
            Event::ClearSelection => {
                let changed = !self.selected.is_empty();
                self.selected.clear();
                return changed.then(|| Output::Selected(Vec::new()));
            }
            Event::Press(key) => self.highlight(key),
            Event::Activate(key) => {
                if !self.page_keys().contains(&key) {
                    return None;
                }
                self.highlighted = Some(key.clone());
                return Some(Output::Activated(key));
            }
            Event::Next => self.step(true),
            Event::Previous => self.step(false),
            Event::First => self.highlighted = self.page_keys().first().cloned(),
            Event::Last => self.highlighted = self.page_keys().last().cloned(),
            Event::ToggleHighlighted => {
                let key = self.highlighted.clone()?;
                let selected = !self.selected.contains(&key);
                return self.select([key], selected);
            }
            Event::ActivateHighlighted => return self.highlighted.clone().map(Output::Activated),
            Event::Columns(event) => {
                if let Some(dropdown_menu::Output::Toggled(id, shown)) =
                    self.columns_menu.update(event)
                {
                    self.show_column(id, shown);
                }
            }
            Event::Focus(focused) => {
                self.focused = focused;
                // Focus lands on the first selected row of the page, or its
                // first row, so the keyboard has somewhere to start.
                if focused && self.highlighted.is_none() {
                    let keys = self.page_keys();
                    self.highlighted = keys
                        .iter()
                        .find(|key| self.selected.contains(*key))
                        .or(keys.first())
                        .cloned();
                }
            }
        }
        None
    }

    /// Turns a key press into an event through `keymap`.
    pub fn key_event(&self, keymap: &Keymap<Action>, key: &keys::Event) -> Option<Event<Key>> {
        keymap
            .resolve_event(key)
            .and_then(|action| action.event(self))
    }

    fn page_indices(&self) -> Vec<usize> {
        self.order
            .get(self.page_range())
            .map(<[usize]>::to_vec)
            .unwrap_or_default()
    }

    fn sort_by(&mut self, sort: Option<(ColumnId, Direction)>) {
        let valid = sort.filter(|(id, _)| self.column(id).is_some_and(|column| column.sortable));
        if sort.is_some() && valid.is_none() {
            return;
        }
        self.sort = valid;
        self.page = 0;
        self.refresh();
    }

    fn go_to(&mut self, page: usize) {
        self.page = page.min(self.page_count() - 1);
        let keys = self.page_keys();
        if self
            .highlighted
            .as_ref()
            .is_some_and(|key| !keys.contains(key))
        {
            self.highlighted = keys.first().cloned();
        }
    }

    fn highlight(&mut self, key: Key) {
        if self.page_keys().contains(&key) {
            self.highlighted = Some(key);
        }
    }

    fn step(&mut self, forward: bool) {
        let keys = self.page_keys();
        let current = self
            .highlighted
            .as_ref()
            .and_then(|key| keys.iter().position(|page| page == key));
        let target = match (current, forward) {
            (None, true) => keys.first().cloned(),
            (None, false) => keys.last().cloned(),
            (Some(index), true) if index + 1 < keys.len() => keys.get(index + 1).cloned(),
            (Some(index), false) if index > 0 => keys.get(index - 1).cloned(),
            (Some(_), true) if self.page + 1 < self.page_count() => {
                self.page += 1;
                self.page_keys().first().cloned()
            }
            (Some(_), false) if self.page > 0 => {
                self.page -= 1;
                self.page_keys().last().cloned()
            }
            (Some(_), _) => self.highlighted.clone(),
        };
        if target.is_some() {
            self.highlighted = target;
        }
    }

    fn select(
        &mut self,
        keys: impl IntoIterator<Item = Key>,
        selected: bool,
    ) -> Option<Output<Key>> {
        if !self.selectable {
            return None;
        }
        let mut changed = false;
        for key in keys {
            if self.row(&key).is_none() {
                continue;
            }
            changed |= if selected {
                self.selected.insert(key)
            } else {
                self.selected.remove(&key)
            };
        }
        changed.then(|| Output::Selected(self.selected()))
    }

    /// Shows or hides a column. The last visible column cannot be hidden.
    fn show_column(&mut self, id: ColumnId, shown: bool) {
        if shown {
            let _ = self.hidden.remove(id);
        } else if self.visible_columns().len() > 1 {
            let _ = self.hidden.insert(id);
        } else {
            self.columns_menu.set_checked(id, true);
        }
        self.refresh();
    }

    fn rebuild_menu(&mut self) {
        let items = self
            .columns
            .iter()
            .filter(|column| column.hideable)
            .map(|column| {
                checkbox_item(
                    column.id,
                    column.header.clone(),
                    !self.hidden.contains(column.id),
                )
            });
        self.columns_menu = dropdown_menu::State::new(items);
    }

    /// Recomputes the filtered, sorted order and everything that depends on
    /// the rows.
    fn refresh(&mut self) {
        self.options = self
            .columns
            .iter()
            .filter(|column| column.filterable)
            .map(|column| {
                let mut values: Vec<(Value, String)> = self
                    .rows
                    .iter()
                    .map(|row| (column.value(row), column.text(row)))
                    .collect();
                values.sort_by(|a, b| compare(&a.0, &b.0));
                let mut texts: Vec<String> = Vec::new();
                for (_, text) in values {
                    if !texts.contains(&text) {
                        texts.push(text);
                    }
                }
                (column.id, texts)
            })
            .collect();

        let query = self.search.trim().to_lowercase();
        let searched: Vec<&Column<Row>> = self
            .columns
            .iter()
            .filter(|column| column.searchable && !self.hidden.contains(column.id))
            .collect();
        let filters: Vec<(&Column<Row>, &str)> = self
            .filters
            .iter()
            .filter_map(|(id, value)| {
                self.columns
                    .iter()
                    .find(|column| column.id == *id)
                    .map(|column| (column, value.as_str()))
            })
            .collect();
        let mut order: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                filters
                    .iter()
                    .all(|(column, value)| column.text(row) == *value)
            })
            .filter(|(_, row)| {
                query.is_empty()
                    || searched
                        .iter()
                        .any(|column| column.text(row).to_lowercase().contains(&query))
            })
            .map(|(index, _)| index)
            .collect();

        if let Some((id, direction)) = self.sort
            && let Some(column) = self.column(id)
        {
            let keys: Vec<Value> = self.rows.iter().map(|row| column.value(row)).collect();
            order.sort_by(|&a, &b| {
                let (a, b) = (&keys[a], &keys[b]);
                match (a, b) {
                    (Value::Empty, Value::Empty) => Ordering::Equal,
                    (Value::Empty, _) => Ordering::Greater,
                    (_, Value::Empty) => Ordering::Less,
                    _ if direction == Direction::Descending => compare(b, a),
                    _ => compare(a, b),
                }
            });
        }
        self.order = order;

        let keys: HashSet<Key> = self.rows.iter().map(self.key).collect();
        self.selected.retain(|key| keys.contains(key));
        self.page = self.page.min(self.page_count() - 1);
        let page = self.page_keys();
        if self
            .highlighted
            .as_ref()
            .is_some_and(|key| !page.contains(key))
        {
            self.highlighted = None;
        }
    }
}

/// What a data table keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
    First,
    Last,
    NextPage,
    PreviousPage,
    Select,
    Activate,
    SelectAll,
    ClearSelection,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it does
    /// nothing, so the key is left for others.
    pub fn event<Row, Key: Clone + Eq + Hash>(self, state: &State<Row, Key>) -> Option<Event<Key>> {
        let rows = !state.page_range().is_empty();
        let highlighted = state.highlighted().is_some();
        let selectable = state.is_selectable();
        match self {
            Action::Next => rows.then_some(Event::Next),
            Action::Previous => rows.then_some(Event::Previous),
            Action::First => rows.then_some(Event::First),
            Action::Last => rows.then_some(Event::Last),
            Action::NextPage => (state.page() + 1 < state.page_count()).then_some(Event::NextPage),
            Action::PreviousPage => (state.page() > 0).then_some(Event::PreviousPage),
            Action::Select => (selectable && highlighted).then_some(Event::ToggleHighlighted),
            Action::Activate => highlighted.then_some(Event::ActivateHighlighted),
            Action::SelectAll => (selectable && rows).then_some(Event::SelectPage(true)),
            Action::ClearSelection => {
                (selectable && !state.selected.is_empty()).then_some(Event::ClearSelection)
            }
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Next,
        Action::Previous,
        Action::First,
        Action::Last,
        Action::NextPage,
        Action::PreviousPage,
        Action::Select,
        Action::Activate,
        Action::SelectAll,
        Action::ClearSelection,
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
            Action::NextPage => "NextPage",
            Action::PreviousPage => "PreviousPage",
            Action::Select => "Select",
            Action::Activate => "Activate",
            Action::SelectAll => "SelectAll",
            Action::ClearSelection => "ClearSelection",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Highlights the next row, going on to the next page after the last.",
            Action::Previous => "Highlights the previous row, going back a page before the first.",
            Action::First => "Highlights the first row on the page.",
            Action::Last => "Highlights the last row on the page.",
            Action::NextPage => "Shows the next page.",
            Action::PreviousPage => "Shows the previous page.",
            Action::Select => "Selects or clears the highlighted row.",
            Action::Activate => "Activates the highlighted row.",
            Action::SelectAll => "Selects every row on the page.",
            Action::ClearSelection => "Clears the selection.",
        }
    }
}

/// The default data table shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowDown` | [`Action::Next`] |
/// | `ArrowUp` | [`Action::Previous`] |
/// | `Home` | [`Action::First`] |
/// | `End` | [`Action::Last`] |
/// | `PageDown` | [`Action::NextPage`] |
/// | `PageUp` | [`Action::PreviousPage`] |
/// | `Space` | [`Action::Select`] |
/// | `Enter` | [`Action::Activate`] |
/// | `Mod+A` (Ctrl, or Cmd on macOS) | [`Action::SelectAll`] |
/// | `Escape` | [`Action::ClearSelection`] |
///
/// The table resolves these itself, only while it has focus, so they never
/// clash with other components. Pass a changed keymap with
/// [`DataTable::keymap`].
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowDown), Action::Next)
        .bind(Chord::named(Named::ArrowUp), Action::Previous)
        .bind(Chord::named(Named::Home), Action::First)
        .bind(Chord::named(Named::End), Action::Last)
        .bind(Chord::named(Named::PageDown), Action::NextPage)
        .bind(Chord::named(Named::PageUp), Action::PreviousPage)
        .bind(Chord::named(Named::Space), Action::Select)
        .bind(Chord::named(Named::Enter), Action::Activate)
        .bind(Chord::character('a').command(), Action::SelectAll)
        .bind(Chord::named(Named::Escape), Action::ClearSelection)
}

/// How a row is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Status {
    pub selected: bool,
    pub hovered: bool,
    /// Carries the keyboard highlight while the table has focus.
    pub highlighted: bool,
}

/// The colours of a row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowStyle {
    pub background: Option<Color>,
    /// The ring around the row carrying the keyboard highlight.
    pub ring: Option<Color>,
    /// The line between rows.
    pub divider: Color,
}

/// Resolves a row's colours: muted when selected, a lighter tint under the
/// pointer.
pub fn row_style(tokens: &Tokens, status: Status) -> RowStyle {
    let background = if status.selected {
        Some(tokens.muted)
    } else if status.hovered {
        Some(fade(tokens.muted, 0.5))
    } else {
        None
    };
    RowStyle {
        background,
        ring: status.highlighted.then_some(tokens.ring),
        divider: tokens.border,
    }
}

/// The frame around the table: a subtle border with rounded corners.
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

/// A placeholder bar in a loading row.
pub fn skeleton_style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens.muted)),
        border: Border {
            radius: radius::SM.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// The sort indicator for a column: arrows up and down while unsorted, or
/// the direction.
pub fn sort_glyph(direction: Option<Direction>) -> Glyph {
    match direction {
        None => crate::lucide!(ArrowUpDown),
        Some(Direction::Ascending) => crate::lucide!(ArrowUp),
        Some(Direction::Descending) => crate::lucide!(ArrowDown),
    }
}

type OnEvent<'a, Key, Message> = dyn Fn(Event<Key>) -> Message + 'a;
type Cell<'a, Row, Message> = Box<dyn Fn(&'a Row) -> Element<'a, Message> + 'a>;
type BuildMenu<'a, Key, Message> =
    dyn FnOnce(Keymap<context_menu::Action>) -> RowMenu<'a, Key, Message> + 'a;

/// A data table builder. Convert it into an [`Element`] to render.
pub struct DataTable<'a, Row, Key, Message> {
    state: &'a State<Row, Key>,
    cells: Vec<(ColumnId, Cell<'a, Row, Message>)>,
    toolbar: bool,
    placeholder: String,
    loading: bool,
    empty: String,
    height: Length,
    row_height: f32,
    breakpoint: f32,
    keymap: Keymap<Action>,
    menu: Option<Box<BuildMenu<'a, Key, Message>>>,
    menu_keymap: Keymap<context_menu::Action>,
    row_actions: bool,
    on_event: Option<Box<OnEvent<'a, Key, Message>>>,
}

/// Renders `state`. Without [`on_event`](DataTable::on_event) every control
/// renders disabled.
pub fn data_table<Row, Key, Message>(state: &State<Row, Key>) -> DataTable<'_, Row, Key, Message> {
    DataTable {
        state,
        cells: Vec::new(),
        toolbar: true,
        placeholder: "Search...".to_owned(),
        loading: false,
        empty: "No results.".to_owned(),
        height: Length::Shrink,
        row_height: ROW_HEIGHT,
        breakpoint: BREAKPOINT,
        keymap: default_keymap(),
        menu: None,
        menu_keymap: context_menu::default_keymap(),
        row_actions: false,
        on_event: None,
    }
}

impl<Row: fmt::Debug, Key: fmt::Debug, Message> fmt::Debug for DataTable<'_, Row, Key, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataTable")
            .field("state", self.state)
            .field("toolbar", &self.toolbar)
            .field("loading", &self.loading)
            .field("height", &self.height)
            .field("breakpoint", &self.breakpoint)
            .field("keymap", &self.keymap)
            .finish_non_exhaustive()
    }
}

impl<'a, Row, Key, Message> DataTable<'a, Row, Key, Message> {
    /// Draws the cells of a column with `render`, such as a badge or a
    /// progress bar, in place of its text. Keep it within a row's height.
    pub fn cell(
        mut self,
        column: ColumnId,
        render: impl Fn(&'a Row) -> Element<'a, Message> + 'a,
    ) -> Self {
        self.cells.retain(|(id, _)| *id != column);
        self.cells.push((column, Box::new(render)));
        self
    }

    /// Shows the search box, column filters and Columns menu above the
    /// table. On by default.
    pub fn toolbar(mut self, toolbar: bool) -> Self {
        self.toolbar = toolbar;
        self
    }

    /// The search box placeholder.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Shows placeholder rows in place of the data.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// The message shown when no rows pass the search and filters.
    pub fn empty(mut self, message: impl Into<String>) -> Self {
        self.empty = message.into();
        self
    }

    /// A fixed or filling height scrolls the rows under the header.
    /// Defaults to shrinking to fit the page.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// The height of each row, such as 32 for a compact table. Defaults to
    /// [`ROW_HEIGHT`]. Custom cells must fit inside it.
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = height;
        self
    }

    /// The width below which rows become cards. Defaults to [`BREAKPOINT`].
    pub fn breakpoint(mut self, width: f32) -> Self {
        self.breakpoint = width;
        self
    }

    /// Replaces the [`default_keymap`] the focused table resolves keys with.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Key>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    /// Gives every row the context menu of `menu`, keyed by row key. It
    /// opens on a right-click or a long press, and on the highlighted row
    /// with its opening chord while the table has focus.
    pub fn context_menu<MenuId>(
        mut self,
        menu: &'a context_menu::State<MenuId, Key>,
        on_event: impl Fn(context_menu::Event<MenuId, Key>) -> Message + 'a,
    ) -> Self
    where
        MenuId: Copy + PartialEq + 'a,
        Key: Clone + PartialEq + 'a,
        Message: Clone + 'a,
    {
        let on_event: Rc<dyn Fn(context_menu::Event<MenuId, Key>) -> Message + 'a> =
            Rc::new(on_event);
        self.menu = Some(Box::new(move |keymap| row_menu(menu, keymap, on_event)));
        self
    }

    /// Replaces the context menu's default keymap.
    pub fn context_menu_keymap(mut self, keymap: Keymap<context_menu::Action>) -> Self {
        self.menu_keymap = keymap;
        self
    }

    /// Adds a button at the end of each row that opens its context menu,
    /// for touch screens and anyone who does not right-click.
    pub fn row_actions(mut self, row_actions: bool) -> Self {
        self.row_actions = row_actions;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_event.is_some()
    }
}

impl<'a, Row, Key, Message> From<DataTable<'a, Row, Key, Message>> for Element<'a, Message>
where
    Row: 'a,
    Key: Clone + Eq + Hash + 'a,
    Message: Clone + 'a,
{
    fn from(table: DataTable<'a, Row, Key, Message>) -> Self {
        let DataTable {
            state,
            cells,
            toolbar,
            placeholder,
            loading,
            empty,
            height,
            row_height,
            breakpoint,
            keymap,
            menu,
            menu_keymap,
            row_actions,
            on_event,
        } = table;
        let on_event: Option<Rc<OnEvent<'a, Key, Message>>> = on_event.map(Rc::from);
        let menu = menu
            .filter(|_| on_event.is_some())
            .map(|build| build(menu_keymap));
        let view = Rc::new(View {
            state,
            cells,
            toolbar,
            placeholder,
            loading,
            empty,
            height,
            row_height: row_height.max(1.0),
            keymap,
            row_actions: row_actions && menu.is_some(),
            menu,
            on_event,
        });
        responsive(move |size| view.body(size.width < breakpoint))
            .height(Length::Shrink)
            .into()
    }
}

/// Everything the table's parts are built from, shared by the layouts.
struct View<'a, Row, Key, Message> {
    state: &'a State<Row, Key>,
    cells: Vec<(ColumnId, Cell<'a, Row, Message>)>,
    toolbar: bool,
    placeholder: String,
    loading: bool,
    empty: String,
    height: Length,
    row_height: f32,
    keymap: Keymap<Action>,
    menu: Option<RowMenu<'a, Key, Message>>,
    row_actions: bool,
    on_event: Option<Rc<OnEvent<'a, Key, Message>>>,
}

fn muted(theme: &Theme) -> Color {
    Tokens::of(theme).muted_foreground
}

fn small<'a>(label: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(label)
        .size(text_size::SM)
        .wrapping(text::Wrapping::None)
        .style(|theme| text::Style {
            color: Some(muted(theme)),
        })
}

impl<'a, Row, Key, Message> View<'a, Row, Key, Message>
where
    Row: 'a,
    Key: Clone + Eq + Hash + 'a,
    Message: Clone + 'a,
{
    fn send(&self, event: Event<Key>) -> Option<Message> {
        self.on_event.as_ref().map(|on_event| on_event(event))
    }

    fn body(self: &Rc<Self>, narrow: bool) -> Element<'a, Message> {
        let mut parts = widget::column![].spacing(space::MD);
        if self.toolbar {
            parts = parts.push(self.toolbar(narrow));
        }
        let grid = if narrow { self.cards() } else { self.table() };
        parts
            .push(self.scoped(grid))
            .push(self.footer(narrow))
            .into()
    }

    /// Lets the grid take focus and resolve its keys while it has it.
    fn scoped(self: &Rc<Self>, grid: Element<'a, Message>) -> Element<'a, Message> {
        let Some(on_event) = self.on_event.clone() else {
            return grid;
        };
        let view = Rc::clone(self);
        let on_focus = on_event.clone();
        scope(grid)
            .on_focus(move |focused| on_focus(Event::Focus(focused)))
            .on_key(move |key| {
                let state = view.state;
                let opened = view
                    .menu
                    .as_ref()
                    .zip(state.highlighted().cloned())
                    .and_then(|(menu, target)| menu.key(key, target));
                opened.or_else(|| {
                    state
                        .key_event(&view.keymap, key)
                        .map(|event| on_event(event))
                })
            })
            .into()
    }

    fn toolbar(&self, narrow: bool) -> Element<'a, Message> {
        let state = self.state;
        let search = {
            let on_event = self.on_event.clone();
            input(&self.placeholder, state.search())
                .icon(crate::lucide!(Search))
                .width(if narrow {
                    Length::Fill
                } else {
                    Length::Fixed(240.0)
                })
                .on_input_maybe(
                    on_event.map(|on_event| move |query| on_event(Event::Search(query))),
                )
        };

        let filters = state
            .columns
            .iter()
            .filter(|column| column.filterable && !state.is_hidden(column.id))
            .map(|column| self.filter(column, narrow));

        let columns_menu: Element<'a, Message> = {
            let trigger = button("Columns")
                .trailing_icon(crate::lucide!(ChevronDown))
                .variant(button::Variant::Outline)
                .on_press_maybe(self.send(Event::Columns(dropdown_menu::Event::Toggle)));
            let menu = dropdown_menu(state.columns_menu(), trigger)
                .align(crate::overlay::anchored::Align::End)
                .width(200.0);
            match self.on_event.clone() {
                Some(on_event) => menu
                    .on_event(move |event| on_event(Event::Columns(event)))
                    .into(),
                None => menu.into(),
            }
        };

        if narrow {
            return widget::column![
                search,
                row(filters)
                    .push(columns_menu)
                    .spacing(space::SM)
                    .align_y(Alignment::Center),
            ]
            .spacing(space::SM)
            .into();
        }
        row![search]
            .extend(filters)
            .push(spacer::horizontal())
            .push(columns_menu)
            .spacing(space::SM)
            .align_y(Alignment::Center)
            .into()
    }

    fn filter(&self, column: &Column<Row>, narrow: bool) -> Element<'a, Message> {
        let state = self.state;
        let choice = |value: Option<&str>| FilterChoice {
            column: column.id,
            header: column.header.clone(),
            value: value.map(str::to_owned),
        };
        let options: Vec<FilterChoice> = std::iter::once(choice(None))
            .chain(
                state
                    .options(column.id)
                    .iter()
                    .map(|value| choice(Some(value))),
            )
            .collect();
        let selected = choice(state.filter(column.id));
        let on_event = self.on_event.clone();
        select(options, Some(selected))
            .width(if narrow {
                Length::Fill
            } else {
                Length::Fixed(170.0)
            })
            .on_select_maybe(on_event.map(|on_event| {
                move |choice: FilterChoice| on_event(Event::Filter(choice.column, choice.value))
            }))
            .into()
    }

    fn table(self: &Rc<Self>) -> Element<'a, Message> {
        let state = self.state;
        let columns = state.visible_columns();

        let mut header = row![].height(Length::Fill).align_y(Alignment::Center);
        if state.is_selectable() {
            let page_empty = state.page_range().is_empty();
            let on_event = self
                .on_event
                .clone()
                .filter(|_| !page_empty && !self.loading);
            header = header.push(
                container(checkbox(state.page_check_state()).on_toggle_maybe(
                    on_event.map(|on_event| move |checked| on_event(Event::SelectPage(checked))),
                ))
                .center_x(CHECK_WIDTH),
            );
        }
        for column in &columns {
            header = header.push(self.header_cell(column));
        }
        if self.row_actions {
            header = header.push(Space::new().width(ACTIONS_WIDTH));
        }
        let divider = container(Space::new())
            .width(Length::Fill)
            .height(1)
            .style(|theme| container::Style {
                background: Some(Background::Color(Tokens::of(theme).border)),
                ..container::Style::default()
            });
        let header = widget::column![container(header).height(HEADER_HEIGHT), divider];

        container(self.table_body(header.into()))
            .width(Length::Fill)
            .clip(true)
            .style(|theme| frame_style(&Tokens::of(theme)))
            .into()
    }

    fn header_cell(&self, column: &Column<Row>) -> Element<'a, Message> {
        let state = self.state;
        let align = column.align.horizontal();
        let label = ellipsis(column.header.clone())
            .font(semibold())
            .width(Length::Shrink)
            .colour(muted);
        let cell = |content: Element<'a, Message>| -> Element<'a, Message> {
            container(content)
                .width(column.width)
                .height(Length::Fill)
                .padding([0.0, space::SM])
                .align_x(align)
                .center_y(Length::Fill)
                .into()
        };
        if !column.sortable {
            return cell(label.into());
        }
        let direction = state
            .sort()
            .filter(|(id, _)| *id == column.id)
            .map(|(_, direction)| direction);
        let indicator = themed(sort_glyph(direction), 14.0, 1.0, move |theme| {
            let tokens = Tokens::of(theme);
            if direction.is_some() {
                tokens.foreground
            } else {
                tokens.muted_foreground
            }
        });
        let content = row![label, indicator]
            .spacing(space::XS)
            .align_y(Alignment::Center);
        let press = pressable(
            container(content).padding([4.0, space::XS]),
            |theme, hovered| Look {
                background: hovered.then(|| Tokens::of(theme).accent),
                border: Border {
                    radius: radius::SM.into(),
                    ..Border::default()
                },
                divider: None,
            },
        );
        let press = match self.on_event.clone() {
            Some(on_event) => {
                let id = column.id;
                press.on_press(move |_| on_event(Event::Sort(id)))
            }
            None => press,
        };
        container(press)
            .width(column.width)
            .height(Length::Fill)
            .padding([0.0, space::XS])
            .align_x(align)
            .center_y(Length::Fill)
            .into()
    }

    /// The header above the rows of the page, placeholder rows while
    /// loading, or the empty message.
    fn table_body(self: &Rc<Self>, header: Element<'a, Message>) -> Element<'a, Message> {
        let state = self.state;
        if self.loading {
            let skeleton =
                (0..SKELETON_ROWS).map(|index| self.skeleton_row(index + 1 == SKELETON_ROWS));
            return widget::column(std::iter::once(header).chain(skeleton)).into();
        }
        let page = state.page_indices();
        if page.is_empty() {
            return widget::column![header, self.empty_view()].into();
        }
        let highlight = self.highlight_position();
        let count = page.len();
        let view = Rc::clone(self);
        rows(count, self.row_height, move |range: Range<usize>| {
            widget::column(
                page.get(range.clone())
                    .unwrap_or_default()
                    .iter()
                    .zip(range)
                    .map(|(&index, position)| view.table_row(index, position + 1 == count)),
            )
            .into()
        })
        .header(header)
        .highlight(highlight)
        .height(self.height)
        .into()
    }

    fn highlight_position(&self) -> Option<usize> {
        let key = self.state.highlighted()?;
        self.state.page_keys().iter().position(|page| page == key)
    }

    fn row_status(&self, key: &Key) -> Status {
        let state = self.state;
        Status {
            selected: state.is_selectable() && state.is_selected(key),
            hovered: false,
            highlighted: state.is_focused() && state.highlighted() == Some(key),
        }
    }

    fn checkbox_for(&self, key: &Key) -> Element<'a, Message> {
        let selected = self.state.is_selected(key);
        let on_event = self.on_event.clone();
        let key = key.clone();
        checkbox(selected)
            .on_toggle_maybe(
                on_event.map(|on_event| move |checked| on_event(Event::Select(key, checked))),
            )
            .into()
    }

    fn actions_for(&self, key: &Key) -> Element<'a, Message> {
        let open = self.menu.as_ref().map(|menu| menu.open(key.clone()));
        icon_button(crate::lucide!(Ellipsis))
            .label("Row actions")
            .tooltip(None)
            .size(button::Size::Sm)
            .on_press_maybe(open)
            .into()
    }

    fn content_for(
        &self,
        column: &Column<Row>,
        row: &'a Row,
        align: Horizontal,
    ) -> Element<'a, Message> {
        if let Some((_, render)) = self.cells.iter().find(|(id, _)| *id == column.id) {
            return render(row);
        }
        ellipsis(column.text(row)).align_x(align).into()
    }

    fn table_row(&self, index: usize, last: bool) -> Element<'a, Message> {
        let state = self.state;
        let Some(row_data) = state.rows.get(index) else {
            return Space::new().into();
        };
        let key = state.key(row_data);
        let mut cells = row![].height(Length::Fill).align_y(Alignment::Center);
        if state.is_selectable() {
            cells = cells.push(container(self.checkbox_for(&key)).center_x(CHECK_WIDTH));
        }
        for column in state.visible_columns() {
            cells = cells.push(
                container(self.content_for(column, row_data, column.align.horizontal()))
                    .width(column.width)
                    .padding([0.0, space::SM])
                    .align_x(column.align.horizontal())
                    .center_y(Length::Fill),
            );
        }
        if self.row_actions {
            cells = cells.push(container(self.actions_for(&key)).center_x(ACTIONS_WIDTH));
        }
        let status = self.row_status(&key);
        let body = pressable(
            container(cells).height(self.row_height),
            move |theme, hovered| {
                let style = row_style(&Tokens::of(theme), Status { hovered, ..status });
                Look {
                    background: style.background,
                    border: Border {
                        color: style.ring.unwrap_or(Color::TRANSPARENT),
                        width: if style.ring.is_some() { 1.0 } else { 0.0 },
                        radius: radius::SM.into(),
                    },
                    divider: (!last).then_some(style.divider),
                }
            },
        );
        self.finish_row(body, key)
    }

    /// Adds the press and double click messages and the context menu.
    fn finish_row(
        &self,
        body: crate::data::interaction::Pressable<'a, Message>,
        key: Key,
    ) -> Element<'a, Message> {
        let Some(on_event) = self.on_event.clone() else {
            return body.into();
        };
        let activate = on_event(Event::Activate(key.clone()));
        let press_key = key.clone();
        let body: Element<'a, Message> = body
            .on_press(move |_| on_event(Event::Press(press_key.clone())))
            .on_double_click(Some(activate))
            .into();
        match &self.menu {
            Some(menu) => menu.wrap(key, body),
            None => body,
        }
    }

    fn skeleton_row(&self, last: bool) -> Element<'a, Message> {
        let state = self.state;
        let bar = || {
            container(Space::new())
                .width(Length::Fill)
                .height(12)
                .style(|theme| skeleton_style(&Tokens::of(theme)))
        };
        let mut cells = row![].height(Length::Fill).align_y(Alignment::Center);
        if state.is_selectable() {
            cells = cells.push(Space::new().width(CHECK_WIDTH));
        }
        for column in state.visible_columns() {
            cells = cells.push(
                container(bar())
                    .width(column.width)
                    .padding([0.0, space::SM]),
            );
        }
        if self.row_actions {
            cells = cells.push(Space::new().width(ACTIONS_WIDTH));
        }
        pressable::<Message>(container(cells).height(self.row_height), move |theme, _| {
            Look {
                divider: (!last).then(|| Tokens::of(theme).border),
                ..Look::default()
            }
        })
        .into()
    }

    fn empty_view(&self) -> Element<'a, Message> {
        let state = self.state;
        let glyph = if state.is_filtered() {
            crate::lucide!(SearchX)
        } else {
            crate::lucide!(Inbox)
        };
        let mut content = widget::column![
            themed(glyph, 24.0, 1.0, muted),
            text(self.empty.clone())
                .size(text_size::SM)
                .style(|theme| text::Style {
                    color: Some(muted(theme)),
                }),
        ]
        .spacing(space::SM)
        .align_x(Alignment::Center);
        if state.is_filtered() {
            content = content.push(
                button("Clear filters")
                    .variant(button::Variant::Outline)
                    .size(button::Size::Sm)
                    .on_press_maybe(self.send(Event::ClearFilters)),
            );
        }
        container(content)
            .padding(space::XL)
            .center_x(Length::Fill)
            .into()
    }

    /// The card height for the visible columns: the first column heads the
    /// card, and each other column is a labelled line.
    fn card_height(&self) -> f32 {
        let lines = self.state.visible_columns().len().max(1);
        CARD_PADDING * 2.0 + lines as f32 * CARD_LINE
    }

    fn cards(self: &Rc<Self>) -> Element<'a, Message> {
        let state = self.state;
        let has_rows = !self.loading && !state.page_range().is_empty();
        let mut bar = row![].spacing(space::MD).align_y(Alignment::Center);
        if state.is_selectable() {
            let on_event = self.on_event.clone().filter(|_| has_rows);
            bar = bar.push(
                checkbox(state.page_check_state())
                    .label("Select all")
                    .on_toggle_maybe(
                        on_event
                            .map(|on_event| move |checked| on_event(Event::SelectPage(checked))),
                    ),
            );
        }
        if has_rows {
            bar = bar.push(self.sort_select());
        }

        let body: Element<'a, Message> = if self.loading {
            widget::column((0..SKELETON_ROWS).map(|_| self.skeleton_card()))
                .spacing(CARD_GAP)
                .into()
        } else if state.page_range().is_empty() {
            container(self.empty_view())
                .style(|theme| frame_style(&Tokens::of(theme)))
                .into()
        } else {
            let page = state.page_indices();
            let count = page.len();
            let view = Rc::clone(self);
            let height = self.card_height();
            rows(count, height + CARD_GAP, move |range: Range<usize>| {
                widget::column(
                    page.get(range)
                        .unwrap_or_default()
                        .iter()
                        .map(|&index| view.card(index, height)),
                )
                .into()
            })
            .highlight(self.highlight_position())
            .height(self.height)
            .into()
        };
        if !state.is_selectable() && !has_rows {
            return body;
        }
        widget::column![bar, body].spacing(space::SM).into()
    }

    fn sort_select(&self) -> Element<'a, Message> {
        let state = self.state;
        let mut options = vec![SortChoice {
            sort: None,
            label: "Default order".to_owned(),
        }];
        for column in state
            .visible_columns()
            .into_iter()
            .filter(|column| column.sortable)
        {
            for direction in Direction::ALL {
                let way = match direction {
                    Direction::Ascending => "ascending",
                    Direction::Descending => "descending",
                };
                options.push(SortChoice {
                    sort: Some((column.id, direction)),
                    label: format!("{}, {way}", column.header),
                });
            }
        }
        let selected = options
            .iter()
            .find(|choice| choice.sort == state.sort())
            .cloned();
        let on_event = self.on_event.clone();
        select(options, selected)
            .width(Length::Fill)
            .on_select_maybe(
                on_event
                    .map(|on_event| move |choice: SortChoice| on_event(Event::SortBy(choice.sort))),
            )
            .into()
    }

    fn card(&self, index: usize, height: f32) -> Element<'a, Message> {
        let state = self.state;
        let Some(row_data) = state.rows.get(index) else {
            return Space::new().into();
        };
        let key = state.key(row_data);
        let columns = state.visible_columns();
        let mut lines = widget::column![];

        let mut head = row![].spacing(space::SM).align_y(Alignment::Center);
        if state.is_selectable() {
            head = head.push(self.checkbox_for(&key));
        }
        if let Some(first) = columns.first() {
            let title: Element<'a, Message> =
                match self.cells.iter().find(|(id, _)| *id == first.id) {
                    Some((_, render)) => container(render(row_data)).width(Length::Fill).into(),
                    None => ellipsis(first.text(row_data)).font(semibold()).into(),
                };
            head = head.push(title);
        }
        if self.row_actions {
            head = head.push(self.actions_for(&key));
        }
        lines = lines.push(container(head).height(CARD_LINE).center_y(CARD_LINE));

        for column in columns.iter().skip(1) {
            let label = ellipsis(column.header.clone())
                .colour(muted)
                .width(Length::FillPortion(2));
            let value = container(self.content_for(column, row_data, Horizontal::Right))
                .width(Length::FillPortion(3))
                .align_x(Horizontal::Right);
            lines = lines.push(
                container(
                    row![label, value]
                        .spacing(space::SM)
                        .align_y(Alignment::Center),
                )
                .height(CARD_LINE)
                .center_y(CARD_LINE),
            );
        }

        let status = self.row_status(&key);
        let card = pressable(
            container(lines).padding(CARD_PADDING).height(height),
            move |theme, hovered| {
                let tokens = Tokens::of(theme);
                let style = row_style(&tokens, Status { hovered, ..status });
                Look {
                    background: style.background,
                    border: Border {
                        color: style.ring.unwrap_or(tokens.border),
                        width: 1.0,
                        radius: radius::MD.into(),
                    },
                    divider: None,
                }
            },
        );
        let card = self.finish_row(card, key);
        container(card)
            .padding(Padding::ZERO.bottom(CARD_GAP))
            .into()
    }

    fn skeleton_card(&self) -> Element<'a, Message> {
        let lines = widget::column((0..self.state.visible_columns().len().max(1)).map(|_| {
            container(
                container(Space::new())
                    .width(Length::Fill)
                    .height(12)
                    .style(|theme| skeleton_style(&Tokens::of(theme))),
            )
            .height(CARD_LINE)
            .center_y(CARD_LINE)
            .into()
        }));
        container(lines)
            .padding(CARD_PADDING)
            .width(Length::Fill)
            .style(|theme| frame_style(&Tokens::of(theme)))
            .into()
    }

    fn footer(&self, narrow: bool) -> Element<'a, Message> {
        let state = self.state;
        let mut summary = state.showing();
        if state.is_selectable() {
            summary.push_str(&format!(", {} selected", state.selected.len()));
        }
        let summary = small(summary);

        let on_event = self.on_event.clone().filter(|_| !self.loading);
        let sizes = select(state.page_sizes().to_vec(), Some(state.page_size()))
            .width(80)
            .on_select_maybe(on_event.map(|on_event| move |size| on_event(Event::PageSize(size))));
        let has_previous = state.page() > 0;
        let has_next = state.page() + 1 < state.page_count();
        let pager = row![
            small(format!(
                "Page {} of {}",
                state.page() + 1,
                state.page_count()
            )),
            icon_button(crate::lucide!(ChevronLeft))
                .label("Previous page")
                .variant(crate::primitives::icon_button::Variant::Outline)
                .size(button::Size::Sm)
                .on_press_maybe(
                    self.send(Event::PreviousPage)
                        .filter(|_| has_previous && !self.loading)
                ),
            icon_button(crate::lucide!(ChevronRight))
                .label("Next page")
                .variant(crate::primitives::icon_button::Variant::Outline)
                .size(button::Size::Sm)
                .on_press_maybe(
                    self.send(Event::NextPage)
                        .filter(|_| has_next && !self.loading)
                ),
        ]
        .spacing(space::SM)
        .align_y(Alignment::Center);

        if narrow {
            return widget::column![
                summary,
                row![sizes, spacer::horizontal(), pager].align_y(Alignment::Center),
            ]
            .spacing(space::SM)
            .into();
        }
        row![
            summary,
            spacer::horizontal(),
            small("Rows per page"),
            sizes,
            pager,
        ]
        .spacing(space::MD)
        .align_y(Alignment::Center)
        .into()
    }
}

/// An option in a column filter's select.
#[derive(Debug, Clone, PartialEq)]
struct FilterChoice {
    column: ColumnId,
    header: String,
    value: Option<String>,
}

impl fmt::Display for FilterChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            Some(value) => write!(f, "{}: {value}", self.header),
            None => write!(f, "{}: All", self.header),
        }
    }
}

/// An option in the sort select shown above cards.
#[derive(Debug, Clone, PartialEq)]
struct SortChoice {
    sort: Option<(ColumnId, Direction)>,
    label: String,
}

impl fmt::Display for SortChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};
    use iced::keyboard::{Key, Modifiers};

    #[derive(Debug, Clone, PartialEq)]
    struct Payment {
        id: u32,
        status: &'static str,
        email: String,
        amount: Option<f64>,
    }

    fn payment(id: u32, status: &'static str, email: &str, amount: f64) -> Payment {
        Payment {
            id,
            status,
            email: email.to_owned(),
            amount: Some(amount),
        }
    }

    fn columns() -> Vec<Column<Payment>> {
        vec![
            column("status", "Status", |p: &Payment| p.status.into()).filterable(true),
            column("email", "Email", |p: &Payment| p.email.clone().into()),
            column("amount", "Amount", |p: &Payment| p.amount.into())
                .align(Align::End)
                .format(|p| {
                    p.amount
                        .map_or_else(String::new, |amount| format!("${amount:.2}"))
                }),
            column("id", "ID", |p: &Payment| p.id.into())
                .sortable(false)
                .hideable(false),
        ]
    }

    fn payments(count: u32) -> Vec<Payment> {
        (1..=count)
            .map(|id| {
                let status = ["paid", "pending", "failed"][(id % 3) as usize];
                payment(
                    id,
                    status,
                    &format!("user{id}@example.com"),
                    f64::from(id * 10),
                )
            })
            .collect()
    }

    fn state(count: u32) -> State<Payment, u32> {
        State::new(columns(), payments(count), |p| p.id)
    }

    fn ids(rows: Vec<&Payment>) -> Vec<u32> {
        rows.into_iter().map(|p| p.id).collect()
    }

    fn selected(output: Option<Output<u32>>) -> Vec<u32> {
        match output {
            Some(Output::Selected(keys)) => keys,
            other => panic!("expected a selection, got {other:?}"),
        }
    }

    #[test]
    fn values_display_and_convert() {
        assert_eq!(Value::from(12.0).to_string(), "12");
        assert_eq!(Value::from(12.5).to_string(), "12.5");
        assert_eq!(Value::from(7_u32), Value::Number(7.0));
        assert_eq!(Value::from("a"), Value::Text("a".into()));
        assert_eq!(Value::from(None::<f64>), Value::Empty);
        assert_eq!(Value::from(Some(3_i64)), Value::Number(3.0));
        assert_eq!(Value::Empty.to_string(), "");
        assert_eq!(Value::default(), Value::Empty);
    }

    #[test]
    fn compare_orders_numbers_then_text_then_empty() {
        let mut values = vec![
            Value::from("banana"),
            Value::Empty,
            Value::from(10.0),
            Value::from("Apple"),
            Value::from(2.0),
            Value::from("apple"),
        ];
        values.sort_by(compare);
        assert_eq!(
            values,
            vec![
                Value::from(2.0),
                Value::from(10.0),
                Value::from("Apple"),
                Value::from("apple"),
                Value::from("banana"),
                Value::Empty,
            ]
        );
    }

    #[test]
    fn starts_unsorted_on_the_first_page() {
        let state = state(23);
        assert_eq!(state.len(), 23);
        assert_eq!(state.filtered_len(), 23);
        assert_eq!(state.page(), 0);
        assert_eq!(state.page_size(), 10);
        assert_eq!(state.page_count(), 3);
        assert_eq!(state.page_range(), 0..10);
        assert_eq!(ids(state.page_rows()), (1..=10).collect::<Vec<_>>());
        assert_eq!(state.showing(), "Showing 1-10 of 23");
        assert_eq!(state.sort(), None);
        assert!(!state.is_selectable());
        assert!(!state.is_filtered());
    }

    #[test]
    fn an_empty_table_has_one_empty_page() {
        let state = state(0);
        assert!(state.is_empty());
        assert_eq!(state.page_count(), 1);
        assert_eq!(state.page_range(), 0..0);
        assert_eq!(state.showing(), "Showing 0 of 0");
        assert_eq!(state.page_check_state(), CheckState::Unchecked);
    }

    #[test]
    fn header_presses_cycle_ascending_descending_and_unsorted() {
        let mut state = state(5).with_page_sizes([5]);
        let _ = state.update(Event::Sort("amount"));
        assert_eq!(state.sort(), Some(("amount", Direction::Ascending)));
        assert_eq!(ids(state.page_rows()), [1, 2, 3, 4, 5]);
        let _ = state.update(Event::Sort("amount"));
        assert_eq!(state.sort(), Some(("amount", Direction::Descending)));
        assert_eq!(ids(state.page_rows()), [5, 4, 3, 2, 1]);
        let _ = state.update(Event::Sort("amount"));
        assert_eq!(state.sort(), None);
        assert_eq!(ids(state.page_rows()), [1, 2, 3, 4, 5]);
        let _ = state.update(Event::Sort("email"));
        assert_eq!(state.sort(), Some(("email", Direction::Ascending)));
    }

    #[test]
    fn unsortable_and_unknown_columns_are_ignored() {
        let mut state = state(5).with_sort("amount", Direction::Descending);
        let _ = state.update(Event::Sort("id"));
        let _ = state.update(Event::SortBy(Some(("missing", Direction::Ascending))));
        assert_eq!(state.sort(), Some(("amount", Direction::Descending)));
        let _ = state.update(Event::SortBy(None));
        assert_eq!(state.sort(), None);
    }

    #[test]
    fn empty_values_sort_last_both_ways() {
        let mut rows = payments(3);
        rows[1].amount = None;
        let mut state = State::new(columns(), rows, |p| p.id);
        let _ = state.update(Event::SortBy(Some(("amount", Direction::Ascending))));
        assert_eq!(ids(state.page_rows()), [1, 3, 2]);
        let _ = state.update(Event::SortBy(Some(("amount", Direction::Descending))));
        assert_eq!(ids(state.page_rows()), [3, 1, 2]);
    }

    #[test]
    fn search_matches_visible_searchable_text_ignoring_case() {
        let mut state = state(23);
        let _ = state.update(Event::NextPage);
        let _ = state.update(Event::Search("USER2".into()));
        assert_eq!(state.page(), 0, "a new search starts on the first page");
        assert_eq!(ids(state.page_rows()), [2, 20, 21, 22, 23]);
        let _ = state.update(Event::Search("$30.00".into()));
        assert_eq!(ids(state.page_rows()), [3], "searches the formatted text");
        let _ = state.update(Event::Columns(dropdown_menu::Event::Activate("amount")));
        assert!(state.is_hidden("amount"));
        assert_eq!(state.filtered_len(), 0, "hidden columns are not searched");
        assert!(state.is_filtered());
    }

    #[test]
    fn column_filters_offer_distinct_values_and_combine_with_search() {
        let mut state = state(9);
        assert_eq!(state.options("status"), ["failed", "paid", "pending"]);
        assert!(state.options("email").is_empty());
        let _ = state.update(Event::Filter("status", Some("paid".into())));
        assert_eq!(state.filter("status"), Some("paid"));
        assert_eq!(ids(state.page_rows()), [3, 6, 9]);
        let _ = state.update(Event::Search("user9".into()));
        assert_eq!(ids(state.page_rows()), [9]);
        let _ = state.update(Event::Filter("status", None));
        assert_eq!(ids(state.page_rows()), [9]);
        let _ = state.update(Event::Filter("status", Some("paid".into())));
        let _ = state.update(Event::ClearFilters);
        assert_eq!(state.filtered_len(), 9);
        assert_eq!(state.search(), "");
        assert_eq!(state.filter("status"), None);
        let _ = state.update(Event::Filter("missing", Some("x".into())));
        assert!(!state.is_filtered(), "unknown columns are not filtered");
    }

    #[test]
    fn pages_clamp_and_page_size_keeps_the_first_row_in_view() {
        let mut state = state(23);
        let _ = state.update(Event::PreviousPage);
        assert_eq!(state.page(), 0);
        let _ = state.update(Event::Page(9));
        assert_eq!(state.page(), 2);
        assert_eq!(state.showing(), "Showing 21-23 of 23");
        let _ = state.update(Event::NextPage);
        assert_eq!(state.page(), 2);
        let _ = state.update(Event::Page(1));
        let _ = state.update(Event::PageSize(20));
        assert_eq!(state.page(), 0, "row 11 is on the first page of 20");
        let _ = state.update(Event::PageSize(5));
        assert_eq!(state.page(), 0);
        let _ = state.update(Event::Page(3));
        let _ = state.update(Event::PageSize(0));
        assert_eq!(state.page_size(), 1, "at least one row a page");
        assert_eq!(state.page(), 15);
    }

    #[test]
    fn page_sizes_can_be_set() {
        let state = state(23).with_page_sizes([0, 5, 25]);
        assert_eq!(state.page_sizes(), [5, 25]);
        assert_eq!(state.page_size(), 5);
        assert_eq!(state.page_count(), 5);
        let unchanged = self::state(3).with_page_sizes([]);
        assert_eq!(unchanged.page_sizes(), PAGE_SIZES);
    }

    #[test]
    fn selection_needs_a_selectable_table() {
        let mut state = state(5);
        assert!(state.update(Event::Select(1, true)).is_none());
        assert!(state.selected().is_empty());
    }

    #[test]
    fn rows_select_and_the_header_acts_on_the_page_only() {
        let mut state = state(23).with_selection(true);
        assert_eq!(selected(state.update(Event::Select(2, true))), [2]);
        assert_eq!(state.page_check_state(), CheckState::Indeterminate);
        assert!(state.update(Event::Select(2, true)).is_none(), "no change");
        assert!(
            state.update(Event::Select(99, true)).is_none(),
            "unknown row"
        );

        let _ = state.update(Event::NextPage);
        assert_eq!(state.page_check_state(), CheckState::Unchecked);
        assert_eq!(
            selected(state.update(Event::SelectPage(true))),
            [2, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]
        );
        assert_eq!(state.page_check_state(), CheckState::Checked);
        assert_eq!(selected(state.update(Event::SelectPage(false))), [2]);
        assert_eq!(
            selected(state.update(Event::ClearSelection)),
            Vec::<u32>::new()
        );
        assert!(state.update(Event::ClearSelection).is_none());
    }

    #[test]
    fn selection_follows_keys_through_sorting_and_row_changes() {
        let mut state = state(5).with_selection(true);
        let _ = state.update(Event::Select(1, true));
        let _ = state.update(Event::Select(4, true));
        let _ = state.update(Event::Sort("amount"));
        let _ = state.update(Event::Sort("amount"));
        assert!(state.is_selected(&1) && state.is_selected(&4));
        state.retain(|p| p.id != 4);
        assert_eq!(state.selected(), [1]);
        state.update_row(&1, |p| p.status = "failed");
        assert_eq!(state.row(&1).map(|p| p.status), Some("failed"));
        state.push(payment(6, "paid", "new@example.com", 5.0));
        assert_eq!(state.len(), 5);
        assert_eq!(ids(state.page_rows()), [5, 3, 2, 1, 6], "sorted into place");
        state.set_rows(payments(2));
        assert_eq!(state.selected(), [1]);
    }

    #[test]
    fn arrows_move_through_the_page_and_across_pages() {
        let mut state = state(23);
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(&1));
        let _ = state.update(Event::Last);
        assert_eq!(state.highlighted(), Some(&10));
        let _ = state.update(Event::Next);
        assert_eq!(state.page(), 1);
        assert_eq!(state.highlighted(), Some(&11));
        let _ = state.update(Event::Previous);
        assert_eq!(state.page(), 0);
        assert_eq!(state.highlighted(), Some(&10));
        let _ = state.update(Event::First);
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(&1), "stays at the very first row");
        let _ = state.update(Event::Page(2));
        let _ = state.update(Event::Last);
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(&23), "stays at the very last row");

        let mut fresh = self::state(23);
        let _ = fresh.update(Event::Previous);
        assert_eq!(fresh.highlighted(), Some(&10));
    }

    #[test]
    fn changing_page_moves_the_highlight_onto_it() {
        let mut state = state(23);
        let _ = state.update(Event::Press(3));
        let _ = state.update(Event::NextPage);
        assert_eq!(state.highlighted(), Some(&11));
        let _ = state.update(Event::Press(3));
        assert_eq!(
            state.highlighted(),
            Some(&11),
            "rows on other pages are ignored"
        );
        let _ = state.update(Event::Search("zzz".into()));
        assert_eq!(state.highlighted(), None);
    }

    #[test]
    fn space_toggles_and_enter_activates_the_highlighted_row() {
        let mut state = state(5).with_selection(true);
        assert!(state.update(Event::ToggleHighlighted).is_none());
        assert!(state.update(Event::ActivateHighlighted).is_none());
        let _ = state.update(Event::Press(2));
        assert_eq!(selected(state.update(Event::ToggleHighlighted)), [2]);
        assert_eq!(
            selected(state.update(Event::ToggleHighlighted)),
            Vec::<u32>::new()
        );
        assert_eq!(
            state.update(Event::ActivateHighlighted),
            Some(Output::Activated(2))
        );
        assert_eq!(state.update(Event::Activate(4)), Some(Output::Activated(4)));
        assert_eq!(state.highlighted(), Some(&4));
        assert!(state.update(Event::Activate(40)).is_none());
    }

    #[test]
    fn the_columns_menu_hides_columns_but_never_the_last() {
        let mut state = state(3);
        let menu: Vec<ColumnId> = state
            .columns_menu()
            .entries()
            .iter()
            .filter_map(|entry| entry.as_item().map(|item| item.id))
            .collect();
        assert_eq!(menu, ["status", "email", "amount"], "ID is not hideable");
        let _ = state.update(Event::Columns(dropdown_menu::Event::Open));
        let _ = state.update(Event::Columns(dropdown_menu::Event::Activate("email")));
        assert!(state.is_hidden("email"));
        assert!(!state.columns_menu().is_checked("email"));
        assert_eq!(state.visible_columns().len(), 3);
        let _ = state.update(Event::Columns(dropdown_menu::Event::Activate("email")));
        assert!(!state.is_hidden("email"));

        let mut single = State::new(
            [column("name", "Name", |p: &Payment| p.email.clone().into())],
            payments(1),
            |p| p.id,
        );
        let _ = single.update(Event::Columns(dropdown_menu::Event::Activate("name")));
        assert!(!single.is_hidden("name"));
        assert!(single.columns_menu().is_checked("name"));
    }

    #[test]
    fn with_hidden_starts_with_columns_hidden() {
        let state = state(3).with_hidden(["email"]);
        assert!(state.is_hidden("email"));
        assert!(!state.columns_menu().is_checked("email"));
        assert!(state.columns_menu().is_checked("status"));
        let visible: Vec<ColumnId> = state.visible_columns().iter().map(|c| c.id).collect();
        assert_eq!(visible, ["status", "amount", "id"]);
    }

    #[test]
    fn focus_is_tracked_and_lands_on_a_row() {
        let mut state = state(3);
        let _ = state.update(Event::Focus(true));
        assert!(state.is_focused());
        assert_eq!(state.highlighted(), Some(&1));

        let mut selected = self::state(3).with_selection(true);
        let _ = selected.update(Event::Select(2, true));
        let _ = selected.update(Event::Focus(true));
        assert_eq!(selected.highlighted(), Some(&2), "the first selected row");
        let _ = selected.update(Event::Focus(false));
        assert!(!selected.is_focused());

        let empty = &mut self::state(0);
        let _ = empty.update(Event::Focus(true));
        assert_eq!(empty.highlighted(), None);
    }

    #[test]
    fn columns_read_and_format_cells() {
        let columns = columns();
        let row = payment(7, "paid", "a@b.c", 12.5);
        assert_eq!(columns[2].value(&row), Value::Number(12.5));
        assert_eq!(columns[2].text(&row), "$12.50");
        assert_eq!(columns[0].text(&row), "paid");
        let column = columns[0]
            .clone()
            .width(120)
            .align(Align::Center)
            .searchable(false);
        assert_eq!(column.width, Length::Fixed(120.0));
        assert_eq!(column.align, Align::Center);
        assert!(!column.searchable);
        assert!(format!("{column:?}").contains("status"));
        assert_eq!(Align::ALL.len(), 3);
        assert_eq!(Direction::ALL.len(), 2);
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "ArrowDown"), Some(Action::Next));
        assert_eq!(press(&keymap, "ArrowUp"), Some(Action::Previous));
        assert_eq!(press(&keymap, "Home"), Some(Action::First));
        assert_eq!(press(&keymap, "End"), Some(Action::Last));
        assert_eq!(press(&keymap, "PageDown"), Some(Action::NextPage));
        assert_eq!(press(&keymap, "PageUp"), Some(Action::PreviousPage));
        assert_eq!(press(&keymap, "Space"), Some(Action::Select));
        assert_eq!(press(&keymap, "Enter"), Some(Action::Activate));
        assert_eq!(press(&keymap, "Mod+A"), Some(Action::SelectAll));
        assert_eq!(press(&keymap, "Escape"), Some(Action::ClearSelection));
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?}");
            assert!(keys::Action::description(*action).ends_with('.'));
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&Chord::named(Named::Escape))
            .bind(Chord::character('j'), Action::Next);
        let mut state = state(5).with_selection(true);
        let _ = state.update(Event::Select(1, true));
        let escape = keys::Event {
            key: Key::Named(Named::Escape),
            modifiers: Modifiers::empty(),
        };
        assert_eq!(state.key_event(&keymap, &escape), None);
        let j = keys::Event {
            key: Key::Character("j".into()),
            modifiers: Modifiers::empty(),
        };
        assert_eq!(state.key_event(&keymap, &j), Some(Event::Next));
        assert!(default_keymap().clear().is_empty());
    }

    #[test]
    fn actions_map_to_events_only_when_they_apply() {
        let mut state = state(23);
        assert_eq!(Action::Next.event(&state), Some(Event::Next));
        assert_eq!(Action::PreviousPage.event(&state), None);
        assert_eq!(Action::NextPage.event(&state), Some(Event::NextPage));
        assert_eq!(Action::Activate.event(&state), None);
        assert_eq!(Action::Select.event(&state), None, "not selectable");
        assert_eq!(Action::SelectAll.event(&state), None);
        let _ = state.update(Event::First);
        assert_eq!(
            Action::Activate.event(&state),
            Some(Event::ActivateHighlighted)
        );

        let mut selectable = self::state(3).with_selection(true);
        assert_eq!(
            Action::SelectAll.event(&selectable),
            Some(Event::SelectPage(true))
        );
        assert_eq!(
            Action::ClearSelection.event(&selectable),
            None,
            "nothing selected"
        );
        let _ = selectable.update(Event::SelectPage(true));
        assert_eq!(
            Action::ClearSelection.event(&selectable),
            Some(Event::ClearSelection)
        );
        let _ = selectable.update(Event::First);
        assert_eq!(
            Action::Select.event(&selectable),
            Some(Event::ToggleHighlighted)
        );

        let empty = self::state(0);
        for action in [Action::Next, Action::Previous, Action::First, Action::Last] {
            assert_eq!(action.event(&empty), None, "{action:?}");
        }
    }

    #[test]
    fn rows_are_muted_when_selected_and_ringed_when_highlighted() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let idle = row_style(&tokens, Status::default());
            assert_eq!(idle.background, None);
            assert_eq!(idle.ring, None);
            assert_eq!(idle.divider, tokens.border);
            let selected = row_style(
                &tokens,
                Status {
                    selected: true,
                    hovered: true,
                    ..Status::default()
                },
            );
            assert_eq!(selected.background, Some(tokens.muted));
            let hovered = row_style(
                &tokens,
                Status {
                    hovered: true,
                    ..Status::default()
                },
            );
            assert_eq!(hovered.background, Some(fade(tokens.muted, 0.5)));
            let highlighted = row_style(
                &tokens,
                Status {
                    highlighted: true,
                    ..Status::default()
                },
            );
            assert_eq!(highlighted.ring, Some(tokens.ring));

            let frame = frame_style(&tokens);
            assert_eq!(frame.border.color, tokens.border);
            assert_eq!(frame.border.radius, radius::MD.into());
            assert_eq!(
                skeleton_style(&tokens).background,
                Some(Background::Color(tokens.muted))
            );
        }
    }

    #[test]
    fn sort_indicators_show_the_direction() {
        assert_eq!(sort_glyph(None), crate::lucide!(ArrowUpDown));
        assert_eq!(
            sort_glyph(Some(Direction::Ascending)),
            crate::lucide!(ArrowUp)
        );
        assert_eq!(
            sort_glyph(Some(Direction::Descending)),
            crate::lucide!(ArrowDown)
        );
    }

    #[test]
    fn filter_and_sort_choices_read_well() {
        let all = FilterChoice {
            column: "status",
            header: "Status".into(),
            value: None,
        };
        assert_eq!(all.to_string(), "Status: All");
        let paid = FilterChoice {
            value: Some("paid".into()),
            ..all
        };
        assert_eq!(paid.to_string(), "Status: paid");
        let sort = SortChoice {
            sort: None,
            label: "Default order".into(),
        };
        assert_eq!(sort.to_string(), "Default order");
    }

    #[test]
    fn builder_defaults_and_options() {
        let state = state(3);
        let table: DataTable<'_, Payment, u32, ()> = data_table(&state);
        assert!(table.toolbar && !table.loading && !table.row_actions);
        assert!(!table.is_enabled());
        assert_eq!(table.height, Length::Shrink);
        assert_eq!(table.breakpoint, BREAKPOINT);
        assert_eq!(table.keymap, default_keymap());
        assert_eq!(table.empty, "No results.");
        let table = table
            .toolbar(false)
            .loading(true)
            .empty("Nothing here")
            .placeholder("Filter emails...")
            .height(300)
            .row_height(32.0)
            .breakpoint(400.0)
            .keymap(Keymap::new())
            .row_actions(true)
            .cell("status", |p: &Payment| text(p.status).into())
            .cell("status", |p: &Payment| text(p.status).into())
            .on_event(|_| ());
        assert!(!table.toolbar && table.loading && table.row_actions);
        assert_eq!(table.cells.len(), 1, "a second renderer replaces the first");
        assert_eq!(table.placeholder, "Filter emails...");
        assert_eq!(table.height, Length::Fixed(300.0));
        assert_eq!(table.row_height, 32.0);
        assert!(table.is_enabled());
        assert!(format!("{table:?}").contains("DataTable"));
    }
}
