use iced::widget::{column, text};
use iced::{Color, Element, Length};
use iced_cube::data::property_grid::{
    Event, Sort, State, choice, colour, number, property_grid, text as text_property, toggle,
};

#[derive(Debug, Clone)]
pub enum Message {
    Button(Event<u8>),
}

#[derive(Debug)]
pub struct Example {
    button: State<u8>,
}

impl Default for Example {
    fn default() -> Self {
        let mut button = State::new([
            text_property(1, "Label", "Save changes").category("Content"),
            text_property(2, "Tooltip", "Saves the draft").category("Content"),
            choice(3, "Variant", ["Primary", "Outline", "Ghost"], "Primary").category("Style"),
            colour(4, "Background colour", Color::from_rgb8(0x18, 0x18, 0x1b)).category("Style"),
            colour(5, "Text colour", Color::from_rgb8(0xfa, 0xfa, 0xfa)).category("Style"),
            colour(6, "Border colour", Color::from_rgb8(0xe4, 0xe4, 0xe7)).category("Style"),
            number(7, "Corner radius", 6.0)
                .category("Style")
                .step(1.0)
                .unit("px"),
            number(8, "Width", 120.0)
                .category("Size")
                .step(1.0)
                .unit("px"),
            number(9, "Height", 36.0)
                .category("Size")
                .step(1.0)
                .unit("px"),
            number(10, "Padding", 16.0)
                .category("Size")
                .step(1.0)
                .unit("px"),
            toggle(11, "Enabled", true).category("State"),
            toggle(12, "Visible", true).category("State"),
        ])
        .with_sort(Sort::Alphabetical);
        let _ = button.update(Event::Search("colour".into()));

        Self { button }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Button(event) = message;
        let _ = self.button.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let shown = self.button.entries().len();
        column![
            property_grid(&self.button)
                .description(false)
                .height(240)
                .on_event(Message::Button),
            text(format!("{shown} rows shown")).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(340)
        .into()
    }
}
