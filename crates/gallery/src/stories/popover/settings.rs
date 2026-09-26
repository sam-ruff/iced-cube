use iced::widget::{column, container, row, text};
use iced::{Alignment, Element};
use iced_cube::overlay::popover::popover;
use iced_cube::primitives::{button::Variant, input::Size};
use iced_cube::theme::{Tokens, semibold, text_size};
use iced_cube::{button, input, label};

#[derive(Debug, Clone)]
pub enum Message {
    Toggle,
    Dismiss,
    Width(String),
    Height(String),
    MaxWidth(String),
    MaxHeight(String),
}

#[derive(Debug)]
pub struct Example {
    open: bool,
    width: String,
    height: String,
    max_width: String,
    max_height: String,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            open: true,
            width: "100%".into(),
            height: "25px".into(),
            max_width: "300px".into(),
            max_height: "none".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Toggle => self.open = !self.open,
            Message::Dismiss => self.open = false,
            Message::Width(value) => self.width = value,
            Message::Height(value) => self.height = value,
            Message::MaxWidth(value) => self.max_width = value,
            Message::MaxHeight(value) => self.max_height = value,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let setting = |name, value: &str, on_input: fn(String) -> Message| {
            row![
                container(label(name)).width(96),
                input("", value).size(Size::Sm).on_input(on_input),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        };

        let form = column![
            text("Dimensions").size(text_size::MD).font(semibold()),
            text("Set the dimensions for the layer.")
                .size(text_size::SM)
                .style(|theme| text::Style {
                    color: Some(Tokens::of(theme).muted_foreground),
                }),
            column![
                setting("Width", &self.width, Message::Width),
                setting("Max. width", &self.max_width, Message::MaxWidth),
                setting("Height", &self.height, Message::Height),
                setting("Max. height", &self.max_height, Message::MaxHeight),
            ]
            .spacing(8),
        ]
        .spacing(8);

        let trigger = button("Open popover")
            .variant(Variant::Outline)
            .on_press(Message::Toggle);

        container(
            popover(trigger, form)
                .open(self.open)
                .on_dismiss(Message::Dismiss),
        )
        .height(300)
        .into()
    }
}
