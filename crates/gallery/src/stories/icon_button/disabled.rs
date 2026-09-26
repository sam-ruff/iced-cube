use iced::widget::{row, text};
use iced::{Alignment, Element, Length};
use iced_cube::{button, icon_button, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    Edit,
    Undo,
    Redo,
}

#[derive(Debug)]
pub struct Example {
    done: u32,
    undone: u32,
}

impl Default for Example {
    fn default() -> Self {
        Self { done: 2, undone: 0 }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Edit => {
                self.done += 1;
                self.undone = 0;
            }
            Message::Undo => {
                self.done -= 1;
                self.undone += 1;
            }
            Message::Redo => {
                self.undone -= 1;
                self.done += 1;
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let edits = match self.done {
            1 => String::from("1 edit"),
            count => format!("{count} edits"),
        };

        row![
            icon_button(lucide!(Undo2))
                .label("Undo")
                .on_press_maybe((self.done > 0).then_some(Message::Undo)),
            icon_button(lucide!(Redo2))
                .label("Redo")
                .on_press_maybe((self.undone > 0).then_some(Message::Redo)),
            button("Make an edit").on_press(Message::Edit),
            text(edits).size(14),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fixed(300.0))
        .into()
    }
}
