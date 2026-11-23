use iced::widget::row;
use iced::{Alignment, Element};
use iced_cube::feedback::avatar::Size;
use iced_cube::feedback::chip::Variant;
use iced_cube::{avatar, chip, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    Remove(usize),
}

#[derive(Debug)]
pub struct Example {
    people: Vec<&'static str>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            people: vec!["Ada Lovelace", "Grace Hopper", "Alan Turing"],
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Remove(index) = message;
        if index < self.people.len() {
            self.people.remove(index);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let people = self.people.iter().enumerate().map(|(index, name)| {
            chip(*name)
                .leading(avatar(name).size(Size::Sm))
                .variant(Variant::Outline)
                .on_remove(Message::Remove(index))
                .into()
        });

        row(people)
            .push(chip("Engineering").icon(lucide!(Users)))
            .push(chip("Room 4").icon(lucide!(MapPin)))
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap()
            .vertical_spacing(8)
            .into()
    }
}
