//! The overview: stat cards, a status alert, running jobs, regional load
//! and runbooks.

use iced::widget::{column, row, space, text};
use iced::{Alignment, Element, Length};
use iced_cube::feedback::alert::Variant as AlertVariant;
use iced_cube::feedback::badge::Variant as BadgeVariant;
use iced_cube::feedback::progress::{Size as ProgressSize, Variant as ProgressVariant};
use iced_cube::feedback::spinner::Size as SpinnerSize;
use iced_cube::layout::accordion::accordion;
use iced_cube::layout::stack::Gap;
use iced_cube::navigation::tabs::{self, Variant as TabsVariant};
use iced_cube::primitives::button::{Size as ButtonSize, Variant};
use iced_cube::theme::{semibold, text_size};
use iced_cube::{
    Glyph, alert, badge, button, card, hstack, icon, lucide, progress, scroll_area, spinner,
    tooltip, vstack,
};

use super::data::{Range, Region, Runbook, Status};
use super::{Cmd, Example, Message, Page, caption, muted};

const GAP: f32 = 16.0;
const MIN_STAT: f32 = 160.0;
const SCROLLBAR: f32 = 16.0;

impl Example {
    pub(super) fn overview(&self, width: f32) -> Element<'_, Message> {
        // Leaves room for the scroll area's scrollbar beside the content.
        let width = width - SCROLLBAR;
        let count = |status: Status| self.jobs.iter().filter(|job| job.status == status).count();
        let failed = count(Status::Failed);

        // Four, two or one to a row, so no card is left on its own.
        let fits = ((width + GAP) / (MIN_STAT + GAP)).floor();
        let columns = [4.0, 2.0, 1.0]
            .into_iter()
            .find(|columns| *columns <= fits)
            .unwrap_or(1.0);
        let stat_width = ((width - GAP * (columns - 1.0)) / columns).floor();
        let stats = row![
            stat(
                "Running",
                lucide!(Loader),
                count(Status::Running),
                badge(format!("{} slots", self.settings.concurrency))
                    .variant(BadgeVariant::Secondary)
                    .into(),
                stat_width,
            ),
            stat(
                "Queued",
                lucide!(Clock),
                count(Status::Queued),
                badge(format!("{} paused", count(Status::Paused)))
                    .variant(BadgeVariant::Outline)
                    .into(),
                stat_width,
            ),
            stat(
                "Failed",
                lucide!(CircleX),
                failed,
                if failed > 0 {
                    badge("Needs attention").variant(BadgeVariant::Destructive)
                } else {
                    badge("All clear").variant(BadgeVariant::Success)
                }
                .into(),
                stat_width,
            ),
            stat(
                "Succeeded",
                lucide!(CircleCheck),
                count(Status::Succeeded) + 214,
                tooltip(
                    badge("+12%")
                        .variant(BadgeVariant::Success)
                        .icon(lucide!(TrendingUp)),
                    "Compared with this time yesterday",
                )
                .into(),
                stat_width,
            ),
        ]
        .spacing(GAP)
        .wrap()
        .vertical_spacing(GAP);

        let status = if failed > 0 {
            let jobs = if failed == 1 { "job" } else { "jobs" };
            alert(format!("{failed} {jobs} failed in the last hour"))
                .variant(AlertVariant::Destructive)
                .description("Most failures ran out of memory. Retry them from the job list or the command palette.")
        } else {
            alert("Warehouse sync is running slowly")
                .variant(AlertVariant::Warning)
                .description("Queries on stock_levels take up to 12 seconds. The runbook below has the workaround.")
        }
        .width(Length::Fill);

        // Two cards side by side when there is room, stacked otherwise.
        let pair: Element<'_, Message> = if width >= 720.0 {
            let half = ((width - GAP) / 2.0).floor();
            row![self.running_card(half), self.capacity_card(half)]
                .spacing(GAP)
                .into()
        } else {
            column![self.running_card(width), self.capacity_card(width)]
                .spacing(GAP)
                .into()
        };

        let runbooks = card()
            .title("Runbooks")
            .description("What to do when things go wrong. Up and Down move between them.")
            .body(
                accordion(&self.runbooks)
                    .item(
                        Runbook::Degraded,
                        "Warehouse sync is degraded",
                        column![
                            muted("Check the stock_levels query plan, then move heavy jobs to the Low queue until it recovers."),
                            button("Show running jobs")
                                .variant(Variant::Outline)
                                .size(ButtonSize::Sm)
                                .on_press(Message::Run(Cmd::RunningJobs)),
                        ]
                        .spacing(10),
                    )
                    .item(
                        Runbook::Retry,
                        "Retrying a failed job",
                        muted("Right-click the job and choose Retry. A retried job starts from the beginning in its original queue."),
                    )
                    .item(
                        Runbook::Escalation,
                        "Who to call",
                        muted("Data platform on call: Tom Okafor. Warehouse: Mei Lin. Anything customer facing: Priya Shah."),
                    )
                    .on_event(Message::Runbooks),
            )
            .width(Length::Fill);

        scroll_area(
            vstack([stats.into(), status.into(), pair, runbooks.into()])
                .gap(Gap::Lg)
                .padding(iced::Padding {
                    bottom: 24.0,
                    ..iced::Padding::ZERO
                })
                .width(Length::Fixed(width)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn running_card(&self, width: f32) -> Element<'_, Message> {
        let running: Vec<Element<'_, Message>> = self
            .jobs
            .iter()
            .filter(|job| job.status == Status::Running)
            .take(4)
            .map(|job| {
                column![
                    row![
                        spinner(self.phase).size(SpinnerSize::Sm),
                        text(&job.name)
                            .size(text_size::SM)
                            .font(semibold())
                            .width(Length::Fill),
                        caption(job.owner),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    progress(job.progress)
                        .size(ProgressSize::Sm)
                        .show_percentage(true),
                ]
                .spacing(6)
                .into()
            })
            .collect();
        let body: Element<'_, Message> = if running.is_empty() {
            muted("Nothing is running. Queued jobs start as soon as a slot is free.").into()
        } else {
            column(running).spacing(14).into()
        };

        iced_cube::card()
            .title("Running now")
            .description("Progress streams in from the worker.")
            .body(body)
            .footer(
                button("View all jobs")
                    .variant(Variant::Link)
                    .trailing_icon(lucide!(ArrowRight))
                    .on_press(Message::Tabs(tabs::Event::Select(Page::Jobs))),
            )
            .width(width)
            .into()
    }

    fn capacity_card(&self, width: f32) -> Element<'_, Message> {
        let range = self.range.selected().unwrap_or(Range::Day);
        let bars = column(Region::ALL.iter().map(|region| {
            let load = region.load(range);
            let variant = if load > 0.85 {
                ProgressVariant::Destructive
            } else if load > 0.7 {
                ProgressVariant::Warning
            } else {
                ProgressVariant::Default
            };
            progress(load)
                .label(region.to_string())
                .show_percentage(true)
                .variant(variant)
                .into()
        }))
        .spacing(14);

        card()
            .title("Capacity by region")
            .description("Share of worker capacity in use.")
            .body(
                column![
                    tabs::tabs(&self.range)
                        .variant(TabsVariant::Pills)
                        .on_event(Message::Range),
                    bars,
                ]
                .spacing(16),
            )
            .width(width)
            .into()
    }
}

/// One figure with its label, icon and a badge underneath.
fn stat<'a>(
    title: &'a str,
    glyph: Glyph,
    value: usize,
    note: Element<'a, Message>,
    width: f32,
) -> Element<'a, Message> {
    card()
        .body(
            vstack([
                hstack([
                    muted(title).into(),
                    space::horizontal().into(),
                    icon::icon(glyph, 16.0).into(),
                ])
                .align(Alignment::Center)
                .width(Length::Fill)
                .into(),
                text(value.to_string()).size(28).font(semibold()).into(),
                note,
            ])
            .gap(Gap::Sm)
            .width(Length::Fill),
        )
        .width(width)
        .into()
}
