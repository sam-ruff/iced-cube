use iced::{Element, Length};
use iced_cube::data::data_table::{self, Align, Event, State, data_table};

#[derive(Debug, Clone)]
pub struct Invoice {
    number: u32,
    customer: String,
    method: &'static str,
    amount: f64,
}

const CUSTOMERS: [&str; 7] = [
    "Fernhill Analytics",
    "Northwind Traders",
    "Blue Harbour Ltd",
    "Oakridge Studio",
    "Maple & Finch",
    "Riverside Clinic",
    "Kestrel Logistics",
];
const METHODS: [&str; 3] = ["Card", "Bank transfer", "Direct debit"];

/// 243 invoices, as a server might return them.
fn invoices() -> Vec<Invoice> {
    (0..243_u32)
        .map(|index| Invoice {
            number: 1001 + index,
            customer: CUSTOMERS[index as usize % CUSTOMERS.len()].to_owned(),
            method: METHODS[index as usize % METHODS.len()],
            amount: f64::from((index * 7919) % 5000) / 4.0 + 20.0,
        })
        .collect()
}

#[derive(Debug, Clone)]
pub enum Message {
    Invoices(Event<u32>),
}

#[derive(Debug)]
pub struct Example {
    invoices: State<Invoice, u32>,
}

impl Default for Example {
    fn default() -> Self {
        let columns = [
            data_table::column("number", "Invoice", |i: &Invoice| i.number.into())
                .width(110)
                .format(|i| format!("INV-{}", i.number)),
            data_table::column("customer", "Customer", |i: &Invoice| {
                i.customer.clone().into()
            })
            .width(Length::FillPortion(3)),
            data_table::column("method", "Method", |i: &Invoice| i.method.into())
                .width(Length::FillPortion(2))
                .filterable(true),
            data_table::column("amount", "Amount", |i: &Invoice| i.amount.into())
                .align(Align::End)
                .format(|i| format!("${:.2}", i.amount)),
        ];

        Self {
            invoices: State::new(columns, invoices(), |i| i.number)
                .with_page_sizes([10, 25, 50, 100]),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Invoices(event) => {
                let _ = self.invoices.update(event);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        // A fixed height scrolls the rows under a header that stays put.
        data_table(&self.invoices)
            .placeholder("Search invoices...")
            .height(280)
            .on_event(Message::Invoices)
            .into()
    }
}
