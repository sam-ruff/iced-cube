use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::{breadcrumb, crumb, lucide};

const PATH: [&str; 4] = ["Home", "Documents", "Invoices", "2024"];

#[derive(Debug, Clone)]
pub enum Message {
    Open(usize),
}

#[derive(Debug)]
pub struct Example {
    depth: usize,
}

impl Default for Example {
    fn default() -> Self {
        Self { depth: PATH.len() }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Open(index) = message;
        self.depth = index + 1;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let crumbs = PATH[..self.depth].iter().enumerate().map(|(index, name)| {
            let crumb = crumb(*name).on_press(Message::Open(index));
            if index == 0 {
                crumb.icon(lucide!(House))
            } else {
                crumb
            }
        });

        let folder = format!("Viewing {}", PATH[..self.depth].join("/"));
        column![breadcrumb(crumbs), text(folder).size(14)]
            .spacing(12)
            .width(Length::Fill)
            .max_width(360)
            .into()
    }
}
