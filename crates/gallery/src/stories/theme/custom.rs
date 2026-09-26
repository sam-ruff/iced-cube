use iced::widget::{column, row};
use iced::{Alignment, Element, Theme, color};
use iced_cube::feedback::badge::Variant as BadgeVariant;
use iced_cube::primitives::button::Variant;
use iced_cube::theme::Config;
use iced_cube::{badge, button, progress, switch};

#[derive(Debug, Clone)]
pub enum Message {
    Dark(bool),
    Save,
}

#[derive(Debug, Default)]
pub struct Example {
    dark: bool,
    saves: u8,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Dark(dark) => self.dark = dark,
            Message::Save => self.saves = self.saves.saturating_add(1).min(10),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let buttons = row![
            button("Save").on_press(Message::Save),
            button("Share")
                .variant(Variant::Secondary)
                .on_press(Message::Save),
            button("Preview")
                .variant(Variant::Outline)
                .on_press(Message::Save),
            badge("Beta").variant(BadgeVariant::Secondary),
            badge("Live"),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        column![
            buttons,
            progress(f32::from(self.saves) / 10.0)
                .label("Saves")
                .show_percentage(true),
            switch(self.dark)
                .label("Dark mode")
                .on_toggle(Message::Dark),
        ]
        .spacing(20)
        .width(420)
        .into()
    }

    pub fn theme(&self) -> Option<Theme> {
        let config = if self.dark {
            Config::dark()
                .background(color!(0x1e1b2e))
                .foreground(color!(0xf5f3ff))
                .primary(color!(0xa78bfa))
                .secondary(color!(0x2e2a45))
                .accent(color!(0x3b3558))
        } else {
            Config::light()
                .background(color!(0xfdfcff))
                .foreground(color!(0x1e1b2e))
                .primary(color!(0x6d28d9))
                .secondary(color!(0xede9fe))
                .accent(color!(0xf5f3ff))
        };
        Some(config.name("Violet").build())
    }
}
