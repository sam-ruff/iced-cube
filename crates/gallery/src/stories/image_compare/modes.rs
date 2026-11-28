use iced::widget::column;
use iced::widget::image::Handle;
use iced::{Element, Length};
use iced_cube::data::image_compare::{self, Mode};
use iced_cube::primitives::slider::slider;
use iced_cube::primitives::toggle_group::{self, State, Variant, item, toggle_group};

#[derive(Debug, Clone)]
pub enum Message {
    Mode(toggle_group::Event<Mode>),
    Divider(f32),
    Opacity(f32),
}

#[derive(Debug)]
pub struct Example {
    before: Handle,
    after: Handle,
    modes: State<Mode>,
    divider: f32,
    opacity: f32,
}

impl Default for Example {
    fn default() -> Self {
        let modes = State::single(Mode::ALL.map(|mode| item(mode, mode.label())));
        Self {
            before: screen(false),
            after: screen(true),
            modes: modes.with_selected([Mode::SideBySide]).required(true),
            divider: 0.5,
            opacity: 0.5,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Mode(event) => {
                let _ = self.modes.update(event);
            }
            Message::Divider(divider) => self.divider = divider,
            Message::Opacity(opacity) => self.opacity = opacity,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mode = self.modes.selected_one().unwrap_or_default();
        let mut content = column![
            toggle_group(&self.modes)
                .variant(Variant::Outline)
                .width(Length::Fill)
                .on_event(Message::Mode),
            image_compare::image_compare(self.before.clone(), self.after.clone())
                .mode(mode)
                .divider(self.divider)
                .opacity(self.opacity)
                .on_divider(Message::Divider),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(400);
        // The opacity only applies to the overlay.
        if mode == Mode::Overlay {
            content = content.push(
                slider(0.0..=1.0, self.opacity)
                    .step(0.05)
                    .label("Before opacity")
                    .on_change(Message::Opacity),
            );
        }
        content.into()
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
