use iced::Element;
use iced_cube::data::calendar::{self, Entries, Entry, EntryId, Repeat, View, calendar};
use jiff::civil::date;

#[derive(Debug, Clone)]
pub enum Message {
    Calendar(calendar::Event),
}

#[derive(Debug)]
pub struct Example {
    calendar: calendar::State,
}

impl Default for Example {
    fn default() -> Self {
        let entries = Entries::utc().with([
            Entry::all_day(
                EntryId(1),
                "Conference",
                date(2026, 3, 9),
                date(2026, 3, 11),
            ),
            Entry::all_day(EntryId(2), "Holiday", date(2026, 6, 26), date(2026, 7, 3)),
            Entry::all_day(EntryId(3), "Invoices", date(2026, 1, 31), date(2026, 1, 31))
                .repeat(Repeat::monthly()),
        ]);

        Self {
            calendar: calendar::State::new(date(2026, 6, 18))
                .with_entries(entries)
                .with_view(View::Year),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Calendar(event) = message;
        // The menu in the header switches to the other views. Selecting a
        // day first means they open on that day.
        let _ = self.calendar.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        calendar(&self.calendar)
            .hours(8..18)
            .on_event(Message::Calendar)
            .into()
    }
}
