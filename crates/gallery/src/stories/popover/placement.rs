use iced::widget::{column, row, text};
use iced::{Element, Length};
use iced_cube::button;
use iced_cube::overlay::popover::{Align, Side, popover};
use iced_cube::primitives::button::{Size, Variant};
use iced_cube::theme::text_size;

#[derive(Debug, Clone)]
pub enum Message {
    Toggle(Side),
    Align(Align),
    Dismiss,
}

#[derive(Debug)]
pub struct Example {
    open: Option<Side>,
    last: Side,
    align: Align,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            open: Some(Side::Bottom),
            last: Side::Bottom,
            align: Align::Start,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Toggle(side) => {
                self.open = (self.open != Some(side)).then_some(side);
                self.last = side;
            }
            // A click on an alignment also dismisses the popover, so reopen it.
            Message::Align(align) => {
                self.align = align;
                self.open = Some(self.last);
            }
            Message::Dismiss => self.open = None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let name = |align| match align {
            Align::Start => "Start",
            Align::Center => "Centre",
            Align::End => "End",
        };

        let alignments = row(Align::ALL.map(|align| {
            let variant = if align == self.align {
                Variant::Secondary
            } else {
                Variant::Ghost
            };
            button(name(align))
                .size(Size::Sm)
                .variant(variant)
                .on_press(Message::Align(align))
                .into()
        }))
        .spacing(4);

        let sides = row(Side::ALL.map(|side| {
            let label = text(format!(
                "{side:?}, aligned to {}",
                name(self.align).to_lowercase()
            ))
            .size(text_size::SM);
            popover(
                button(format!("{side:?}"))
                    .variant(Variant::Outline)
                    .on_press(Message::Toggle(side)),
                label,
            )
            .open(self.open == Some(side))
            .side(side)
            .align(self.align)
            .width(Length::Shrink)
            .padding(12.0)
            .on_dismiss(Message::Dismiss)
            .into()
        }))
        .spacing(8);

        column![alignments, sides].spacing(16).into()
    }
}
