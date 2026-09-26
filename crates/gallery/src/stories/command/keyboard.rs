use iced::widget::{column, text};
use iced::{Element, Length, Subscription};
use iced_cube::keys::{self, Chord, Keymap};
use iced_cube::lucide;
use iced_cube::navigation::command::{self, Action, Output, State, command, group, item};

#[derive(Debug, Clone)]
pub enum Message {
    Command(command::Event<&'static str>),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    command: State<&'static str>,
    keymap: Keymap<Action>,
    last_run: Option<&'static str>,
}

impl Default for Example {
    fn default() -> Self {
        let command = State::new([
            group(
                "Navigation",
                [
                    item("home", "Go to home").icon(lucide!(House)),
                    item("inbox", "Go to inbox").icon(lucide!(Mail)),
                    item("terminal", "Open terminal")
                        .icon(lucide!(Terminal))
                        .shortcut("Ctrl+`"),
                ],
            ),
            group(
                "Danger zone",
                [
                    item("sign-out", "Sign out").icon(lucide!(LogOut)),
                    item("delete", "Delete project")
                        .icon(lucide!(Trash))
                        .destructive(true),
                ],
            ),
        ]);
        let keymap = command::default_keymap()
            .bind(Chord::character('n').ctrl(), Action::Next)
            .bind(Chord::character('p').ctrl(), Action::Previous);

        Self {
            command,
            keymap,
            last_run: None,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Command(event) => Some(event),
            // Presses the focused search field did not handle.
            Message::Key(key) => self
                .keymap
                .resolve_event(&key)
                .and_then(|action| action.event(&self.command)),
        };
        let Some(event) = event else {
            return;
        };
        if let Some(Output::Activated(id)) = self.command.update(event) {
            self.last_run = Some(id);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status = match self.last_run {
            Some(id) => format!("Ran \"{id}\""),
            None => "Arrows or Ctrl+N and Ctrl+P move, Home and End jump, Enter runs.".to_owned(),
        };

        column![
            command(&self.command)
                .placeholder("Search actions...")
                .keymap(self.keymap.clone())
                .height(300)
                .on_event(Message::Command),
            text(status).size(14),
        ]
        .spacing(12)
        .width(Length::Fixed(420.0))
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
