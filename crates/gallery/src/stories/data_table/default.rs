use iced::{Element, Length};
use iced_cube::badge;
use iced_cube::data::data_table::{self, Align, Event, State, data_table};
use iced_cube::feedback::badge::Variant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Paid,
    Pending,
    Processing,
    Failed,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Status::Paid => "Paid",
            Status::Pending => "Pending",
            Status::Processing => "Processing",
            Status::Failed => "Failed",
        }
    }

    fn badge(self) -> Variant {
        match self {
            Status::Paid => Variant::Success,
            Status::Pending => Variant::Secondary,
            Status::Processing => Variant::Outline,
            Status::Failed => Variant::Destructive,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Payment {
    id: &'static str,
    status: Status,
    email: &'static str,
    amount: f64,
}

const PAYMENTS: [Payment; 6] = [
    Payment {
        id: "PAY-3021",
        status: Status::Paid,
        email: "ken99@example.com",
        amount: 316.0,
    },
    Payment {
        id: "PAY-3022",
        status: Status::Paid,
        email: "abe45@example.com",
        amount: 242.0,
    },
    Payment {
        id: "PAY-3023",
        status: Status::Processing,
        email: "monserrat44@example.com",
        amount: 837.0,
    },
    Payment {
        id: "PAY-3024",
        status: Status::Failed,
        email: "silas22@example.com",
        amount: 874.5,
    },
    Payment {
        id: "PAY-3025",
        status: Status::Pending,
        email: "carmella@example.com",
        amount: 721.0,
    },
    Payment {
        id: "PAY-3026",
        status: Status::Paid,
        email: "jordan.lee@example.com",
        amount: 1250.0,
    },
];

#[derive(Debug, Clone)]
pub enum Message {
    Payments(Event<&'static str>),
}

#[derive(Debug)]
pub struct Example {
    payments: State<Payment, &'static str>,
}

impl Default for Example {
    fn default() -> Self {
        let columns = [
            data_table::column("status", "Status", |p: &Payment| p.status.label().into())
                .width(130)
                .filterable(true),
            data_table::column("email", "Email", |p: &Payment| p.email.into())
                .width(Length::FillPortion(3)),
            data_table::column("amount", "Amount", |p: &Payment| p.amount.into())
                .align(Align::End)
                .format(|p| format!("${:.2}", p.amount)),
        ];

        Self {
            payments: State::new(columns, PAYMENTS, |p| p.id),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Payments(event) => {
                let _ = self.payments.update(event);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        data_table(&self.payments)
            .placeholder("Search payments...")
            .cell("status", |p: &Payment| {
                badge(p.status.label()).variant(p.status.badge()).into()
            })
            .on_event(Message::Payments)
            .into()
    }
}
