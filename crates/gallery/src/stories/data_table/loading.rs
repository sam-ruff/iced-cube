use iced::widget::column;
use iced::{Element, Length};
use iced_cube::data::data_table::{self, Align, Event, State, data_table};
use iced_cube::primitives::switch::switch;

#[derive(Debug, Clone)]
pub struct Order {
    id: u32,
    customer: &'static str,
    total: f64,
}

#[derive(Debug, Clone)]
pub enum Message {
    Orders(Event<u32>),
    Loading(bool),
}

#[derive(Debug)]
pub struct Example {
    orders: State<Order, u32>,
    loading: bool,
}

impl Default for Example {
    fn default() -> Self {
        let columns = [
            data_table::column("id", "Order", |o: &Order| o.id.into()).width(90),
            data_table::column("customer", "Customer", |o: &Order| o.customer.into())
                .width(Length::FillPortion(2)),
            data_table::column("total", "Total", |o: &Order| o.total.into())
                .align(Align::End)
                .format(|o| format!("${:.2}", o.total)),
        ];
        let rows = [
            Order {
                id: 1042,
                customer: "Oakridge Studio",
                total: 189.0,
            },
            Order {
                id: 1043,
                customer: "Maple & Finch",
                total: 64.5,
            },
            Order {
                id: 1044,
                customer: "Riverside Clinic",
                total: 1210.0,
            },
        ];

        Self {
            orders: State::new(columns, rows, |o| o.id),
            loading: true,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Orders(event) => {
                let _ = self.orders.update(event);
            }
            Message::Loading(loading) => self.loading = loading,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            switch(self.loading)
                .label("Loading")
                .on_toggle(Message::Loading),
            data_table(&self.orders)
                .toolbar(false)
                .loading(self.loading)
                .on_event(Message::Orders),
        ]
        .spacing(16)
        .into()
    }
}
