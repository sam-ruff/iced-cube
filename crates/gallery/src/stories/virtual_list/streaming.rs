use std::hash::{Hash, Hasher};
use std::time::Duration;

use futures_timer::Delay;
use iced::futures::{SinkExt, Stream, StreamExt, stream};
use iced::widget::{column, row, space, text};
use iced::{Element, Length, Subscription};
use iced_cube::button;
use iced_cube::data::virtual_list::{self, Event, Output, State, virtual_list};
use iced_cube::primitives::button::{Size, Variant};
use iced_cube::theme::{Tokens, text_size};

#[derive(Debug, Clone)]
pub enum Message {
    Feed(Event<Reading>),
}

#[derive(Debug, Clone)]
pub struct Reading {
    sensor: &'static str,
    value: f32,
}

const SENSORS: [&str; 4] = ["Boiler", "Loft", "Garage", "Greenhouse"];

#[derive(Debug)]
pub struct Example {
    feed: State<Reading>,
    sensors: Option<Sensors>,
}

impl Default for Example {
    fn default() -> Self {
        // Keeps the newest thousand readings and sticks to the end.
        let feed = State::new((0..24).map(reading))
            .with_limit(1000)
            .with_follow(true);

        Self {
            feed,
            sensors: None,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Feed(event) = message;
        if let Some(Output::Ready(sender)) = self.feed.update(event) {
            self.sensors = Some(Sensors { sender });
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let list = virtual_list(&self.feed, |index, reading: &Reading| {
            row![
                text(format!("#{}", self.feed.id(index)))
                    .size(text_size::SM)
                    .width(56),
                text(reading.sensor).size(text_size::SM),
                space::horizontal(),
                text(format!("{:.1} \u{b0}C", reading.value))
                    .size(text_size::SM)
                    .style(|theme| text::Style {
                        color: Some(Tokens::of(theme).muted_foreground),
                    }),
            ]
            .spacing(12)
            .into()
        })
        .height(220)
        .on_event(Message::Feed);

        let status: Element<'_, Message> = if self.feed.is_following() {
            text("Following the newest reading. Scroll up to pause.")
                .size(text_size::SM)
                .into()
        } else {
            row![
                text("Paused while you look back.").size(text_size::SM),
                space::horizontal(),
                button("Jump to latest")
                    .size(Size::Sm)
                    .variant(Variant::Secondary)
                    .on_press(Message::Feed(Event::Last)),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .into()
        };

        column![list, status]
            .spacing(12)
            .width(Length::Fill)
            .max_width(420)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let sensors = match &self.sensors {
            Some(sensors) => Subscription::run_with(sensors.clone(), Sensors::run),
            None => Subscription::none(),
        };

        Subscription::batch([virtual_list::subscription().map(Message::Feed), sensors])
    }
}

fn reading(tick: u32) -> Reading {
    Reading {
        sensor: SENSORS[tick as usize % SENSORS.len()],
        value: 18.0 + ((tick * 37) % 90) as f32 / 10.0,
    }
}

/// Simulated sensors that report a few readings every 400 milliseconds.
/// They hold only a sender, as a worker thread would, and wait with
/// futures-timer, so they also run in the browser.
#[derive(Debug, Clone)]
struct Sensors {
    sender: virtual_list::Sender<Reading>,
}

impl Hash for Sensors {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "sensors".hash(state);
    }
}

impl Sensors {
    fn run(&self) -> impl Stream<Item = Message> + use<> {
        let mut sender = self.sender.clone();

        stream::once(async move {
            let mut tick: u32 = 24;
            loop {
                Delay::new(Duration::from_millis(400)).await;
                for _ in 0..3 {
                    if sender.send(reading(tick)).await.is_err() {
                        return;
                    }
                    tick += 1;
                }
            }
        })
        .filter_map(|()| async { None::<Message> })
    }
}
