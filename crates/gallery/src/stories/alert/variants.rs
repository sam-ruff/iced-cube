use iced::Element;
use iced::widget::{column, row};
use iced_cube::alert;
use iced_cube::feedback::alert::Variant;

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        column![
            row![
                alert("New version available")
                    .description("Restart to update to 2.4.")
                    .variant(Variant::Info),
                alert("Backup complete")
                    .description("All 1,284 files were saved.")
                    .variant(Variant::Success),
            ]
            .spacing(12),
            row![
                alert("Storage almost full")
                    .description("You have 312 MB left.")
                    .variant(Variant::Warning),
                alert("Sync failed")
                    .description("Check your connection and retry.")
                    .variant(Variant::Destructive),
            ]
            .spacing(12),
        ]
        .spacing(12)
        .width(640)
        .into()
    }
}
