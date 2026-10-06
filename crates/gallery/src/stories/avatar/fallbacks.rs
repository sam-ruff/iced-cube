use iced::widget::{column, image, row, text};
use iced::{Alignment, Element};
use iced_cube::feedback::avatar::Size;
use iced_cube::{avatar, lucide};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug)]
pub struct Example {
    photo: image::Handle,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            photo: gradient([236, 72, 153], [99, 102, 241]),
        }
    }
}

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        let labelled = |avatar: Element<'static, Message>, label: &'static str| {
            column![avatar, text(label).size(12)]
                .spacing(6)
                .align_x(Alignment::Center)
        };

        row![
            labelled(
                avatar("Ada Lovelace")
                    .image(self.photo.clone())
                    .size(Size::Lg)
                    .into(),
                "Image",
            ),
            labelled(avatar("Ada Lovelace").size(Size::Lg).into(), "Initials"),
            labelled(
                avatar("Release bot")
                    .icon(lucide!(Bot))
                    .size(Size::Lg)
                    .into(),
                "Icon",
            ),
            labelled(avatar("").size(Size::Lg).into(), "No name"),
        ]
        .spacing(24)
        .wrap()
        .into()
    }
}

/// A diagonal blend between two colours, standing in for a profile photo.
fn gradient(from: [u8; 3], to: [u8; 3]) -> image::Handle {
    const SIDE: u32 = 64;
    let mut pixels = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let t = (x + y) as f32 / (2 * SIDE - 2) as f32;
            for channel in 0..3 {
                let blend = f32::from(from[channel]) * (1.0 - t) + f32::from(to[channel]) * t;
                pixels.push(blend.round() as u8);
            }
            pixels.push(255);
        }
    }
    image::Handle::from_rgba(SIDE, SIDE, pixels)
}
