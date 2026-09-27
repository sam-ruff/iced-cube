use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::context_menu;
use iced_cube::data::data_table::{self, Event, State, data_table};
use iced_cube::dropdown_menu::{Output, item, separator};
use iced_cube::feedback::progress::Size as ProgressSize;
use iced_cube::{lucide, progress};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    CopyId,
    ViewLogs,
    Retry,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct Deploy {
    id: &'static str,
    service: &'static str,
    progress: f32,
}

const DEPLOYS: [Deploy; 4] = [
    Deploy {
        id: "DEP-771",
        service: "billing-api",
        progress: 1.0,
    },
    Deploy {
        id: "DEP-772",
        service: "search-indexer",
        progress: 0.64,
    },
    Deploy {
        id: "DEP-773",
        service: "web-frontend",
        progress: 0.18,
    },
    Deploy {
        id: "DEP-774",
        service: "notifications",
        progress: 0.9,
    },
];

#[derive(Debug, Clone)]
pub enum Message {
    Deploys(Event<&'static str>),
    Menu(context_menu::Event<Action, &'static str>),
}

#[derive(Debug)]
pub struct Example {
    deploys: State<Deploy, &'static str>,
    menu: context_menu::State<Action, &'static str>,
    last: String,
}

impl Default for Example {
    fn default() -> Self {
        let columns = [
            data_table::column("id", "Deploy", |d: &Deploy| d.id.into()).width(100),
            data_table::column("service", "Service", |d: &Deploy| d.service.into()),
            data_table::column("progress", "Progress", |d: &Deploy| d.progress.into())
                .searchable(false),
        ];
        let menu = context_menu::State::new([
            item(Action::CopyId, "Copy deploy ID").icon(lucide!(Copy)),
            item(Action::ViewLogs, "View logs").icon(lucide!(ScrollText)),
            item(Action::Retry, "Retry").icon(lucide!(RotateCw)),
            separator(),
            item(Action::Cancel, "Cancel deploy")
                .icon(lucide!(CircleX))
                .destructive(true),
        ]);

        Self {
            deploys: State::new(columns, DEPLOYS, |d| d.id),
            menu,
            last: "Right-click a row, press its menu button, or press Shift+F10.".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Deploys(event) => {
                let _ = self.deploys.update(event);
            }
            Message::Menu(event) => {
                let chosen = self.menu.update(event);
                if let (Some(Output::Activated(action)), Some(id)) = (chosen, self.menu.target()) {
                    self.last = format!("{action:?} on {id}");
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            data_table(&self.deploys)
                .toolbar(false)
                .cell("progress", |d: &Deploy| {
                    progress(d.progress)
                        .size(ProgressSize::Sm)
                        .show_percentage(true)
                        .into()
                })
                .context_menu(&self.menu, Message::Menu)
                .row_actions(true)
                .on_event(Message::Deploys),
            text(&self.last).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .into()
    }
}
