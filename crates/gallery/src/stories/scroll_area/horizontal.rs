use iced::widget::{column, row, text};
use iced::{Element, Length};
use iced_cube::primitives::scroll_area::Direction;
use iced_cube::{card, scroll_area};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

const ARTWORKS: [(&str, &str); 8] = [
    ("Harbour at dawn", "Nia Evans"),
    ("Salt flats", "Priya Shah"),
    ("Glass house", "Owen Hughes"),
    ("Low tide", "Ines Castro"),
    ("Night market", "Kenji Sato"),
    ("First snow", "Mari Lund"),
    ("Orchard", "Ada Okafor"),
    ("Quarry", "Luca Moretti"),
];

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        let items = row(ARTWORKS.map(|(title, artist)| {
            card()
                .body(column![text(title).size(14), text(artist).size(12)].spacing(4))
                .width(160)
                .into()
        }))
        .spacing(12)
        .padding(24);

        scroll_area(items)
            .direction(Direction::Horizontal)
            .width(Length::Fill)
            .into()
    }
}
