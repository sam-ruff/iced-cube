use iced::time::Instant;
use iced::widget::{column, row, text};
use iced::{Alignment, Element, Subscription, window};
use iced_cube::feedback::spinner::advance;
use iced_cube::primitives::button::Variant;
use iced_cube::{button, spinner};

#[derive(Debug, Clone)]
pub enum Message {
    Toggle,
    Frame(Instant),
}

#[derive(Debug)]
pub struct Example {
    loading: bool,
    phase: f32,
    last_frame: Option<Instant>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            loading: true,
            phase: 0.15,
            last_frame: None,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Toggle => {
                self.loading = !self.loading;
                self.last_frame = None;
            }
            Message::Frame(now) => {
                if let Some(last) = self.last_frame {
                    self.phase = advance(self.phase, now - last);
                }
                self.last_frame = Some(now);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status: Element<'_, Message> = if self.loading {
            row![spinner(self.phase), text("Fetching latest data").size(14)]
                .spacing(12)
                .align_y(Alignment::Center)
                .into()
        } else {
            text("Up to date").size(14).into()
        };

        column![
            status,
            button(if self.loading { "Stop" } else { "Refresh" })
                .variant(Variant::Outline)
                .on_press(Message::Toggle),
        ]
        .spacing(16)
        .align_x(Alignment::Center)
        .into()
    }

    /// Frames are only requested while the spinner is visible.
    pub fn subscription(&self) -> Subscription<Message> {
        if !self.loading {
            return Subscription::none();
        }
        window::frames().map(Message::Frame)
    }
}
