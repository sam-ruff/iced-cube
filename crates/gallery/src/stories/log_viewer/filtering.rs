use iced::{Element, Task};
use iced_cube::data::log_viewer::{self, Event, Level, Output, State, entry, log_viewer};

use crate::bridge;

#[derive(Debug, Clone)]
pub enum Message {
    Logs(Event),
}

#[derive(Debug)]
pub struct Example {
    logs: State,
}

impl Default for Example {
    fn default() -> Self {
        // Trace and debug lines are hidden, and the search marks every
        // line that mentions a timeout, starting at the first.
        let mut logs = State::new()
            .with_levels([Level::Info, Level::Warn, Level::Error])
            .with_wrap(true);
        logs.extend((0..240).map(|n| {
            let (level, message) = match n % 8 {
                0 => (Level::Info, format!("job {n} started")),
                1 => (Level::Debug, format!("job {n} read 3 inputs")),
                2 => (Level::Trace, format!("job {n} polled the queue")),
                3 => (
                    Level::Info,
                    format!("job {n} finished in {}ms", 40 + n % 50),
                ),
                4 => (
                    Level::Warn,
                    format!(
                        "job {n} is retrying after a timeout talking to the storage \
                         service, attempt 2 of 5, backing off for 800ms"
                    ),
                ),
                5 => (Level::Info, format!("uploaded report {n}")),
                6 => (
                    Level::Error,
                    format!("job {n} failed: timeout waiting for a lock"),
                ),
                _ => (Level::Info, format!("job {n} cleaned up")),
            };
            entry(level, message).target("scheduler")
        }));
        let _ = logs.update(Event::Search("timeout".into()));
        let _ = logs.update(Event::NextMatch);

        Self { logs }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let Message::Logs(event) = message;
        match self.logs.update(event) {
            Some(Output::Copy(text)) => bridge::copy(text),
            Some(Output::FocusSearch) => log_viewer::focus_search(&self.logs),
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        log_viewer(&self.logs)
            .height(380)
            .on_event(Message::Logs)
            .into()
    }
}
