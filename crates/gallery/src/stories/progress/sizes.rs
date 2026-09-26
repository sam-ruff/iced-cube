use iced::Element;
use iced::widget::column;
use iced_cube::feedback::progress::Size;
use iced_cube::progress;

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        column![
            progress(0.25).size(Size::Sm).label("Small"),
            progress(0.5).size(Size::Md).label("Medium"),
            progress(0.75).size(Size::Lg).label("Large"),
        ]
        .spacing(20)
        .width(360)
        .into()
    }
}
