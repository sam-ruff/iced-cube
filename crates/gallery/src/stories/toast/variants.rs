use iced::widget::row;
use iced::{Element, Subscription};
use iced_cube::button;
use iced_cube::overlay::toast::{self, Variant, toast, toasts};
use iced_cube::primitives::button::Variant as ButtonVariant;

#[derive(Debug, Clone)]
pub enum Message {
    Show(Variant),
    Toast(toast::Event),
}

#[derive(Debug)]
pub struct Example {
    toasts: toast::State,
}

impl Default for Example {
    fn default() -> Self {
        let mut toasts = toast::State::new();
        toasts.push(toast("Event created").description("Sunday, 4 October at 9:00"));
        Self { toasts }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Show(variant) => {
                let toast = match variant {
                    Variant::Default => {
                        toast("Event created").description("Sunday, 4 October at 9:00")
                    }
                    Variant::Success => {
                        toast("Profile saved").description("Your changes are live.")
                    }
                    Variant::Destructive => {
                        toast("Payment declined").description("Check your card details.")
                    }
                };
                self.toasts.push(toast.variant(variant));
            }
            Message::Toast(event) => {
                let _ = self.toasts.update(event);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let buttons = row(Variant::ALL.map(|variant| {
            button(format!("{variant:?}"))
                .variant(ButtonVariant::Outline)
                .on_press(Message::Show(variant))
                .into()
        }))
        .spacing(8)
        .wrap();

        toasts(&self.toasts, buttons)
            .on_event(Message::Toast)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        toast::timer(&self.toasts).map(Message::Toast)
    }
}
