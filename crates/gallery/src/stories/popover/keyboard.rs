use iced::widget::{column, container, text};
use iced::{Element, Subscription};
use iced_cube::button;
use iced_cube::keys::{self, Chord, Keymap};
use iced_cube::overlay::popover::{self, Action, popover};
use iced_cube::primitives::button::Variant;
use iced_cube::theme::{Tokens, semibold, text_size};

#[derive(Debug, Clone)]
pub enum Message {
    Toggle,
    Dismiss,
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    open: bool,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            open: true,
            keymap: popover::default_keymap().bind(Chord::character('i'), Action::Toggle),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Toggle => self.open = !self.open,
            Message::Dismiss => self.open = false,
            Message::Key(key) => {
                let next = self
                    .keymap
                    .resolve_event(&key)
                    .and_then(|action| action.apply(self.open));
                if let Some(open) = next {
                    self.open = open;
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let info = column![
            text("Release notes").size(text_size::SM).font(semibold()),
            text("Version 0.0.2 adds popovers and menus.")
                .size(text_size::SM)
                .style(|theme| text::Style {
                    color: Some(Tokens::of(theme).muted_foreground),
                }),
        ]
        .spacing(4);

        let trigger = button("What's new")
            .variant(Variant::Outline)
            .on_press(Message::Toggle);

        column![
            text("Press I to open or close the popover, or Escape to close it.").size(14),
            container(
                popover(trigger, info)
                    .open(self.open)
                    .width(240)
                    .keymap(self.keymap.clone())
                    .on_dismiss(Message::Dismiss),
            )
            .height(140),
        ]
        .spacing(16)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
