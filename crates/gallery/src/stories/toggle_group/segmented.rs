use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::lucide;
use iced_cube::primitives::toggle_group::{self, State, Variant, item, toggle_group};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone)]
pub enum Message {
    Appearance(toggle_group::Event<Appearance>),
}

#[derive(Debug)]
pub struct Example {
    appearance: State<Appearance>,
}

impl Default for Example {
    fn default() -> Self {
        let appearance = State::single([
            item(Appearance::System, "System").icon(lucide!(Monitor)),
            item(Appearance::Light, "Light").icon(lucide!(Sun)),
            item(Appearance::Dark, "Dark").icon(lucide!(Moon)),
        ]);
        Self {
            appearance: appearance.required(true),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Appearance(event) = message;
        let _ = self.appearance.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let chosen = self
            .appearance
            .selected_one()
            .map_or(String::new(), |appearance| {
                format!("Appearance: {appearance:?}")
            });

        // A fill width shares the row equally, like a segmented control.
        column![
            text("Appearance").size(14),
            toggle_group(&self.appearance)
                .variant(Variant::Outline)
                .width(Length::Fill)
                .on_event(Message::Appearance),
            text(chosen).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(360)
        .into()
    }
}
