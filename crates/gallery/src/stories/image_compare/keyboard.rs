use iced::widget::image::Handle;
use iced::widget::{column, text};
use iced::{Element, Length, Subscription};
use iced_cube::data::image_compare::{self, Action};
use iced_cube::keys::{self, Chord, Keymap};

#[derive(Debug, Clone)]
pub enum Message {
    Divider(f32),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    before: Handle,
    after: Handle,
    divider: f32,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            before: stripes(false),
            after: stripes(true),
            divider: 0.25,
            keymap: image_compare::default_keymap()
                .bind(Chord::character('h'), Action::Left)
                .bind(Chord::character('l'), Action::Right),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Divider(divider) => self.divider = divider,
            Message::Key(key) => {
                if let Some(action) = self.keymap.resolve_event(&key) {
                    self.divider = action.apply(self.divider);
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            image_compare::image_compare(self.before.clone(), self.after.clone())
                .divider(self.divider)
                .on_divider(Message::Divider),
            text("Arrow keys move the divider, Home and End send it to the edges, and H and L move it too.")
                .size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(400)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}

/// Diagonal stripes, grey before and blue after.
fn stripes(after: bool) -> Handle {
    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 120;
    let (light, dark) = if after {
        ([191, 219, 254], [37, 99, 235])
    } else {
        ([228, 228, 231], [113, 113, 122])
    };
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let colour = if (x + y) / 20 % 2 == 0 { light } else { dark };
            pixels.extend([colour[0], colour[1], colour[2], 255]);
        }
    }
    Handle::from_rgba(WIDTH, HEIGHT, pixels)
}
