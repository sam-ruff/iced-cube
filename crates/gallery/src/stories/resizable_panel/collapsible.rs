use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length};
use iced_cube::layout::card;
use iced_cube::layout::resizable_panel::{Event, Extent, State, panel, resizable_panel};
use iced_cube::primitives::button::{Size, Variant};
use iced_cube::theme::{Tokens, semibold, text_size};
use iced_cube::{button, icon, lucide};

const FOLDERS: [&str; 4] = ["Inbox", "Drafts", "Sent", "Archive"];

#[derive(Debug, Clone)]
pub enum Message {
    Panels(Event),
    Toggle,
}

#[derive(Debug)]
pub struct Example {
    panels: State,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            panels: State::new([
                panel(Extent::Fraction(0.35))
                    .min(Extent::Pixels(140.0))
                    .max(Extent::Fraction(0.5))
                    .collapsible(true),
                panel(Extent::Fraction(0.65)).min(Extent::Pixels(120.0)),
            ]),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Panels(event) => event,
            Message::Toggle => Event::Toggle(0),
        };
        let _ = self.panels.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let folders = column(FOLDERS.map(|name| {
            row![
                icon::icon(lucide!(Folder), 16.0),
                text(name).size(text_size::SM),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        }))
        .spacing(12)
        .padding(16);

        let collapsed = self.panels.is_collapsed(0);
        let label = if collapsed { "Show" } else { "Hide" };
        let content = container(
            column![
                text("Messages").size(text_size::SM).font(semibold()),
                text("Drag the handle below 70px and the folders snap shut. They open again past it, and stay between 140px and half the width.")
                    .size(text_size::XS)
                    .style(|theme| text::Style {
                        color: Some(Tokens::of(theme).muted_foreground),
                    }),
                button(label)
                    .icon(lucide!(PanelLeft))
                    .variant(Variant::Outline)
                    .size(Size::Sm)
                    .on_press(Message::Toggle),
            ]
            .spacing(8),
        )
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fill);

        container(
            resizable_panel(&self.panels, [folders.into(), content.into()])
                .grip(true)
                .on_event(Message::Panels),
        )
        .padding(1)
        .height(220)
        .style(|theme| card::style(&Tokens::of(theme)))
        .into()
    }
}
