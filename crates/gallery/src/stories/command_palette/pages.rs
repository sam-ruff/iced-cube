use iced::widget::{column, container, text};
use iced::{Alignment, Element, Length, Subscription};
use iced_cube::command_palette::{self, Action, Event, Output, State, command_palette, page};
use iced_cube::keys::{self, Keymap};
use iced_cube::navigation::command::{group, item};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    NewFile,
    OpenFolder,
    Theme,
    Light,
    Dark,
    System,
    Settings,
    Shortcuts,
}

#[derive(Debug, Clone)]
pub enum Message {
    Palette(command_palette::Event<Command>),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    palette: State<Command>,
    keymap: Keymap<Action>,
    last_run: Option<Command>,
}

impl Default for Example {
    fn default() -> Self {
        let mut palette = State::new([
            group(
                "Files",
                [
                    item(Command::NewFile, "New file")
                        .icon(lucide!(FilePlus))
                        .shortcut("Ctrl+N"),
                    item(Command::OpenFolder, "Open folder").icon(lucide!(FolderOpen)),
                ],
            ),
            group(
                "Preferences",
                [
                    item(Command::Theme, "Change theme...")
                        .icon(lucide!(SunMoon))
                        .keywords(["dark", "light", "appearance"]),
                    item(Command::Settings, "Settings")
                        .icon(lucide!(Settings))
                        .shortcut("Ctrl+,"),
                    item(Command::Shortcuts, "Keyboard shortcuts").icon(lucide!(Keyboard)),
                ],
            ),
        ])
        .with_page(
            page(
                Command::Theme,
                "Change theme",
                [group(
                    "Theme",
                    [
                        item(Command::Light, "Light").icon(lucide!(Sun)),
                        item(Command::Dark, "Dark").icon(lucide!(Moon)),
                        item(Command::System, "Match system").icon(lucide!(Monitor)),
                    ],
                )],
            )
            .placeholder("Pick a theme..."),
        )
        .with_recent([Command::OpenFolder])
        .with_visible_rows(5);
        let _ = palette.update(Event::Open);

        Self {
            palette,
            keymap: command_palette::default_keymap(),
            last_run: None,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Palette(event) => Some(event),
            Message::Key(key) => self.palette.key_event(&self.keymap, &key),
        };
        if let Some(Output::Activated(command)) = event.and_then(|event| self.palette.update(event))
        {
            self.last_run = Some(command);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status = match self.last_run {
            Some(command) => format!("Ran {command:?}"),
            None => "Press Ctrl+K or Ctrl+Shift+P anywhere.".to_owned(),
        };
        let base = column![
            button("Open command palette")
                .icon(lucide!(Search))
                .variant(Variant::Outline)
                .on_press(Message::Palette(Event::Open)),
            text(status).size(14),
        ]
        .spacing(12)
        .align_x(Alignment::Center);

        command_palette(
            &self.palette,
            container(base).center(Length::Fill).padding(24),
        )
        .placeholder("Search commands...")
        .max_height(320.0)
        .keymap(self.keymap.clone())
        .on_event(Message::Palette)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
