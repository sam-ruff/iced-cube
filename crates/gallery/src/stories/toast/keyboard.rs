use iced::widget::{column, text};
use iced::{Element, Subscription};
use iced_cube::button;
use iced_cube::keys::{self, Keymap};
use iced_cube::overlay::toast::{self, Action, toast, toasts};
use iced_cube::primitives::button::Variant;

#[derive(Debug, Clone)]
pub enum Message {
    Notify,
    Toast(toast::Event),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    toasts: toast::State,
    keymap: Keymap<Action>,
    sent: u32,
}

impl Default for Example {
    fn default() -> Self {
        let mut toasts = toast::State::new();
        toasts.push(toast("Press Escape to close me").persistent());
        Self {
            toasts,
            keymap: toast::default_keymap(),
            sent: 0,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Notify => {
                self.sent += 1;
                self.toasts
                    .push(toast(format!("Message {} sent", self.sent)).persistent());
            }
            Message::Toast(event) => {
                let _ = self.toasts.update(event);
            }
            Message::Key(key) => {
                let Some(action) = self.keymap.resolve_event(&key) else {
                    return;
                };
                for event in action.events(&self.toasts) {
                    let _ = self.toasts.update(event);
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = column![
            button("Send message")
                .variant(Variant::Outline)
                .on_press(Message::Notify),
            text("Escape closes the newest toast, Shift+Escape closes them all.").size(14),
        ]
        .spacing(12);

        toasts(&self.toasts, content)
            .on_event(Message::Toast)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            toast::timer(&self.toasts).map(Message::Toast),
            keys::subscription().map(Message::Key),
        ])
    }
}
