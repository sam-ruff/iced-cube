use std::hash::{Hash, Hasher};
use std::time::Duration;

use futures_timer::Delay;
use iced::futures::{SinkExt, Stream, StreamExt, stream};
use iced::widget::{column, container, text};
use iced::{Element, Length, Subscription};
use iced_cube::feedback::badge::Variant;
use iced_cube::lucide;
use iced_cube::status_bar::{
    self, Output, Section, State, Update, action, badge, progress, spinner, status_bar,
};
use iced_cube::theme::text_size;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Item {
    Branch,
    Sync,
    Build,
    Problems,
    Cursor,
    Encoding,
}

#[derive(Debug, Clone)]
pub enum Message {
    Status(status_bar::Event<Item>),
}

#[derive(Debug)]
pub struct Example {
    status: State<Item>,
    worker: Option<Worker>,
    last_pressed: Option<Item>,
}

impl Default for Example {
    fn default() -> Self {
        let status = State::new().with_items([
            status_bar::text(Item::Branch, "main").icon(lucide!(GitBranch)),
            spinner(Item::Sync, "Syncing"),
            progress(Item::Build, 0.35)
                .label("Building")
                .section(Section::Centre),
            action(Item::Problems, "2 problems")
                .icon(lucide!(TriangleAlert))
                .tooltip("Show problems")
                .section(Section::End),
            status_bar::text(Item::Cursor, "Ln 12, Col 4").section(Section::End),
            badge(Item::Encoding, "UTF-8", Variant::Outline).section(Section::End),
        ]);

        Self {
            status,
            worker: None,
            last_pressed: None,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Status(event) = message;
        match self.status.update(event) {
            // The worker holds only a sender, like a background thread would.
            Some(Output::Ready(sender)) => self.worker = Some(Worker { sender }),
            Some(Output::Pressed(item)) => self.last_pressed = Some(item),
            None => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let note = match self.last_pressed {
            Some(item) => format!("Pressed {item:?} in the status bar."),
            None => "A background worker updates the bar through a channel.".to_owned(),
        };

        column![
            container(text(note).size(text_size::SM))
                .padding(16)
                .height(Length::Fill),
            status_bar(&self.status).on_event(Message::Status),
        ]
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let worker = match &self.worker {
            Some(worker) => Subscription::run_with(worker.clone(), Worker::run),
            None => Subscription::none(),
        };

        Subscription::batch([
            status_bar::subscription().map(Message::Status),
            status_bar::animation(&self.status).map(Message::Status),
            worker,
        ])
    }
}

/// Simulated background work: a git sync, a build and a moving cursor.
/// It waits with futures-timer, so it also runs in the browser.
#[derive(Debug, Clone)]
struct Worker {
    sender: status_bar::Sender<Item>,
}

impl Hash for Worker {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "status-worker".hash(state);
    }
}

impl Worker {
    fn run(&self) -> impl Stream<Item = Message> + use<> {
        let mut sender = self.sender.clone();

        stream::once(async move {
            let mut tick: u32 = 0;
            loop {
                Delay::new(Duration::from_millis(600)).await;
                tick += 1;

                let line = 12 + tick % 40;
                let column = 1 + (tick * 7) % 60;
                let mut updates = vec![Update::Set(
                    status_bar::text(Item::Cursor, format!("Ln {line}, Col {column}"))
                        .section(Section::End),
                )];

                let step = tick % 12;
                updates.push(Update::Set(if step < 4 {
                    spinner(Item::Sync, "Syncing")
                } else {
                    status_bar::text(Item::Sync, "Synced").icon(lucide!(Check))
                }));
                updates.push(Update::Set(if step < 10 {
                    progress(Item::Build, step as f32 / 10.0)
                        .label("Building")
                        .section(Section::Centre)
                } else {
                    badge(Item::Build, "Build passed", Variant::Success).section(Section::Centre)
                }));

                for update in updates {
                    if sender.send(update).await.is_err() {
                        return;
                    }
                }
            }
        })
        .filter_map(|()| async { None::<Message> })
    }
}
