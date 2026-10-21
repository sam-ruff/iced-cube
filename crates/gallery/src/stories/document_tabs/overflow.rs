use iced::widget::{column, container, text};
use iced::{Border, Element, Length, Theme, border};
use iced_cube::lucide;
use iced_cube::navigation::document_tabs::{Event, Output, State, document, document_tabs};
use iced_cube::theme::{Tokens, radius, text_size};

const CHAPTERS: [&str; 12] = [
    "Introduction",
    "Getting started",
    "Installation",
    "Configuration",
    "Command line",
    "Plugins",
    "Theming",
    "Keyboard shortcuts",
    "Accessibility",
    "Deployment",
    "Troubleshooting",
    "Release notes",
];

#[derive(Debug, Clone)]
pub enum Message {
    Tabs(Event<usize>),
}

#[derive(Debug)]
pub struct Example {
    tabs: State<usize>,
}

impl Default for Example {
    fn default() -> Self {
        let documents = CHAPTERS.iter().enumerate().map(|(index, title)| {
            document(index, format!("{title}.md"))
                .icon(lucide!(FileText))
                .modified(index == 4)
        });
        Self {
            tabs: State::new(documents).with_active(9),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Tabs(event) = message;
        if let Some(Output::CloseRequested(ids)) = self.tabs.update(event) {
            let _ = self.tabs.close(&ids);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let title = self
            .tabs
            .active()
            .and_then(|id| self.tabs.document(id))
            .map_or("", |doc| doc.label.as_str());
        let page = container(
            column![
                text(title).size(text_size::SM),
                text("Scroll the strip with the wheel, or pick a tab from the list at its end.")
                    .size(text_size::SM)
                    .style(muted),
            ]
            .spacing(4),
        )
        .padding([12, 16])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(page);

        container(column![
            document_tabs(&self.tabs).on_event(Message::Tabs),
            page
        ])
        .padding(4)
        .width(Length::Fill)
        .max_width(560)
        .height(140)
        .style(window)
        .into()
    }
}

fn window(theme: &Theme) -> container::Style {
    let tokens = Tokens::of(theme);
    container::Style {
        background: Some(tokens.muted.into()),
        border: Border {
            color: tokens.border,
            width: 1.0,
            radius: radius::LG.into(),
        },
        ..container::Style::default()
    }
}

fn page(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Tokens::of(theme).background.into()),
        border: border::rounded(border::bottom(radius::SM)),
        ..container::Style::default()
    }
}

fn muted(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(Tokens::of(theme).muted_foreground),
    }
}
