use iced::widget::{column, container, text};
use iced::{Border, Element, Length, Theme, border};
use iced_cube::alert_dialog;
use iced_cube::lucide;
use iced_cube::navigation::document_tabs::{Event, Output, State, document, document_tabs};
use iced_cube::theme::{Tokens, radius, text_size};

#[derive(Debug, Clone)]
pub enum Message {
    Tabs(Event<u32>),
    Discard,
    Cancel,
}

#[derive(Debug)]
pub struct Example {
    tabs: State<u32>,
    /// Documents waiting to close while the user decides about unsaved ones.
    pending: Vec<u32>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            tabs: State::new([
                document(1, "budget.csv").icon(lucide!(FileText)),
                document(2, "letter.md")
                    .icon(lucide!(FileText))
                    .modified(true),
                document(3, "todo.md").icon(lucide!(FileText)),
            ])
            .with_active(2),
            pending: vec![2],
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Tabs(event) => {
                let Some(Output::CloseRequested(ids)) = self.tabs.update(event) else {
                    return;
                };
                if self.unsaved(&ids).is_empty() {
                    let _ = self.tabs.close(&ids);
                } else {
                    self.pending = ids;
                }
            }
            Message::Discard => {
                let _ = self.tabs.close(&self.pending);
                self.pending.clear();
            }
            Message::Cancel => self.pending.clear(),
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
        let page = container(
            text("Close the modified tab to be asked first. Saved tabs close at once.")
                .size(text_size::SM)
                .style(muted),
        )
        .padding([12, 16])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(page);

        let editor = container(column![
            document_tabs(&self.tabs).on_event(Message::Tabs),
            page
        ])
        .padding(4)
        .width(Length::Fill)
        .max_width(480)
        .height(160)
        .style(window);

        let unsaved = self.unsaved(&self.pending).join(", ");
        alert_dialog(
            container(editor).padding(24).center_x(Length::Fill),
            format!("Close {unsaved} without saving?"),
            "Your changes are lost if you close without saving.",
        )
        .open(!self.pending.is_empty())
        .confirm("Close without saving")
        .on_cancel(Message::Cancel)
        .on_confirm(Message::Discard)
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
