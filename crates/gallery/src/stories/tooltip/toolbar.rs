use iced::widget::{column, row, text};
use iced::{Alignment, Element};
use iced_cube::overlay::tooltip::tooltip;
use iced_cube::{Glyph, icon_button, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    Format(&'static str),
}

#[derive(Debug, Default)]
pub struct Example {
    last: Option<&'static str>,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Format(name) = message;
        self.last = Some(name);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let tool = |glyph: Glyph, name: &'static str| {
            tooltip(icon_button(glyph).on_press(Message::Format(name)), name)
        };

        let status = match self.last {
            Some(name) => format!("Applied: {name}"),
            None => String::from("Hover an icon to see what it does."),
        };

        column![
            row![
                tool(lucide!(Bold), "Bold"),
                tool(lucide!(Italic), "Italic"),
                tool(lucide!(Underline), "Underline"),
                tool(lucide!(Link), "Insert link"),
            ]
            .spacing(4),
            text(status).size(14),
        ]
        .spacing(12)
        .align_x(Alignment::Center)
        .into()
    }
}
