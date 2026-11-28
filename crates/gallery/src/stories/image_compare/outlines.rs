use iced::widget::image::Handle;
use iced::widget::{column, text};
use iced::{Element, Length, Point, Rectangle, Size};
use iced_cube::data::image_compare::{self, Outline, Tone};

const BANNER: Rectangle = Rectangle {
    x: 24.0,
    y: 44.0,
    width: 272.0,
    height: 24.0,
};
const BUTTON: Rectangle = Rectangle {
    x: 200.0,
    y: 150.0,
    width: 96.0,
    height: 28.0,
};

#[derive(Debug, Clone)]
pub enum Message {
    Hovered(Option<Point>),
    Pressed(Point),
}

#[derive(Debug)]
pub struct Example {
    screen: Handle,
    hovered: Option<Point>,
    picked: Option<Rectangle>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            screen: screen(),
            hovered: None,
            picked: Some(BUTTON),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Hovered(point) => self.hovered = point,
            Message::Pressed(point) => {
                self.picked = [BANNER, BUTTON]
                    .into_iter()
                    .find(|region| region.contains(point));
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut outlines = vec![Outline::new(BANNER).tone(Tone::Warning)];
        if let Some(picked) = self.picked {
            outlines.push(Outline::new(picked.expand(3.0)));
        }
        let pointer = match self.hovered {
            Some(point) => format!("Pixel {:.0}, {:.0}", point.x, point.y),
            None => "Point at the picture, and press the banner or the button.".to_owned(),
        };

        column![
            image_compare::single(self.screen.clone())
                .pixelated(true)
                .outlines(outlines)
                .on_hover(Message::Hovered)
                .on_press(Message::Pressed),
            text(pointer).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(400)
        .into()
    }
}

/// A small window drawn from pixels, with a banner and a button.
fn screen() -> Handle {
    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 200;
    let inside =
        |x: u32, y: u32, region: Rectangle| region.contains(Point::new(x as f32, y as f32));
    let lines = [
        Rectangle::new(Point::new(24.0, 84.0), Size::new(200.0, 8.0)),
        Rectangle::new(Point::new(24.0, 104.0), Size::new(150.0, 8.0)),
    ];
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let colour = if y < 28 {
                [39, 39, 42]
            } else if inside(x, y, BANNER) {
                [254, 215, 170]
            } else if inside(x, y, BUTTON) {
                [37, 99, 235]
            } else if lines.iter().any(|line| inside(x, y, *line)) {
                [212, 212, 216]
            } else {
                [250, 250, 250]
            };
            pixels.extend([colour[0], colour[1], colour[2], 255]);
        }
    }
    Handle::from_rgba(WIDTH, HEIGHT, pixels)
}
