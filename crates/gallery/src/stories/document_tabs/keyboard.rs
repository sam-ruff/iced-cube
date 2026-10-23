use iced::keyboard::key::Named;
use iced::widget::{column, container, row, text};
use iced::{Border, Element, Length, Subscription, Theme, border};
use iced_cube::button;
use iced_cube::keys::{self, Chord, Keymap};
use iced_cube::lucide;
use iced_cube::navigation::document_tabs::{
    self, Action, Event, Output, State, document, document_tabs,
};
use iced_cube::primitives::button::{Size, Variant};
use iced_cube::theme::{Tokens, radius, text_size};

#[derive(Debug, Clone)]
pub enum Message {
    Tabs(Event<u32>),
    Key(keys::Event),
    Discard,
    Cancel,
}

#[derive(Debug)]
pub struct Example {
    tabs: State<u32>,
    keymap: Keymap<Action>,
    /// Documents waiting to close while the user decides about unsaved ones.
    pending: Vec<u32>,
}

impl Default for Example {
    fn default() -> Self {
        // Browsers keep Ctrl+Tab, Ctrl+W and Ctrl+PageDown for their own
        // tabs, so the example adds chords that reach the page.
        let keymap = document_tabs::default_keymap()
            .bind(Chord::character(']').ctrl(), Action::Next)
            .bind(Chord::character('[').ctrl(), Action::Previous)
            .bind(Chord::character('.').ctrl(), Action::MoveRight)
            .bind(Chord::character(',').ctrl(), Action::MoveLeft)
            .bind(Chord::named(Named::Backspace).ctrl(), Action::Close);

        Self {
            tabs: State::new([
                document(1, "index.html").icon(lucide!(FileCode)),
                document(2, "styles.css").icon(lucide!(FileCode)),
                document(3, "app.js").icon(lucide!(FileCode)).modified(true),
                document(4, "favicon.svg").icon(lucide!(FileText)),
            ]),
            keymap,
            pending: Vec::new(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Tabs(event) => event,
            Message::Key(_) if !self.pending.is_empty() => return,
            Message::Key(key) => {
                let action = self.keymap.resolve_event(&key);
                let Some(event) = action.and_then(|action| action.event(&self.tabs)) else {
                    return;
                };
                event
            }
            Message::Discard => {
                let _ = self.tabs.close(&self.pending);
                self.pending.clear();
                return;
            }
            Message::Cancel => {
                self.pending.clear();
                return;
            }
        };
        let Some(Output::CloseRequested(ids)) = self.tabs.update(event) else {
            return;
        };
        if self.unsaved(&ids).is_empty() {
            let _ = self.tabs.close(&ids);
        } else {
            self.pending = ids;
        }
    }

    fn unsaved(&self, ids: &[u32]) -> Vec<&str> {
        ids.iter()
            .filter_map(|id| self.tabs.document(*id))
            .filter(|doc| doc.modified)
            .map(|doc| doc.label.as_str())
            .collect()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = if self.pending.is_empty() {
            let recent: Vec<&str> = self
                .tabs
                .recent()
                .iter()
                .filter_map(|id| self.tabs.document(*id))
                .map(|doc| doc.label.as_str())
                .collect();
            column![
                text("Ctrl+[ and Ctrl+] switch tabs, Ctrl+, and Ctrl+. move one, Ctrl+Backspace closes. In the desktop app, Ctrl+Tab switches to the last used tab.")
                    .size(text_size::SM),
                text(format!("Recently used: {}", recent.join(", ")))
                    .size(text_size::SM)
                    .style(muted),
            ]
            .spacing(8)
            .into()
        } else {
            let unsaved = self.unsaved(&self.pending).join(", ");
            column![
                text(format!("Close {unsaved} without saving?")).size(text_size::SM),
                row![
                    button("Cancel")
                        .variant(Variant::Outline)
                        .size(Size::Sm)
                        .on_press(Message::Cancel),
                    button("Close without saving")
                        .variant(Variant::Destructive)
                        .size(Size::Sm)
                        .on_press(Message::Discard),
                ]
                .spacing(8)
                .wrap(),
            ]
            .spacing(12)
            .into()
        };
        let page = container(body)
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
