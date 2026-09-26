//! The job list: a search input, a status select, selectable rows with a
//! context menu each, and bulk actions for the checked rows.

use iced::widget::{column, container, mouse_area, row, space, text};
use iced::{Alignment, Border, Element, Length};
use iced_cube::context_menu::context_menu;
use iced_cube::dropdown_menu::{Entry, item, radio_item, separator, submenu};
use iced_cube::feedback::progress::{Size as ProgressSize, Variant as ProgressVariant};
use iced_cube::feedback::spinner::Size as SpinnerSize;
use iced_cube::primitives::button::Variant;
use iced_cube::primitives::checkbox::CheckState;
use iced_cube::primitives::icon_button::Variant as IconVariant;
use iced_cube::theme::{Tokens, radius, semibold, text_size};
use iced_cube::{
    badge, button, checkbox, icon_button, input, lucide, progress, scroll_area, select, spinner,
};

use super::data::{Job, Status, StatusFilter};
use super::{Example, Message, RowAction, caption, muted};

pub fn row_menu_entries() -> Vec<Entry<RowAction>> {
    vec![
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
    pub(super) fn jobs_page(&self, narrow: bool) -> Element<'_, Message> {
        let search = input("Search jobs, owners or IDs", &self.search)
            .icon(lucide!(Search))
            .width(Length::Fill)
            .on_input(Message::Search);
        let filter = select(&StatusFilter::ALL[..], Some(self.filter))
            .width(if narrow {
                Length::Fill
            } else {
                Length::Fixed(170.0)
            })
            .on_select(Message::Filter);
        let toolbar: Element<'_, Message> = if narrow {
            column![
                search,
                row![
                    filter,
                    icon_button(lucide!(Plus))
                        .label("New job")
                        .variant(IconVariant::Outline)
                        .on_press(Message::OpenNewJob),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(8)
            .into()
        } else {
            row![search, filter].spacing(8).into()
        };

        let visible: Vec<&Job> = self.visible_jobs().collect();
        let checked = visible.iter().filter(|job| job.checked).count();
        let all = match checked {
            0 => CheckState::Unchecked,
            n if n == visible.len() => CheckState::Checked,
            _ => CheckState::Indeterminate,
        };
        let select_all =
            checkbox(all).on_toggle_maybe((!visible.is_empty()).then_some(Message::CheckAll));

        let heading: Element<'_, Message> = if checked > 0 {
            row![
                select_all,
                badge(format!("{checked} selected")),
                space::horizontal(),
                button("Pause")
                    .icon(lucide!(Pause))
                    .variant(Variant::Outline)
                    .on_press(Message::PauseSelected),
                button("Delete")
                    .icon(lucide!(Trash))
                    .variant(Variant::Destructive)
                    .on_press(Message::DeleteSelected),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .into()
        } else if narrow {
            row![select_all, caption("Select all")]
                .spacing(10)
                .align_y(Alignment::Center)
                .into()
        } else {
            row![
                select_all,
                caption("Job").width(Length::FillPortion(4)),
                caption("Owner").width(Length::FillPortion(2)),
                caption("Queue").width(80),
                caption("Status").width(100),
                caption("Progress").width(Length::FillPortion(3)),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into()
        };

        let rows: Element<'_, Message> = if visible.is_empty() {
            container(muted(
                "No jobs match. Clear the search or pick another status.",
            ))
            .padding(24)
            .center_x(Length::Fill)
            .into()
        } else {
            column(visible.into_iter().map(|job| self.job_row(job, narrow)))
                .spacing(2)
                .into()
        };

        column![
            toolbar,
            container(heading)
                .padding([4, 8])
                .height(36)
                .align_y(Alignment::Center),
            scroll_area(rows).width(Length::Fill).height(Length::Fill),
            caption(
                "Right-click a job, or press Shift+F10, for more. Up and Down move, Space ticks."
            ),
        ]
        .spacing(8)
        .padding(iced::Padding {
            bottom: 12.0,
            ..iced::Padding::ZERO
        })
        .height(Length::Fill)
        .into()
    }

    fn job_row<'a>(&'a self, job: &'a Job, narrow: bool) -> Element<'a, Message> {
        let id = job.id;
        let tick = checkbox(job.checked).on_toggle(move |checked| Message::Check(id, checked));
        let name = column![
            text(&job.name).size(text_size::SM).font(semibold()),
            caption(format!("{id}  {}", job.source)),
        ]
        .spacing(2);
        let status = badge(job.status.label()).variant(job.status.badge());

        let content: Element<'a, Message> = if narrow {
            let mut details = column![
                row![name.width(Length::Fill), status]
                    .spacing(8)
                    .align_y(Alignment::Center),
                caption(format!("{} - {} queue", job.owner, job.queue)),
            ]
            .spacing(6)
            .width(Length::Fill);
            if matches!(job.status, Status::Running | Status::Paused) {
                details = details.push(self.progress_cell(job));
            }
            row![tick, details].spacing(12).into()
        } else {
            row![
                tick,
                name.width(Length::FillPortion(4)),
                muted(job.owner).width(Length::FillPortion(2)),
                container(
                    badge(job.queue.to_string())
                        .variant(iced_cube::feedback::badge::Variant::Outline)
                )
                .width(80),
                container(status).width(100),
                container(self.progress_cell(job)).width(Length::FillPortion(3)),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into()
        };

        let current = self.cursor == Some(id);
        let padding = if self.compact { [4, 8] } else { [10, 8] };
        let row = mouse_area(
            container(content)
                .padding(padding)
                .width(Length::Fill)
                .style(move |theme| {
                    let tokens = Tokens::of(theme);
                    container::Style {
                        background: current.then(|| tokens.accent.into()),
                        border: Border {
                            color: if current {
                                tokens.border
                            } else {
                                iced::Color::TRANSPARENT
                            },
                            width: 1.0,
                            radius: radius::MD.into(),
                        },
                        ..container::Style::default()
                    }
                }),
        )
        .on_press(Message::Cursor(id));

        // Only the row whose menu is open shows it; the others share a closed state.
        let state = if self.menu_job == Some(id) {
            &self.row_menu
        } else {
            &self.idle_menu
        };
        context_menu(state, row)
            .keymap(self.keys.row_menu.clone())
            .on_event(move |event| Message::RowMenu(id, event))
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
            _ => caption(job.detail()).into(),
        }
    }
}
