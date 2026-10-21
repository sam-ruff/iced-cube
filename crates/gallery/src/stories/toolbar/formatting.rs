use iced::keyboard::{Key, Modifiers};
use iced::widget::{column, container, text};
use iced::{Border, Element, Length, Subscription};
use iced_cube::keys::{self, Keymap};
use iced_cube::lucide;
use iced_cube::primitives::slider;
use iced_cube::theme::{Tokens, radius, text_size};
use iced_cube::toolbar::{
    self, Action, Output, State, button, choice, separator, spacer, toggle, toolbar,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tool {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Left,
    Centre,
    Right,
    Justify,
    Link,
    Undo,
    Redo,
}

const TOGGLES: [Tool; 4] = [
    Tool::Bold,
    Tool::Italic,
    Tool::Underline,
    Tool::Strikethrough,
];

#[derive(Debug, Clone, Copy, PartialEq)]
struct Format {
    on: [bool; 4],
    align: Tool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Toolbar(toolbar::Event<Tool>),
    Width(f32),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    toolbar: State<Tool>,
    keymap: Keymap<Action>,
    format: Format,
    undo: Vec<Format>,
    redo: Vec<Format>,
    width: f32,
}

impl Default for Example {
    fn default() -> Self {
        let toolbar = State::new([
            toggle(Tool::Bold, lucide!(Bold), "Bold", false).shortcut("Ctrl+B"),
            toggle(Tool::Italic, lucide!(Italic), "Italic", false).shortcut("Ctrl+I"),
            toggle(Tool::Underline, lucide!(Underline), "Underline", false).shortcut("Ctrl+U"),
            toggle(
                Tool::Strikethrough,
                lucide!(Strikethrough),
                "Strikethrough",
                false,
            ),
            separator(),
            choice(Tool::Left, lucide!(TextAlignStart), "Align left", true),
            choice(
                Tool::Centre,
                lucide!(TextAlignCenter),
                "Align centre",
                false,
            ),
            choice(Tool::Right, lucide!(TextAlignEnd), "Align right", false),
            choice(Tool::Justify, lucide!(TextAlignJustify), "Justify", false),
            separator(),
            button(Tool::Link, lucide!(Link), "Insert link"),
            spacer(),
            button(Tool::Undo, lucide!(Undo2), "Undo").disabled(true),
            button(Tool::Redo, lucide!(Redo2), "Redo").disabled(true),
        ]);

        Self {
            toolbar,
            keymap: toolbar::default_keymap(),
            format: Format {
                on: [false; 4],
                align: Tool::Left,
            },
            undo: Vec::new(),
            redo: Vec::new(),
            width: 300.0,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Toolbar(event) => {
                if let Some(output) = self.toolbar.update(event) {
                    self.apply(output);
                }
            }
            Message::Width(width) => self.width = width,
            Message::Key(key) => {
                if let Some(tool) = shortcut(&key) {
                    let on = !self.toolbar.is_on(tool);
                    self.toolbar.set_on(tool, on);
                    self.apply(Output::Toggled(tool, on));
                    return;
                }
                let event = self.toolbar.key_event(&self.keymap, &key);
                if let Some(output) = event.and_then(|event| self.toolbar.update(event)) {
                    self.apply(output);
                }
            }
        }
    }

    fn apply(&mut self, output: Output<Tool>) {
        match output {
            Output::Toggled(tool, on) => {
                let before = self.format;
                if let Some(index) = TOGGLES.iter().position(|toggle| *toggle == tool) {
                    self.format.on[index] = on;
                }
                self.record(before);
            }
            Output::Selected(align) => {
                let before = self.format;
                self.format.align = align;
                self.record(before);
            }
            Output::Activated(Tool::Undo) => {
                if let Some(format) = self.undo.pop() {
                    self.redo.push(self.format);
                    self.restore(format);
                }
            }
            Output::Activated(Tool::Redo) => {
                if let Some(format) = self.redo.pop() {
                    self.undo.push(self.format);
                    self.restore(format);
                }
            }
            Output::Activated(_) => {}
        }
        self.toolbar.set_disabled(Tool::Undo, self.undo.is_empty());
        self.toolbar.set_disabled(Tool::Redo, self.redo.is_empty());
    }

    fn record(&mut self, before: Format) {
        self.undo.push(before);
        self.redo.clear();
    }

    fn restore(&mut self, format: Format) {
        self.format = format;
        for (tool, on) in TOGGLES.into_iter().zip(format.on) {
            self.toolbar.set_on(tool, on);
        }
        self.toolbar.set_on(format.align, true);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let on: Vec<String> = TOGGLES
            .into_iter()
            .zip(self.format.on)
            .filter(|(_, on)| *on)
            .map(|(tool, _)| format!("{tool:?}").to_lowercase())
            .collect();
        let styles = if on.is_empty() {
            String::from("plain")
        } else {
            on.join(", ")
        };
        let status = format!("Text: {styles}. Alignment: {:?}.", self.format.align);

        let editor = container(column![
            toolbar(&self.toolbar).on_event(Message::Toolbar),
            container(text(status).size(text_size::SM))
                .padding(12)
                .height(72),
        ])
        .padding(1)
        .max_width(self.width)
        .style(|theme| {
            let tokens = Tokens::of(theme);
            container::Style {
                border: Border {
                    color: tokens.border,
                    width: 1.0,
                    radius: radius::MD.into(),
                },
                ..container::Style::default()
            }
        });

        column![
            editor,
            slider(200.0..=680.0, self.width)
                .step(10.0)
                .label("Toolbar width")
                .format_value(|width| format!("{width:.0}px"))
                .width(Length::Fixed(280.0))
                .on_change(Message::Width),
        ]
        .spacing(24)
        .width(Length::Fill)
        .align_x(iced::Alignment::Center)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}

/// Ctrl+B, Ctrl+I and Ctrl+U toggle their format from anywhere.
fn shortcut(key: &keys::Event) -> Option<Tool> {
    if key.modifiers != Modifiers::COMMAND {
        return None;
    }
    let Key::Character(letter) = &key.key else {
        return None;
    };
    match letter.as_str() {
        "b" => Some(Tool::Bold),
        "i" => Some(Tool::Italic),
        "u" => Some(Tool::Underline),
        _ => None,
    }
}
