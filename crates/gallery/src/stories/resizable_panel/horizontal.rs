use iced::widget::{column, container, text};
use iced::{Alignment, Element, Length};
use iced_cube::layout::card;
use iced_cube::layout::resizable_panel::{Event, Extent, State, panel, resizable_panel};
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
                panel(Extent::Fraction(0.25)).min(Extent::Pixels(64.0)),
                panel(Extent::Fraction(0.5)).min(Extent::Pixels(64.0)),
                panel(Extent::Fraction(0.25)).min(Extent::Pixels(64.0)),
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
        let sizes = self.panels.sizes();
        let panes = ["One", "Two", "Three"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
                let share = sizes.get(index).copied().unwrap_or_default();
                pane(name, format!("{:.0}%", share * 100.0))
            });

        container(
            resizable_panel(&self.panels, panes)
                .grip(true)
                .on_event(Message::Panels),
        )
        .padding(1)
        .height(200)
        .style(|theme| card::style(&Tokens::of(theme)))
        .into()
    }
}

fn pane<'a>(name: &'a str, share: String) -> Element<'a, Message> {
    container(
        column![
            text(name).size(text_size::SM).font(semibold()),
            text(share).size(text_size::XS).style(|theme| text::Style {
                color: Some(Tokens::of(theme).muted_foreground),
            }),
        ]
        .spacing(2)
        .align_x(Alignment::Center),
    )
    .center(Length::Fill)
    .into()
}
