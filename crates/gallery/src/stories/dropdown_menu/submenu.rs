use iced::widget::container;
use iced::{Element, Length};
use iced_cube::button;
use iced_cube::dropdown_menu::{Event, State, dropdown_menu, item, separator, submenu};
use iced_cube::primitives::button::Variant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Id {
    NewTab,
    NewWindow,
    Private,
    MoreTools,
    SavePage,
    Shortcut,
    NameWindow,
    DevTools,
    Print,
}

#[derive(Debug, Clone)]
pub enum Message {
    Menu(Event<Id>),
}

#[derive(Debug)]
pub struct Example {
    menu: State<Id>,
}

impl Default for Example {
    fn default() -> Self {
        let mut menu = State::new([
            item(Id::NewTab, "New tab").shortcut("Ctrl+T"),
            item(Id::NewWindow, "New window").shortcut("Ctrl+N"),
            item(Id::Private, "Private window")
                .shortcut("Shift+Ctrl+N")
                .disabled(true),
            submenu(
                Id::MoreTools,
                "More tools",
                [
                    item(Id::SavePage, "Save page as").shortcut("Ctrl+S"),
                    item(Id::Shortcut, "Create shortcut"),
                    item(Id::NameWindow, "Name window"),
                    separator(),
                    item(Id::DevTools, "Developer tools"),
                ],
            ),
            separator(),
            item(Id::Print, "Print").shortcut("Ctrl+P"),
        ]);
        let _ = menu.update(Event::Open);
        let _ = menu.update(Event::Highlight(Id::MoreTools));
        Self { menu }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Menu(event) = message;
        let _ = self.menu.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let trigger = button("File")
            .variant(Variant::Outline)
            .on_press(Message::Menu(Event::Toggle));

        container(dropdown_menu(&self.menu, trigger).on_event(Message::Menu))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
