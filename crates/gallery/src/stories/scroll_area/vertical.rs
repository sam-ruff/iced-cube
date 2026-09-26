use iced::widget::{column, rule, text};
use iced::{Element, Length};
use iced_cube::{card, scroll_area};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        let tags = column((1..=40).flat_map(|i| {
            [
                text(format!("v1.{i}.0")).size(14).into(),
                rule::horizontal(1).into(),
            ]
        }))
        .spacing(8)
        .width(Length::Fill);

        card()
            .title("Releases")
            .body(scroll_area(tags).height(150).width(Length::Fill))
            .width(240)
            .into()
    }
}
