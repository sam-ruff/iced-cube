use iced::widget::{container, text};
use iced::{Element, Length};
use iced_cube::layout::card;
use iced_cube::layout::resizable_panel::{Axis, Event, Extent, State, panel, resizable_panel};
use iced_cube::theme::{Tokens, semibold, text_size};

#[derive(Debug, Clone)]
pub enum Message {
    Outer(Event),
    Inner(Event),
}

#[derive(Debug)]
pub struct Example {
    outer: State,
    inner: State,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            outer: State::new([
                panel(Extent::Fraction(0.35)).min(Extent::Pixels(96.0)),
                panel(Extent::Fraction(0.65)).min(Extent::Pixels(160.0)),
            ]),
            inner: State::new([
                panel(Extent::Fraction(0.6)).min(Extent::Pixels(40.0)),
                panel(Extent::Fraction(0.4)).min(Extent::Pixels(40.0)),
            ]),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let _ = match message {
            Message::Outer(event) => self.outer.update(event),
            Message::Inner(event) => self.inner.update(event),
        };
    }

    pub fn view(&self) -> Element<'_, Message> {
        let editor = resizable_panel(&self.inner, [pane("Editor"), pane("Terminal")])
            .axis(Axis::Vertical)
            .on_event(Message::Inner);

        container(
            resizable_panel(&self.outer, [pane("Files"), editor.into()])
                .stack_below(480.0)
                .on_event(Message::Outer),
        )
        .padding(1)
        .height(240)
        .style(|theme| card::style(&Tokens::of(theme)))
        .into()
    }
}

fn pane(name: &str) -> Element<'_, Message> {
    container(text(name).size(text_size::SM).font(semibold()))
        .center(Length::Fill)
        .into()
}
