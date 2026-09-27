use iced::keyboard::Modifiers;
use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::data::tree::{self, Action, Event, Mode, Output, State, node, tree};
use iced_cube::keys::{Chord, Keymap};

#[derive(Debug, Clone)]
pub enum Message {
    Outline(Event<u32>),
}

#[derive(Debug)]
pub struct Example {
    outline: State<u32>,
    keymap: Keymap<Action>,
    last: String,
}

impl Default for Example {
    fn default() -> Self {
        let mut outline = State::new([
            node(1, "Getting started").children([
                node(2, "Installation"),
                node(3, "Quick tour"),
                node(4, "Migrating").disabled(true),
            ]),
            node(5, "Guides").children([
                node(6, "Theming"),
                node(7, "Keyboard shortcuts"),
                node(8, "Streaming data"),
            ]),
            node(9, "Reference").children([
                node(10, "Components").children([node(11, "Tree"), node(12, "Data table")]),
                node(13, "Changelog"),
            ]),
        ])
        .with_mode(Mode::Multiple)
        .with_expanded([1]);
        let _ = outline.update(Event::Press(3, Modifiers::empty()));

        // The defaults, plus J and K, which take precedence over typeahead.
        let keymap = tree::default_keymap()
            .bind(Chord::character('j'), Action::Next)
            .bind(Chord::character('k'), Action::Previous);

        Self {
            outline,
            keymap,
            last: "Nothing opened yet".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Outline(event) => {
                if let Some(Output::Activated(id)) = self.outline.update(event) {
                    let label = self.outline.node(id).map_or("", |node| node.label.as_str());
                    self.last = format!("Opened \"{label}\"");
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let selected = self.outline.selected().len();

        column![
            text(
                "Click the outline, then use the arrow keys or J and K. Right expands, Left \
                 collapses, * expands every sibling, Shift+arrows extend the selection and \
                 letters jump."
            )
            .size(14),
            tree(&self.outline)
                .keymap(self.keymap.clone())
                .height(224)
                .on_event(Message::Outline),
            text(format!("{selected} selected. {}", self.last)).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(420)
        .into()
    }
}
