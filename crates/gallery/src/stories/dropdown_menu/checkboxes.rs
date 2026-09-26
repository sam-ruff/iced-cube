use iced::widget::{column, container, text};
use iced::{Element, Length};
use iced_cube::button;
use iced_cube::dropdown_menu::{
    Event, State, checkbox_item, dropdown_menu, group_label, separator,
};
use iced_cube::primitives::button::Variant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    StatusBar,
    ActivityBar,
    Minimap,
}

#[derive(Debug, Clone)]
pub enum Message {
    Menu(Event<Panel>),
}

#[derive(Debug)]
pub struct Example {
    menu: State<Panel>,
}

impl Default for Example {
    fn default() -> Self {
        let mut menu = State::new([
            group_label("Appearance"),
            separator(),
            checkbox_item(Panel::StatusBar, "Status bar", true),
            checkbox_item(Panel::ActivityBar, "Activity bar", false).disabled(true),
            checkbox_item(Panel::Minimap, "Minimap", false),
        ]);
        let _ = menu.update(Event::Open);
        Self { menu }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Menu(event) = message;
        let _ = self.menu.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let shown: Vec<&str> = [Panel::StatusBar, Panel::ActivityBar, Panel::Minimap]
            .into_iter()
            .filter(|&panel| self.menu.is_checked(panel))
            .filter_map(|panel| self.menu.item(panel))
            .map(|item| item.label.as_str())
            .collect();
        let summary = match shown.as_slice() {
            [] => "Showing nothing".to_string(),
            shown => format!("Showing: {}", shown.join(", ")),
        };
        let trigger = button("View")
            .variant(Variant::Outline)
            .on_press(Message::Menu(Event::Toggle));

        column![
            text(summary).size(14),
            container(dropdown_menu(&self.menu, trigger).on_event(Message::Menu))
                .height(Length::Fill),
        ]
        .spacing(12)
        .width(240)
        .into()
    }
}
