use iced::Element;
use iced::widget::{column, row};
use iced_cube::primitives::switch;

#[derive(Debug, Clone)]
pub enum Message {
    Toggled,
}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        let enabled = column![
            switch(false).label("Off").on_toggle(|_| Message::Toggled),
            switch(true).label("On").on_toggle(|_| Message::Toggled),
        ]
        .spacing(16);

        let disabled = column![
            switch(false).label("Disabled off"),
            switch(true).label("Disabled on"),
        ]
        .spacing(16);

        row![enabled, disabled].spacing(48).into()
    }
}
