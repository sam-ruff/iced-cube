use iced::keyboard::{Key, Modifiers, key::Named};
use iced_cube::keys;
use iced_cube::overlay::toast::{self, Variant as ToastVariant};

use super::data::{JobId, Queue, Status};
use super::feed::{self, Update};
use super::settings::Control;
use super::{Example, Message, Page, Pending};

fn press(example: &mut Example, key: Key, modifiers: Modifiers) {
    example.update(Message::Key(keys::Event { key, modifiers }));
}

fn named(example: &mut Example, key: Named) {
    press(example, Key::Named(key), Modifiers::empty());
}

fn ctrl_k(example: &mut Example) {
    press(example, Key::Character("k".into()), Modifiers::COMMAND);
}

fn status(example: &Example, id: u32) -> Option<Status> {
    example.job(JobId(id)).map(|job| job.status)
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
    example.update(Message::OpenNewJob);
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
fn deleting_can_be_undone_from_the_toast() {
    let mut example = Example {
        pending: Some(Pending::Delete(JobId(1049))),
        ..Example::default()
    };
    example.update(Message::Confirm);
    assert_eq!(status(&example, 1049), None);

    let Some((id, _)) = example.undo.clone() else {
        panic!("no undo toast");
    };
    example.update(Message::Toast(toast::Event::Action(id)));
    assert_eq!(status(&example, 1049), Some(Status::Succeeded));
}

#[test]
fn the_scheduler_fills_free_slots_high_priority_first() {
    let mut example = Example::default();
    example.update(Message::Feed(feed::Event::Received(vec![
        Update::Succeeded(JobId(1043)),
    ])));
    assert_eq!(example.count(Status::Running), 3);
    // Both queued jobs are Normal or Low; the Normal one starts first.
    assert_eq!(status(&example, 1046), Some(Status::Running));
    assert_eq!(status(&example, 1045), Some(Status::Queued));
}

#[test]
fn a_paused_scheduler_starts_nothing() {
    let mut example = Example::default();
    example.settings.scheduler = false;
    example.update(Message::Feed(feed::Event::Received(vec![Update::Failed(
        JobId(1041),
        0.5,
    )])));
    assert_eq!(status(&example, 1041), Some(Status::Failed));
    assert_eq!(example.count(Status::Running), 2);
}

#[test]
fn submissions_stop_while_the_queue_is_long() {
    let mut example = Example::default();
    example.settings.scheduler = false;
    let submit = |example: &mut Example| {
        example.update(Message::Feed(feed::Event::Received(vec![
            Update::Submitted(0),
        ])));
    };
    for _ in 0..6 {
        submit(&mut example);
    }
    assert_eq!(example.count(Status::Queued), super::QUEUE_LIMIT);
}

#[test]
fn paused_logs_are_held_until_live_again() {
    let mut example = Example::default();
    let before = example.logs.len();
    example.update(Message::Live(false));
    example.update(Message::Feed(feed::Event::Received(vec![Update::Log(
        super::data::LogLine::new(super::data::Level::Info, "auth", "held"),
    )])));
    assert_eq!(example.logs.len(), before);
    assert_eq!(example.held.len(), 1);
    example.update(Message::Live(true));
    assert_eq!(example.logs.len(), before + 1);
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
    example.settings.failures_only = true;
    assert_eq!(example.allowed(batch()).len(), 1);
    example.settings.toasts = false;
    assert!(example.allowed(batch()).is_empty());
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
    example.update(Message::RowMenu(iced_cube::context_menu::Event::Open(
        id,
        iced::Point::ORIGIN,
    )));
    assert!(example.row_menu.is_open_on(&id));
    assert_eq!(example.cursor, Some(id));
    example.update(Message::RowMenu(iced_cube::context_menu::Event::Menu(
        iced_cube::dropdown_menu::Event::Activate(super::RowAction::High),
    )));
    assert_eq!(example.job(id).map(|job| job.queue), Some(Queue::High));
    assert!(!example.row_menu.is_open());
}
