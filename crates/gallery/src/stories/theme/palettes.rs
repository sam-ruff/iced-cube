use iced::widget::{column, container, row, text, themer};
use iced::{Background, Element, Length, Theme, color};
use iced_cube::primitives::button::Variant;
use iced_cube::theme::{self, Config, Tokens, radius};
use iced_cube::{badge, button, checkbox, switch};

#[derive(Debug, Clone)]
pub enum Message {
    Toggled(bool),
    Checked(bool),
    Pressed,
}

#[derive(Debug)]
pub struct Example {
    on: bool,
    checked: bool,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            on: true,
            checked: true,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Toggled(on) => self.on = on,
            Message::Checked(checked) => self.checked = checked,
            Message::Pressed => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let brand = Config::light()
            .name("Brand")
            .primary(color!(0x4f46e5))
            .accent(color!(0xeef2ff))
            .build();

        row![
            self.panel("Light", theme::light()),
            self.panel("Dark", theme::dark()),
            self.panel("Custom", brand),
        ]
        .spacing(12)
        .into()
    }

    /// The same controls rendered under one theme, whatever the app's theme is.
    fn panel(&self, name: &'static str, theme: Theme) -> Element<'_, Message> {
        let content = column![
            text(name).size(14).font(theme::semibold()),
            row![
                button("Save").on_press(Message::Pressed),
                button("Cancel")
                    .variant(Variant::Outline)
                    .on_press(Message::Pressed),
            ]
            .spacing(6),
            switch(self.on).label("Sync").on_toggle(Message::Toggled),
            checkbox(self.checked)
                .label("Notify me")
                .on_toggle(Message::Checked),
            badge("New"),
        ]
        .spacing(12);

        themer(
            Some(theme),
            container(content)
                .padding(16)
                .width(Length::Fixed(200.0))
                .style(|theme: &Theme| {
                    let tokens = Tokens::of(theme);
                    container::Style {
                        background: Some(Background::Color(tokens.background)),
                        border: iced::Border {
                            color: tokens.border,
                            width: 1.0,
                            radius: radius::LG.into(),
                        },
                        text_color: Some(tokens.foreground),
                        ..container::Style::default()
                    }
                }),
        )
        .into()
    }
}
