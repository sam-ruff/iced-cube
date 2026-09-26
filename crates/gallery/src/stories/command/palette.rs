use iced::widget::{column, text};
use iced::{Alignment, Element, Subscription};
use iced_cube::keys::{self, Chord};
use iced_cube::navigation::command::{self, Event, Output, State, command, group, item};
use iced_cube::overlay::dialog::{self, dialog};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    Open,
    Close,
    Command(command::Event<&'static str>),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    open: bool,
    command: State<&'static str>,
    last_run: Option<&'static str>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            open: true,
            command: palette(),
            last_run: None,
        }
    }
}

fn palette() -> State<&'static str> {
    State::new([
        group(
            "Pages",
            [
                item("home", "Home").icon(lucide!(House)),
                item("inbox", "Inbox").icon(lucide!(Inbox)),
                item("settings", "Settings")
                    .icon(lucide!(Settings))
                    .keywords(["preferences"]),
            ],
        ),
        group(
            "Actions",
            [
                item("new-file", "New file")
                    .icon(lucide!(FilePlus))
                    .shortcut("Ctrl+N"),
                item("theme", "Toggle theme").icon(lucide!(SunMoon)),
            ],
        ),
    ])
    .with_visible_rows(5)
}

/// Opens the palette from anywhere, and closes it again while it is open.
fn toggle() -> Chord {
    Chord::character('k').command()
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Open => self.show(),
            Message::Close => self.open = false,
            Message::Key(key) if toggle().matches(&key.key, key.modifiers) => {
                if self.open {
                    self.open = false;
                } else {
                    self.show();
                }
            }
            Message::Key(_) => {}
            Message::Command(event) => match self.command.update(event) {
                Some(Output::Activated(id)) => {
                    self.last_run = Some(id);
                    self.open = false;
                }
                Some(Output::Closed) => self.open = false,
                _ => {}
            },
        }
    }

    fn show(&mut self) {
        self.open = true;
        let _ = self.command.update(Event::Input(String::new()));
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status = match self.last_run {
            Some(id) => format!("Ran \"{id}\""),
            None => "Press Ctrl+K anywhere to search pages and actions.".to_owned(),
        };
        let base = column![
            button("Open command palette")
                .icon(lucide!(Search))
                .variant(Variant::Outline)
                .on_press(Message::Open),
            text(status).size(14),
        ]
        .spacing(12)
        .align_x(Alignment::Center);

        dialog(base)
            .open(self.open)
            .title("Command palette")
            .body(
                command(&self.command)
                    .placeholder("Search pages and actions...")
                    .height(280)
                    .on_event(Message::Command),
            )
            .size(dialog::Size::Sm)
            .close_button(false)
            .pass_through([toggle()])
            .on_dismiss(Message::Close)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
