use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::forms::combobox::{self, Event, State, combobox};

const FRUITS: &[&str] = &[
    "Apple",
    "Apricot",
    "Banana",
    "Blueberry",
    "Cherry",
    "Grape",
    "Lemon",
    "Mango",
    "Orange",
    "Pear",
];

#[derive(Debug, Clone)]
pub enum Message {
    Fruit(combobox::Event),
}

#[derive(Debug)]
pub struct Example {
    fruit: State<&'static str>,
}

impl Default for Example {
    fn default() -> Self {
        let mut fruit = State::new(FRUITS.iter().copied()).with_visible_rows(5);
        let _ = fruit.update(Event::Open);
        Self { fruit }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Fruit(event) = message;
        let _ = self.fruit.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let chosen = match self.fruit.selected() {
            Some(fruit) => format!("Chosen: {fruit}"),
            None => "Type to filter, or use the arrow keys.".to_owned(),
        };

        column![
            text(chosen).size(14),
            combobox(&self.fruit)
                .placeholder("Search fruit...")
                .on_event(Message::Fruit),
        ]
        .spacing(12)
        .width(Length::Fixed(280.0))
        .into()
    }
}
