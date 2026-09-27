use iced::{Element, Length};
use iced_cube::data::data_table::{self, Event, State, data_table};

#[derive(Debug, Clone)]
pub struct Customer {
    name: &'static str,
    plan: &'static str,
}

#[derive(Debug, Clone)]
pub enum Message {
    Customers(Event<&'static str>),
}

#[derive(Debug)]
pub struct Example {
    customers: State<Customer, &'static str>,
}

impl Default for Example {
    fn default() -> Self {
        let columns = [
            data_table::column("name", "Customer", |c: &Customer| c.name.into())
                .width(Length::FillPortion(2)),
            data_table::column("plan", "Plan", |c: &Customer| c.plan.into()).filterable(true),
        ];
        let rows = [
            Customer {
                name: "Oakridge Studio",
                plan: "Team",
            },
            Customer {
                name: "Kestrel Logistics",
                plan: "Enterprise",
            },
        ];
        let mut customers = State::new(columns, rows, |c| c.name);
        // A search that matches nothing, so the empty state shows.
        let _ = customers.update(Event::Search("northwind".into()));

        Self { customers }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Customers(event) => {
                let _ = self.customers.update(event);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        data_table(&self.customers)
            .placeholder("Search customers...")
            .empty("No customers match your search.")
            .on_event(Message::Customers)
            .into()
    }
}
