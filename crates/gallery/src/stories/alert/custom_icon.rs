use iced::Element;
use iced::widget::column;
use iced_cube::feedback::alert::Variant;
use iced_cube::{alert, lucide};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        column![
            alert("Build cache enabled")
                .description(
                    "Repeat builds reuse compiled dependencies, so they finish much faster."
                )
                .icon(lucide!(Terminal)),
            alert("Your session expires in five minutes")
                .icon(lucide!(Clock))
                .variant(Variant::Warning),
        ]
        .spacing(12)
        .width(480)
        .into()
    }
}
