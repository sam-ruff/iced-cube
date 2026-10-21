use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::data::tree::{Event, Mode, State, node, tree};

#[derive(Debug, Clone)]
pub enum Message {
    Permissions(Event<u32>),
}

#[derive(Debug)]
pub struct Example {
    permissions: State<u32>,
}

impl Default for Example {
    fn default() -> Self {
        let mut permissions = State::new([
            node(1, "Content").children([
                node(2, "Read articles"),
                node(3, "Write articles"),
                node(4, "Publish articles"),
            ]),
            node(5, "Billing").children([
                node(6, "View invoices"),
                node(7, "Manage payment methods").disabled(true),
                node(8, "Download receipts"),
            ]),
            node(9, "Members").children([node(10, "Invite members"), node(11, "Remove members")]),
        ])
        .with_mode(Mode::Checkbox)
        .with_expanded([1, 5, 9]);
        let _ = permissions.update(Event::Check(1, true));
        let _ = permissions.update(Event::Check(6, true));

        Self { permissions }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Permissions(event) => {
                let _ = self.permissions.update(event);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let granted = self
            .permissions
            .checked()
            .into_iter()
            .filter(|&id| self.permissions.children(id).is_empty())
            .count();

        column![
            tree(&self.permissions).on_event(Message::Permissions),
            text(format!("{granted} of 10 permissions granted")).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(340)
        .into()
    }
}
