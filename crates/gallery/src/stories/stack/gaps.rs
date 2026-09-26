use iced::widget::text;
use iced::{Alignment, Element};
use iced_cube::feedback::badge::Variant;
use iced_cube::layout::stack::Gap;
use iced_cube::{badge, hstack, vstack};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        let rows = [Gap::Xs, Gap::Sm, Gap::Md, Gap::Lg, Gap::Xl].map(|gap| {
            let items = ["One", "Two", "Three", "Four"]
                .map(|label| badge(label).variant(Variant::Secondary).into());

            hstack([
                text(format!("{gap:?}")).size(14).width(40).into(),
                hstack(items).gap(gap).into(),
            ])
            .align(Alignment::Center)
            .into()
        });

        vstack(rows).gap(Gap::Md).into()
    }
}
