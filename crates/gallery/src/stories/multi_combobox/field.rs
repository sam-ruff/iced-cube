use iced::widget::column;
use iced::{Element, Length};
use iced_cube::field;
use iced_cube::forms::multi_combobox::{self, State, multi_combobox};
use iced_cube::primitives::input::Size;

const PEOPLE: &[&str] = &[
    "Ada Lovelace",
    "Alan Turing",
    "Barbara Liskov",
    "Grace Hopper",
    "Katherine Johnson",
    "Linus Torvalds",
];

const LABELS: &[&str] = &["Bug", "Design", "Docs", "Release", "Security"];

#[derive(Debug, Clone)]
pub enum Message {
    Members(multi_combobox::Event),
    Labels(multi_combobox::Event),
}

#[derive(Debug)]
pub struct Example {
    members: State<&'static str>,
    labels: State<&'static str>,
    owners: State<&'static str>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            members: State::new(PEOPLE.iter().copied()),
            labels: State::new(LABELS.iter().copied()).with_chosen(&["Design", "Docs"]),
            owners: State::new(["Admins", "Owners"]).with_chosen(&["Admins", "Owners"]),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Members(event) => {
                let _ = self.members.update(event);
            }
            Message::Labels(event) => {
                let _ = self.labels.update(event);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        // A group needs at least one member.
        let members_error = self
            .members
            .chosen()
            .next()
            .is_none()
            .then_some("Add at least one member.");

        column![
            field(
                "Members",
                multi_combobox(&self.members)
                    .placeholder("Search people...")
                    .width(Length::Fill)
                    .invalid(members_error.is_some())
                    .on_event(Message::Members),
            )
            .required(true)
            .error_maybe(members_error),
            field(
                "Labels",
                multi_combobox(&self.labels)
                    .placeholder("Add labels...")
                    .size(Size::Sm)
                    .width(Length::Fill)
                    .on_event(Message::Labels),
            ),
            field(
                "Managed by",
                multi_combobox(&self.owners).width(Length::Fill)
            )
            .description("Set by your organisation.")
            .disabled(true),
        ]
        .spacing(20)
        .width(Length::Fixed(320.0))
        .height(Length::Fill)
        .into()
    }
}
