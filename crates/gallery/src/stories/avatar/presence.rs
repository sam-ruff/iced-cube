use iced::widget::{column, row, text};
use iced::{Alignment, Element, Length};
use iced_cube::avatar;
use iced_cube::feedback::avatar::Presence;

const TEAM: [(&str, &str, Presence); 4] = [
    ("Ada Lovelace", "Online", Presence::Online),
    ("Grace Hopper", "Back in ten minutes", Presence::Away),
    ("Alan Turing", "In a meeting", Presence::Busy),
    ("Edsger Dijkstra", "Last seen yesterday", Presence::Offline),
];

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        let members = TEAM.map(|(name, status, presence)| {
            row![
                avatar(name).presence(presence),
                column![text(name).size(14), text(status).size(12)].spacing(2),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into()
        });

        column(members)
            .spacing(12)
            .width(Length::Fill)
            .max_width(280)
            .into()
    }
}
