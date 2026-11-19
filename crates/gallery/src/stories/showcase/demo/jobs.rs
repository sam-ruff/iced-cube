//! The job list: a data table that searches, filters, sorts and resizes,
//! with a context menu on every row and bulk actions for the ticked rows.

use iced::widget::{column, container, mouse_area, responsive, row, space, stack, text};
use iced::{Alignment, Element, Length, Padding};
use iced_cube::data::data_table::{self, Column, data_table};
use iced_cube::dropdown_menu::{Entry, item, radio_item, separator, submenu};
use iced_cube::feedback::badge::Variant as BadgeVariant;
use iced_cube::feedback::progress::{Size as ProgressSize, Variant as ProgressVariant};
use iced_cube::feedback::spinner::Size as SpinnerSize;
use iced_cube::primitives::button::{Size as ButtonSize, Variant};
use iced_cube::primitives::icon_button::Variant as IconVariant;
use iced_cube::theme::{Tokens, semibold, text_size};
use iced_cube::{badge, button, icon, icon_button, lucide, progress, spinner, tooltip};

use super::data::{Job, Queue, Status};
use super::inspector::SPLIT;
use super::{Example, Message, RowAction, caption, count};

/// Below this width the table shows each job as a card. The toolbar's
/// search, two filters and Columns menu need this much side by side.
const CARDS: f32 = 720.0;
/// Rows tall enough for a name over its ID and source.
const ROW: f32 = 52.0;
const COMPACT_ROW: f32 = 40.0;
/// How far the "new jobs" pill floats above the bottom of the table.
const PILL_LIFT: f32 = 56.0;

/// The widget id of the search field above the job list.
pub const SEARCH_ID: &str = "demo-job-search";

pub const JOB: data_table::ColumnId = "job";
pub const OWNER: data_table::ColumnId = "owner";
pub const QUEUE: data_table::ColumnId = "queue";
pub const STATUS: data_table::ColumnId = "status";
pub const PROGRESS: data_table::ColumnId = "progress";

/// The job list's columns. The queue sorts by priority and progress by
/// how far the job has got, whatever their cells say.
pub fn columns() -> Vec<Column<Job>> {
    vec![
        data_table::column(JOB, "Job", |job: &Job| job.name.clone().into())
            .format(|job| format!("{} {}", job.name, job.id))
            .width(Length::FillPortion(4))
            .hideable(false),
        data_table::column(OWNER, "Owner", |job: &Job| job.owner.into())
            .width(Length::FillPortion(2))
            .filterable(true),
        data_table::column(QUEUE, "Queue", |job: &Job| {
            Queue::ALL
                .iter()
                .position(|queue| *queue == job.queue)
                .into()
        })
        .format(|job| job.queue.to_string())
        .width(100),
        data_table::column(STATUS, "Status", |job: &Job| job.status.label().into())
            .width(130)
            .filterable(true),
        data_table::column(PROGRESS, "Progress", |job: &Job| job.progress.into())
            .format(Job::detail)
            .width(Length::FillPortion(3))
            .searchable(false),
    ]
}

pub fn row_menu_entries() -> Vec<Entry<RowAction>> {
    vec![
        item(RowAction::Open, "Open")
            .icon(lucide!(PanelRight))
            .shortcut("Enter"),
        item(RowAction::Logs, "View logs").icon(lucide!(ScrollText)),
        item(RowAction::Retry, "Retry").icon(lucide!(RotateCw)),
        item(RowAction::Pause, "Pause").icon(lucide!(Pause)),
        item(RowAction::Resume, "Resume").icon(lucide!(Play)),
        submenu(
            RowAction::Queue,
            "Queue",
            [
                radio_item(RowAction::High, "High", false),
                radio_item(RowAction::Normal, "Normal", true),
                radio_item(RowAction::Low, "Low", false),
            ],
        )
        .icon(lucide!(ListOrdered)),
        item(RowAction::CopyId, "Copy job ID").icon(lucide!(Copy)),
        separator(),
        item(RowAction::Delete, "Delete job")
            .icon(lucide!(Trash))
            .shortcut("Delete")
            .destructive(true),
    ]
}

impl Example {
    /// The job list, or an open job, under the tabs while a job is open.
    pub(super) fn jobs_page(&self, narrow: bool, content: f32) -> Element<'_, Message> {
        let body = match self.active_job() {
            Some(id) => self.job_view(id, narrow, content >= SPLIT),
            None => self.job_list(narrow),
        };
        if !self.documents_shown() {
            return body;
        }
        column![self.document_strip(), body]
            .spacing(12)
            .height(Length::Fill)
            .into()
    }

    fn job_list(&self, narrow: bool) -> Element<'_, Message> {
        // The table turns into cards by its own width, and a card's
        // heading is one line.
        let table = responsive(move |size| self.job_table(size.width < CARDS));

        // New jobs wait behind a pill while the pointer is over the list, so
        // nothing moves under it.
        let mut layers = stack![table].width(Length::Fill).height(Length::Fill);
        let fresh = self.fresh_shown();
        if fresh > 0 {
            layers = layers.push(
                container(
                    button(format!("Show {}", count(fresh, "new job")))
                        .icon(lucide!(ArrowUp))
                        .size(ButtonSize::Sm)
                        .on_press(Message::ShowNew),
                )
                .padding(Padding::ZERO.bottom(PILL_LIFT))
                .center_x(Length::Fill)
                .align_bottom(Length::Fill),
            );
        }
        let list = mouse_area(layers)
            .on_enter(Message::ListHover(true))
            .on_exit(Message::ListHover(false));

        column![
            container(self.job_bar(narrow))
                .height(36)
                .align_y(Alignment::Center),
            list,
        ]
        .spacing(8)
        .padding(Padding::ZERO.bottom(12))
        .height(Length::Fill)
        .into()
    }

    /// Bulk actions for the ticked rows, or how to use the list.
    fn job_bar(&self, narrow: bool) -> Element<'_, Message> {
        let selected = self.selected();
        if !selected.is_empty() {
            let any_running = selected
                .iter()
                .filter_map(|id| self.job(*id))
                .any(|job| job.status == Status::Running);
            return row![
                badge(format!("{} selected", selected.len())),
                space::horizontal(),
                button("Pause")
                    .icon(lucide!(Pause))
                    .variant(Variant::Outline)
                    .on_press_maybe(any_running.then_some(Message::PauseSelected)),
                button("Delete")
                    .icon(lucide!(Trash))
                    .variant(Variant::Destructive)
                    .on_press(Message::DeleteSelected),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .into();
        }
        if narrow {
            return row![
                caption("Tap a job to open it. Hold it, or tap its menu button, for more.")
                    .width(Length::Fill),
                icon_button(lucide!(Plus))
                    .label("New job")
                    .variant(IconVariant::Outline)
                    .on_press(Message::OpenNewJob),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into();
        }
        caption(
            "Double-click or press Enter to open a job. Right-click or Shift+F10 for more. \
             Drag the edge of a column heading to resize it.",
        )
        .into()
    }

    fn job_table(&self, cards: bool) -> Element<'_, Message> {
        data_table(&self.table)
            .placeholder("Search jobs, owners or IDs")
            .search_id(SEARCH_ID)
            .empty("No jobs match. Clear the search or pick another filter.")
            .height(Length::Fill)
            .row_height(if self.compact { COMPACT_ROW } else { ROW })
            .breakpoint(CARDS)
            .keymap(self.keys.table.clone())
            .cell(JOB, move |job| self.name_cell(job, cards))
            .cell(QUEUE, |job: &Job| {
                badge(job.queue.to_string())
                    .variant(BadgeVariant::Outline)
                    .into()
            })
            .cell(STATUS, |job: &Job| {
                badge(job.status.label()).variant(job.status.badge()).into()
            })
            .cell(PROGRESS, |job| self.progress_cell(job))
            .context_menu(&self.row_menu, Message::RowMenu)
            .context_menu_keymap(self.keys.row_menu.clone())
            .row_actions(true)
            .on_event(Message::Table)
            .into()
    }

    /// A job's name over its ID and source, or on one line in a card.
    fn name_cell<'a>(&'a self, job: &'a Job, cards: bool) -> Element<'a, Message> {
        let mut title = row![
            text(&job.name)
                .size(text_size::SM)
                .font(semibold())
                .wrapping(text::Wrapping::None)
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        if !job.notes.is_empty() {
            title = title.push(tooltip(
                icon::themed(lucide!(StickyNote), 14.0, 1.0, |theme| {
                    Tokens::of(theme).muted_foreground
                }),
                job.notes.as_str(),
            ));
        }
        if cards {
            return title
                .push(caption(job.id.to_string()).wrapping(text::Wrapping::None))
                .into();
        }
        column![
            title,
            caption(format!("{}  {}", job.id, job.source)).wrapping(text::Wrapping::None)
        ]
        .spacing(2)
        .into()
    }

    fn progress_cell<'a>(&self, job: &'a Job) -> Element<'a, Message> {
        match job.status {
            Status::Running => row![
                spinner(self.phase).size(SpinnerSize::Sm),
                progress(job.progress)
                    .size(ProgressSize::Sm)
                    .show_percentage(true),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
            Status::Paused => progress(job.progress)
                .size(ProgressSize::Sm)
                .variant(ProgressVariant::Warning)
                .show_percentage(true)
                .into(),
            _ => caption(job.detail()).wrapping(text::Wrapping::None).into(),
        }
    }
}
