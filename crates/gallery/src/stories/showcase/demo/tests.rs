use iced::keyboard::{Key, Modifiers, key::Named};
use iced_cube::context_menu;
use iced_cube::dropdown_menu;
use iced_cube::keys;
use iced_cube::overlay::toast::{self, Variant as ToastVariant};

use super::data::{JobId, Level, LogLine, Queue, Schedule, Status, StatusFilter};
use super::feed::{self, Update};
use super::settings::{self, Control};
use super::{Example, LOG_LIMIT, Message, Page, Pending, RowAction, ThemeMode};
use crate::app::ThemeChoice;
use crate::bridge;

fn send(example: &mut Example, message: Message) {
    let _ = example.update(message);
}

fn press(example: &mut Example, key: Key, modifiers: Modifiers) {
    send(example, Message::Key(keys::Event { key, modifiers }));
}

fn named(example: &mut Example, key: Named) {
    press(example, Key::Named(key), Modifiers::empty());
}

fn ctrl_k(example: &mut Example) {
    press(example, Key::Character("k".into()), Modifiers::COMMAND);
}

fn feed(example: &mut Example, updates: Vec<Update>) {
    send(example, Message::Feed(feed::Event::Received(updates)));
}

fn status(example: &Example, id: u32) -> Option<Status> {
    example.job(JobId(id)).map(|job| job.status)
}

fn visible(example: &Example) -> Vec<u32> {
    example.visible_jobs().map(|job| job.id.0).collect()
}

fn titles(example: &Example) -> Vec<String> {
    example
        .toasts
        .visible()
        .map(|(_, toast)| toast.title.clone())
        .collect()
}

fn undo_toast(example: &Example) -> toast::Id {
    let Some((id, _)) = example
        .toasts
        .visible()
        .find(|(_, toast)| toast.action.is_some())
    else {
        panic!("no undo toast");
    };
    id
}

#[test]
fn ctrl_k_opens_the_palette_and_closes_it_again() {
    let mut example = Example::default();
    ctrl_k(&mut example);
    assert!(example.palette_open);
    ctrl_k(&mut example);
    assert!(!example.palette_open);
}

#[test]
fn an_open_dialog_keeps_keys_from_the_page() {
    let mut example = Example::default();
    example.go(Page::Jobs);
    send(&mut example, Message::OpenNewJob);
    named(&mut example, Named::ArrowDown);
    ctrl_k(&mut example);
    assert_eq!(example.cursor, None);
    assert!(!example.palette_open);
}

#[test]
fn palette_commands_switch_pages_and_filter_jobs() {
    let mut example = Example::default();
    example.run(super::Cmd::FailedJobs);
    assert_eq!(example.page.selected(), Some(Page::Jobs));
    assert!(
        example
            .visible_jobs()
            .all(|job| job.status == Status::Failed)
    );
}

#[test]
fn the_palette_tells_jobs_with_the_same_name_apart() {
    let example = Example::default();
    let palette = example.palette_state();
    let hint = palette
        .groups()
        .iter()
        .flat_map(|group| &group.items)
        .find(|item| item.id == super::Cmd::Open(JobId(1049)))
        .and_then(|item| item.shortcut.clone());
    assert_eq!(hint.as_deref(), Some("JOB-1049"));
}

#[test]
fn arrows_space_and_delete_drive_the_job_list() {
    let mut example = Example::default();
    example.go(Page::Jobs);
    named(&mut example, Named::ArrowDown);
    assert_eq!(example.cursor, Some(JobId(1041)));
    named(&mut example, Named::ArrowDown);
    assert_eq!(example.cursor, Some(JobId(1042)));

    named(&mut example, Named::Space);
    assert!(example.job(JobId(1042)).is_some_and(|job| job.checked));

    named(&mut example, Named::Delete);
    assert_eq!(example.pending, Some(Pending::Delete(JobId(1042))));
    let (title, ..) = example.pending_text();
    assert_eq!(title, "Delete ledger-reconcile (JOB-1042)?");
}

#[test]
fn shift_f10_opens_the_menu_for_the_current_row() {
    let mut example = Example::default();
    example.go(Page::Jobs);
    named(&mut example, Named::ArrowDown);
    press(&mut example, Key::Named(Named::F10), Modifiers::SHIFT);
    assert!(example.row_menu.is_open_on(&JobId(1041)));
    assert_eq!(example.row_menu.position(), None);
}

#[test]
fn bulk_actions_only_touch_the_rows_on_show() {
    let mut example = Example::default();
    send(
        &mut example,
        Message::Filter(StatusFilter::Only(Status::Succeeded)),
    );
    send(&mut example, Message::CheckAll(true));
    send(
        &mut example,
        Message::Filter(StatusFilter::Only(Status::Failed)),
    );
    send(&mut example, Message::Check(JobId(1047), true));
    assert_eq!(example.selected(), [JobId(1047)]);

    send(&mut example, Message::DeleteSelected);
    let (title, ..) = example.pending_text();
    assert_eq!(title, "Delete 1 selected job?");
    send(&mut example, Message::Confirm);
    assert_eq!(status(&example, 1047), None);
    assert_eq!(status(&example, 1048), Some(Status::Succeeded));
    assert_eq!(status(&example, 1049), Some(Status::Succeeded));
}

#[test]
fn a_hidden_row_loses_its_tick() {
    let mut example = Example::default();
    send(&mut example, Message::Check(JobId(1049), true));
    send(&mut example, Message::Search("ledger".into()));
    send(&mut example, Message::Search(String::new()));
    assert!(example.job(JobId(1049)).is_some_and(|job| !job.checked));
}

#[test]
fn deleting_can_be_undone_from_the_toast() {
    let mut example = Example {
        pending: Some(Pending::Delete(JobId(1049))),
        ..Example::default()
    };
    send(&mut example, Message::Confirm);
    assert_eq!(status(&example, 1049), None);
    assert!(titles(&example).contains(&"Deleted crm-dedupe (JOB-1049)".to_owned()));

    let id = undo_toast(&example);
    send(&mut example, Message::Toast(toast::Event::Action(id)));
    assert_eq!(status(&example, 1049), Some(Status::Succeeded));
    assert!(titles(&example).contains(&"Restored crm-dedupe".to_owned()));
}

#[test]
fn alt_z_undoes_the_newest_delete_even_behind_other_toasts() {
    let mut example = Example::default();
    for title in ["one", "two", "three"] {
        example.notify(toast::toast(title));
    }
    example.pending = Some(Pending::Delete(JobId(1049)));
    send(&mut example, Message::Confirm);
    assert!(
        titles(&example).contains(&"Deleted crm-dedupe (JOB-1049)".to_owned()),
        "the undo toast shows at once"
    );

    press(&mut example, Key::Character("z".into()), Modifiers::ALT);
    assert_eq!(status(&example, 1049), Some(Status::Succeeded));
}

#[test]
fn undo_puts_several_jobs_back_in_their_places() {
    let mut example = Example::default();
    let order = |example: &Example| example.jobs.iter().map(|job| job.id).collect::<Vec<_>>();
    let before = order(&example);
    send(&mut example, Message::Check(JobId(1042), true));
    send(&mut example, Message::Check(JobId(1045), true));
    send(&mut example, Message::DeleteSelected);
    send(&mut example, Message::Confirm);
    assert_eq!(example.jobs.len(), before.len() - 2);
    assert!(titles(&example).contains(&"Deleted 2 jobs".to_owned()));

    let id = undo_toast(&example);
    send(&mut example, Message::Toast(toast::Event::Action(id)));
    assert_eq!(order(&example), before);
}

#[test]
fn new_jobs_wait_behind_the_pill_while_a_row_menu_is_open() {
    let mut example = Example::default();
    let row = JobId(1045);
    send(
        &mut example,
        Message::RowMenu(context_menu::Event::Open(row, iced::Point::ORIGIN)),
    );
    let before = visible(&example);
    send(&mut example, Message::Submit);
    assert_eq!(example.fresh.len(), 1);
    assert_eq!(visible(&example), before, "no row moves under the menu");

    send(
        &mut example,
        Message::RowMenu(context_menu::Event::Menu(dropdown_menu::Event::Close)),
    );
    assert!(example.fresh.is_empty());
    assert_eq!(visible(&example).len(), before.len() + 1);
}

#[test]
fn rows_stay_put_while_the_pointer_is_over_the_list() {
    let mut example = Example::default();
    send(
        &mut example,
        Message::Filter(StatusFilter::Only(Status::Running)),
    );
    send(&mut example, Message::ListHover(true));
    feed(&mut example, vec![Update::Succeeded(JobId(1043))]);
    assert!(visible(&example).contains(&1043));

    send(&mut example, Message::Submit);
    assert_eq!(example.fresh.len(), 1);
    assert_eq!(
        example.fresh_shown(),
        0,
        "a queued job does not pass the filter"
    );
    send(&mut example, Message::ShowNew);
    assert!(example.fresh.is_empty());

    send(&mut example, Message::ListHover(false));
    assert!(!visible(&example).contains(&1043));
}

#[test]
fn the_succeeded_total_only_goes_up() {
    let mut example = Example::default();
    let start = example.succeeded;
    example.pending = Some(Pending::ClearFinished);
    send(&mut example, Message::Confirm);
    assert_eq!(example.succeeded, start);

    feed(&mut example, vec![Update::Succeeded(JobId(1043))]);
    feed(&mut example, vec![Update::Succeeded(JobId(1043))]);
    assert_eq!(example.succeeded, start + 1);
}

#[test]
fn the_host_toggle_hands_the_theme_back_to_the_page() {
    let mut example = Example::default();
    example.set_theme(ThemeMode::Dark);
    assert!(example.theme().is_some());
    send(
        &mut example,
        Message::Host(bridge::Event::Theme(ThemeChoice::Light)),
    );
    assert_eq!(example.theme, ThemeMode::System);
    assert!(example.theme().is_none());
    assert!(example.menu.is_checked(super::MenuItem::System));
}

#[test]
fn settings_apply_on_save_and_reset_goes_back_to_the_last_save() {
    let mut example = Example::default();
    let settings = |message| Message::Settings(message);
    send(&mut example, settings(settings::Message::Concurrency(5)));
    assert_eq!(example.settings.applied().concurrency, 3);
    assert!(example.settings.is_dirty());

    send(&mut example, settings(settings::Message::Save));
    assert_eq!(example.settings.applied().concurrency, 5);
    assert!(titles(&example).contains(&"Settings saved".to_owned()));

    send(&mut example, settings(settings::Message::Concurrency(1)));
    send(&mut example, settings(settings::Message::Reset));
    assert_eq!(example.settings.concurrency, 5);
    assert!(!example.settings.is_dirty());
}

#[test]
fn the_scheduler_fills_free_slots_high_priority_first() {
    let mut example = Example::default();
    feed(&mut example, vec![Update::Succeeded(JobId(1043))]);
    assert_eq!(example.count(Status::Running), 3);
    // Both queued jobs are Normal or Low; the Normal one starts first.
    assert_eq!(status(&example, 1046), Some(Status::Running));
    assert_eq!(status(&example, 1045), Some(Status::Queued));
}

#[test]
fn a_paused_scheduler_starts_nothing() {
    let mut example = Example::default();
    example.settings.set_scheduler(false);
    example.settings.saved.retries = 0;
    feed(&mut example, vec![Update::Failed(JobId(1041), 0.5)]);
    assert_eq!(status(&example, 1041), Some(Status::Failed));
    assert_eq!(example.count(Status::Running), 2);
}

#[test]
fn failed_jobs_are_retried_before_the_owner_hears() {
    let mut example = Example::default();
    example.settings.set_scheduler(false);
    example.settings.saved.retries = 1;
    feed(&mut example, vec![Update::Failed(JobId(1041), 0.5)]);
    assert_eq!(status(&example, 1041), Some(Status::Queued));
    assert!(titles(&example).is_empty());

    feed(&mut example, vec![Update::Failed(JobId(1041), 0.5)]);
    assert_eq!(status(&example, 1041), Some(Status::Failed));
    let failure = example
        .toasts
        .visible()
        .find(|(_, toast)| toast.title == "nightly-orders-etl failed")
        .map(|(_, toast)| toast.description.clone());
    assert_eq!(
        failure.flatten().as_deref(),
        Some("It ran out of memory after 2 attempts. The owner was alerted by team chat.")
    );
}

#[test]
fn submissions_come_from_their_own_timer_and_stop_while_the_queue_is_long() {
    let mut example = Example::default();
    example.settings.set_scheduler(false);
    let jobs = example.jobs.len();
    send(&mut example, Message::Submit);
    assert_eq!(example.jobs.len(), jobs + 1);
    for _ in 0..6 {
        send(&mut example, Message::Submit);
    }
    assert_eq!(example.count(Status::Queued), super::QUEUE_LIMIT);
}

#[test]
fn a_scheduled_job_waits_for_its_schedule() {
    let mut example = Example::default();
    send(&mut example, Message::OpenNewJob);
    let draft = |message| Message::Draft(message);
    send(
        &mut example,
        draft(super::new_job::Message::Name("nightly-report".into())),
    );
    send(
        &mut example,
        draft(super::new_job::Message::Source(
            iced_cube::forms::combobox::Event::Activate(0),
        )),
    );
    send(
        &mut example,
        draft(super::new_job::Message::Schedule(Schedule::Nightly)),
    );
    send(&mut example, Message::CreateJob);

    let Some(job) = example.jobs.first() else {
        panic!("the job was created");
    };
    assert_eq!(job.name, "nightly-report");
    assert_eq!(job.status, Status::Scheduled);
    assert_eq!(job.detail(), "Next run: Nightly at 02:00");
}

#[test]
fn paused_logs_are_held_until_live_again_up_to_the_limit() {
    let mut example = Example::default();
    let before = example.logs.len();
    send(&mut example, Message::Live(false));
    feed(
        &mut example,
        vec![Update::Log(LogLine::new(Level::Info, "auth", "held"))],
    );
    assert_eq!(example.logs.len(), before);
    assert_eq!(example.held.len(), 1);
    send(&mut example, Message::Live(true));
    assert_eq!(example.logs.len(), before + 1);

    send(&mut example, Message::Live(false));
    let flood = (0..LOG_LIMIT + 50)
        .map(|_| Update::Log(LogLine::new(Level::Info, "auth", "flood")))
        .collect();
    feed(&mut example, flood);
    assert_eq!(example.held.len(), LOG_LIMIT);
}

#[test]
fn notification_settings_filter_worker_toasts() {
    let mut example = Example::default();
    let batch = || {
        vec![
            toast::toast("done").variant(ToastVariant::Success),
            toast::toast("broke").variant(ToastVariant::Destructive),
        ]
    };
    assert_eq!(example.allowed(batch()).len(), 2);
    example.settings.saved.failures_only = true;
    assert_eq!(example.allowed(batch()).len(), 1);
    example.settings.saved.toasts = false;
    assert!(example.allowed(batch()).is_empty());
}

#[test]
fn pausing_says_how_many_jobs_it_paused() {
    let mut example = Example::default();
    send(&mut example, Message::Check(JobId(1041), true));
    send(&mut example, Message::Check(JobId(1045), true));
    send(&mut example, Message::PauseSelected);
    assert!(titles(&example).contains(&"Paused 1 job".to_owned()));
    assert_eq!(status(&example, 1045), Some(Status::Queued));
}

#[test]
fn settings_keys_move_the_cursor_and_change_the_control() {
    let mut example = Example::default();
    example.go(Page::Settings);
    named(&mut example, Named::ArrowDown);
    assert_eq!(example.settings.current, Control::Concurrency);
    named(&mut example, Named::ArrowRight);
    assert_eq!(example.settings.concurrency, 4);
    named(&mut example, Named::End);
    assert_eq!(example.settings.concurrency, 6);
    named(&mut example, Named::ArrowUp);
    named(&mut example, Named::Space);
    assert!(!example.settings.scheduler);
}

#[test]
fn arrows_switch_pages_where_nothing_else_uses_them() {
    let mut example = Example::default();
    named(&mut example, Named::ArrowRight);
    assert_eq!(example.page.selected(), Some(Page::Jobs));
}

#[test]
fn the_row_menu_changes_the_queue() {
    let mut example = Example::default();
    let id = JobId(1045);
    send(
        &mut example,
        Message::RowMenu(context_menu::Event::Open(id, iced::Point::ORIGIN)),
    );
    assert!(example.row_menu.is_open_on(&id));
    assert_eq!(example.cursor, Some(id));
    send(
        &mut example,
        Message::RowMenu(context_menu::Event::Menu(dropdown_menu::Event::Activate(
            RowAction::High,
        ))),
    );
    assert_eq!(example.job(id).map(|job| job.queue), Some(Queue::High));
    assert!(!example.row_menu.is_open());
}

#[test]
fn on_a_narrow_window_the_sidebar_opens_as_a_dialog() {
    let mut example = Example::default();
    send(
        &mut example,
        Message::Resized(iced::Size::new(390.0, 800.0)),
    );
    example.run(super::Cmd::ToggleSidebar);
    assert!(example.drawer);
    assert!(example.sidebar, "the wide layout keeps its sidebar");

    send(
        &mut example,
        Message::Resized(iced::Size::new(1400.0, 800.0)),
    );
    assert!(!example.drawer);
    example.run(super::Cmd::ToggleSidebar);
    assert!(!example.sidebar);
}
