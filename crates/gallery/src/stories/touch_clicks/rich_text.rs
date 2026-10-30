use iced::widget::{column, rich_text, span, text};
use iced::{Element, Length, Theme};
use iced_cube::theme::{self, Tokens};
use iced_cube::touch_clicks;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Guide,
    Changelog,
}

#[derive(Debug, Clone)]
pub enum Message {
    Open(Link),
}

#[derive(Debug, Default)]
pub struct Example {
    opened: Option<Link>,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Open(link) = message;
        self.opened = Some(link);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let link = |label: &'static str, target: Link| {
            span(label)
                .link(target)
                .font(theme::semibold())
                .underline(true)
        };

        // Rich text and markdown only read the mouse, so taps are turned
        // into clicks for them. A drag over the text is left alone.
        let paragraph = touch_clicks(
            rich_text![
                span("Read the "),
                link("guide", Link::Guide),
                span(" to get started, or the "),
                link("changelog", Link::Changelog),
                span(" to see what is new."),
            ]
            .size(14)
            .font(theme::font())
            .on_link_click(Message::Open),
        );

        let status = match self.opened {
            Some(link) => format!("Opened the {link:?} link"),
            None => "Tap or click a link.".to_owned(),
        };

        column![
            paragraph,
            text(status).size(14).style(|theme: &Theme| text::Style {
                color: Some(Tokens::of(theme).muted_foreground),
            }),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(320)
        .into()
    }
}
