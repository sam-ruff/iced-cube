use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::lucide;
use iced_cube::navigation::command::{self, Output, State, command, group, item};

#[derive(Debug, Clone)]
pub enum Message {
    Command(command::Event<&'static str>),
}

#[derive(Debug)]
pub struct Example {
    command: State<&'static str>,
    last_run: Option<&'static str>,
}

impl Default for Example {
    fn default() -> Self {
        let command = State::new([
            group(
                "Suggestions",
                [
                    item("calendar", "Calendar").icon(lucide!(Calendar)),
                    item("new-file", "New file")
                        .icon(lucide!(FilePlus))
                        .keywords(["create", "document"]),
                    item("calculator", "Calculator")
                        .icon(lucide!(Calculator))
                        .disabled(true),
                ],
            ),
            group(
                "Settings",
                [
                    item("profile", "Profile")
                        .icon(lucide!(User))
                        .shortcut("Ctrl+P"),
                    item("billing", "Billing")
                        .icon(lucide!(CreditCard))
                        .shortcut("Ctrl+B"),
                    item("settings", "Settings")
                        .icon(lucide!(Settings))
                        .shortcut("Ctrl+S")
                        .keywords(["preferences", "options"]),
                ],
            ),
        ]);

        Self {
            command,
            last_run: None,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Command(event) = message;
        if let Some(Output::Activated(id)) = self.command.update(event) {
            self.last_run = Some(id);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status = match self.last_run {
            Some(id) => format!("Ran \"{id}\""),
            None => "Click a row, or type and press Enter.".to_owned(),
        };

        column![
            command(&self.command)
                .height(320)
                .on_event(Message::Command),
            text(status).size(14),
        ]
        .spacing(12)
        .width(Length::Fixed(420.0))
        .into()
    }
}
