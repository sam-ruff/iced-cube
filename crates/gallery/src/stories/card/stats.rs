use iced::Element;
use iced::widget::{column, row, text};
use iced_cube::feedback::badge::Variant;
use iced_cube::{badge, card};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

const STATS: [(&str, &str, &str, Variant); 3] = [
    ("Revenue", "£45,231", "+20.1%", Variant::Success),
    ("Subscribers", "2,350", "+180", Variant::Secondary),
    ("Churn", "3.2%", "+0.4%", Variant::Destructive),
];

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        row(STATS.map(|(label, value, change, variant)| {
            card()
                .description(label)
                .body(
                    column![
                        text(value).size(24),
                        badge(format!("{change} this month")).variant(variant),
                    ]
                    .spacing(8),
                )
                .width(200)
                .into()
        }))
        .spacing(16)
        .wrap()
        .into()
    }
}
