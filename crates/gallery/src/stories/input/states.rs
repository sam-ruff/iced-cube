use iced::widget::column;
use iced::{Element, Length};
use iced_cube::{input, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    UsernameChanged(String),
}

#[derive(Debug)]
pub struct Example {
    username: String,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            username: "ada lovelace".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::UsernameChanged(username) = message;
        self.username = username;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let invalid = self.username.contains(char::is_whitespace);

        column![
            input("Username", &self.username)
                .icon(lucide!(AtSign))
                .invalid(invalid)
                .on_input(Message::UsernameChanged),
            input("Workspace", "acme-corp").icon(lucide!(Building)),
        ]
        .spacing(12)
        .width(Length::Fixed(320.0))
        .into()
    }
}
