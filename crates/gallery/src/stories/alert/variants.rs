use iced::Element;
use iced::widget::row;
use iced_cube::alert;
use iced_cube::feedback::alert::Variant;

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

const ALERTS: [(&str, &str, Variant); 4] = [
    (
        "New version available",
        "Restart to update to 2.4.",
        Variant::Info,
    ),
    (
        "Backup complete",
        "All 1,284 files were saved.",
        Variant::Success,
    ),
    (
        "Storage almost full",
        "You have 312 MB left.",
        Variant::Warning,
    ),
    (
        "Sync failed",
        "Check your connection and retry.",
        Variant::Destructive,
    ),
];

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        row(ALERTS.map(|(title, description, variant)| {
            alert(title)
                .description(description)
                .variant(variant)
                .width(276)
                .into()
        }))
        .spacing(12)
        .wrap()
        .into()
    }
}
