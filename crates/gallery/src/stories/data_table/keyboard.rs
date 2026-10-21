use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::data::data_table::{self, Action, Align, Event, Output, State, data_table};
use iced_cube::keys::{Chord, Keymap};

#[derive(Debug, Clone)]
pub struct Ticket {
    id: u32,
    title: &'static str,
    points: u32,
}

const TITLES: [&str; 8] = [
    "Fix login redirect",
    "Add export to CSV",
    "Update onboarding copy",
    "Cache report queries",
    "Retry failed webhooks",
    "Dark mode for emails",
    "Audit log filters",
    "Remove legacy flags",
];

#[derive(Debug, Clone)]
pub enum Message {
    Tickets(Event<u32>),
}

#[derive(Debug)]
pub struct Example {
    tickets: State<Ticket, u32>,
    keymap: Keymap<Action>,
    last: String,
}

impl Default for Example {
    fn default() -> Self {
        let columns = [
            data_table::column("id", "Ticket", |t: &Ticket| t.id.into())
                .width(90)
                .format(|t| format!("#{}", t.id)),
            data_table::column("title", "Title", |t: &Ticket| t.title.into())
                .width(Length::FillPortion(3)),
            data_table::column("points", "Points", |t: &Ticket| t.points.into()).align(Align::End),
        ];
        let rows = TITLES.iter().zip(1_u32..).map(|(&title, n)| Ticket {
            id: 400 + n,
            title,
            points: n % 5 + 1,
        });
        let mut tickets = State::new(columns, rows, |t| t.id)
            .with_selection(true)
            .with_page_sizes([4, 8]);
        let _ = tickets.update(Event::Press(402));

        // The defaults, plus J and K.
        let keymap = data_table::default_keymap()
            .bind(Chord::character('j'), Action::Next)
            .bind(Chord::character('k'), Action::Previous);

        Self {
            tickets,
            keymap,
            last: "Nothing opened yet".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Tickets(event) => {
                if let Some(Output::Activated(id)) = self.tickets.update(event) {
                    self.last = format!("Opened #{id}");
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text(
                "Click the table, then use the arrow keys or J and K. Space selects, Enter \
                 opens, Ctrl+A selects the page, Escape clears and PageDown turns the page."
            )
            .size(14),
            data_table(&self.tickets)
                .toolbar(false)
                .keymap(self.keymap.clone())
                .on_event(Message::Tickets),
            text(&self.last).size(14),
        ]
        .spacing(12)
        .into()
    }
}
