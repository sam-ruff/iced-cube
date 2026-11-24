use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::forms::multi_combobox::{self, Event, Output, State, multi_combobox};

const PEOPLE: &[&str] = &[
    "Ada Lovelace",
    "Alan Turing",
    "Barbara Liskov",
    "Edsger Dijkstra",
    "Grace Hopper",
    "Katherine Johnson",
    "Linus Torvalds",
    "Margaret Hamilton",
];

#[derive(Debug, Clone)]
pub enum Message {
    Guests(multi_combobox::Event),
}

#[derive(Debug)]
pub struct Example {
    guests: State<&'static str>,
    last: String,
}

impl Default for Example {
    fn default() -> Self {
        let mut guests = State::new(PEOPLE.iter().copied())
            .with_chosen(&["Grace Hopper", "Alan Turing"])
            .with_visible_rows(5);
        let _ = guests.update(Event::Open);
        Self {
            guests,
            last: "Type a name, or use the arrow keys.".to_owned(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Guests(event) = message;
        match self.guests.update(event) {
            Some(Output::Added(name)) => self.last = format!("Invited {name}"),
            Some(Output::Removed(name)) => self.last = format!("Removed {name}"),
            None => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text(&self.last).size(14),
            multi_combobox(&self.guests)
                .placeholder("Add guests...")
                .width(Length::Fill)
                .on_event(Message::Guests),
        ]
        .spacing(12)
        .width(Length::Fixed(320.0))
        .height(Length::Fill)
        .into()
    }
}
