---
title: Calendar
description: Entries in day, week, month, year, schedule and four-day views, with triggers for when they are created, due or about to be.
group: Data
order: 6
module: data::calendar
imports: |
  use iced_cube::calendar;
  use iced_cube::data::calendar::{Entries, Entry, EntryId, Output, Repeat, State, Tone, View};
  use iced_cube::data::calendar::schedule::{Anchor, Scheduler, Trigger};
keywords: [events, schedule, scheduler, reminders, alarm, agenda, month, week, date, appointment]
related: [toast, data-table, dialog]
hero: calendar/month
stories: [calendar/month, calendar/week, calendar/agenda, calendar/year, calendar/create, calendar/read-only, calendar/repeating, calendar/themed, calendar/keyboard, calendar/reminders, calendar/missed, calendar/triggers]
api:
  - name: "State::new(today) / State::at(now, entries)"
    description: "A calendar showing the month of today. The calendar never reads the clock, so today's date, or the current time for the now line, is passed in."
  - name: ".with_entries(entries) / .with_view(View) / .with_week_start(weekday) / .with_cursor(date) / .with_selection(selection)"
    description: "Sets the entries, the starting view (Day, Week, Month, Year, Agenda or FourDays), the day weeks start on (Monday by default), the period on show and what is selected."
  - name: ".read_only() / .with_allow(Allow) / state.set_allow(Allow)"
    description: "Limits what the user may do through the calendar itself: Allow has create, reschedule and remove, and Allow::NONE is read-only. Insert and Remove events from the app always apply."
  - name: "Entries::utc() / Entries::system() / Entries::new(time_zone)"
    description: "An empty collection shown in UTC, the system's time zone or one you name. Add to it with .with([...]) or .insert(entry)."
  - name: "Entry::new(id, title, start, end) / Entry::all_day(id, title, first, last)"
    description: "A timed entry between two instants, or one covering whole days. Chain .tone(Tone), .tag(\"work\") and .repeat(Repeat)."
  - name: "instant(date, time, &time_zone)"
    description: "The instant it is a time on a date in a time zone, for building entries without handling errors."
  - name: "Repeat::daily() / weekly() / monthly() / yearly()"
    description: "How an entry repeats. Chain .interval(n) for every n units, and .count(n) or .until(date) to stop."
  - name: "state.update(Event)"
    description: "Applies an event and returns an Output: Selected, Opened, Requested, Changed or Viewed."
  - name: "Event::Press(Selection)"
    description: "A press on a day, a time slot or an entry. The first press selects it and a press on what is already selected activates it."
  - name: "Event::Insert(entry) / Event::Remove(id) / Event::Reschedule { .. }"
    description: "Adds or replaces, removes or moves an entry, and returns Output::Changed with what changed."
  - name: "Event::Show(View) / Next / Previous / Today / GoTo(date) / OpenDay(date)"
    description: "Switches view or moves the period on show. Returns Output::Viewed with the new range when it changes."
  - name: "Event::Tick(now)"
    description: "Sets the current time, which moves today and the now line."
  - name: "Output::Requested(draft)"
    description: "The user asked for a new entry, by pressing a day or time twice or dragging across empty space. Nothing is added until you send Event::Insert."
  - name: "Output::Opened { entry, start }"
    description: "The user pressed a selected entry again. Show or edit it."
  - name: "state.entries() / state.selection() / state.selected_entry() / state.range() / state.title()"
    description: "The entries, what is selected, the first and last date on show and the period in words."
  - name: "entries.occurrences(from, to) / entries.between(first, last) / entries.on(date)"
    description: "Every occurrence in a range, with repeating entries expanded."
  - name: "calendar(&state)"
    description: "Renders the calendar. Until .on_event is set it is read-only: entries keep their colours, and nothing can be pressed or dragged."
  - name: ".on_event(f)"
    description: "Maps calendar events to your message, such as Message::Calendar."
  - name: ".hours(8..18) / .hour_height(pixels)"
    description: "The hours the week and day views show, and the least height of each. Fewer hours stretch to fill the space; the whole day scrolls."
  - name: ".header(bool) / .width(length) / .height(length)"
    description: "Hides the Today button, the arrows, the title and the menu of views, or sets the size. The calendar fills the space it is given."
  - name: "calendar::scroll_to_hour(hour, hour_height) / calendar::scroll_id()"
    description: "A task that scrolls the week or day view so an hour is at the top, such as 8 when the calendar opens, and the id of the scrollable for anything else."
  - name: "editor::State::new(&draft, &time_zone) / editor::State::edit(&entry, &time_zone)"
    description: "The state of the entry form, started from the span the calendar asked for or from an existing entry."
  - name: "editor(&form).on_event(f).on_submit(message)"
    description: "Renders the form: title, all day, start and end date and time, repeat and colour. on_submit is sent on Enter in the title. .shortcuts(Shortcuts) sets the title field's editing shortcuts."
  - name: "form.update(editor::Event) / form.entry(new_id) / form.when()"
    description: "Applies a change, and gives the entry or span the form describes. A changed entry keeps its id; a blank title is saved as \"(No title)\"."
  - name: "Scheduler::new().with(trigger)"
    description: "Collects triggers. Chain .resume_from(watermark) to receive what was missed since a saved time, and .grace(duration) to change what counts as late."
  - name: "Trigger::new(key).at(anchor) / .before(anchor, lead) / .after(anchor, delay) / .on(change)"
    description: "A rule with your own key. Anchors are Start and End; changes are Created, Updated and Removed. Chain .every(period) to repeat before the anchor and .scope(Scope) to limit it to a tag or an entry."
  - name: "scheduler.poll(&entries, now)"
    description: "Returns every Fired that came due since the last poll, earliest first, each once. Late ones carry Delivery::Missed with how late they are."
  - name: "scheduler.observe(&changed)"
    description: "Returns the Fired for triggers on changes. Pass what Output::Changed holds."
  - name: "scheduler.snooze(&fired, until) / scheduler.watermark() / scheduler.next_due(&entries, after)"
    description: "Delivers a fired trigger again later, gives the time of the last poll to save, and finds the next time anything is due."
  - name: "schedule::timer(&scheduler, &entries) / schedule::clock()"
    description: "Subscriptions that tick every 30 seconds with the current time: the timer while the scheduler has a timed trigger and there are entries, the clock always."
  - name: "calendar::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.event(&state)"
    description: "Turns a resolved Action into the Event to pass to state.update, or None when it would do nothing."
---

The app owns a `calendar::State` and routes events into it. The state holds the entries, so adding, moving and removing them all go through `state.update`, and each change comes back as `Output::Changed`.

## Views

The header has a Today button, arrows for the previous and next period, the period in words and a menu of views:

- **Day**, **Week** and **4 days** put days side by side against the hours, with whole-day entries in a strip along the top and a line at the current time. Four days start on the day on show, where a week starts on Monday or the day you set.
- **Month** is a grid of weeks. Whole-day entries and ones that span days are bars; a timed entry within a day is a dot in its tone, its start time and its title. A day with more than fit shows a count that opens the day.
- **Year** shows twelve small months, with a dot under each day that has an entry. Select a day and switch view to see it.
- **Schedule** lists the month's entries, each day's date beside them. In code it is `View::Agenda`.

Switching view goes to the selected day, so pressing a day in the year or the month and choosing Day opens that day.

## Passing in the time

The calendar never reads the clock. Give it today's date with `State::new(today)`, or the current time with `State::at(now, entries)`, and keep it current by sending `Event::Tick(now)` from `schedule::clock()`. That keeps "today" right in an app left open overnight, and makes every test repeatable.

Timed entries are stored as instants and shown in the time zone of the `Entries` collection. Whole-day entries are dates, so a holiday is the same day wherever it is shown.

## Selecting, opening and creating

The first press on a day, a time or an entry selects it. A press on what is already selected activates it: an entry returns `Output::Opened`, and a day or a time returns `Output::Requested` with the span to create. Two quick presses therefore open or create, and so do two taps on a touch screen.

In the week and day views a mouse can also drag: across empty space to ask for an entry of that length, on an entry to move it, and on its lower edge to change when it ends. Drags snap to 15 minutes. A finger scrolls the hours instead, so on a touch screen entries are created with two taps and edited in whatever the app opens.

`Requested` and `Opened` are where the app shows its dialog or side panel, as the "Creating and editing" example does.

## The entry form

`calendar::editor` is a ready-made form for that dialog: a title, an "All day" switch, a start and an end each with a date and a time, how the entry repeats and its colour. It has no buttons, so it fits a dialog, a popover or a side panel with whatever actions the app wants.

```rust
use iced_cube::data::calendar::editor::{self, editor};

// The calendar asked for a new entry, or the user opened one.
let form = editor::State::new(&draft, calendar.time_zone());
let form = editor::State::edit(entry, calendar.time_zone());

// In the dialog's body.
editor(&form).on_event(Message::Form).on_submit(Message::Save)

// On Save. A new entry takes the id given; a changed one keeps its own.
let entry = form.entry(calendar.entries().next_id());
calendar.update(calendar::Event::Insert(entry));
```

A date opens a small month to pick from. The time menus step in quarter hours: the start's opens an hour before the time chosen, and the end's begins just after the start. Moving the start carries the end along, so the entry keeps its length, and an end earlier than the start on the same day is taken to mean the next day. Turning "All day" on hides the times and keeps the dates. A repeat rule the form does not offer, such as every two weeks, shows as "Custom" and is saved unchanged.

The form is optional. An app that wants different fields builds its own and sends `Event::Insert`.

## Read-only calendars and your own controls

`State::read_only()` makes a calendar people can look through but not change: days and entries can still be selected, entries opened, and the period and view switched, but a second press or a drag asks for nothing, entries cannot be dragged, and the Remove shortcut does nothing. For something in between, `with_allow(Allow { .. })` sets `create`, `reschedule` and `remove` one by one, and `set_allow` changes them later, such as when a shared calendar comes on show.

These limits are about the calendar's own gestures and shortcuts. `Event::Insert` and `Event::Remove` always apply, so anything in your app can change the entries: a form, a toolbar button, a context menu, a sync from a server. Build whatever controls suit the app and send those events; the "Read-only, with your own controls" example does this with a text field and two buttons. The same works on an editable calendar, where your controls sit beside the built-in gestures, and with `.header(false)` you can replace the header too, using `state.title()` and the navigation events.

A calendar with no `.on_event` at all is a picture: it shows the entries and responds to nothing.

## Repeating entries

A repeating entry is one entry with a rule. Occurrences are counted from the first, so an entry on the 31st that repeats monthly uses the last day of shorter months and returns to the 31st afterwards, and a timed entry keeps its local time when the clocks change. Dragging one occurrence moves the whole series by the same amount. Rules are simple: there is no "last Friday of the month" and no way to skip a single occurrence.

## Triggers

Triggers tell the app when an entry is created, due, almost due or over. They are built from a few base parts and carry a key of your own type. The [Calendar events](../../calendar-events/) guide works through them, from a single reminder to catching up on the ones missed while the app was closed.

## Theming

The calendar has no colours of its own. Surfaces, lines, text, today's pill, the selection ring and the now line come from the theme's tokens, so it follows the light and dark themes and any palette built with `theme::Config`, like every other component; see [Theming](../../theming/).

An entry's `Tone` names a token, not a colour: `Primary`, `Neutral` (the muted text colour), `Success`, `Warning` and `Destructive`. An entry is drawn as a wash of that colour over the background, and filled with it when selected. With the default palettes the primary is near black or near white, so `Primary` and `Neutral` look alike; give the theme a coloured primary, as the "Custom theme" example does, to tell them apart. For finer control, the functions in `calendar::style` return each part's colours from a `Tokens` value.

## Narrow screens

Below 480 pixels the month shows a dot for each entry and lists the selected day underneath, and the schedule puts each date above its entries. A week or four days too narrow for their columns show one day under a strip of the days to pick from. The year drops to as many months across as fit. The header keeps the title beside the menu of views and moves its buttons to a second line.

## Limits

- Day and month names are in English, and times use the 24-hour clock.
- One time zone per calendar.
- Entries cannot be dragged in the month view.
- A calendar showing the whole day opens at midnight. Run `calendar::scroll_to_hour(8, hour_height)` once it is on screen to start at the working day, or limit it with `.hours(...)`.
- Removing an entry from the keyboard does not ask first. `Output::Changed` carries the removed entry, so an app can offer to undo it, or unbind `Action::Remove`.
