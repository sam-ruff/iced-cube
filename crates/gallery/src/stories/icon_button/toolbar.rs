use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length};
use iced_cube::{Glyph, icon_button, lucide, vertical_separator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleBold,
    ToggleItalic,
    ToggleUnderline,
    Align(Align),
}

#[derive(Debug)]
pub struct Example {
    bold: bool,
    italic: bool,
    underline: bool,
    align: Align,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            bold: true,
            italic: false,
            underline: false,
            align: Align::Start,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::ToggleBold => self.bold = !self.bold,
            Message::ToggleItalic => self.italic = !self.italic,
            Message::ToggleUnderline => self.underline = !self.underline,
            Message::Align(align) => self.align = align,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let toggle = |glyph: Glyph, label: &'static str, on: bool, message: Message| {
            icon_button(glyph)
                .label(label)
                .pressed(on)
                .on_press(message)
        };
        let align = |glyph: Glyph, label: &'static str, align: Align| {
            toggle(glyph, label, self.align == align, Message::Align(align))
        };

        let toolbar = row![
            toggle(lucide!(Bold), "Bold", self.bold, Message::ToggleBold),
            toggle(
                lucide!(Italic),
                "Italic",
                self.italic,
                Message::ToggleItalic
            ),
            toggle(
                lucide!(Underline),
                "Underline",
                self.underline,
                Message::ToggleUnderline
            ),
            container(vertical_separator()).height(20),
            align(lucide!(TextAlignStart), "Align left", Align::Start),
            align(lucide!(TextAlignCenter), "Align centre", Align::Center),
            align(lucide!(TextAlignEnd), "Align right", Align::End),
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        let on: Vec<&str> = [
            (self.bold, "bold"),
            (self.italic, "italic"),
            (self.underline, "underline"),
        ]
        .into_iter()
        .filter_map(|(on, name)| on.then_some(name))
        .collect();
        let status = match on.as_slice() {
            [] => String::from("Plain text"),
            names => format!("Formatting: {}", names.join(", ")),
        };

        column![toolbar, text(status).size(14)]
            .spacing(12)
            .width(Length::Fixed(260.0))
            .into()
    }
}
