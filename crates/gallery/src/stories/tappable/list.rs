use iced::widget::{column, container, row, scrollable, text};
use iced::{Alignment, Element, Length, Theme};
use iced_cube::icon::themed;
use iced_cube::lucide;
use iced_cube::tappable;
use iced_cube::theme::Tokens;

const NOTES: [&str; 12] = [
    "Shopping list",
    "Holiday plans",
    "Book club questions",
    "Garden jobs",
    "Recipe ideas",
    "Meeting notes",
    "Gift ideas",
    "Reading list",
    "Bike repairs",
    "Party checklist",
    "Language practice",
    "Things to fix",
];

#[derive(Debug, Clone)]
pub enum Message {
    Open(usize),
    Menu(usize),
}

#[derive(Debug, Default)]
pub struct Example {
    last: Option<String>,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        self.last = Some(match message {
            Message::Open(index) => format!("Opened {}", NOTES[index]),
            Message::Menu(index) => format!("Menu for {}", NOTES[index]),
        });
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Rows react to taps and long presses, but a finger dragged across
        // them still scrolls the list.
        let rows = NOTES.iter().enumerate().map(|(index, title)| {
            let icon = themed(lucide!(StickyNote), 16.0, 1.0, |theme: &Theme| {
                Tokens::of(theme).muted_foreground
            });
            tappable(
                row![icon, text(*title).size(14)]
                    .spacing(12)
                    .align_y(Alignment::Center),
            )
            .on_tap(Message::Open(index))
            .on_long_press(Message::Menu(index))
            .width(Length::Fill)
            .padding([10, 12])
            .into()
        });

        let status = self
            .last
            .clone()
            .unwrap_or_else(|| "Tap a note, or hold or right-click it.".to_owned());

        column![
            text(status).size(14),
            container(scrollable(column(rows).spacing(2)).spacing(8).height(220))
                .width(Length::Fill),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(320)
        .into()
    }
}
