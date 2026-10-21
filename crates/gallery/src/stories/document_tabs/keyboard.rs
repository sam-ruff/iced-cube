use iced::keyboard::key::Named;
use iced::widget::{column, container, text};
use iced::{Border, Element, Length, Subscription, Theme, border};
use iced_cube::keys::{self, Chord, Keymap};
use iced_cube::lucide;
use iced_cube::navigation::document_tabs::{
    self, Action, Event, Output, State, document, document_tabs,
};
use iced_cube::theme::{Tokens, radius, text_size};

#[derive(Debug, Clone)]
pub enum Message {
    Tabs(Event<u32>),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    tabs: State<u32>,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        // Browsers keep Ctrl+Tab, Ctrl+W and Ctrl+PageDown for their own
        // tabs, so the example adds Alt chords that reach the page.
        let keymap = document_tabs::default_keymap()
            .bind(Chord::named(Named::PageDown).alt(), Action::Next)
            .bind(Chord::named(Named::PageUp).alt(), Action::Previous)
            .bind(
                Chord::named(Named::PageDown).alt().shift(),
                Action::MoveRight,
            )
            .bind(Chord::named(Named::PageUp).alt().shift(), Action::MoveLeft)
            .bind(Chord::character('w').alt(), Action::Close);

        Self {
            tabs: State::new([
                document(1, "index.html").icon(lucide!(FileCode)),
                document(2, "styles.css").icon(lucide!(FileCode)),
                document(3, "app.js").icon(lucide!(FileCode)).modified(true),
                document(4, "favicon.svg").icon(lucide!(FileText)),
            ]),
            keymap,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Tabs(event) => Some(event),
            Message::Key(key) => self
                .keymap
                .resolve_event(&key)
                .and_then(|action| action.event(&self.tabs)),
        };
        let Some(event) = event else {
            return;
        };
        if let Some(Output::CloseRequested(ids)) = self.tabs.update(event) {
            let _ = self.tabs.close(&ids);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let recent: Vec<&str> = self
            .tabs
            .recent()
            .iter()
            .filter_map(|id| self.tabs.document(*id))
            .map(|doc| doc.label.as_str())
            .collect();
        let page = container(
            column![
                text("Alt+PageUp and Alt+PageDown switch tabs, add Shift to move one, Alt+W closes. In the desktop app, Ctrl+Tab switches to the last used tab.")
                    .size(text_size::SM),
                text(format!("Recently used: {}", recent.join(", ")))
                    .size(text_size::SM)
                    .style(muted),
            ]
            .spacing(8),
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
        .height(200)
        .style(window)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            keys::subscription().map(Message::Key),
            document_tabs::cycle_end().map(|end| Message::Tabs(end.into())),
        ])
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
