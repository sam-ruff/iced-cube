use iced::widget::{column, row};
use iced::{Alignment, Element, Length};
use iced_cube::data::data_table::{self, Align, Event, State, data_table};
use iced_cube::feedback::badge::Variant as BadgeVariant;
use iced_cube::primitives::button::{Size, Variant};
use iced_cube::{badge, button, lucide};

#[derive(Debug, Clone)]
pub struct Payment {
    id: u32,
    email: &'static str,
    status: &'static str,
    amount: f64,
}

fn payment(id: u32, email: &'static str, status: &'static str, amount: f64) -> Payment {
    Payment {
        id,
        email,
        status,
        amount,
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Payments(Event<u32>),
    MarkPaid,
    Delete,
}

#[derive(Debug)]
pub struct Example {
    payments: State<Payment, u32>,
}

impl Default for Example {
    fn default() -> Self {
        let columns = [
            data_table::column("email", "Email", |p: &Payment| p.email.into())
                .width(Length::FillPortion(3)),
            data_table::column("status", "Status", |p: &Payment| p.status.into()).width(110),
            data_table::column("amount", "Amount", |p: &Payment| p.amount.into())
                .align(Align::End)
                .format(|p| format!("${:.2}", p.amount)),
        ];
        let rows = [
            payment(1, "ken99@example.com", "Pending", 316.0),
            payment(2, "abe45@example.com", "Paid", 242.0),
            payment(3, "monserrat44@example.com", "Pending", 837.0),
            payment(4, "silas22@example.com", "Failed", 874.5),
            payment(5, "carmella@example.com", "Pending", 721.0),
        ];
        let mut payments = State::new(columns, rows, |p| p.id).with_selection(true);
        let _ = payments.update(Event::Select(1, true));
        let _ = payments.update(Event::Select(3, true));

        Self { payments }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Payments(event) => {
                let _ = self.payments.update(event);
            }
            Message::MarkPaid => {
                for id in self.payments.selected() {
                    self.payments.update_row(&id, |p| p.status = "Paid");
                }
                let _ = self.payments.update(Event::ClearSelection);
            }
            Message::Delete => {
                let selected = self.payments.selected();
                self.payments.retain(|p| !selected.contains(&p.id));
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let count = self.payments.selected().len();
        let any = count > 0;
        let actions = row![
            badge(format!("{count} selected")).variant(if any {
                BadgeVariant::Default
            } else {
                BadgeVariant::Secondary
            }),
            button("Mark as paid")
                .icon(lucide!(Check))
                .variant(Variant::Outline)
                .size(Size::Sm)
                .on_press_maybe(any.then_some(Message::MarkPaid)),
            button("Delete")
                .icon(lucide!(Trash))
                .variant(Variant::Destructive)
                .size(Size::Sm)
                .on_press_maybe(any.then_some(Message::Delete)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap();

        column![
            actions,
            data_table(&self.payments)
                .toolbar(false)
                .on_event(Message::Payments),
        ]
        .spacing(12)
        .into()
    }
}
