use iced::widget::{column, text};
use iced::{Element, Length, Subscription};
use iced_cube::keys::{self, Keymap};
use iced_cube::primitives::toggle_group::{
    self, Action, Event, State, Variant, item, toggle_group,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone)]
pub enum Message {
    Range(Event<Range>),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    range: State<Range>,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        let range = State::single([
            item(Range::Day, "Day"),
            item(Range::Week, "Week"),
            item(Range::Month, "Month"),
            item(Range::Year, "Year"),
        ]);
        Self {
            range: range.with_selected([Range::Week]).required(true),
            keymap: toggle_group::default_keymap(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Range(event) => Some(event),
            Message::Key(key) => self
                .keymap
                .resolve_event(&key)
                .and_then(|action| action.event(&self.range)),
        };
        if let Some(event) = event {
            let _ = self.range.update(event);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            toggle_group(&self.range)
                .variant(Variant::Outline)
                .on_event(Message::Range),
            text("The left and right arrow keys move the selection.").size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(360)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
