use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::lucide;
use iced_cube::primitives::toggle_group::{self, State, Variant, item, toggle_group};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Centre,
    Right,
}

#[derive(Debug, Clone)]
pub enum Message {
    Align(toggle_group::Event<Align>),
}

#[derive(Debug)]
pub struct Example {
    align: State<Align>,
}

impl Default for Example {
    fn default() -> Self {
        let align = State::single([
            item(Align::Left, "Left").icon(lucide!(TextAlignStart)),
            item(Align::Centre, "Centre").icon(lucide!(TextAlignCenter)),
            item(Align::Right, "Right").icon(lucide!(TextAlignEnd)),
        ]);
        Self {
            align: align.required(true),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Align(event) = message;
        let _ = self.align.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let align = self
            .align
            .selected_one()
            .map_or(String::new(), |align| format!("Text aligned: {align:?}"));

        column![
            toggle_group(&self.align)
                .variant(Variant::Outline)
                .on_event(Message::Align),
            text(align).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(320)
        .into()
    }
}
