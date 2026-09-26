use iced::widget::{row, text};
use iced::{Alignment, Element};
use iced_cube::button;
use iced_cube::overlay::dialog::alert_dialog;
use iced_cube::primitives::button::Variant;

#[derive(Debug, Clone)]
pub enum Message {
    Ask,
    Cancel,
    Delete,
}

#[derive(Debug)]
pub struct Example {
    asking: bool,
    deleted: bool,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            asking: true,
            deleted: false,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Ask => self.asking = true,
            Message::Cancel => self.asking = false,
            Message::Delete => {
                self.deleted = true;
                self.asking = false;
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status = if self.deleted {
            "The project was deleted."
        } else {
            "3 members, 128 files."
        };
        let delete = (!self.deleted).then_some(Message::Ask);
        let base = row![
            button("Delete project")
                .variant(Variant::Destructive)
                .on_press_maybe(delete),
            text(status).size(14),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        alert_dialog(
            base,
            "Delete this project?",
            "This permanently deletes the project and its files. It cannot be undone.",
        )
        .open(self.asking)
        .confirm("Delete project")
        .on_cancel(Message::Cancel)
        .on_confirm(Message::Delete)
        .into()
    }
}
