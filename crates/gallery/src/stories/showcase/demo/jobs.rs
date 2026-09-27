//! The job list: a search input, a status select, selectable rows with a
//! context menu each, and bulk actions for the checked rows.

use iced::widget::{column, container, mouse_area, row, scrollable, space, stack, text};
use iced::{Alignment, Border, Element, Length, Padding};
use iced_cube::context_menu;
use iced_cube::dropdown_menu::{Entry, item, radio_item, separator, submenu};
use iced_cube::feedback::progress::{Size as ProgressSize, Variant as ProgressVariant};
use iced_cube::feedback::spinner::Size as SpinnerSize;
use iced_cube::primitives::button::{Size as ButtonSize, Variant};
use iced_cube::primitives::checkbox::CheckState;
use iced_cube::primitives::icon_button::Variant as IconVariant;
use iced_cube::primitives::scroll_area;
use iced_cube::theme::{Tokens, radius, semibold, text_size};
use iced_cube::{
    badge, button, checkbox, icon, icon_button, input, lucide, progress, select, spinner, tooltip,
};

use super::data::{Job, Status, StatusFilter};
use super::{Example, Message, RowAction, caption, count, muted};

/// Room kept beside the rows for the scrollbar, whether it shows or not, so
/// the columns never shift when the list grows long enough to scroll.
const GUTTER: f32 = scroll_area::THICKNESS + 8.0;
/// Height of the progress line on a narrow row, whatever it shows.
const PROGRESS_LINE: f32 = 30.0;

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
        let checked: Vec<&Job> = visible.iter().copied().filter(|job| job.checked).collect();
        let all = match checked.len() {
            0 => CheckState::Unchecked,
            n if n == visible.len() => CheckState::Checked,
            _ => CheckState::Indeterminate,
        };
        let select_all =
            checkbox(all).on_toggle_maybe((!visible.is_empty()).then_some(Message::CheckAll));
        let any_running = checked.iter().any(|job| job.status == Status::Running);

        let heading: Element<'_, Message> = if !checked.is_empty() {
            row![
                select_all,
                badge(format!("{} selected", checked.len())),
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
                .padding(Padding::ZERO.right(GUTTER))
                .into()
        };
        let list = scrollable::Scrollable::with_direction(
            rows,
            scrollable::Direction::Vertical(scroll_area::floating_scrollbar()),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme, status| scroll_area::style(&Tokens::of(theme), status));

        // New jobs wait behind a pill while the pointer is over the list, so
        // nothing moves under it.
        let mut layers = stack![list].width(Length::Fill).height(Length::Fill);
        let fresh = self.fresh_shown();
        if fresh > 0 {
            layers = layers.push(
                container(
                    button(format!("Show {}", count(fresh, "new job")))
                        .icon(lucide!(ArrowUp))
                        .size(ButtonSize::Sm)
                        .on_press(Message::ShowNew),
                )
                .padding(8)
                .center_x(Length::Fill),
            );
        }
        let list = mouse_area(layers)
            .on_enter(Message::ListHover(true))
            .on_exit(Message::ListHover(false));

        let hint = if narrow {
            "Tap and hold a job, or tap its menu button, for more."
        } else {
            "Right-click a job, or press Shift+F10, for more. Up and Down move, Space ticks."
        };

        column![
            toolbar,
            container(heading)
                .padding(Padding {
                    top: 4.0,
                    right: 8.0 + GUTTER,
                    bottom: 4.0,
                    left: 8.0,
                })
                .height(36)
                .align_y(Alignment::Center),
            list,
            caption(hint),
        ]
        .spacing(8)
        .padding(Padding {
            bottom: 12.0,
            ..Padding::ZERO
        })
        .height(Length::Fill)
        .into()
    }

    fn job_row<'a>(&'a self, job: &'a Job, narrow: bool) -> Element<'a, Message> {
        let id = job.id;
        let tick = checkbox(job.checked).on_toggle(move |checked| Message::Check(id, checked));
        let mut title = row![text(&job.name).size(text_size::SM).font(semibold())]
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
        let name = column![title, caption(format!("{id}  {}", job.source))].spacing(2);
        let status = badge(job.status.label()).variant(job.status.badge());

        let content: Element<'a, Message> = if narrow {
            // Every row has the same three lines, so a row that changes
            // status never moves the rows below it.
            let details = column![
                row![name.width(Length::Fill), status]
                    .spacing(8)
                    .align_y(Alignment::Center),
                caption(format!("{} - {} queue", job.owner, job.queue)),
                container(self.progress_cell(job))
                    .height(PROGRESS_LINE)
                    .align_y(Alignment::Center),
            ]
            .spacing(6)
            .width(Length::Fill);
            let actions = icon_button(lucide!(EllipsisVertical))
                .label("Job actions")
                .tooltip(None)
                .on_press(Message::RowMenu(context_menu::Event::OpenFromKeyboard(id)));
            row![tick, details, actions].spacing(12).into()
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

        context_menu::keyed(&self.row_menu, id, row)
            .keymap(self.keys.row_menu.clone())
            .on_event(Message::RowMenu)
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
