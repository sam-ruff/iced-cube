use iced::widget::{column, container, scrollable, stack, text};
use iced::{Element, Length, Theme};
use iced_cube::theme::Tokens;
use iced_cube::{fab, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    Add,
}

#[derive(Debug)]
pub struct Example {
    notes: usize,
}

impl Default for Example {
    fn default() -> Self {
        Self { notes: 8 }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Add = message;
        self.notes += 1;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let rows = (1..=self.notes).map(|n| {
            container(text(format!("Note {n}")).size(14))
                .padding([12, 16])
                .width(Length::Fill)
                .style(|theme: &Theme| container::Style {
                    border: iced::Border {
                        color: Tokens::of(theme).border,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..container::Style::default()
                })
                .into()
        });
        let list = scrollable(column(rows).spacing(8)).spacing(8);

        // The app places the fab: a stack over the list and a container
        // aligned to the bottom-right corner.
        let add = container(
            fab(lucide!(Plus))
                .tooltip("New note")
                .on_press(Message::Add),
        )
        .align_right(Length::Fill)
        .align_bottom(Length::Fill)
        .padding(16);

        container(stack![list, add].width(Length::Fill).height(280))
            .max_width(320)
            .into()
    }
}
