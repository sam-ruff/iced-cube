use iced::widget::container;
use iced::widget::image::Handle;
use iced::{Element, Length};
use iced_cube::data::image_compare;

#[derive(Debug, Clone)]
pub enum Message {
    Divider(f32),
}

#[derive(Debug)]
pub struct Example {
    before: Handle,
    after: Handle,
    divider: f32,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            before: screen(false),
            after: screen(true),
            divider: 0.5,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Divider(divider) = message;
        self.divider = divider;
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(
            image_compare::image_compare(self.before.clone(), self.after.clone())
                .divider(self.divider)
                .on_divider(Message::Divider),
        )
        .width(Length::Fill)
        .max_width(400)
        .into()
    }
}

/// A small window drawn from pixels: the after version moves the button
/// and adds a warning banner.
fn screen(after: bool) -> Handle {
    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 200;
    let inside = |x: u32, y: u32, (left, top, w, h): (u32, u32, u32, u32)| {
        x >= left && x < left + w && y >= top && y < top + h
    };
    let button = if after {
        (200, 150, 96, 28)
    } else {
        (24, 150, 96, 28)
    };
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let colour = if y < 28 {
                [39, 39, 42]
            } else if after && inside(x, y, (24, 44, 272, 24)) {
                [254, 215, 170]
            } else if inside(x, y, button) {
                [37, 99, 235]
            } else if inside(x, y, (24, 84, 200, 8)) || inside(x, y, (24, 104, 150, 8)) {
                [212, 212, 216]
            } else {
                [250, 250, 250]
            };
            pixels.extend([colour[0], colour[1], colour[2], 255]);
        }
    }
    Handle::from_rgba(WIDTH, HEIGHT, pixels)
}
