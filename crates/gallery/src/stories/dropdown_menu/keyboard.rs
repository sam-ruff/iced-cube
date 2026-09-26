use iced::keyboard::key::Named;
use iced::widget::{column, container, text};
use iced::{Element, Length, Subscription};
use iced_cube::button;
use iced_cube::dropdown_menu::{
    self, Action, Event, Output, State, dropdown_menu, item, separator,
};
use iced_cube::keys::{self, Chord, Keymap};
use iced_cube::primitives::button::Variant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Edit {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

#[derive(Debug, Clone)]
pub enum Message {
    Menu(Event<Edit>),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    menu: State<Edit>,
    keymap: Keymap<Action>,
    chosen: Option<Edit>,
}

impl Default for Example {
    fn default() -> Self {
        let mut menu = State::new([
            item(Edit::Undo, "Undo").shortcut("Ctrl+Z"),
            item(Edit::Redo, "Redo").shortcut("Ctrl+Y").disabled(true),
            separator(),
            item(Edit::Cut, "Cut").shortcut("Ctrl+X"),
            item(Edit::Copy, "Copy").shortcut("Ctrl+C"),
            item(Edit::Paste, "Paste").shortcut("Ctrl+V"),
            separator(),
            item(Edit::SelectAll, "Select all").shortcut("Ctrl+A"),
        ]);
        let _ = menu.update(Event::First);
        // Alt+ArrowDown opens the closed menu; the open menu handles the rest.
        let keymap = dropdown_menu::default_keymap()
            .bind(Chord::named(Named::ArrowDown).alt(), Action::Open);
        Self {
            menu,
            keymap,
            chosen: None,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Menu(event) => Some(event),
            // Only presses nothing else captured arrive here, so an open
            // menu has already taken its own keys.
            Message::Key(key) => self.menu.key_event(&self.keymap, &key),
        };
        if let Some(Output::Activated(edit)) = event.and_then(|event| self.menu.update(event)) {
            self.chosen = Some(edit);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let chosen = match self.chosen {
            Some(edit) => format!("Chose {edit:?}"),
            None => "Nothing chosen yet".into(),
        };
        let trigger = button("Edit")
            .variant(Variant::Outline)
            .on_press(Message::Menu(Event::Toggle));

        column![
            text("Arrows move, Enter chooses, Escape closes, letters jump. Alt+ArrowDown opens.")
                .size(14),
            text(chosen).size(14),
            container(
                dropdown_menu(&self.menu, trigger)
                    .keymap(self.keymap.clone())
                    .on_event(Message::Menu)
            )
            .height(Length::Fill),
        ]
        .spacing(12)
        .width(Length::Fixed(560.0))
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
