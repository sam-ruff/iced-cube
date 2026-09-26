use iced::widget::{column, container};
use iced::{Element, padding};
use iced_cube::primitives::checkbox;
use iced_cube::primitives::checkbox::CheckState;

const TOPPINGS: [&str; 3] = ["Mushrooms", "Olives", "Peppers"];

#[derive(Debug, Clone)]
pub enum Message {
    All(bool),
    Topping(usize, bool),
}

#[derive(Debug)]
pub struct Example {
    chosen: [bool; 3],
}

impl Default for Example {
    fn default() -> Self {
        Self {
            chosen: [true, false, true],
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::All(checked) => self.chosen = [checked; 3],
            Message::Topping(index, checked) => self.chosen[index] = checked,
        }
    }

    fn all(&self) -> CheckState {
        match self.chosen.iter().filter(|chosen| **chosen).count() {
            0 => CheckState::Unchecked,
            n if n == self.chosen.len() => CheckState::Checked,
            _ => CheckState::Indeterminate,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let toppings = column(TOPPINGS.iter().enumerate().map(|(index, name)| {
            checkbox(self.chosen[index])
                .label(*name)
                .on_toggle(move |checked| Message::Topping(index, checked))
                .into()
        }))
        .spacing(12);

        column![
            checkbox(self.all())
                .label("All toppings")
                .on_toggle(Message::All),
            container(toppings).padding(padding::left(24)),
        ]
        .spacing(12)
        .into()
    }
}
