use std::hash::{Hash, Hasher};
use std::time::Duration;

use futures_timer::Delay;
use iced::futures::{SinkExt, Stream, stream};
use iced::widget::{column, text};
use iced::{Element, Length, Subscription};
use iced_cube::data::tree::{self, Event, Node, Output, State, loaded, node, tree};

#[derive(Debug, Clone)]
pub enum Message {
    Folders(Event<u32>),
    Finished(u32),
}

#[derive(Debug)]
pub struct Example {
    folders: State<u32>,
    sender: Option<tree::Sender<u32>>,
    /// Folders whose children are on their way.
    loading: Vec<u32>,
}

impl Default for Example {
    fn default() -> Self {
        let mut folders = State::new([
            node(1, "Photos").folder().lazy(),
            node(2, "Documents").folder().lazy(),
            node(3, "Shared with me").folder().lazy(),
        ]);
        // Start with one folder open, so its children load straight away.
        let loading = match folders.update(Event::Expand(1)) {
            Some(Output::Load(ids)) => ids,
            _ => Vec::new(),
        };

        Self {
            folders,
            sender: None,
            loading,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Folders(event) => match self.folders.update(event) {
                Some(Output::Ready(sender)) => self.sender = Some(sender),
                Some(Output::Load(ids)) => self.loading.extend(ids),
                _ => {}
            },
            Message::Finished(id) => self.loading.retain(|&loading| loading != id),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status = match self.loading.len() {
            0 => "Every open folder has loaded. Expand another.".to_owned(),
            n => format!("Loading {n} folder(s) from the server..."),
        };

        column![
            tree(&self.folders).height(280).on_event(Message::Folders),
            text(status).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(340)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let loads = self.sender.iter().flat_map(|sender| {
            self.loading.iter().map(|&parent| {
                let load = Load {
                    parent,
                    sender: sender.clone(),
                };
                Subscription::run_with(load, Load::run)
            })
        });

        Subscription::batch(
            std::iter::once(tree::subscription().map(Message::Folders)).chain(loads),
        )
    }
}

/// A simulated request for one folder's children. It only holds a sender,
/// as a worker thread or a network task would.
#[derive(Debug, Clone)]
struct Load {
    parent: u32,
    sender: tree::Sender<u32>,
}

impl Hash for Load {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.parent.hash(state);
    }
}

impl Load {
    fn run(&self) -> impl Stream<Item = Message> + use<> {
        let parent = self.parent;
        let mut sender = self.sender.clone();

        stream::once(async move {
            Delay::new(Duration::from_millis(900)).await;
            let _ = sender.send(loaded(parent, children(parent))).await;
            Message::Finished(parent)
        })
    }
}

/// Two subfolders and a file, down to three levels.
fn children(parent: u32) -> Vec<Node<u32>> {
    let depth = parent.ilog10();
    let file = node(parent * 10 + 3, format!("notes-{parent}.txt")).file();
    if depth >= 2 {
        return vec![file];
    }
    vec![
        node(parent * 10 + 1, "2025").folder().lazy(),
        node(parent * 10 + 2, "Archive").folder().lazy(),
        file,
    ]
}
