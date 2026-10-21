use iced::widget::{column, container, text};
use iced::{Element, Length};
use iced_cube::layout::card;
use iced_cube::layout::resizable_panel::{Axis, Event, Extent, State, panel, resizable_panel};
use iced_cube::theme::{Tokens, semibold, text_size};

#[derive(Debug, Clone)]
pub enum Message {
    Panels(Event),
}

#[derive(Debug)]
pub struct Example {
    panels: State,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            panels: State::new([
                panel(Extent::Fraction(0.3)).min(Extent::Pixels(48.0)),
                panel(Extent::Fraction(0.7)).min(Extent::Pixels(64.0)),
            ]),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Panels(event) = message;
        let _ = self.panels.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header =
            container(text("Header").size(text_size::SM).font(semibold())).center(Length::Fill);
        let body = container(
            column![
                text("Content").size(text_size::SM).font(semibold()),
                text("Drag the handle up or down.")
                    .size(text_size::XS)
                    .style(|theme| text::Style {
                        color: Some(Tokens::of(theme).muted_foreground),
                    }),
            ]
            .spacing(2)
            .align_x(iced::Alignment::Center),
        )
        .center(Length::Fill);

        container(
            resizable_panel(&self.panels, [header.into(), body.into()])
                .axis(Axis::Vertical)
                .on_event(Message::Panels),
        )
        .padding(1)
        .width(Length::Fill)
        .max_width(420)
        .height(240)
        .style(|theme| card::style(&Tokens::of(theme)))
        .into()
    }
}
