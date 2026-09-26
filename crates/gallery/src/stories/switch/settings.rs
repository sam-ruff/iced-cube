use iced::widget::{column, row, space, text};
use iced::{Alignment, Element, Length, Theme};
use iced_cube::primitives::{Switch, switch};
use iced_cube::theme::Tokens;

#[derive(Debug, Clone)]
pub enum Message {
    Notifications(bool),
    Sounds(bool),
}

#[derive(Debug)]
pub struct Example {
    notifications: bool,
    sounds: bool,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            notifications: true,
            sounds: false,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Notifications(on) => self.notifications = on,
            Message::Sounds(on) => self.sounds = on,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let sounds_enabled = self.notifications.then_some(Message::Sounds);

        column![
            setting(
                "Notifications",
                "Show a banner for new messages.",
                switch(self.notifications).on_toggle(Message::Notifications),
            ),
            setting(
                "Sounds",
                "Only available while notifications are on.",
                switch(self.sounds).on_toggle_maybe(sounds_enabled),
            ),
        ]
        .spacing(20)
        .width(360)
        .into()
    }
}

fn setting<'a>(
    title: &'a str,
    detail: &'a str,
    control: Switch<'a, Message>,
) -> Element<'a, Message> {
    let detail = text(detail).size(12).style(|theme: &Theme| text::Style {
        color: Some(Tokens::of(theme).muted_foreground),
    });

    row![
        column![text(title).size(14), detail].spacing(2),
        space().width(Length::Fill),
        control,
    ]
    .align_y(Alignment::Center)
    .into()
}
