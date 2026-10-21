use iced::keyboard::key::Named;
use iced::widget::{self, column, container, text};
use iced::{Element, Length, Task};
use iced_cube::button;
use iced_cube::keys::{Chord, Keymap};
use iced_cube::layout::card;
use iced_cube::layout::resizable_panel::{
    self, Action, Event, Extent, State, panel, resizable_panel,
};
use iced_cube::primitives::button::{Size, Variant};
use iced_cube::theme::{Tokens, semibold, text_size};

const HANDLE: widget::Id = widget::Id::new("keyboard-handle");

#[derive(Debug, Clone)]
pub enum Message {
    Panels(Event),
    FocusHandle,
}

#[derive(Debug)]
pub struct Example {
    panels: State,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            panels: State::new([
                panel(Extent::Fraction(0.4))
                    .min(Extent::Pixels(80.0))
                    .collapsible(true),
                panel(Extent::Fraction(0.6)).min(Extent::Pixels(120.0)),
            ]),
            keymap: resizable_panel::default_keymap()
                .bind(Chord::named(Named::PageUp), Action::DecreaseMore)
                .bind(Chord::named(Named::PageDown), Action::IncreaseMore),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Panels(event) => {
                let _ = self.panels.update(event);
                Task::none()
            }
            Message::FocusHandle => widget::operation::focus(HANDLE),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let share = self.panels.sizes().first().copied().unwrap_or_default();
        let left = container(
            text(format!("{:.0}%", share * 100.0))
                .size(text_size::SM)
                .font(semibold()),
        )
        .center(Length::Fill);
        let right = container(
            button("Focus the handle")
                .variant(Variant::Outline)
                .size(Size::Sm)
                .on_press(Message::FocusHandle),
        )
        .center(Length::Fill);

        column![
            container(
                resizable_panel(&self.panels, [left.into(), right.into()])
                    .id(HANDLE)
                    .grip(true)
                    .keymap(self.keymap.clone())
                    .on_event(Message::Panels),
            )
            .padding(1)
            .height(140)
            .style(|theme| card::style(&Tokens::of(theme))),
            text("Click the handle or the button, then use the arrows to step by 5%. Shift, Page Up or Page Down step by 20%, Home and End go to the limits, Enter collapses the left panel and Escape lets go.")
                .size(text_size::SM),
        ]
        .spacing(16)
        .width(Length::Fill)
        .max_width(480)
        .into()
    }
}
