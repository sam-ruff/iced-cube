use iced::widget::{column, container};
use iced::{Element, Length, Padding, Subscription};
use iced_cube::navigation::bottom_nav::{self, bottom_nav};
use iced_cube::overlay::toast::{self, Position, toast, toasts};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Notes,
    Search,
    Trash,
}

#[derive(Debug, Clone)]
pub enum Message {
    Delete,
    Restore,
    Go(Screen),
    Toast(toast::Event),
}

#[derive(Debug)]
pub struct Example {
    notes: u32,
    screen: Screen,
    toasts: toast::State<Message>,
}

impl Default for Example {
    fn default() -> Self {
        let mut toasts = toast::State::default();
        toasts.push_with(deleted().persistent(), Message::Restore);
        Self {
            notes: 5,
            screen: Screen::Notes,
            toasts,
        }
    }
}

fn deleted() -> toast::Toast {
    toast("Note moved to the bin").action("Undo")
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Delete => {
                self.notes -= 1;
                self.toasts.push_with(deleted(), Message::Restore);
            }
            Message::Restore => self.notes += 1,
            Message::Go(screen) => self.screen = screen,
            Message::Toast(event) => {
                if let Some(toast::Output::Payload(message)) = self.toasts.update(event) {
                    self.update(message);
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let delete = (self.notes > 0).then_some(Message::Delete);
        let screen = container(
            button(format!("Delete one of {} notes", self.notes))
                .icon(lucide!(Trash))
                .variant(Variant::Secondary)
                .on_press_maybe(delete),
        )
        .center(Length::Fill)
        .height(Length::Fill);

        let bar = bottom_nav(self.screen)
            .item(Screen::Notes, lucide!(NotebookText), "Notes")
            .item(Screen::Search, lucide!(Search), "Search")
            .item(Screen::Trash, lucide!(Trash), "Bin")
            .on_select(Message::Go);

        // Bottom centre and lifted above the bar, filling a narrow window
        // up to 420 pixels like a snackbar.
        toasts(&self.toasts, column![screen, bar].height(Length::Fill))
            .position(Position::BottomCenter)
            .inset(Padding::from(12).bottom(12.0 + bottom_nav::HEIGHT))
            .max_width(420.0)
            .on_event(Message::Toast)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        toast::timer(&self.toasts).map(Message::Toast)
    }
}
