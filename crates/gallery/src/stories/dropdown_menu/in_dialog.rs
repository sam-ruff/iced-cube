use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length};
use iced_cube::dropdown_menu::{Event, Output, State, dropdown_menu, radio_item};
use iced_cube::overlay::dialog::{self, dialog};
use iced_cube::primitives::button::Variant;
use iced_cube::theme::{Tokens, text_size};
use iced_cube::{button, lucide};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Access {
    View,
    Comment,
    Edit,
}

impl Access {
    fn label(self) -> &'static str {
        match self {
            Access::View => "Can view",
            Access::Comment => "Can comment",
            Access::Edit => "Can edit",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Open,
    Close,
    Menu(Event<Access>),
}

#[derive(Debug)]
pub struct Example {
    open: bool,
    menu: State<Access>,
    access: Access,
}

impl Default for Example {
    fn default() -> Self {
        let mut menu = State::new(
            [Access::View, Access::Comment, Access::Edit]
                .map(|access| radio_item(access, access.label(), access == Access::View)),
        );
        let _ = menu.update(Event::Open);
        let _ = menu.update(Event::Highlight(Access::Comment));
        Self {
            open: true,
            menu,
            access: Access::View,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Open => self.open = true,
            // Escape or a click outside closes the menu first, so the
            // dialog only closes once the menu is gone.
            Message::Close => self.open = false,
            Message::Menu(event) => {
                if let Some(Output::Selected(access)) = self.menu.update(event) {
                    self.access = access;
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let trigger = button(self.access.label())
            .trailing_icon(lucide!(ChevronDown))
            .variant(Variant::Outline)
            .on_press(Message::Menu(Event::Toggle));
        let link = row![
            column![
                text("Anyone with the link").size(text_size::SM),
                text("No sign-in required")
                    .size(text_size::SM)
                    .style(|theme| text::Style {
                        color: Some(Tokens::of(theme).muted_foreground),
                    }),
            ]
            .width(Length::Fill),
            dropdown_menu(&self.menu, trigger)
                .width(160.0)
                .on_event(Message::Menu),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let base = container(
            button("Share")
                .variant(Variant::Outline)
                .on_press(Message::Open),
        )
        .padding(24);

        dialog(base)
            .open(self.open)
            .title("Share document")
            .description("Choose what people with the link can do.")
            .body(link)
            .action(button("Done").on_press(Message::Close))
            .size(dialog::Size::Sm)
            .on_dismiss(Message::Close)
            .into()
    }
}
