use iced::keyboard::Modifiers;
use iced::widget::{column, text};
use iced::{Element, Length, Task};
use iced_cube::context_menu;
use iced_cube::data::virtual_list::{
    self, Action, Event, Mode, Output, RowId, State, virtual_list,
};
use iced_cube::dropdown_menu::{self, item};
use iced_cube::keys::{Chord, Keymap};
use iced_cube::lucide;
use iced_cube::theme::text_size;

use crate::bridge;

#[derive(Debug, Clone)]
pub enum Message {
    Chapters(Event<String>),
    Menu(context_menu::Event<MenuItem, RowId>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuItem {
    Open,
    CopyTitle,
}

#[derive(Debug)]
pub struct Example {
    chapters: State<String>,
    menu: context_menu::State<MenuItem, RowId>,
    keymap: Keymap<Action>,
    last: String,
}

impl Default for Example {
    fn default() -> Self {
        let chapters = (1..=500).map(|n| format!("Chapter {n}"));

        // The defaults, plus J and K.
        let keymap = virtual_list::default_keymap()
            .bind(Chord::character('j'), Action::Next)
            .bind(Chord::character('k'), Action::Previous);

        let mut chapters = State::new(chapters).with_mode(Mode::Multiple);
        let _ = chapters.update(Event::Press(2, Modifiers::empty()));

        let menu = context_menu::State::new([
            item(MenuItem::Open, "Open").icon(lucide!(BookOpen)),
            item(MenuItem::CopyTitle, "Copy title").icon(lucide!(Copy)),
        ]);

        Self {
            chapters,
            menu,
            keymap,
            last: "Nothing opened yet".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Chapters(event) => {
                if let Some(Output::Activated(index)) = self.chapters.update(event) {
                    self.last = format!("Opened {}", self.title(index));
                }
                Task::none()
            }
            Message::Menu(event) => {
                let chosen = self.menu.update(event);
                let target = self.menu.target().and_then(|&id| self.chapters.index(id));
                let (Some(dropdown_menu::Output::Activated(item)), Some(index)) = (chosen, target)
                else {
                    return Task::none();
                };
                let title = self.title(index).to_owned();
                match item {
                    MenuItem::Open => {
                        self.last = format!("Opened {title}");
                        Task::none()
                    }
                    MenuItem::CopyTitle => {
                        self.last = format!("Copied {title}");
                        bridge::copy(title)
                    }
                }
            }
        }
    }

    fn title(&self, index: usize) -> &str {
        self.chapters.get(index).map_or("", String::as_str)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let selected = self.chapters.selected().len();

        column![
            text(
                "Click the list, then use the arrow keys or J and K. Page Up and Page Down \
                 move a screenful, Shift+arrows extend the selection, Space selects and \
                 Enter opens. Right-click or Shift+F10 opens a row's menu."
            )
            .size(text_size::SM),
            virtual_list(&self.chapters, |_, title: &String| {
                text(title.as_str()).size(text_size::SM).into()
            })
            .keymap(self.keymap.clone())
            .context_menu(&self.menu, Message::Menu)
            .height(200)
            .on_event(Message::Chapters),
            text(format!("{selected} selected. {}", self.last)).size(text_size::SM),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(420)
        .into()
    }
}
