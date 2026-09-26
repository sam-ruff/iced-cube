use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::navigation::tabs::{self, State, tab, tabs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Overview,
    Activity,
    Settings,
    Billing,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tabs(tabs::Event<Section>),
}

#[derive(Debug)]
pub struct Example {
    tabs: State<Section>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            tabs: State::new([
                tab(Section::Overview, "Overview"),
                tab(Section::Activity, "Activity"),
                tab(Section::Settings, "Settings"),
                tab(Section::Billing, "Billing").disabled(true),
            ]),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Tabs(event) = message;
        let _ = self.tabs.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let body = match self.tabs.selected() {
            Some(Section::Overview) => "Three projects, two of them active this week.",
            Some(Section::Activity) => "Nothing new since your last visit.",
            Some(Section::Settings) => "Project name, visibility and members.",
            Some(Section::Billing) | None => "",
        };

        column![
            tabs(&self.tabs).on_event(Message::Tabs),
            text(body).size(14),
        ]
        .spacing(16)
        .width(Length::Fixed(420.0))
        .into()
    }
}
