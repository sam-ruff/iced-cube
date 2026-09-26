use iced::widget::{column, text};
use iced::{Alignment, Element};
use iced_cube::lucide;
use iced_cube::navigation::tabs::{self, State, Variant, tab, tabs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    List,
    Board,
    Calendar,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tabs(tabs::Event<View>),
}

#[derive(Debug)]
pub struct Example {
    tabs: State<View>,
}

impl Default for Example {
    fn default() -> Self {
        let tabs = State::new([
            tab(View::List, "List").icon(lucide!(List)),
            tab(View::Board, "Board").icon(lucide!(Columns3)),
            tab(View::Calendar, "Calendar").icon(lucide!(Calendar)),
        ]);
        Self {
            tabs: tabs.with_selected(View::Board),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Tabs(event) = message;
        let _ = self.tabs.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let showing = self
            .tabs
            .selected()
            .map_or(String::new(), |view| format!("Showing the {view:?} view"));

        column![
            tabs(&self.tabs)
                .variant(Variant::Pills)
                .on_event(Message::Tabs),
            text(showing).size(14),
        ]
        .spacing(16)
        .align_x(Alignment::Center)
        .into()
    }
}
