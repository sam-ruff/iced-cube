use iced::widget::{column, container, text};
use iced::{Alignment, Border, Element, Point};
use iced_cube::context_menu::{Event, State, context_menu};
use iced_cube::dropdown_menu::{checkbox_item, item, separator, submenu};
use iced_cube::theme::{Tokens, radius, text_size};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Id {
    Back,
    Forward,
    Reload,
    MoreTools,
    SavePage,
    DevTools,
    Bookmarks,
    FullUrls,
}

#[derive(Debug, Clone)]
pub enum Message {
    Menu(Event<Id>),
}

#[derive(Debug)]
pub struct Example {
    menu: State<Id>,
}

impl Default for Example {
    fn default() -> Self {
        let mut menu = State::new([
            item(Id::Back, "Back").shortcut("Alt+Left"),
            item(Id::Forward, "Forward")
                .shortcut("Alt+Right")
                .disabled(true),
            item(Id::Reload, "Reload").shortcut("Ctrl+R"),
            submenu(
                Id::MoreTools,
                "More tools",
                [
                    item(Id::SavePage, "Save page as").shortcut("Ctrl+S"),
                    item(Id::DevTools, "Developer tools"),
                ],
            ),
            separator(),
            checkbox_item(Id::Bookmarks, "Show bookmarks", true),
            checkbox_item(Id::FullUrls, "Show full URLs", false),
        ]);
        let _ = menu.update(Event::Open((), Point::new(290.0, 24.0)));
        Self { menu }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Menu(event) = message;
        let _ = self.menu.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let hint = column![
            text("Right-click here").size(text_size::SM),
            text("or press and hold").size(text_size::XS),
        ]
        .spacing(2)
        .align_x(Alignment::Center);
        let area = container(hint).center_x(400).center_y(200).style(|theme| {
            let tokens = Tokens::of(theme);
            container::Style {
                text_color: Some(tokens.muted_foreground),
                border: Border {
                    color: tokens.border,
                    width: 1.0,
                    radius: radius::LG.into(),
                },
                ..container::Style::default()
            }
        });

        context_menu(&self.menu, area)
            .on_event(Message::Menu)
            .into()
    }
}
