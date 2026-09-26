use iced::{Alignment, Element, Length};
use iced_cube::layout::stack::Gap;
use iced_cube::{badge, card, hstack, vstack};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        let panel = |title: &'static str, align: Alignment| -> Element<'_, Message> {
            card()
                .title(title)
                .body(
                    vstack([
                        badge("Short").into(),
                        badge("A longer label").into(),
                        badge("Medium one").into(),
                    ])
                    .gap(Gap::Sm)
                    .align(align)
                    .width(Length::Fill),
                )
                .width(Length::Fill)
                .into()
        };

        hstack([
            panel("Start", Alignment::Start),
            panel("Centre", Alignment::Center),
            panel("End", Alignment::End),
        ])
        .gap(Gap::Lg)
        .width(600)
        .into()
    }
}
