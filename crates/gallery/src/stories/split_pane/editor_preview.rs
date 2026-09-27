use iced::widget::text_editor::{Action, Content};
use iced::widget::{column, container, text};
use iced::{Element, Length};
use iced_cube::layout::card;
use iced_cube::layout::split_pane::{Event, Extent, State, split_pane};
use iced_cube::theme::{Tokens, semibold, text_size};
use iced_cube::{scroll_area, textarea};

const DRAFT: &str =
    "# Release notes\n\nPanels can now be resized.\n\n# Fixes\n\nMenus close on Escape.";

#[derive(Debug, Clone)]
pub enum Message {
    Edit(Action),
    Split(Event),
}

#[derive(Debug)]
pub struct Example {
    content: Content,
    split: State,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            content: Content::with_text(DRAFT),
            split: State::new(0.5).min(Extent::Pixels(120.0), Extent::Pixels(120.0)),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Edit(action) => self.content.perform(action),
            Message::Split(event) => {
                let _ = self.split.update(event);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let editor = container(
            textarea(&self.content)
                .height(Length::Fill)
                .on_action(Message::Edit),
        )
        .padding(12);

        let preview = self
            .content
            .lines()
            .filter(|line| !line.text.trim().is_empty())
            .map(|line| match line.text.strip_prefix("# ") {
                Some(heading) => text(heading.to_owned())
                    .size(text_size::LG)
                    .font(semibold())
                    .into(),
                None => text(line.text.to_string()).size(text_size::SM).into(),
            });
        let preview = scroll_area(column(preview).spacing(8).padding(16).width(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill);

        container(
            split_pane(&self.split, editor, preview)
                .stack_below(480.0)
                .on_event(Message::Split),
        )
        .padding(1)
        .height(300)
        .style(|theme| card::style(&Tokens::of(theme)))
        .into()
    }
}
