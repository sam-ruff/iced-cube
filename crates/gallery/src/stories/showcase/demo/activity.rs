//! The live log: lines stream in from the worker through the log viewer's
//! channel, and the viewer's own toolbar pauses, filters and searches them.

use iced::widget::{column, row, space, text};
use iced::{Alignment, Element, Length, Task};
use iced_cube::data::log_viewer::{self, Output, log_viewer};
use iced_cube::dropdown_menu;
use iced_cube::feedback::badge::Variant as BadgeVariant;
use iced_cube::feedback::spinner::Size as SpinnerSize;
use iced_cube::{badge, context_menu, keys, spinner};

use super::data::SERVICES;
use super::{Example, Message, count, muted};
use crate::bridge;

impl Example {
    pub(super) fn activity(&self) -> Element<'_, Message> {
        let status: Element<'_, Message> = if self.logs.is_paused() {
            row![
                badge("Paused").variant(BadgeVariant::Warning),
                muted(format!(
                    "{} waiting",
                    count(self.logs.pending(), "new line")
                ))
                .wrapping(text::Wrapping::None),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        } else {
            row![
                spinner(self.phase).size(SpinnerSize::Sm),
                muted(format!("Streaming from {} services", SERVICES.len()))
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        };

        column![
            row![status, space::horizontal()].align_y(Alignment::Center),
            log_viewer(&self.logs)
                .placeholder("Search messages")
                .empty("No lines match the filters yet.")
                .keymap(self.keys.log.clone())
                .context_menu(&self.log_menu, Message::LogMenu)
                .height(Length::Fill)
                .on_event(Message::Logs),
        ]
        .spacing(12)
        .padding(iced::Padding {
            bottom: 16.0,
            ..iced::Padding::ZERO
        })
        .height(Length::Fill)
        .into()
    }

    pub(super) fn logs_event(&mut self, event: log_viewer::Event) -> Task<Message> {
        match self.logs.update(event) {
            Some(Output::Ready(sender)) => {
                self.log_sender = Some(sender);
                Task::none()
            }
            Some(Output::Copy(text)) => bridge::copy(text),
            Some(Output::FocusSearch) => log_viewer::focus_search(&self.logs),
            None => Task::none(),
        }
    }

    pub(super) fn log_menu_event(
        &mut self,
        event: context_menu::Event<log_viewer::MenuItem, log_viewer::LineId>,
    ) -> Task<Message> {
        let chosen = self.log_menu.update(event);
        match (chosen, self.log_menu.target()) {
            (Some(dropdown_menu::Output::Activated(item)), Some(&id)) => {
                self.logs_event(item.event(id))
            }
            _ => Task::none(),
        }
    }

    /// Keys nothing else used, while the log is on screen but not focused.
    /// A focused log resolves the same keymap itself.
    pub(super) fn activity_key(&mut self, key: &keys::Event) -> Option<Task<Message>> {
        let event = self.logs.key_event(&self.keys.log, key)?;
        Some(self.logs_event(event))
    }
}
