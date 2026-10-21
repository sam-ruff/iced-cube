use iced::widget::{column, text};
use iced::{Alignment, Element};
use iced_cube::navigation::pagination::{self, State, pagination};

const RESULTS: usize = 386;
const PER_PAGE: usize = 20;

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
            pages: State::for_items(RESULTS, PER_PAGE).with_page(6),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Page(event) = message;
        let _ = self.pages.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let shown = self.pages.range(RESULTS, PER_PAGE);
        let summary = format!("Results {} to {} of {RESULTS}", shown.start + 1, shown.end);

        column![
            pagination(&self.pages).on_event(Message::Page),
            text(summary).size(14),
        ]
        .spacing(12)
        .align_x(Alignment::Center)
        .into()
    }
}
