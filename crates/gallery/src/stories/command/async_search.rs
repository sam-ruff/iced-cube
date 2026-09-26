use std::hash::{Hash, Hasher};
use std::time::Duration;

use futures_timer::Delay;
use iced::futures::{SinkExt, Stream, stream};
use iced::widget::{column, text};
use iced::{Element, Length, Subscription};
use iced_cube::lucide;
use iced_cube::navigation::command::{self, Output, State, command, group, item, results, score};

const FILES: &[&str] = &[
    "budget-2026.xlsx",
    "design-review.pdf",
    "invoice-0142.pdf",
    "meeting-notes.md",
    "readme.md",
    "release-plan.md",
    "report-q3.pdf",
    "roadmap.png",
    "team-photo.jpg",
    "wireframes.fig",
];

#[derive(Debug, Clone)]
pub enum Message {
    Command(command::Event<&'static str>),
    SearchFinished(String),
}

#[derive(Debug)]
pub struct Example {
    command: State<&'static str>,
    sender: Option<command::Sender<&'static str>>,
    search: Option<Search>,
    last_run: Option<&'static str>,
}

impl Default for Example {
    fn default() -> Self {
        let command = State::new([group(
            "Actions",
            [
                item("new-file", "New file").icon(lucide!(FilePlus)),
                item("upload", "Upload files").icon(lucide!(FileUp)),
                item("open-folder", "Open folder").icon(lucide!(FolderOpen)),
            ],
        )])
        .with_visible_rows(6);

        Self {
            command,
            sender: None,
            search: None,
            last_run: None,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Command(event) => match self.command.update(event) {
                Some(Output::Ready(sender)) => self.sender = Some(sender),
                Some(Output::Search(query)) => {
                    self.search = self
                        .sender
                        .clone()
                        .filter(|_| !query.trim().is_empty())
                        .map(|sender| Search { query, sender });
                }
                Some(Output::Run(id)) => self.last_run = Some(id),
                Some(Output::Dismiss) | None => {}
            },
            Message::SearchFinished(query) => {
                if self
                    .search
                    .as_ref()
                    .is_some_and(|search| search.query == query)
                {
                    self.search = None;
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status = match self.last_run {
            Some(id) => format!("Opened \"{id}\""),
            None => "Type to search files. Matches stream in from a background task.".to_owned(),
        };

        column![
            command(&self.command)
                .placeholder("Search files and actions...")
                .loading(self.search.is_some())
                .height(300)
                .on_event(Message::Command),
            text(status).size(14),
        ]
        .spacing(12)
        .width(Length::Fixed(420.0))
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let search = match &self.search {
            Some(search) => Subscription::run_with(search.clone(), Search::run),
            None => Subscription::none(),
        };

        Subscription::batch([command::subscription().map(Message::Command), search])
    }
}

/// A simulated remote search that only holds a sender, like a worker would.
/// A new query replaces the subscription, which stops the old search.
#[derive(Debug, Clone)]
struct Search {
    query: String,
    sender: command::Sender<&'static str>,
}

impl Hash for Search {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.query.hash(state);
    }
}

impl Search {
    fn run(&self) -> impl Stream<Item = Message> + use<> {
        let query = self.query.clone();
        let mut sender = self.sender.clone();

        stream::once(async move {
            let found: Vec<&'static str> = FILES
                .iter()
                .copied()
                .filter(|file| score(&query, file).is_some())
                .collect();

            for chunk in found.chunks(2) {
                Delay::new(Duration::from_millis(400)).await;
                let files = chunk
                    .iter()
                    .map(|&file| item(file, file).icon(lucide!(File)));
                let _ = sender.send(results(query.as_str(), "Files", files)).await;
            }
            Message::SearchFinished(query)
        })
    }
}
