use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::{column, container, mouse_area, text};
use iced::{Border, Element, Length, Subscription};
use iced_cube::context_menu::{self, Action, Event, State, keyed};
use iced_cube::dropdown_menu::{Output, item, separator};
use iced_cube::keys::{self, Keymap};
use iced_cube::theme::{Tokens, radius, text_size};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Id {
    Open,
    Rename,
    Duplicate,
    Delete,
}

const FILES: [&str; 4] = ["notes.md", "budget.xlsx", "photo.png", "report.pdf"];

#[derive(Debug, Clone)]
pub enum Message {
    Select(usize),
    Menu(Event<Id, usize>),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    selected: usize,
    // One menu for every row. Its open events carry the row's index.
    menu: State<Id, usize>,
    keymap: Keymap<Action>,
    last: String,
}

impl Default for Example {
    fn default() -> Self {
        let mut menu = State::new([
            item(Id::Open, "Open").shortcut("Enter"),
            item(Id::Rename, "Rename").shortcut("F2"),
            item(Id::Duplicate, "Duplicate").shortcut("Ctrl+D"),
            separator(),
            item(Id::Delete, "Delete").shortcut("Del").destructive(true),
        ]);
        let _ = menu.update(Event::OpenFromKeyboard(1));
        Self {
            selected: 1,
            menu,
            keymap: context_menu::default_keymap(),
            last: "Nothing chosen yet".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Select(row) => {
                self.selected = row;
                None
            }
            Message::Menu(event) => Some(event),
            // The open menu takes its own keys, so only the opening chord
            // and the row keys arrive here.
            Message::Key(key) => self.key(&key),
        };
        let Some(event) = event else {
            return;
        };
        if let Event::Open(row, _) | Event::OpenFromKeyboard(row) = event {
            self.selected = row;
        }
        let chosen = self.menu.update(event);
        let (Some(Output::Activated(id)), Some(file)) = (chosen, self.target_file()) else {
            return;
        };
        self.last = format!("{id:?} {file}");
    }

    /// The file the menu last opened on.
    fn target_file(&self) -> Option<&'static str> {
        self.menu.target().and_then(|&row| FILES.get(row)).copied()
    }

    fn key(&mut self, key: &keys::Event) -> Option<Event<Id, usize>> {
        let event = self.menu.key_event(&self.keymap, key, self.selected);
        if event.is_some() || self.menu.is_open() {
            return event;
        }
        match key.key {
            Key::Named(Named::ArrowDown) => {
                self.selected = (self.selected + 1).min(FILES.len() - 1)
            }
            Key::Named(Named::ArrowUp) => self.selected = self.selected.saturating_sub(1),
            _ => {}
        }
        None
    }

    pub fn view(&self) -> Element<'_, Message> {
        let rows = FILES.iter().enumerate().map(|(index, name)| {
            let selected = index == self.selected;
            let row = mouse_area(
                container(text(*name).size(text_size::SM))
                    .padding([8, 12])
                    .width(Length::Fill)
                    .style(move |theme| {
                        let tokens = Tokens::of(theme);
                        container::Style {
                            background: selected.then(|| tokens.accent.into()),
                            border: Border {
                                radius: radius::MD.into(),
                                ..Border::default()
                            },
                            ..container::Style::default()
                        }
                    }),
            )
            .on_press(Message::Select(index));

            keyed(&self.menu, index, row)
                .keymap(self.keymap.clone())
                .on_event(Message::Menu)
                .into()
        });

        column![
            text("Up and Down pick a file. Shift+F10 or the Menu key opens its menu below it.")
                .size(14),
            text(&self.last).size(14),
            column(rows).spacing(2).width(Length::Fill),
        ]
        .spacing(12)
        .width(360)
        .height(Length::Fill)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
