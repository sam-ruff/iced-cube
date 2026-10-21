use iced::widget::{Space, column, container, responsive, row, text};
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
        responsive(|size| {
            let columns = if size.width >= 596.0 {
                4
            } else if size.width >= 252.0 {
                2
            } else {
                1
            };
            let width =
                ((size.width - 12.0 * (columns - 1) as f32) / columns as f32).clamp(0.0, 140.0);
            let rows = SWATCHES.chunks(columns).map(|chunk| {
                row(chunk
                    .iter()
                    .map(|&(name, colour)| swatch(name, colour, width)))
                .spacing(12)
                .into()
            });
            column(rows).spacing(14).into()
        })
        .width(Length::Shrink)
        .height(Length::Shrink)
        .into()
    }
}

fn swatch(name: &'static str, colour: Pick, width: f32) -> Element<'static, Message> {
    let chip = container(Space::new())
        .width(Length::Fixed(width))
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
