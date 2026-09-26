use iced::widget::{column, row, text};
use iced::{Element, Font, Length, Padding};
use iced_cube::primitives::scroll_area::Direction;
use iced_cube::{card, scroll_area};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        let grid = column((1..=30).map(|y| {
            row((1..=16).map(|x| {
                text(format!("{:>3}", x * y))
                    .font(Font::MONOSPACE)
                    .size(13)
                    .width(40)
                    .into()
            }))
            .into()
        }))
        .spacing(6)
        // Room for the scrollbars, which float over the edges.
        .padding(Padding::ZERO.right(12).bottom(12));

        card()
            .title("Times table")
            .body(
                scroll_area(grid)
                    .direction(Direction::Both)
                    .width(Length::Fill)
                    .height(150),
            )
            .width(420)
            .into()
    }
}
