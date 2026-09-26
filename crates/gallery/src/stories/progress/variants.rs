use iced::Element;
use iced::widget::column;
use iced_cube::feedback::progress::Variant;
use iced_cube::progress;

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        column![
            progress(0.62)
                .label("Uploading photos")
                .show_percentage(true),
            progress(1.0)
                .variant(Variant::Success)
                .label("Backup complete")
                .show_percentage(true),
            progress(0.87)
                .variant(Variant::Warning)
                .label("Disk usage")
                .show_percentage(true),
            progress(0.34)
                .variant(Variant::Destructive)
                .label("Import stopped")
                .show_percentage(true),
        ]
        .spacing(16)
        .width(360)
        .into()
    }
}
