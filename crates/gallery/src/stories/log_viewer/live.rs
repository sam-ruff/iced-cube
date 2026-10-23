use std::hash::{Hash, Hasher};
use std::time::Duration;

use futures_timer::Delay;
use iced::futures::{SinkExt, Stream, StreamExt, stream};
use iced::{Element, Subscription, Task};
use iced_cube::data::log_viewer::{self, Entry, Event, Level, Output, State, entry, log_viewer};

use crate::bridge;

#[derive(Debug, Clone)]
pub enum Message {
    Logs(Event),
}

#[derive(Debug)]
pub struct Example {
    logs: State,
    server: Option<Server>,
}

impl Default for Example {
    fn default() -> Self {
        // Keeps the newest 2,000 lines, so the log never grows without bound.
        let mut logs = State::new().with_limit(2000);
        logs.extend((0..14).map(line));

        Self { logs, server: None }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let Message::Logs(event) = message;
        match self.logs.update(event) {
            Some(Output::Ready(sender)) => {
                self.server = Some(Server { sender });
                Task::none()
            }
            Some(Output::Copy(text)) => bridge::copy(text),
            Some(Output::FocusSearch) => log_viewer::focus_search(&self.logs),
            None => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        log_viewer(&self.logs)
            .height(380)
            .on_event(Message::Logs)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let server = match &self.server {
            Some(server) => Subscription::run_with(server.clone(), Server::run),
            None => Subscription::none(),
        };

        Subscription::batch([log_viewer::subscription().map(Message::Logs), server])
    }
}

/// A simulated server writing a few lines every 300 milliseconds. It holds
/// only a sender, as a worker thread would, and waits with futures-timer,
/// so it also runs in the browser.
#[derive(Debug, Clone)]
struct Server {
    sender: log_viewer::Sender,
}

impl Hash for Server {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "server".hash(state);
    }
}

impl Server {
    fn run(&self) -> impl Stream<Item = Message> + use<> {
        let mut sender = self.sender.clone();

        stream::once(async move {
            let mut tick: u32 = 14;
            loop {
                Delay::new(Duration::from_millis(300)).await;
                for _ in 0..1 + tick % 3 {
                    if sender.send(line(tick)).await.is_err() {
                        return;
                    }
                    tick += 1;
                }
            }
        })
        .filter_map(|()| async { None::<Message> })
    }
}

/// The line written at `tick`, with a clock that starts at 09:41.
fn line(tick: u32) -> Entry {
    let (level, target, message) = match tick % 11 {
        0 => (Level::Info, "http", "GET /api/orders 200 in 14ms"),
        1 => (Level::Debug, "db::pool", "checked out connection 3 of 16"),
        2 => (Level::Info, "http", "POST /api/orders 201 in 38ms"),
        3 => (Level::Trace, "cache", "hit for key orders:recent"),
        4 => (
            Level::Warn,
            "db::pool",
            "slow query took 840ms: select * from orders where status = 'open'",
        ),
        5 => (Level::Info, "worker", "sent 12 receipts"),
        6 => (Level::Debug, "http", "keep-alive connection reused"),
        7 => (
            Level::Error,
            "payments",
            "request to the payment provider timed out after 5s",
        ),
        8 => (Level::Info, "http", "GET /api/health 200 in 1ms"),
        9 => (Level::Trace, "cache", "evicted 4 stale entries"),
        _ => (Level::Info, "worker", "queue is empty, waiting"),
    };
    let millis = 41 * 60_000 + tick * 300;
    let timestamp = format!(
        "09:{:02}:{:02}.{:03}",
        millis / 60_000,
        millis / 1000 % 60,
        millis % 1000
    );

    entry(level, message).timestamp(timestamp).target(target)
}
