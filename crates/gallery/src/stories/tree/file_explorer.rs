use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::context_menu;
use iced_cube::data::tree::{Event, Output, State, node, tree};
use iced_cube::dropdown_menu::{self, item, separator};
use iced_cube::lucide;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Open,
    Rename,
    Delete,
}

#[derive(Debug, Clone)]
pub enum Message {
    Files(Event<u32>),
    Menu(context_menu::Event<Action, u32>),
}

#[derive(Debug)]
pub struct Example {
    files: State<u32>,
    menu: context_menu::State<Action, u32>,
    status: String,
}

impl Default for Example {
    fn default() -> Self {
        let files = State::new([
            node(1, "src").folder().children([
                node(2, "components").folder().children([
                    node(3, "button.rs").file().trailing("4 KB"),
                    node(4, "data_table.rs").file().trailing("38 KB"),
                    node(5, "tree.rs").file().trailing("21 KB"),
                ]),
                node(6, "main.rs").file().trailing("2 KB"),
                node(7, "a_file_whose_name_is_far_too_long_for_the_row.rs").file(),
            ]),
            node(8, "assets").folder().badge("3").children([
                node(9, "logo.svg").file(),
                node(10, "fonts")
                    .folder()
                    .children([node(11, "Inter.ttf").file()]),
                node(12, "banner.png").file(),
            ]),
            node(13, "target").folder().disabled(true),
            node(14, "Cargo.toml").file().trailing("1 KB"),
            node(15, "README.md").file(),
        ])
        .with_expanded([1, 2]);

        let menu = context_menu::State::new([
            item(Action::Open, "Open").icon(lucide!(FileText)),
            item(Action::Rename, "Rename")
                .icon(lucide!(Pencil))
                .shortcut("F2"),
            separator(),
            item(Action::Delete, "Delete")
                .icon(lucide!(Trash))
                .destructive(true),
        ]);

        Self {
            files,
            menu,
            status: "Double-click to open, right-click for more.".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Files(event) => match self.files.update(event) {
                Some(Output::Activated(id)) => self.status = format!("Opened {}", self.label(id)),
                Some(Output::Selected(ids)) => {
                    if let Some(&id) = ids.first() {
                        self.status = format!("Selected {}", self.label(id));
                    }
                }
                _ => {}
            },
            Message::Menu(event) => {
                let chosen = self.menu.update(event);
                if let (Some(dropdown_menu::Output::Activated(action)), Some(&id)) =
                    (chosen, self.menu.target())
                {
                    self.status = format!("{action:?} {}", self.label(id));
                }
            }
        }
    }

    fn label(&self, id: u32) -> &str {
        self.files.node(id).map_or("", |node| node.label.as_str())
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            tree(&self.files)
                .guides(true)
                .height(300)
                .context_menu(&self.menu, Message::Menu)
                .on_event(Message::Files),
            text(&self.status).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(340)
        .into()
    }
}
