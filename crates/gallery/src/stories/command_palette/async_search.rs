use std::hash::{Hash, Hasher};
use std::time::Duration;

use futures_timer::Delay;
use iced::futures::{SinkExt, Stream, stream};
use iced::widget::{column, container, text};
use iced::{Alignment, Element, Length, Subscription};
use iced_cube::command_palette::{self, Event, Output, State, command_palette};
use iced_cube::keys::{self, Keymap};
use iced_cube::navigation::command::{self, group, item, results, score};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide};

const FILES: &[&str] = &[
    "budget-2026.xlsx",
    "design-review.pdf",
    "invoice-0142.pdf",
    "meeting-notes.md",
    "release-plan.md",
    "report-q3.pdf",
    "roadmap.png",
];

#[derive(Debug, Clone)]
pub enum Message {
    Palette(command_palette::Event<&'static str>),
    Key(keys::Event),
    SearchFinished(String),
}

#[derive(Debug)]
pub struct Example {
    palette: State<&'static str>,
    keymap: Keymap<command_palette::Action>,
    sender: Option<command::Sender<&'static str>>,
    search: Option<Search>,
    opened: Option<&'static str>,
}

impl Default for Example {
    fn default() -> Self {
        let mut palette = State::new([group(
            "Actions",
            [
                item("new-file", "New file").icon(lucide!(FilePlus)),
                item("upload", "Upload files").icon(lucide!(FileUp)),
            ],
        )])
        .with_visible_rows(6);
        let _ = palette.update(Event::Open);

        Self {
            palette,
            keymap: command_palette::default_keymap(),
            sender: None,
            search: None,
            opened: None,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Palette(event) => Some(event),
            Message::Key(key) => self.palette.key_event(&self.keymap, &key),
            Message::SearchFinished(query) => {
                if self
                    .search
                    .as_ref()
                    .is_some_and(|search| search.query == query)
                {
                    self.search = None;
                }
                None
            }
        };
        match event.and_then(|event| self.palette.update(event)) {
            Some(Output::Ready(sender)) => self.sender = Some(sender),
            // A new query replaces the search, which stops the old one.
            Some(Output::Search(query)) => {
                self.search = self
                    .sender
                    .clone()
                    .filter(|_| !query.trim().is_empty())
                    .map(|sender| Search { query, sender });
            }
            Some(Output::Activated(id)) => self.opened = Some(id),
            None => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status = match self.opened {
            Some(id) => format!("Opened \"{id}\""),
            None => "Type to search files. Matches stream in from a background task.".to_owned(),
        };
        let base = column![
            button("Search files")
                .icon(lucide!(Search))
                .variant(Variant::Outline)
                .on_press(Message::Palette(Event::Open)),
            text(status).size(14),
        ]
        .spacing(12)
        .align_x(Alignment::Center);

        command_palette(
            &self.palette,
            container(base).center(Length::Fill).padding(24),
        )
        .title("Search")
        .placeholder("Search files and actions...")
        .loading(self.search.is_some())
        .max_height(300.0)
        .keymap(self.keymap.clone())
        .on_event(Message::Palette)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let search = match &self.search {
            Some(search) => Subscription::run_with(search.clone(), Search::run),
            None => Subscription::none(),
        };

        Subscription::batch([
            command_palette::subscription().map(Message::Palette),
            keys::subscription().map(Message::Key),
            search,
        ])
    }
}

/// A simulated remote search that only holds a sender, like a worker would.
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
