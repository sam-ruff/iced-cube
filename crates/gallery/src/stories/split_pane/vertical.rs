use iced::widget::{column, container, row, text};
use iced::{Element, Length};
use iced_cube::layout::card;
use iced_cube::layout::split_pane::{Axis, Event, Extent, State, split_pane};
use iced_cube::scroll_area;
use iced_cube::theme::{Tokens, semibold, text_size};

const RESULTS: [(&str, &str); 5] = [
    ("Nightly backup", "Succeeded"),
    ("Search index", "Running"),
    ("Invoice export", "Queued"),
    ("Image resize", "Failed"),
    ("Cache warm-up", "Succeeded"),
];

#[derive(Debug, Clone)]
pub enum Message {
    Split(Event),
}

#[derive(Debug)]
pub struct Example {
    split: State,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            split: State::new(0.4).min(Extent::Pixels(56.0), Extent::Pixels(72.0)),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Split(event) = message;
        let _ = self.split.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let query = container(
            column![
                text("Query").size(text_size::SM).font(semibold()),
                text("status != archived, sorted by name")
                    .size(text_size::SM)
                    .style(muted),
            ]
            .spacing(4),
        )
        .padding(16)
        .width(Length::Fill);

        let rows = RESULTS.map(|(name, status)| {
            row![
                text(name).size(text_size::SM).width(Length::Fill),
                text(status).size(text_size::SM).style(muted),
            ]
            .into()
        });
        let results = scroll_area(column(rows).spacing(10).padding(16))
            .width(Length::Fill)
            .height(Length::Fill);

        container(
            split_pane(&self.split, query, results)
                .axis(Axis::Vertical)
                .on_event(Message::Split),
        )
        .padding(1)
        .width(Length::Fill)
        .max_width(480)
        .height(300)
        .style(|theme| card::style(&Tokens::of(theme)))
        .into()
    }
}

fn muted(theme: &iced::Theme) -> text::Style {
    text::Style {
        color: Some(Tokens::of(theme).muted_foreground),
    }
}
