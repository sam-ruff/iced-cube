use iced::widget::{row, text};
use iced::{Alignment, Element};
use iced_cube::button;
use iced_cube::overlay::dialog::dialog;
use iced_cube::primitives::button::Variant;

#[derive(Debug, Clone)]
pub enum Message {
    Open,
    Close,
    Publish,
}

#[derive(Debug)]
pub struct Example {
    open: bool,
    published: bool,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            open: true,
            published: false,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Open => self.open = true,
            Message::Close => self.open = false,
            Message::Publish => {
                self.published = true;
                self.open = false;
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status = if self.published {
            "The post is live."
        } else {
            "Draft saved two minutes ago."
        };
        let base = row![
            button("Publish post")
                .variant(Variant::Outline)
                .on_press(Message::Open),
            text(status).size(14),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        dialog(base)
            .open(self.open)
            .title("Publish this post?")
            .description("Subscribers get an email as soon as it goes live.")
            .action(
                button("Cancel")
                    .variant(Variant::Outline)
                    .on_press(Message::Close),
            )
            .action(button("Publish").on_press(Message::Publish))
            .on_dismiss(Message::Close)
            .into()
    }
}
