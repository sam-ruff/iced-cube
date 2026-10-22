use iced::widget::{column, text};
use iced::{Element, Task};
use iced_cube::context_menu;
use iced_cube::data::log_viewer::{
    self, Action, Event, Level, LineId, MenuItem, Output, State, entry, log_viewer,
};
use iced_cube::dropdown_menu;
use iced_cube::keys::{Chord, Keymap};
use iced_cube::theme::text_size;

use crate::bridge;

#[derive(Debug, Clone)]
pub enum Message {
    Logs(Event),
    Menu(context_menu::Event<MenuItem, LineId>),
}

#[derive(Debug)]
pub struct Example {
    logs: State,
    menu: context_menu::State<MenuItem, LineId>,
    keymap: Keymap<Action>,
    copied: String,
}

impl Default for Example {
    fn default() -> Self {
        let mut logs = State::new();
        logs.extend((0..120).map(|n| {
            let level = Level::ALL[n % Level::ALL.len()];
            entry(level, format!("step {n} of the nightly backup")).timestamp(format!(
                "02:{:02}:{:02}.000",
                n / 60,
                n % 60
            ))
        }));

        // The defaults, plus J and K.
        let keymap = log_viewer::default_keymap()
            .bind(Chord::character('j'), Action::Next)
            .bind(Chord::character('k'), Action::Previous);

        Self {
            logs,
            menu: log_viewer::menu(),
            keymap,
            copied: "Nothing copied yet".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Logs(event) => self.logs(event),
            Message::Menu(event) => {
                let chosen = self.menu.update(event);
                match (chosen, self.menu.target()) {
                    (Some(dropdown_menu::Output::Activated(item)), Some(&id)) => {
                        self.logs(item.event(id))
                    }
                    _ => Task::none(),
                }
            }
        }
    }

    fn logs(&mut self, event: Event) -> Task<Message> {
        match self.logs.update(event) {
            Some(Output::Copy(lines)) => {
                self.copied = format!("Copied {} line(s)", lines.lines().count());
                bridge::copy(lines)
            }
            Some(Output::FocusSearch) => log_viewer::focus_search(&self.logs),
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text(
                "Click a line, then use the arrows or J and K. Space and Shift+arrows select, \
                 Ctrl+C copies, Ctrl+F searches and F3 steps through matches. Right-click or \
                 Shift+F10 opens a line's menu."
            )
            .size(text_size::SM),
            log_viewer(&self.logs)
                .keymap(self.keymap.clone())
                .context_menu(&self.menu, Message::Menu)
                .height(320)
                .on_event(Message::Logs),
            text(&self.copied).size(text_size::SM),
        ]
        .spacing(12)
        .into()
    }
}
