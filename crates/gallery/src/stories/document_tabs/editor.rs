use iced::widget::{column, container, row, text};
use iced::{Border, Element, Length, Theme, border};
use iced_cube::lucide;
use iced_cube::navigation::document_tabs::{
    Event, MenuItem, Output, State, document, document_tabs,
};
use iced_cube::overlay::dropdown_menu::item;
use iced_cube::theme::{Tokens, radius, text_size};

const COPY_PATH: MenuItem = MenuItem::Custom("copy-path");

#[derive(Debug, Clone)]
pub enum Message {
    Tabs(Event<u32>),
}

#[derive(Debug)]
pub struct Example {
    tabs: State<u32>,
    next: u32,
    status: String,
}

impl Default for Example {
    fn default() -> Self {
        let tabs = State::new([
            document(1, "README.md")
                .icon(lucide!(BookOpen))
                .pinned(true),
            document(2, "main.rs").icon(lucide!(FileCode)),
            document(3, "parser.rs")
                .icon(lucide!(FileCode))
                .modified(true),
            document(4, "Cargo.toml").icon(lucide!(FileCog)),
            document(5, "notes.md")
                .icon(lucide!(FileText))
                .modified(true),
        ])
        .with_active(3)
        .with_menu_items([item(COPY_PATH, "Copy path")]);

        Self {
            tabs,
            next: 6,
            status: "Right-click a tab for more.".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Tabs(event) = message;
        match self.tabs.update(event) {
            Some(Output::CloseRequested(ids)) => {
                let _ = self.tabs.close(&ids);
            }
            Some(Output::NewTab) => {
                let label = format!("untitled-{}.md", self.next);
                self.tabs
                    .open(document(self.next, label).icon(lucide!(FileText)));
                self.next += 1;
            }
            Some(Output::Custom(_, id)) => {
                let label = self.tabs.document(id).map_or("", |doc| doc.label.as_str());
                self.status = format!("Copied src/{label}");
            }
            _ => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let lines = self.tabs.active().map_or(&[][..], contents);
        let numbered = lines.iter().enumerate().map(|(index, line)| {
            row![
                text(index + 1).size(text_size::SM).width(24).style(muted),
                text(*line).size(text_size::SM),
            ]
            .into()
        });
        let editor = container(column(numbered).spacing(4))
            .padding([12, 16])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(page);

        let frame = column![
            document_tabs(&self.tabs)
                .new_tab(true)
                .on_event(Message::Tabs),
            editor,
        ];

        column![
            container(frame)
                .padding([4, 4])
                .width(Length::Fill)
                .height(240)
                .style(window),
            text(&self.status).size(text_size::SM).style(muted),
        ]
        .spacing(12)
        .into()
    }
}

fn contents(id: u32) -> &'static [&'static str] {
    match id {
        1 => &["# Parser", "", "A small expression parser."],
        2 => &["mod parser;", "", "fn main() {", "    parser::run();", "}"],
        3 => &[
            "pub fn run() {",
            "    let tokens = lex(\"1 + 2\");",
            "    let tree = parse(&tokens);",
            "    println!(\"{tree:?}\");",
            "}",
        ],
        4 => &["[package]", "name = \"parser\"", "edition = \"2024\""],
        5 => &["Handle unary minus.", "Report the column of errors."],
        _ => &[""],
    }
}

fn window(theme: &Theme) -> container::Style {
    let tokens = Tokens::of(theme);
    container::Style {
        background: Some(tokens.muted.into()),
        border: Border {
            color: tokens.border,
            width: 1.0,
            radius: radius::LG.into(),
        },
        ..container::Style::default()
    }
}

fn page(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Tokens::of(theme).background.into()),
        border: border::rounded(border::bottom(radius::SM)),
        ..container::Style::default()
    }
}

fn muted(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(Tokens::of(theme).muted_foreground),
    }
}
