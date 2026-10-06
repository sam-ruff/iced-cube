use iced::widget::{column, row, text};
use iced::{Element, Length};
use iced_cube::lucide;
use iced_cube::primitives::toggle;

#[derive(Debug, Clone)]
pub enum Message {
    Grid(bool),
    Rulers(bool),
    Snap(bool),
}

#[derive(Debug)]
pub struct Example {
    grid: bool,
    rulers: bool,
    snap: bool,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            grid: true,
            rulers: false,
            snap: false,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Grid(on) => self.grid = on,
            Message::Rulers(on) => self.rulers = on,
            Message::Snap(on) => self.snap = on,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let toggles = row![
            toggle("Grid")
                .icon(lucide!(Grid3x3))
                .pressed(self.grid)
                .on_toggle(Message::Grid),
            toggle("Rulers")
                .icon(lucide!(Ruler))
                .pressed(self.rulers)
                .on_toggle(Message::Rulers),
            toggle("Snap to grid")
                .icon(lucide!(Magnet))
                .pressed(self.snap)
                .on_toggle(Message::Snap),
        ]
        .spacing(4)
        .wrap();

        let showing: Vec<&str> = [(self.grid, "grid"), (self.rulers, "rulers")]
            .into_iter()
            .filter_map(|(on, name)| on.then_some(name))
            .collect();
        let status = match showing.as_slice() {
            [] => String::from("Showing the canvas only"),
            names => format!("Showing the {}", names.join(" and ")),
        };

        column![toggles, text(status).size(14)]
            .spacing(12)
            .width(Length::Fill)
            .max_width(340)
            .into()
    }
}
