use iced::keyboard::Modifiers;
use iced::widget::{column, row, space, text};
use iced::{Element, Length};
use iced_cube::data::virtual_list::{Event, Mode, Output, State, virtual_list};
use iced_cube::theme::{Tokens, text_size};

#[derive(Debug, Clone)]
pub enum Message {
    Orders(Event<Order>),
}

#[derive(Debug, Clone)]
pub struct Order {
    number: u32,
    customer: &'static str,
    total: f32,
}

const CUSTOMERS: [&str; 6] = ["Avery", "Blake", "Casey", "Devon", "Emery", "Finley"];

#[derive(Debug)]
pub struct Example {
    orders: State<Order>,
    selected: usize,
}

impl Default for Example {
    fn default() -> Self {
        let orders = (0..100_000u32).map(|number| Order {
            number: 10_000 + number,
            customer: CUSTOMERS[number as usize % CUSTOMERS.len()],
            total: (number % 97) as f32 * 3.25 + 12.0,
        });

        let mut orders = State::new(orders).with_mode(Mode::Multiple);
        let _ = orders.update(Event::Press(1, Modifiers::empty()));
        let _ = orders.update(Event::Press(3, Modifiers::SHIFT));

        Self {
            selected: orders.selected().len(),
            orders,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Orders(event) = message;
        if let Some(Output::Selected(indices)) = self.orders.update(event) {
            self.selected = indices.len();
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let list = virtual_list(&self.orders, |_, order: &Order| {
            row![
                text(format!("#{}", order.number)).size(text_size::SM),
                text(order.customer).size(text_size::SM),
                space::horizontal(),
                text(format!("${:.2}", order.total))
                    .size(text_size::SM)
                    .style(|theme| text::Style {
                        color: Some(Tokens::of(theme).muted_foreground),
                    }),
            ]
            .spacing(12)
            .into()
        })
        .height(240)
        .on_event(Message::Orders);

        let summary = format!(
            "{} of 100,000 orders selected. Shift and Ctrl click to select several.",
            self.selected
        );

        column![list, text(summary).size(text_size::SM)]
            .spacing(12)
            .width(Length::Fill)
            .max_width(420)
            .into()
    }
}
