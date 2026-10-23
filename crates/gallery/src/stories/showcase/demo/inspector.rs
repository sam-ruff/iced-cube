//! Open jobs: a pinned tab holds the job list and each opened job gets a
//! tab of its own, with its settings in a property grid and its definition
//! as JSON.

use std::collections::HashSet;

use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length, Task};
use iced_cube::data::json_viewer::{self, NodeId, json_viewer};
use iced_cube::data::property_grid::{
    self, Property, Value, choice, number, property_grid, read_only,
};
use iced_cube::dropdown_menu;
use iced_cube::navigation::document_tabs::{self, Output, document, document_tabs};
use iced_cube::primitives::button::{Size as ButtonSize, Variant};
use iced_cube::primitives::icon_button::Variant as IconVariant;
use iced_cube::primitives::toggle_group::{self, toggle_group};
use iced_cube::theme::{semibold, text_size};
use iced_cube::{badge, button, context_menu, icon_button, lucide};

use super::data::{Job, JobId, Queue, SOURCES, Schedule, Status};
use super::{Example, Message, caption};
use crate::bridge;

/// From this content width the inspector and the JSON sit side by side.
pub const SPLIT: f32 = 760.0;
const INSPECTOR_WIDTH: f32 = 340.0;
const MAX_RETRIES: f64 = 5.0;

/// A tab on the Jobs page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Doc {
    /// The job list, pinned first.
    Jobs,
    Job(JobId),
}

/// A row of a job's inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    Id,
    Name,
    Owner,
    Source,
    Notes,
    Queue,
    Schedule,
    Retries,
    Status,
    Attempts,
    Started,
}

/// What a narrow job tab shows: the inspector or the JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Properties,
    Json,
}

/// The tabs on the Jobs page, starting with the job list alone. The job
/// list stays pinned and never closes.
pub fn documents() -> document_tabs::State<Doc> {
    document_tabs::State::new([document(Doc::Jobs, "All jobs")
        .pinned(true)
        .closable(false)
        .locked(true)])
}

pub fn panes() -> toggle_group::State<Pane> {
    toggle_group::State::single([
        toggle_group::item(Pane::Properties, "Properties"),
        toggle_group::item(Pane::Json, "JSON"),
    ])
    .required(true)
}

/// One open job: its settings and its definition.
#[derive(Debug)]
pub struct Inspector {
    pub grid: property_grid::State<Field>,
    pub json: json_viewer::State,
}

impl Inspector {
    pub fn new(job: &Job, workspace_retries: u8) -> Self {
        Self {
            grid: property_grid::State::new(properties(job, workspace_retries)),
            json: json_viewer::State::new(job.spec(workspace_retries)).with_depth(1),
        }
    }

    /// Brings the grid and the JSON up to date with the job, keeping what
    /// is being edited, expanded, highlighted and searched.
    pub fn sync(&mut self, job: &Job, workspace_retries: u8) {
        for property in properties(job, workspace_retries) {
            if self.grid.value(property.id) != Some(&property.value) {
                let _ = self.grid.set(property.id, property.value);
            }
        }
        let spec = job.spec(workspace_retries);
        if self.json.value() != &spec {
            self.json = carry_over(&self.json, spec);
        }
    }
}

/// The inspector's rows for `job`, with their current values.
fn properties(job: &Job, workspace_retries: u8) -> Vec<Property<Field>> {
    let mut sources: Vec<&str> = SOURCES.to_vec();
    if !sources.contains(&job.source.as_str()) {
        sources.push(&job.source);
    }
    let retries = job.retries.unwrap_or(workspace_retries);

    vec![
        read_only(Field::Id, "ID", job.id.to_string()).category("General"),
        property_grid::text(Field::Name, "Name", job.name.as_str())
            .category("General")
            .validate(check_name)
            .description("Lowercase letters, digits and dashes, as in a URL."),
        read_only(Field::Owner, "Owner", job.owner).category("General"),
        choice(Field::Source, "Source", sources, job.source.as_str())
            .category("General")
            .description("The table, bucket or database the job reads."),
        property_grid::text(Field::Notes, "Notes", job.notes.as_str())
            .category("General")
            .description("Shown to anyone who opens the job."),
        choice(
            Field::Queue,
            "Queue",
            Queue::ALL.map(|queue| queue.to_string()),
            job.queue.to_string(),
        )
        .category("Scheduling")
        .default(Queue::Normal.to_string())
        .description("High priority jobs take free slots first."),
        choice(
            Field::Schedule,
            "Schedule",
            Schedule::ALL.map(|schedule| schedule.to_string()),
            job.schedule.to_string(),
        )
        .category("Scheduling")
        .default(Schedule::Once.to_string()),
        number(Field::Retries, "Retries", f64::from(retries))
            .category("Scheduling")
            .range(0.0, MAX_RETRIES)
            .step(1.0)
            .decimals(0)
            .default(f64::from(workspace_retries))
            .description("Runs after a failure before the owner is told. Reset to follow the workspace setting."),
        read_only(Field::Status, "Status", job.status.label()).category("Last run"),
        read_only(Field::Attempts, "Retries used", job.attempts.to_string()).category("Last run"),
        read_only(Field::Started, "Started", job.started.as_str()).category("Last run"),
    ]
}

fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("A job needs a name.".into());
    }
    let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
    if !name.chars().all(allowed) {
        return Err("Use lowercase letters, digits and dashes.".into());
    }
    Ok(())
}

/// Writes an edit from the inspector into the job.
pub fn apply(job: &mut Job, field: Field, value: &Value, workspace_retries: u8) {
    let text = value.as_text();
    match field {
        Field::Name => {
            if let Some(name) = text {
                job.name = name.to_owned();
            }
        }
        Field::Source => {
            if let Some(source) = text {
                job.source = source.to_owned();
            }
        }
        Field::Notes => {
            if let Some(notes) = text {
                job.notes = notes.to_owned();
            }
        }
        Field::Queue => {
            if let Some(queue) = text.and_then(Queue::from_label) {
                job.queue = queue;
            }
        }
        Field::Schedule => {
            let Some(schedule) = text.and_then(Schedule::from_label) else {
                return;
            };
            job.schedule = schedule;
            // A job waiting for its schedule runs as soon as it can once
            // it has none.
            if schedule == Schedule::Once && job.status == Status::Scheduled {
                job.status = Status::Queued;
            }
        }
        Field::Retries => {
            let Some(retries) = value.as_number() else {
                return;
            };
            let retries = retries.round().clamp(0.0, MAX_RETRIES) as u8;
            job.retries = (retries != workspace_retries).then_some(retries);
        }
        Field::Id | Field::Owner | Field::Status | Field::Attempts | Field::Started => {}
    }
}

/// A viewer of `value` with the same nodes expanded and highlighted, and
/// the same search, as `old`, matching nodes by path.
fn carry_over(old: &json_viewer::State, value: serde_json::Value) -> json_viewer::State {
    use json_viewer::Event;

    let expanded: HashSet<String> = nodes(old)
        .into_iter()
        .filter(|id| old.is_expanded(*id))
        .map(|id| old.path(id))
        .collect();
    let highlighted = old.highlighted().map(|id| old.path(id));

    let mut new = json_viewer::State::new(value);
    for id in nodes(&new) {
        let path = new.path(id);
        if expanded.contains(&path) {
            let _ = new.update(Event::Expand(id));
        }
        if highlighted.as_ref() == Some(&path) {
            let _ = new.update(Event::Press(id));
        }
    }
    if !old.query().is_empty() {
        let _ = new.update(Event::Search(old.query().to_owned()));
    }
    if old.is_focused() {
        let _ = new.update(Event::Focus(true));
    }
    new
}

/// Every node of a viewer, in document order.
fn nodes(state: &json_viewer::State) -> Vec<NodeId> {
    let mut nodes = Vec::with_capacity(state.len());
    let mut stack: Vec<NodeId> = state.roots().iter().rev().copied().collect();
    while let Some(id) = stack.pop() {
        nodes.push(id);
        stack.extend(state.children(id).iter().rev());
    }
    nodes
}

impl Example {
    /// Whether the job tabs are on screen: the Jobs page with a job open.
    pub(super) fn documents_shown(&self) -> bool {
        self.page.selected() == Some(super::Page::Jobs) && self.documents.documents().len() > 1
    }

    /// The job whose tab is active, if it is not the job list.
    pub(super) fn active_job(&self) -> Option<JobId> {
        match self.documents.active() {
            Some(Doc::Job(id)) => Some(id),
            _ => None,
        }
    }

    /// Opens a job in a tab of its own, or switches to its tab.
    pub(super) fn open_job(&mut self, id: JobId) {
        let retries = self.settings.applied().retries;
        let Some(job) = self.jobs.iter().find(|job| job.id == id) else {
            return;
        };
        let label = job.name.clone();
        let _ = self
            .inspectors
            .entry(id)
            .or_insert_with(|| Inspector::new(job, retries));
        self.documents.open(document(Doc::Job(id), label));
        self.cursor = Some(id);
        self.go(super::Page::Jobs);
    }

    pub(super) fn documents_event(&mut self, event: document_tabs::Event<Doc>) {
        if let Some(Output::CloseRequested(ids)) = self.documents.update(event) {
            self.close_documents(&ids);
        }
    }

    /// Closes job tabs and drops their inspectors.
    fn close_documents(&mut self, ids: &[Doc]) {
        let _ = self.documents.close(ids);
        for doc in ids {
            if let Doc::Job(id) = doc {
                let _ = self.inspectors.remove(id);
            }
        }
    }

    pub(super) fn inspector_event(
        &mut self,
        id: JobId,
        event: property_grid::Event<Field>,
    ) -> Task<Message> {
        let retries = self.settings.applied().retries;
        let Some(inspector) = self.inspectors.get_mut(&id) else {
            return Task::none();
        };
        let output = inspector.grid.update(event);
        property_grid::sync_menu(&mut self.grid_menu, &inspector.grid);
        match output {
            Some(
                property_grid::Output::Changed { id: field, value }
                | property_grid::Output::Reset { id: field, value },
            ) => {
                if let Some(job) = self.job_mut(id) {
                    apply(job, field, &value, retries);
                }
                if matches!(field, Field::Queue | Field::Schedule) {
                    self.schedule();
                }
                Task::none()
            }
            Some(property_grid::Output::CopyValue(value)) => bridge::copy(value),
            None => Task::none(),
        }
    }

    pub(super) fn json_event(&mut self, id: JobId, event: json_viewer::Event) -> Task<Message> {
        let Some(inspector) = self.inspectors.get_mut(&id) else {
            return Task::none();
        };
        match inspector.json.update(event) {
            Some(json_viewer::Output::CopyValue(text) | json_viewer::Output::CopyPath(text)) => {
                bridge::copy(text)
            }
            Some(json_viewer::Output::FocusSearch) => json_viewer::focus_search(&inspector.json),
            None => Task::none(),
        }
    }

    pub(super) fn grid_menu_event(
        &mut self,
        event: context_menu::Event<property_grid::MenuItem, Field>,
    ) -> Task<Message> {
        let chosen = self.grid_menu.update(event);
        let Some(id) = self.active_job() else {
            return Task::none();
        };
        match (chosen, self.grid_menu.target()) {
            (Some(dropdown_menu::Output::Activated(item)), Some(&field)) => {
                self.inspector_event(id, item.event(field))
            }
            _ => Task::none(),
        }
    }

    pub(super) fn json_menu_event(
        &mut self,
        event: context_menu::Event<json_viewer::MenuItem, NodeId>,
    ) -> Task<Message> {
        let chosen = self.json_menu.update(event);
        let Some(id) = self.active_job() else {
            return Task::none();
        };
        match (chosen, self.json_menu.target()) {
            (Some(dropdown_menu::Output::Activated(item)), Some(&node)) => {
                self.json_event(id, item.event(node))
            }
            _ => Task::none(),
        }
    }

    /// Closes the tabs of jobs that have gone and brings the others up to
    /// date with their jobs.
    pub(super) fn sync_documents(&mut self) {
        let gone: Vec<Doc> = self
            .documents
            .documents()
            .iter()
            .filter_map(|doc| match doc.id {
                Doc::Job(id) if self.job(id).is_none() => Some(doc.id),
                _ => None,
            })
            .collect();
        if !gone.is_empty() {
            self.close_documents(&gone);
        }
        // The list is out of sight, so the pointer is not over it.
        if self.active_job().is_some() {
            self.list_hovered = false;
        }

        let retries = self.settings.applied().retries;
        for (id, inspector) in &mut self.inspectors {
            let Some(job) = self.jobs.iter().find(|job| job.id == *id) else {
                continue;
            };
            inspector.sync(job, retries);
            let doc = Doc::Job(*id);
            if self
                .documents
                .document(doc)
                .is_some_and(|tab| tab.label != job.name)
            {
                self.documents.set_label(doc, job.name.clone());
            }
        }
    }

    /// Resolves the job tabs' keymap while they are on screen.
    pub(super) fn documents_key(&mut self, key: &iced_cube::keys::Event) -> bool {
        if !self.documents_shown() {
            return false;
        }
        let Some(event) = self
            .keys
            .documents
            .resolve_event(key)
            .and_then(|action| action.event(&self.documents))
        else {
            return false;
        };
        self.documents_event(event);
        true
    }

    /// Moves the keyboard to the search of whatever the Jobs page shows:
    /// the job list, the inspector, or the JSON on a narrow window.
    pub(super) fn focus_job_search(&self) -> Task<Message> {
        let Some(inspector) = self.active_job().and_then(|id| self.inspectors.get(&id)) else {
            return iced::widget::operation::focus(super::jobs::SEARCH_ID);
        };
        let split = self.content_width(self.width) >= SPLIT;
        if !split && self.panes.selected_one() == Some(Pane::Json) {
            return json_viewer::focus_search(&inspector.json);
        }
        property_grid::focus_search(&inspector.grid)
    }

    /// An open job: a heading, then the inspector and the JSON side by
    /// side, or one of them at a time on narrow windows.
    pub(super) fn job_view(&self, id: JobId, narrow: bool, split: bool) -> Element<'_, Message> {
        let (Some(job), Some(inspector)) = (self.job(id), self.inspectors.get(&id)) else {
            return caption("This job is no longer in the list.").into();
        };

        let logs: Element<'_, Message> = if narrow {
            icon_button(lucide!(ScrollText))
                .label("View logs")
                .variant(IconVariant::Outline)
                .on_press(Message::ViewLogs(id))
                .into()
        } else {
            button("View logs")
                .icon(lucide!(ScrollText))
                .variant(Variant::Outline)
                .size(ButtonSize::Sm)
                .on_press(Message::ViewLogs(id))
                .into()
        };
        let heading = row![
            column![
                text(&job.name)
                    .size(text_size::MD)
                    .font(semibold())
                    .wrapping(text::Wrapping::None),
                caption(format!("{id}  {}", job.owner)).wrapping(text::Wrapping::None),
            ]
            .spacing(2)
            .width(Length::Fill),
            badge(job.status.label()).variant(job.status.badge()),
            logs,
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let grid = property_grid(&inspector.grid)
            .placeholder("Filter properties")
            .keymap(self.keys.grid.clone())
            .context_menu(&self.grid_menu, Message::GridMenu)
            .height(Length::Fill)
            .on_event(move |event| Message::Inspector(id, event));
        let json = json_viewer(&inspector.json)
            .search_bar(true)
            .placeholder("Search the definition")
            .keymap(self.keys.json.clone())
            .context_menu(&self.json_menu, Message::JsonMenu)
            .width(Length::Fill)
            .height(Length::Fill)
            .on_event(move |event| Message::Json(id, event));

        let body: Element<'_, Message> = if split {
            row![
                container(grid).width(INSPECTOR_WIDTH).height(Length::Fill),
                json
            ]
            .spacing(16)
            .height(Length::Fill)
            .into()
        } else {
            let pane: Element<'_, Message> = match self.panes.selected_one() {
                Some(Pane::Json) => json.into(),
                _ => grid.width(Length::Fill).into(),
            };
            column![toggle_group(&self.panes).on_event(Message::Pane), pane]
                .spacing(12)
                .height(Length::Fill)
                .into()
        };

        column![heading, body]
            .spacing(16)
            .padding(iced::Padding {
                bottom: 12.0,
                ..iced::Padding::ZERO
            })
            .height(Length::Fill)
            .into()
    }

    /// The tab strip above the Jobs page while a job is open.
    pub(super) fn document_strip(&self) -> Element<'_, Message> {
        document_tabs(&self.documents)
            .on_event(Message::Documents)
            .into()
    }
}
