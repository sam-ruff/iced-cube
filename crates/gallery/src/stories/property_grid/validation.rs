use iced::widget::{column, text};
use iced::{Color, Element, Length};
use iced_cube::data::property_grid::{
    Event, Output, State, colour, number, property_grid, text as text_property, toggle,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Setting {
    Host,
    Port,
    Timeout,
    Retries,
    Secure,
    Accent,
}

#[derive(Debug, Clone)]
pub enum Message {
    Settings(Event<Setting>),
}

#[derive(Debug)]
pub struct Example {
    settings: State<Setting>,
    last: String,
}

fn host(value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err("Enter a host name".into());
    }
    if value.contains(' ') {
        return Err("Host names have no spaces".into());
    }
    Ok(())
}

impl Default for Example {
    fn default() -> Self {
        let mut settings = State::new([
            text_property(Setting::Host, "Host", "db.internal")
                .validate(host)
                .default("localhost"),
            number(Setting::Port, "Port", 5432.0)
                .range(1.0, 65535.0)
                .step(1.0)
                .default(5432.0),
            number(Setting::Timeout, "Timeout", 30.0)
                .range(0.5, 120.0)
                .step(0.5)
                .decimals(1)
                .unit("s")
                .default(30.0),
            number(Setting::Retries, "Retries", 5.0)
                .range(0.0, 10.0)
                .step(1.0)
                .default(3.0),
            toggle(Setting::Secure, "Require TLS", true).default(true),
            colour(
                Setting::Accent,
                "Status colour",
                Color::from_rgb8(0x16, 0xa3, 0x4a),
            )
            .default(Color::from_rgb8(0x16, 0xa3, 0x4a)),
        ]);
        let _ = settings.update(Event::Edit(Setting::Port));
        let _ = settings.update(Event::Input(Setting::Port, "70000".into()));

        Self {
            settings,
            last: "Port stays 5432 until the draft is valid.".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Settings(event) = message;
        let (id, verb) = match self.settings.update(event) {
            Some(Output::Changed { id, .. }) => (id, "changed to"),
            Some(Output::Reset { id, .. }) => (id, "reset to"),
            Some(Output::CopyValue(_)) | None => return,
        };
        if let Some(property) = self.settings.property(id) {
            self.last = format!("{} {verb} {}", property.name, property.display());
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            property_grid(&self.settings)
                .toolbar(false)
                .on_event(Message::Settings),
            text(&self.last).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(340)
        .into()
    }
}
