use iced::widget::column;
use iced::{Element, Length};
use iced_cube::navigation::breadcrumb::Separator;
use iced_cube::{breadcrumb, crumb};

const TRAIL: [&str; 6] = [
    "Workspace",
    "Engineering",
    "Platform",
    "Services",
    "Billing",
    "Settings",
];

#[derive(Debug, Clone)]
pub enum Message {
    Open(usize),
    Expand,
}

#[derive(Debug, Default)]
pub struct Example {
    expanded: bool,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        // Opening a level collapses the trail again.
        self.expanded = matches!(message, Message::Expand);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let crumbs = TRAIL
            .iter()
            .enumerate()
            .map(|(index, name)| crumb(*name).on_press(Message::Open(index)));

        let mut trail = breadcrumb(crumbs).separator(Separator::Slash);
        if !self.expanded {
            trail = trail.max_items(3).on_expand(Message::Expand);
        }

        column![trail].width(Length::Fill).max_width(420).into()
    }
}
