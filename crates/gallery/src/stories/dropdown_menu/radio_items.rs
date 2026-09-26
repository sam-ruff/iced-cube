use iced::widget::{column, container, text};
use iced::{Element, Length};
use iced_cube::button;
use iced_cube::dropdown_menu::{
    Event, Output, State, dropdown_menu, group_label, radio_item, separator,
};
use iced_cube::primitives::button::Variant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
    Top,
    Bottom,
    Right,
}

#[derive(Debug, Clone)]
pub enum Message {
    Menu(Event<Position>),
}

#[derive(Debug)]
pub struct Example {
    menu: State<Position>,
    position: Position,
}

impl Default for Example {
    fn default() -> Self {
        let mut menu = State::new([
            group_label("Panel position"),
            separator(),
            radio_item(Position::Top, "Top", false),
            radio_item(Position::Bottom, "Bottom", true),
            radio_item(Position::Right, "Right", false),
        ]);
        let _ = menu.update(Event::Open);
        Self {
            menu,
            position: Position::Bottom,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Menu(event) = message;
        if let Some(Output::Selected(position)) = self.menu.update(event) {
            self.position = position;
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let trigger = button("Panel")
            .variant(Variant::Outline)
            .on_press(Message::Menu(Event::Toggle));

        column![
            text(format!("Panel position: {:?}", self.position)).size(14),
            container(dropdown_menu(&self.menu, trigger).on_event(Message::Menu))
                .height(Length::Fill),
        ]
        .spacing(12)
        .width(240)
        .into()
    }
}
