use iced::Element;
use iced::widget::{column, row};
use iced_cube::primitives::checkbox;
use iced_cube::primitives::checkbox::CheckState;

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
            checkbox(CheckState::Unchecked)
                .label("Unchecked")
                .on_toggle(|_| Message::Toggled),
            checkbox(CheckState::Checked)
                .label("Checked")
                .on_toggle(|_| Message::Toggled),
            checkbox(CheckState::Indeterminate)
                .label("Indeterminate")
                .on_toggle(|_| Message::Toggled),
        ]
        .spacing(12);

        let disabled = column![
            checkbox(CheckState::Unchecked).label("Disabled"),
            checkbox(CheckState::Checked).label("Disabled"),
            checkbox(CheckState::Indeterminate).label("Disabled"),
        ]
        .spacing(12);

        row![enabled, disabled].spacing(48).into()
    }
}
