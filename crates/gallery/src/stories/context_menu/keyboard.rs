use iced::widget::{column, container, text};
use iced::{Alignment, Border, Element, Length, Subscription};
use iced_cube::context_menu::{self, Action, Event, State, context_menu};
use iced_cube::dropdown_menu::{Output, group_label, item, radio_item, separator};
use iced_cube::keys::{self, Keymap};
use iced_cube::theme::{Tokens, radius, text_size};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Id {
    Cut,
    Copy,
    Paste,
    ByName,
    ByDate,
    BySize,
}

#[derive(Debug, Clone)]
pub enum Message {
    Menu(Event<Id>),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    menu: State<Id>,
    keymap: Keymap<Action>,
    last: String,
}

impl Default for Example {
    fn default() -> Self {
        let mut menu = State::new([
            item(Id::Cut, "Cut").shortcut("Ctrl+X"),
            item(Id::Copy, "Copy").shortcut("Ctrl+C"),
            item(Id::Paste, "Paste").shortcut("Ctrl+V"),
            separator(),
            group_label("Sort by"),
            radio_item(Id::ByName, "Name", true),
            radio_item(Id::ByDate, "Date modified", false),
            radio_item(Id::BySize, "Size", false),
        ]);
        let _ = menu.update(Event::OpenFromKeyboard);
        Self {
            menu,
            keymap: context_menu::default_keymap(),
            last: "Nothing chosen yet".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Menu(event) => Some(event),
            // The open menu takes its own keys, so only the opening chord
            // arrives here.
            Message::Key(key) => self.menu.key_event(&self.keymap, &key),
        };
        match event.and_then(|event| self.menu.update(event)) {
            Some(Output::Activated(id)) => self.last = format!("Chose {id:?}"),
            Some(Output::Selected(id)) => self.last = format!("Sorted {id:?}"),
            Some(Output::Toggled(..)) | None => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let area = container(text("Document").size(text_size::SM))
            .width(400)
            .height(160)
            .padding(16)
            .align_x(Alignment::End)
            .align_y(Alignment::End)
            .style(|theme| {
                let tokens = Tokens::of(theme);
                container::Style {
                    text_color: Some(tokens.muted_foreground),
                    border: Border {
                        color: tokens.border,
                        width: 1.0,
                        radius: radius::LG.into(),
                    },
                    ..container::Style::default()
                }
            });

        column![
            text("Shift+F10 or the Menu key opens the menu at the corner of the area.").size(14),
            text(&self.last).size(14),
            context_menu(&self.menu, area)
                .keymap(self.keymap.clone())
                .on_event(Message::Menu),
        ]
        .spacing(12)
        .width(480)
        .height(Length::Fill)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
