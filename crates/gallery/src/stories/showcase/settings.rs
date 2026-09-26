use iced::widget::{column, row, space, text};
use iced::{Alignment, Element, Length};
use iced_cube::feedback::badge::Variant as BadgeVariant;
use iced_cube::primitives::button::Variant;
use iced_cube::{badge, button, card, lucide, select, slider, switch};

const THEMES: &[&str] = &["System", "Light", "Dark"];

#[derive(Debug, Clone)]
pub enum Message {
    Notifications(bool),
    Digest(bool),
    Theme(&'static str),
    Volume(f32),
    Save,
    Reset,
}

#[derive(Debug)]
pub struct Example {
    notifications: bool,
    digest: bool,
    theme: &'static str,
    volume: f32,
    saved: bool,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            notifications: true,
            digest: false,
            theme: "System",
            volume: 60.0,
            saved: true,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Notifications(on) => self.notifications = on,
            Message::Digest(on) => self.digest = on,
            Message::Theme(theme) => self.theme = theme,
            Message::Volume(volume) => self.volume = volume,
            Message::Save => {
                self.saved = true;
                return;
            }
            Message::Reset => {
                *self = Self::default();
                return;
            }
        }
        self.saved = false;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status = if self.saved {
            badge("Saved")
                .variant(BadgeVariant::Success)
                .icon(lucide!(Check))
        } else {
            badge("Unsaved changes").variant(BadgeVariant::Warning)
        };

        let body = column![
            switch(self.notifications)
                .label("Desktop notifications")
                .on_toggle(Message::Notifications),
            switch(self.digest)
                .label("Weekly email digest")
                .on_toggle(Message::Digest),
            row![
                text("Theme").size(14).width(Length::Fill),
                select(THEMES, Some(self.theme))
                    .width(160)
                    .on_select(Message::Theme),
            ]
            .align_y(Alignment::Center),
            slider(0.0..=100.0, self.volume)
                .label("Alert volume")
                .show_value()
                .step(5.0)
                .on_change(Message::Volume),
        ]
        .spacing(14);

        let footer = row![
            status,
            space::horizontal(),
            button("Reset")
                .variant(Variant::Ghost)
                .on_press(Message::Reset),
            button("Save")
                .icon(lucide!(Save))
                .on_press_maybe((!self.saved).then_some(Message::Save)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        card()
            .title("Preferences")
            .description("Changes apply to this device only.")
            .body(body)
            .footer(footer)
            .width(420)
            .into()
    }
}
