use iced::widget::row;
use iced::{Alignment, Element};
use iced_cube::feedback::badge::Variant;
use iced_cube::{badge, lucide};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        row![
            badge("Verified")
                .icon(lucide!(BadgeCheck))
                .variant(Variant::Secondary),
            badge("Deployed")
                .icon(lucide!(CircleCheck))
                .variant(Variant::Success),
            badge("Degraded")
                .icon(lucide!(TriangleAlert))
                .variant(Variant::Warning),
            badge("Offline")
                .icon(lucide!(CircleX))
                .variant(Variant::Destructive),
            badge("v2.4.0").icon(lucide!(Tag)).variant(Variant::Outline),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap()
        .into()
    }
}
