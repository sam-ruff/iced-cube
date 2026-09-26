//! The live log: lines stream in from the worker through the channel and
//! can be paused, filtered by level and searched.

use iced::widget::{column, container, row, space, text};
use iced::{Alignment, Border, Element, Length};
use iced_cube::feedback::badge::Variant as BadgeVariant;
use iced_cube::feedback::spinner::Size as SpinnerSize;
use iced_cube::primitives::radio::Direction;
use iced_cube::theme::{Tokens, radius, text_size};
use iced_cube::{badge, input, lucide, radio_group, scroll_area, spinner, switch};

use super::data::{LevelFilter, LogLine, SERVICES, clock};
use super::{Example, Message, caption, muted};

impl Example {
    pub(super) fn activity(&self, narrow: bool) -> Element<'_, Message> {
        let status: Element<'_, Message> = if self.live {
            row![
                spinner(self.phase).size(SpinnerSize::Sm),
                muted(format!("Streaming from {} services", SERVICES.len()))
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        } else {
            row![
                badge("Paused").variant(BadgeVariant::Warning),
                muted(format!("{} new lines waiting", self.held.len()))
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        };

        let toolbar = row![
            status,
            space::horizontal(),
            switch(self.live).label("Live").on_toggle(Message::Live),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let filters = row![
            radio_group(LevelFilter::ALL, Some(self.level))
                .direction(Direction::Horizontal)
                .on_select(Message::Level),
            input("Filter lines", &self.log_query)
                .icon(lucide!(Search))
                .width(if narrow {
                    Length::Fill
                } else {
                    Length::Fixed(240.0)
                })
                .on_input(Message::LogQuery),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .wrap()
        .vertical_spacing(10);

        let query = self.log_query.trim().to_lowercase();
        let lines: Vec<Element<'_, Message>> = self
            .logs
            .iter()
            .filter(|(_, line)| self.level.allows(line.level))
            .filter(|(_, line)| {
                query.is_empty()
                    || line.message.to_lowercase().contains(&query)
                    || line.service.contains(&query)
            })
            .map(|(time, line)| log_row(*time, line, narrow))
            .collect();
        let list: Element<'_, Message> = if lines.is_empty() {
            container(muted("No lines match the filters yet."))
                .padding(24)
                .center_x(Length::Fill)
                .into()
        } else {
            column(lines).spacing(2).padding(8).into()
        };

        column![
            toolbar,
            filters,
            container(scroll_area(list).width(Length::Fill).height(Length::Fill))
                .height(Length::Fill)
                .style(|theme| {
                    let tokens = Tokens::of(theme);
                    container::Style {
                        border: Border {
                            color: tokens.border,
                            width: 1.0,
                            radius: radius::LG.into(),
                        },
                        ..container::Style::default()
                    }
                }),
        ]
        .spacing(12)
        .padding(iced::Padding {
            bottom: 16.0,
            ..iced::Padding::ZERO
        })
        .height(Length::Fill)
        .into()
    }
}

fn log_row<'a>(time: u32, line: &'a LogLine, narrow: bool) -> Element<'a, Message> {
    let level = badge(line.level.label()).variant(line.level.badge());
    if narrow {
        return column![
            row![
                level,
                caption(line.service),
                space::horizontal(),
                caption(clock(time))
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text(&line.message).size(text_size::SM),
        ]
        .spacing(4)
        .padding([6, 4])
        .into();
    }
    row![
        caption(clock(time)).width(64),
        container(level).width(56),
        caption(line.service).width(96),
        text(&line.message).size(text_size::SM).width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([4, 4])
    .into()
}
