use iced::widget::column;
use iced::{Element, Length};
use iced_cube::{field, input, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    EmailChanged(String),
}

#[derive(Debug)]
pub struct Example {
    email: String,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            email: "ada@".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::EmailChanged(email) = message;
        self.email = email;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let error = email_error(&self.email);

        column![
            field(
                "Email",
                input("you@example.com", &self.email)
                    .icon(lucide!(Mail))
                    .invalid(error.is_some())
                    .on_input(Message::EmailChanged),
            )
            .required(true)
            .error_maybe(error),
            field("Workspace", input("acme-corp", ""))
                .description("Only an owner can change the workspace.")
                .disabled(true),
        ]
        .spacing(20)
        .width(Length::Fixed(320.0))
        .into()
    }
}

fn email_error(email: &str) -> Option<&'static str> {
    let Some((user, domain)) = email.split_once('@') else {
        return Some("An email address needs an @.");
    };
    if user.is_empty() || !domain.contains('.') {
        return Some("Enter a full address, like ada@example.com.");
    }
    None
}
