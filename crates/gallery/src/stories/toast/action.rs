use iced::widget::{column, text};
use iced::{Element, Subscription};
use iced_cube::keys::{self, Keymap};
use iced_cube::overlay::toast::{self, Action, Position, toast, toasts};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    Archive,
    Unarchive,
    Toast(toast::Event),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    inbox: u32,
    // Undo buttons hand back the message that reverses the archive.
    toasts: toast::State<Message>,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        let mut toasts = toast::State::default();
        toasts.push_with(archived().persistent(), Message::Unarchive);
        Self {
            inbox: 11,
            toasts,
            keymap: toast::default_keymap(),
        }
    }
}

// A toast with an action stays at least ten seconds, and pauses while the
// pointer is over it.
fn archived() -> toast::Toast {
    toast("Conversation archived").action("Undo")
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Archive => {
                self.inbox -= 1;
                self.toasts.push_with(archived(), Message::Unarchive);
            }
            Message::Unarchive => self.inbox += 1,
            Message::Toast(event) => {
                if let Some(toast::Output::Payload(message)) = self.toasts.update(event) {
                    self.update(message);
                }
            }
            // Alt+Z presses Undo on the newest toast.
            Message::Key(key) => {
                let Some(action) = self.keymap.resolve_event(&key) else {
                    return;
                };
                for event in action.events(&self.toasts) {
                    self.update(Message::Toast(event));
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let archive = (self.inbox > 0).then_some(Message::Archive);

        let content = column![
            text(format!("{} conversations in your inbox", self.inbox)).size(14),
            button("Archive")
                .icon(lucide!(Archive))
                .variant(Variant::Secondary)
                .on_press_maybe(archive),
            text("Alt+Z undoes the newest archive.").size(14),
        ]
        .spacing(12);

        toasts(&self.toasts, content)
            .position(Position::TopRight)
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
