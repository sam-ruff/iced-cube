use iced::widget::{column, container, scrollable, span, text};
use iced::{Element, Length, Task, Theme};
use iced_cube::context_menu::{Event, State, context_menu};
use iced_cube::data::selectable_text::{self, selectable_text, selection_area};
use iced_cube::dropdown_menu::{Output, item};
use iced_cube::theme::{self, Tokens};

use crate::bridge;

const AREA: &str = "release-notes";

const NOTES: [&str; 8] = [
    "Search now finds text inside attachments as well as in the documents themselves.",
    "Tables keep their column widths when a document is exported.",
    "The sidebar remembers which collections were folded.",
    "Comments can be resolved and reopened from the keyboard.",
    "Large images are scaled down before they are uploaded.",
    "Drafts are saved half a second after typing stops.",
    "Links to other documents show the document's title.",
    "The editor no longer loses the selection when the window is resized.",
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Id {
    Copy,
    SelectAll,
}

#[derive(Debug, Clone)]
pub enum Message {
    Menu(Event<Id>),
    Copy(String),
    Selected(String),
}

#[derive(Debug)]
pub struct Example {
    menu: State<Id>,
    status: Option<String>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            menu: State::new([
                item(Id::Copy, "Copy").shortcut("Ctrl+C"),
                item(Id::SelectAll, "Select all").shortcut("Ctrl+A"),
            ]),
            status: None,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // The menu asks the area for its text through a task.
            Message::Menu(event) => match self.menu.update(event) {
                Some(Output::Activated(Id::Copy)) => {
                    selectable_text::selected(AREA).map(Message::Copy)
                }
                Some(Output::Activated(Id::SelectAll)) => {
                    selectable_text::select_all(AREA).map(Message::Selected)
                }
                _ => Task::none(),
            },
            Message::Copy(text) if text.is_empty() => Task::none(),
            Message::Copy(text) => {
                self.status = Some(format!("Copied {} characters", text.chars().count()));
                bridge::copy(text)
            }
            Message::Selected(text) => {
                self.status = Some(format!("Selected {} characters", text.chars().count()));
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let notes = NOTES.iter().map(|note| {
            selectable_text::<String, _>(vec![span(*note)])
                .size(14)
                .font(theme::font())
                .into()
        });

        // The scrollable sits inside the area, so a drag past its top or
        // bottom edge scrolls it and keeps selecting.
        let area = selection_area(
            scrollable(container(column(notes).spacing(12)).padding([0, 12]))
                .spacing(8)
                .height(180),
        )
        .id(AREA)
        .on_copy(Message::Copy)
        .on_select(Message::Selected);

        let status = self
            .status
            .clone()
            .unwrap_or_else(|| "Select some text, then right-click or press and hold.".to_owned());

        column![
            context_menu(&self.menu, area).on_event(Message::Menu),
            text(status).size(14).style(|theme: &Theme| text::Style {
                color: Some(Tokens::of(theme).muted_foreground),
            }),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(360)
        .into()
    }
}
