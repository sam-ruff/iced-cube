use iced::widget::{row, text};
use iced::{Element, Length};
use iced_cube::card;
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    Later,
    Upgrade,
}

#[derive(Debug, Default)]
pub struct Example {
    upgraded: bool,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Later => self.upgraded = false,
            Message::Upgrade => self.upgraded = true,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let body = if self.upgraded {
            "You are on the Pro plan. Thanks for your support."
        } else {
            "You have used 9.2 GB of your 10 GB. Upgrade for 100 GB and priority support."
        };

        card()
            .title("Storage almost full")
            .description("Personal workspace")
            .body(text(body).size(14).width(Length::Fill))
            .footer(
                row![
                    button("Maybe later")
                        .variant(Variant::Outline)
                        .on_press(Message::Later),
                    button("Upgrade")
                        .icon(lucide!(Sparkles))
                        .on_press_maybe((!self.upgraded).then_some(Message::Upgrade)),
                ]
                .spacing(8),
            )
            .width(380)
            .into()
    }
}
