use iced::widget::{Space, column, container, row, text};
use iced::{Background, Color, Element, Length, Theme};
use iced_cube::theme::{Tokens, radius};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

/// Reads one colour from the resolved tokens.
type Pick = fn(&Tokens) -> Color;

/// Every token and how to read it from [`Tokens`].
const SWATCHES: [(&str, Pick); 12] = [
    ("background", |t| t.background),
    ("foreground", |t| t.foreground),
    ("muted", |t| t.muted),
    ("muted_foreground", |t| t.muted_foreground),
    ("border", |t| t.border),
    ("ring", |t| t.ring),
    ("primary", |t| t.primary),
    ("secondary", |t| t.secondary),
    ("accent", |t| t.accent),
    ("destructive", |t| t.destructive),
    ("success", |t| t.success),
    ("warning", |t| t.warning),
];

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {}
    }

    pub fn view(&self) -> Element<'_, Message> {
        let rows = SWATCHES.chunks(4).map(|chunk| {
            row(chunk.iter().map(|&(name, colour)| swatch(name, colour)))
                .spacing(12)
                .into()
        });

        column(rows).spacing(14).into()
    }
}

fn swatch(name: &'static str, colour: Pick) -> Element<'static, Message> {
    let chip = container(Space::new())
        .width(Length::Fixed(140.0))
        .height(Length::Fixed(36.0))
        .style(move |theme: &Theme| {
            let tokens = Tokens::of(theme);
            container::Style {
                background: Some(Background::Color(colour(&tokens))),
                border: iced::Border {
                    color: tokens.border,
                    width: 1.0,
                    radius: radius::MD.into(),
                },
                ..container::Style::default()
            }
        });

    column![chip, text(name).size(12)].spacing(4).into()
}
