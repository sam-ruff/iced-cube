use iced::widget::{column, container, text};
use iced::{Element, Length, Subscription};
use iced_cube::dropdown_menu::{
    self, checkbox_item, group_label, item, radio_item, separator, submenu,
};
use iced_cube::keys::{self, Keymap};
use iced_cube::lucide;
use iced_cube::menubar::{self, Action, Output, State, menu, menubar};
use iced_cube::theme::{Tokens, text_size};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    New,
    Open,
    Recent,
    Notes,
    Todo,
    Save,
    Quit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    WordWrap,
    LineNumbers,
    Small,
    Medium,
    Large,
    Shortcuts,
    About,
}

#[derive(Debug, Clone)]
pub enum Message {
    Menubar(menubar::Event<Command>),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    menubar: State<Command>,
    keymap: Keymap<Action>,
    menu_keymap: Keymap<dropdown_menu::Action>,
    status: String,
}

impl Default for Example {
    fn default() -> Self {
        let mut menubar = State::new([
            menu(
                "File",
                [
                    item(Command::New, "New file")
                        .icon(lucide!(FilePlus))
                        .shortcut("Ctrl+N"),
                    item(Command::Open, "Open...")
                        .icon(lucide!(FolderOpen))
                        .shortcut("Ctrl+O"),
                    submenu(
                        Command::Recent,
                        "Open recent",
                        [
                            item(Command::Notes, "notes.md"),
                            item(Command::Todo, "todo.md"),
                        ],
                    )
                    .icon(lucide!(Clock)),
                    separator(),
                    item(Command::Save, "Save")
                        .icon(lucide!(Save))
                        .shortcut("Ctrl+S"),
                    separator(),
                    item(Command::Quit, "Quit").shortcut("Ctrl+Q"),
                ],
            ),
            menu(
                "Edit",
                [
                    item(Command::Undo, "Undo").shortcut("Ctrl+Z"),
                    item(Command::Redo, "Redo")
                        .shortcut("Ctrl+Shift+Z")
                        .disabled(true),
                    separator(),
                    item(Command::Cut, "Cut").shortcut("Ctrl+X"),
                    item(Command::Copy, "Copy").shortcut("Ctrl+C"),
                    item(Command::Paste, "Paste").shortcut("Ctrl+V"),
                ],
            ),
            menu(
                "View",
                [
                    checkbox_item(Command::WordWrap, "Word wrap", true),
                    checkbox_item(Command::LineNumbers, "Line numbers", false),
                    separator(),
                    group_label("Text size"),
                    radio_item(Command::Small, "Small", false),
                    radio_item(Command::Medium, "Medium", true),
                    radio_item(Command::Large, "Large", false),
                ],
            ),
            menu(
                "Help",
                [
                    item(Command::Shortcuts, "Keyboard shortcuts")
                        .icon(lucide!(Keyboard))
                        .shortcut("F1"),
                    item(Command::About, "About"),
                ],
            ),
        ]);
        // Starts with the File menu open so the preview shows it.
        let _ = menubar.update(menubar::Event::Toggle(0));

        Self {
            menubar,
            keymap: menubar::default_keymap(),
            menu_keymap: dropdown_menu::default_keymap(),
            status: String::from("Press F10 or Alt, then use the arrow keys."),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Menubar(event) => Some(event),
            Message::Key(key) => self
                .menubar
                .key_event(&self.keymap, &self.menu_keymap, &key),
        };
        let Some(output) = event.and_then(|event| self.menubar.update(event)) else {
            return;
        };
        self.status = match output {
            Output::Activated(Command::Save) => "Saved notes.md".into(),
            Output::Activated(Command::Notes) => "Opened notes.md".into(),
            Output::Activated(Command::Todo) => "Opened todo.md".into(),
            Output::Activated(command) => format!("Ran {command:?}"),
            Output::Toggled(Command::WordWrap, on) => {
                format!("Word wrap {}", if on { "on" } else { "off" })
            }
            Output::Toggled(command, on) => format!("{command:?}: {on}"),
            Output::Selected(size) => format!("Text size: {size:?}"),
        };
    }

    pub fn view(&self) -> Element<'_, Message> {
        let page = container(
            text("# notes.md\n\nChoose a command from the menus above.")
                .size(text_size::SM)
                .style(|theme| text::Style {
                    color: Some(Tokens::of(theme).muted_foreground),
                }),
        )
        .padding(16)
        .height(Length::Fill);

        column![
            menubar(&self.menubar)
                .keymap(self.keymap.clone())
                .menu_keymap(self.menu_keymap.clone())
                .on_event(Message::Menubar),
            page,
            container(text(&self.status).size(text_size::XS)).padding([6, 16]),
        ]
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
