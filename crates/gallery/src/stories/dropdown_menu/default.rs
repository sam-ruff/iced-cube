use iced::widget::{column, container, text};
use iced::{Element, Length};
use iced_cube::dropdown_menu::{
    Event, Output, State, dropdown_menu, group_label, item, separator, submenu,
};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Id {
    Profile,
    Billing,
    Settings,
    Invite,
    Email,
    Chat,
    NewTeam,
    Support,
    Api,
    Delete,
}

#[derive(Debug, Clone)]
pub enum Message {
    Menu(Event<Id>),
}

#[derive(Debug)]
pub struct Example {
    menu: State<Id>,
    chosen: Option<Id>,
}

impl Default for Example {
    fn default() -> Self {
        let mut menu = State::new([
            group_label("My account"),
            item(Id::Profile, "Profile")
                .icon(lucide!(User))
                .shortcut("Shift+Ctrl+P"),
            item(Id::Billing, "Billing")
                .icon(lucide!(CreditCard))
                .shortcut("Ctrl+B"),
            item(Id::Settings, "Settings")
                .icon(lucide!(Settings))
                .shortcut("Ctrl+S"),
            separator(),
            submenu(
                Id::Invite,
                "Invite users",
                [
                    item(Id::Email, "Email").icon(lucide!(Mail)),
                    item(Id::Chat, "Message").icon(lucide!(MessageSquare)),
                ],
            )
            .icon(lucide!(UserPlus)),
            item(Id::NewTeam, "New team")
                .icon(lucide!(Plus))
                .shortcut("Ctrl+T"),
            separator(),
            item(Id::Support, "Support").icon(lucide!(LifeBuoy)),
            item(Id::Api, "API").icon(lucide!(Cloud)).disabled(true),
            separator(),
            item(Id::Delete, "Delete account")
                .icon(lucide!(Trash))
                .destructive(true),
        ]);
        let _ = menu.update(Event::Open);
        Self { menu, chosen: None }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Menu(event) = message;
        if let Some(Output::Activated(id)) = self.menu.update(event) {
            self.chosen = Some(id);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let trigger = button("Open menu")
            .variant(Variant::Outline)
            .on_press(Message::Menu(Event::Toggle));
        let chosen = match self.chosen {
            Some(id) => format!("Chose {id:?}"),
            None => "Nothing chosen yet".into(),
        };

        column![
            text(chosen).size(14),
            container(dropdown_menu(&self.menu, trigger).on_event(Message::Menu))
                .height(Length::Fill),
        ]
        .spacing(12)
        .width(240)
        .into()
    }
}
