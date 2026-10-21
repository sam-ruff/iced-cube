use iced::widget::{column, text};
use iced::{Color, Element, Length};
use iced_cube::data::property_grid::{
    Entry, Event, Output, Property, State, choice, colour, custom, number, property_grid,
    read_only, toggle,
};
use iced_cube::primitives::slider;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    Id,
    X,
    Y,
    Width,
    Height,
    Rotation,
    Fill,
    Stroke,
    Opacity,
    Radius,
    Blend,
    Visible,
    Locked,
}

#[derive(Debug, Clone)]
pub enum Message {
    Shape(Event<Field>),
    Opacity(u8),
}

#[derive(Debug)]
pub struct Example {
    shape: State<Field>,
    last: String,
}

impl Default for Example {
    fn default() -> Self {
        let mut shape = State::new([
            read_only(Field::Id, "Id", "rect-12")
                .description("Given to the shape when it was drawn."),
            number(Field::X, "X", 48.0)
                .category("Layout")
                .step(1.0)
                .unit("px"),
            number(Field::Y, "Y", 24.0)
                .category("Layout")
                .step(1.0)
                .unit("px"),
            number(Field::Width, "Width", 160.0)
                .category("Layout")
                .min(1.0)
                .step(1.0)
                .unit("px")
                .default(120.0)
                .description("Width of the bounding box, before rotation."),
            number(Field::Height, "Height", 96.0)
                .category("Layout")
                .min(1.0)
                .step(1.0)
                .unit("px")
                .default(96.0),
            number(Field::Rotation, "Rotation", 0.0)
                .category("Layout")
                .range(-180.0, 180.0)
                .step(1.0)
                .unit("°")
                .default(0.0),
            colour(Field::Fill, "Fill", Color::from_rgb8(0x3b, 0x82, 0xf6))
                .category("Appearance")
                .default(Color::WHITE),
            colour(Field::Stroke, "Stroke", Color::from_rgb8(0x1e, 0x29, 0x3b))
                .category("Appearance"),
            custom(Field::Opacity, "Opacity", 80.0)
                .category("Appearance")
                .default(100.0)
                .description("Drawn by the app with a slider."),
            number(Field::Radius, "Corner radius", 8.0)
                .category("Appearance")
                .range(0.0, 64.0)
                .step(1.0)
                .unit("px")
                .default(0.0),
            choice(
                Field::Blend,
                "Blend mode",
                ["Normal", "Multiply", "Screen", "Overlay"],
                "Normal",
            )
            .category("Appearance")
            .default("Normal"),
            toggle(Field::Visible, "Visible", true)
                .switch()
                .category("Behaviour")
                .default(true),
            toggle(Field::Locked, "Locked", false)
                .category("Behaviour")
                .default(false),
        ]);
        let _ = shape.update(Event::Press(Entry::Property(Field::Width)));

        Self {
            shape,
            last: "Click a value to edit it.".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Shape(event) => {
                if let Some(Output::Changed { id, .. } | Output::Reset { id, .. }) =
                    self.shape.update(event)
                {
                    self.describe(id);
                }
            }
            Message::Opacity(opacity) => {
                let _ = self.shape.set(Field::Opacity, f64::from(opacity));
                self.describe(Field::Opacity);
            }
        }
    }

    fn describe(&mut self, id: Field) {
        if let Some(property) = self.shape.property(id) {
            self.last = format!("{} is now {}", property.name, property.display());
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            property_grid(&self.shape)
                .custom(Field::Opacity, opacity)
                .height(340)
                .on_event(Message::Shape),
            text(&self.last).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(340)
        .into()
    }
}

fn opacity(property: &Property<Field>) -> Element<'_, Message> {
    let value = property.value.as_number().unwrap_or(100.0) as u8;
    slider(0..=100, value).on_change(Message::Opacity).into()
}
