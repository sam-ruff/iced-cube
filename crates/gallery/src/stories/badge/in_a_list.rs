use iced::widget::{column, row, space, text};
use iced::{Alignment, Element};
use iced_cube::badge;
use iced_cube::feedback::badge::Variant;

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

const INVOICES: [(&str, &str, Variant); 4] = [
    ("INV-1024", "Paid", Variant::Success),
    ("INV-1025", "Pending", Variant::Secondary),
    ("INV-1026", "Overdue", Variant::Destructive),
    ("INV-1027", "Draft", Variant::Outline),
];

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        column(INVOICES.map(|(number, status, variant)| {
            row![
                text(number).size(14),
                space::horizontal(),
                badge(status).variant(variant),
            ]
            .align_y(Alignment::Center)
            .into()
        }))
        .spacing(12)
        .width(280)
        .into()
    }
}
