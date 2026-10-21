use iced::widget::{column, row, space, text};
use iced::{Alignment, Element, Length};
use iced_cube::navigation::pagination::{self, State, Variant, pagination};
use iced_cube::primitives::button::Size;

const INVOICES: [(&str, &str); 11] = [
    ("INV-1041", "£120.00"),
    ("INV-1042", "£86.50"),
    ("INV-1043", "£240.00"),
    ("INV-1044", "£18.99"),
    ("INV-1045", "£302.10"),
    ("INV-1046", "£75.00"),
    ("INV-1047", "£64.20"),
    ("INV-1048", "£150.00"),
    ("INV-1049", "£9.99"),
    ("INV-1050", "£410.00"),
    ("INV-1051", "£33.40"),
];
const PER_PAGE: usize = 4;

#[derive(Debug, Clone)]
pub enum Message {
    Page(pagination::Event),
}

#[derive(Debug)]
pub struct Example {
    pages: State,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            pages: State::for_items(INVOICES.len(), PER_PAGE),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Page(event) = message;
        let _ = self.pages.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let rows = INVOICES[self.pages.range(INVOICES.len(), PER_PAGE)]
            .iter()
            .map(|(number, amount)| {
                row![
                    text(*number).size(14),
                    space::horizontal(),
                    text(*amount).size(14),
                ]
                .height(36)
                .align_y(Alignment::Center)
                .into()
            });

        column![
            column(rows).height(Length::Fixed(4.0 * 36.0)),
            pagination(&self.pages)
                .variant(Variant::Compact)
                .size(Size::Sm)
                .on_event(Message::Page),
        ]
        .spacing(12)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .max_width(300)
        .into()
    }
}
