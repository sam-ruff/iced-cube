use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::data::property_grid::{
    self, Action, Event, Output, State, choice, number, property_grid, text as text_property,
    toggle,
};
use iced_cube::keys::{Chord, Keymap};

#[derive(Debug, Clone)]
pub enum Message {
    Widget(Event<u8>),
}

#[derive(Debug)]
pub struct Example {
    widget: State<u8>,
    keymap: Keymap<Action>,
    last: String,
}

impl Default for Example {
    fn default() -> Self {
        let widget = State::new([
            text_property(1, "Name", "search_field").category("General"),
            text_property(2, "Placeholder", "Search").category("General"),
            toggle(3, "Enabled", true).category("General").default(true),
            number(4, "Width", 240.0)
                .category("Layout")
                .range(40.0, 800.0)
                .step(8.0)
                .unit("px")
                .default(200.0),
            number(5, "Margin", 8.0)
                .category("Layout")
                .range(0.0, 64.0)
                .step(1.0)
                .unit("px"),
            choice(6, "Align", ["Start", "Centre", "End"], "Start").category("Layout"),
        ]);

        // The defaults, plus J and K to move between rows.
        let keymap = property_grid::default_keymap()
            .bind(Chord::character('j'), Action::Next)
            .bind(Chord::character('k'), Action::Previous);

        Self {
            widget,
            keymap,
            last: "Nothing changed yet".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Widget(event) = message;
        if let Some(Output::Changed { id, .. } | Output::Reset { id, .. }) =
            self.widget.update(event)
            && let Some(property) = self.widget.property(id)
        {
            self.last = format!("{} is now {}", property.name, property.display());
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text(
                "Click the grid, then move with the arrows or J and K. Left and Right close \
                 and open a category, Enter or F2 edits, Up and Down step a number being \
                 edited, Escape cancels, Delete resets and Ctrl+F jumps to the filter."
            )
            .size(14),
            property_grid(&self.widget)
                .keymap(self.keymap.clone())
                .height(260)
                .on_event(Message::Widget),
            text(&self.last).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(360)
        .into()
    }
}
