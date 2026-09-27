//! Workspace settings: a form with fields, switches, sliders, a select,
//! a radio group and a checkbox, plus a keyboard cursor over the controls.

use iced::keyboard::key::Named;
use iced::widget::text_editor::{Action as EditorAction, Content};
use iced::widget::{column, container, row, space, text};
use iced::{Alignment, Border, Element, Length};
use iced_cube::feedback::alert::Variant as AlertVariant;
use iced_cube::feedback::badge::Variant as BadgeVariant;
use iced_cube::forms::select;
use iced_cube::keys::{self, Chord, Keymap};
use iced_cube::primitives::button::Variant;
use iced_cube::primitives::checkbox::CheckState;
use iced_cube::primitives::radio::Direction;
use iced_cube::primitives::{slider, switch};
use iced_cube::theme::{Tokens, radius, semibold, text_size};
use iced_cube::{
    alert, badge, button, card, checkbox, field, input, lucide, radio_group, separator, textarea,
    vstack,
};

use super::data::{Channel, Region};
use super::muted;

/// The controls the keyboard cursor moves between, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Scheduler,
    Concurrency,
    Retries,
    Region,
    Toasts,
    FailuresOnly,
    Retention,
}

impl Control {
    const ALL: [Control; 7] = [
        Control::Scheduler,
        Control::Concurrency,
        Control::Retries,
        Control::Region,
        Control::Toasts,
        Control::FailuresOnly,
        Control::Retention,
    ];

    fn step(self, forward: bool) -> Self {
        let len = Self::ALL.len();
        let index = Self::ALL.iter().position(|c| *c == self).unwrap_or(0);
        let next = if forward {
            (index + 1) % len
        } else {
            (index + len - 1) % len
        };
        Self::ALL[next]
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Workspace(String),
    Banner(EditorAction),
    Scheduler(bool),
    Concurrency(u8),
    Retries(u8),
    Region(Region),
    Channel(Channel),
    Toasts(bool),
    FailuresOnly(bool),
    Retention(u8),
    Focus(Control),
    Save,
    Reset,
    /// Handled by the app, which asks for confirmation first.
    ClearFinished,
}

/// The shortcuts the settings page resolves, one keymap per component.
#[derive(Debug, Clone)]
pub struct Keys {
    pub slider: Keymap<slider::Action>,
    pub select: Keymap<select::Action>,
    pub switch: Keymap<switch::Action>,
    pub checkbox: Keymap<checkbox::Action>,
}

impl Default for Keys {
    fn default() -> Self {
        // Up and Down move between controls here, so the slider keeps only
        // the horizontal keys and the select moves with Left and Right.
        let slider = slider::default_keymap()
            .unbind(&Chord::named(Named::ArrowUp))
            .unbind(&Chord::named(Named::ArrowDown));
        let select = Keymap::new()
            .bind(Chord::named(Named::ArrowRight), select::Action::Next)
            .bind(Chord::named(Named::ArrowLeft), select::Action::Previous);
        Self {
            slider,
            select,
            switch: switch::default_keymap(),
            checkbox: checkbox::default_keymap(),
        }
    }
}

/// The settings the rest of the app runs on. The form edits a copy and
/// only hands it over on Save.
#[derive(Debug, Clone, PartialEq)]
pub struct Values {
    pub workspace: String,
    pub banner: String,
    pub scheduler: bool,
    pub concurrency: u8,
    pub retries: u8,
    pub region: Region,
    pub channel: Channel,
    pub toasts: bool,
    pub failures_only: bool,
    pub retention: u8,
}

impl Default for Values {
    fn default() -> Self {
        Self {
            workspace: String::from("Fernhill Analytics"),
            banner: String::from("Planned maintenance on Sunday from 02:00 to 04:00."),
            scheduler: true,
            concurrency: 3,
            retries: 2,
            region: Region::London,
            channel: Channel::Chat,
            toasts: true,
            failures_only: false,
            retention: 30,
        }
    }
}

/// The settings form: the fields being edited, and the values last saved.
#[derive(Debug)]
pub struct Settings {
    pub workspace: String,
    pub banner: Content,
    pub scheduler: bool,
    pub concurrency: u8,
    pub retries: u8,
    pub region: Region,
    pub channel: Channel,
    pub toasts: bool,
    pub failures_only: bool,
    pub retention: u8,
    pub current: Control,
    pub(super) saved: Values,
}

impl Default for Settings {
    fn default() -> Self {
        let mut settings = Self {
            workspace: String::new(),
            banner: Content::new(),
            scheduler: true,
            concurrency: 0,
            retries: 0,
            region: Region::London,
            channel: Channel::Chat,
            toasts: true,
            failures_only: false,
            retention: 0,
            current: Control::Scheduler,
            saved: Values::default(),
        };
        settings.restore();
        settings
    }
}

const CONCURRENCY: std::ops::RangeInclusive<u8> = 1..=6;
const RETRIES: std::ops::RangeInclusive<u8> = 0..=5;
const RETENTION: std::ops::RangeInclusive<u8> = 7..=90;

impl Settings {
    /// The values the app runs on: the last saved ones.
    pub fn applied(&self) -> &Values {
        &self.saved
    }

    /// The form as it stands, saved or not.
    pub fn draft(&self) -> Values {
        Values {
            workspace: self.workspace.clone(),
            banner: self.banner.text().trim_end().to_owned(),
            scheduler: self.scheduler,
            concurrency: self.concurrency,
            retries: self.retries,
            region: self.region,
            channel: self.channel,
            toasts: self.toasts,
            failures_only: self.failures_only,
            retention: self.retention,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.draft() != self.saved
    }

    /// Pauses or resumes the scheduler straight away, from outside the form.
    pub fn set_scheduler(&mut self, on: bool) {
        self.scheduler = on;
        self.saved.scheduler = on;
    }

    /// Puts the form back to the last saved values.
    fn restore(&mut self) {
        let saved = self.saved.clone();
        self.workspace = saved.workspace;
        self.banner = Content::with_text(&saved.banner);
        self.scheduler = saved.scheduler;
        self.concurrency = saved.concurrency;
        self.retries = saved.retries;
        self.region = saved.region;
        self.channel = saved.channel;
        self.toasts = saved.toasts;
        self.failures_only = saved.failures_only;
        self.retention = saved.retention;
    }

    /// Applies a change to the form and returns whether it was a save.
    pub fn update(&mut self, message: Message) -> bool {
        match message {
            Message::Workspace(name) => self.workspace = name,
            Message::Banner(action) => self.banner.perform(action),
            Message::Scheduler(on) => self.select(Control::Scheduler, |s| s.scheduler = on),
            Message::Concurrency(value) => {
                self.select(Control::Concurrency, |s| s.concurrency = value)
            }
            Message::Retries(value) => self.select(Control::Retries, |s| s.retries = value),
            Message::Region(region) => self.select(Control::Region, |s| s.region = region),
            Message::Channel(channel) => self.channel = channel,
            Message::Toasts(on) => self.select(Control::Toasts, |s| s.toasts = on),
            Message::FailuresOnly(on) => {
                self.select(Control::FailuresOnly, |s| s.failures_only = on)
            }
            Message::Retention(days) => self.select(Control::Retention, |s| s.retention = days),
            Message::Focus(control) => self.current = control,
            Message::ClearFinished => {}
            Message::Save => {
                if self.workspace.trim().is_empty() {
                    return false;
                }
                self.saved = self.draft();
                return true;
            }
            Message::Reset => self.restore(),
        }
        false
    }

    fn select(&mut self, control: Control, change: impl FnOnce(&mut Self)) {
        self.current = control;
        change(self);
    }

    /// Resolves a key press for the current control, returning the change
    /// it makes, if any. Up and Down move the cursor.
    pub fn key(&self, keys: &Keys, key: &keys::Event) -> Option<Message> {
        if key.modifiers.is_empty() {
            match key.key {
                iced::keyboard::Key::Named(Named::ArrowDown) => {
                    return Some(Message::Focus(self.current.step(true)));
                }
                iced::keyboard::Key::Named(Named::ArrowUp) => {
                    return Some(Message::Focus(self.current.step(false)));
                }
                _ => {}
            }
        }

        let slide = |value: u8, range: std::ops::RangeInclusive<u8>, step: u8| {
            keys.slider
                .resolve_event(key)
                .map(|action| action.apply(value, range, Some(step)))
        };
        match self.current {
            Control::Scheduler => keys
                .switch
                .resolve_event(key)
                .map(|action| Message::Scheduler(action.apply(self.scheduler))),
            Control::Toasts => keys
                .switch
                .resolve_event(key)
                .map(|action| Message::Toasts(action.apply(self.toasts))),
            Control::FailuresOnly => keys.checkbox.resolve_event(key).map(|action| {
                Message::FailuresOnly(action.apply(CheckState::from(self.failures_only)))
            }),
            Control::Concurrency => {
                slide(self.concurrency, CONCURRENCY, 1).map(Message::Concurrency)
            }
            Control::Retries => slide(self.retries, RETRIES, 1).map(Message::Retries),
            Control::Retention => slide(self.retention, RETENTION, 1).map(Message::Retention),
            Control::Region => keys
                .select
                .resolve_event(key)
                .and_then(|action| action.apply(&Region::ALL, Some(&self.region)))
                .map(Message::Region),
        }
    }

    pub fn view(&self, narrow: bool) -> Element<'_, Message> {
        let dirty = self.is_dirty();
        let status = if !dirty {
            badge("Saved")
                .variant(BadgeVariant::Success)
                .icon(lucide!(Check))
        } else {
            badge("Unsaved changes").variant(BadgeVariant::Warning)
        };

        let workspace = card()
            .title("Workspace")
            .description("Shown to everyone in the organisation.")
            .body(
                vstack([
                    field(
                        "Workspace name",
                        input("Fernhill Analytics", &self.workspace).on_input(Message::Workspace),
                    )
                    .required(true)
                    .error_maybe(
                        self.workspace
                            .trim()
                            .is_empty()
                            .then_some("Give the workspace a name."),
                    )
                    .into(),
                    field(
                        "Maintenance banner",
                        textarea(&self.banner)
                            .placeholder("Nothing planned")
                            .min_height(64.0)
                            .on_action(Message::Banner),
                    )
                    .description("Appears above every page until you clear it.")
                    .into(),
                ])
                .gap(iced_cube::layout::stack::Gap::Lg)
                .width(Length::Fill),
            )
            .width(Length::Fill);

        let scheduler = card()
            .title("Scheduler")
            .description("How jobs are picked up and retried.")
            .body(column![
                self.setting(
                    Control::Scheduler,
                    "Run scheduled jobs",
                    "Pausing leaves running jobs where they are.",
                    switch(self.scheduler).on_toggle(Message::Scheduler),
                    narrow,
                ),
                self.setting(
                    Control::Concurrency,
                    "Concurrent jobs",
                    "Jobs that may run at the same time.",
                    slider::slider(CONCURRENCY, self.concurrency)
                        .step(1)
                        .show_value()
                        .width(Length::Fill)
                        .on_change(Message::Concurrency),
                    narrow,
                ),
                self.setting(
                    Control::Retries,
                    "Retry attempts",
                    "Times a failed job starts again before the owner is alerted.",
                    slider::slider(RETRIES, self.retries)
                        .step(1)
                        .show_value()
                        .width(Length::Fill)
                        .on_change(Message::Retries),
                    narrow,
                ),
                self.setting(
                    Control::Region,
                    "Default region",
                    "Where new jobs run unless they say otherwise.",
                    iced_cube::select(&Region::ALL[..], Some(self.region))
                        .width(Length::Fill)
                        .on_select(Message::Region),
                    narrow,
                ),
            ])
            .width(Length::Fill);

        let notifications = card()
            .title("Notifications")
            .description("Where job alerts go.")
            .body(column![
                column![
                    text("Alert channel").size(text_size::SM).font(semibold()),
                    radio_group(Channel::ALL, Some(self.channel))
                        .direction(if narrow {
                            Direction::Vertical
                        } else {
                            Direction::Horizontal
                        })
                        .on_select(Message::Channel),
                ]
                .spacing(10)
                .padding([8, 10]),
                self.setting(
                    Control::Toasts,
                    "Pop-up notifications",
                    "Show a toast when a job finishes or fails.",
                    switch(self.toasts).on_toggle(Message::Toasts),
                    narrow,
                ),
                self.setting(
                    Control::FailuresOnly,
                    "Failures only",
                    "Skip messages for jobs that succeed.",
                    checkbox(self.failures_only)
                        .label("Only failures")
                        .on_toggle(Message::FailuresOnly),
                    narrow,
                ),
                self.setting(
                    Control::Retention,
                    "Log retention",
                    "How long logs are kept, including those of deleted jobs.",
                    slider::slider(RETENTION, self.retention)
                        .step(1)
                        .show_value()
                        .format_value(|days| format!("{days} days"))
                        .width(Length::Fill)
                        .on_change(Message::Retention),
                    narrow,
                ),
            ])
            .width(Length::Fill);

        let footer = row![
            status,
            space::horizontal(),
            button("Reset")
                .variant(Variant::Ghost)
                .on_press_maybe(dirty.then_some(Message::Reset)),
            button("Save").icon(lucide!(Save)).on_press_maybe(
                (dirty && !self.workspace.trim().is_empty()).then_some(Message::Save),
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let danger = row![
            column![
                text("Clear finished jobs")
                    .size(text_size::SM)
                    .font(semibold()),
                muted("Removes every succeeded and failed job from the list."),
            ]
            .spacing(2)
            .width(Length::Fill),
            button("Clear")
                .icon(lucide!(Trash))
                .variant(Variant::Destructive)
                .on_press(Message::ClearFinished),
        ]
        .spacing(16)
        .align_y(Alignment::Center);

        column![
            alert("Settings apply to the whole workspace")
                .variant(AlertVariant::Info)
                .description(
                    "Changes apply when you save; Reset goes back to the last save. Up and Down move between controls; Space, Left and Right change them."
                )
                .width(Length::Fill),
            workspace,
            scheduler,
            notifications,
            footer,
            separator().label("Danger zone"),
            danger,
        ]
        .spacing(16)
        .max_width(760)
        .into()
    }

    /// One labelled row, outlined while the keyboard cursor is on it.
    fn setting<'a>(
        &self,
        control: Control,
        title: &'a str,
        description: &'a str,
        input: impl Into<Element<'a, Message>>,
        narrow: bool,
    ) -> Element<'a, Message> {
        let labels = column![
            text(title).size(text_size::SM).font(semibold()),
            muted(description),
        ]
        .spacing(2);

        let body: Element<'a, Message> = if narrow {
            column![labels, input.into()].spacing(10).into()
        } else {
            row![
                labels.width(Length::FillPortion(3)),
                container(input).width(Length::FillPortion(2)),
            ]
            .spacing(16)
            .align_y(Alignment::Center)
            .into()
        };

        let current = self.current == control;
        iced::widget::mouse_area(container(body).padding([8, 10]).width(Length::Fill).style(
            move |theme| {
                let tokens = Tokens::of(theme);
                container::Style {
                    border: Border {
                        color: if current {
                            tokens.ring
                        } else {
                            iced::Color::TRANSPARENT
                        },
                        width: 1.0,
                        radius: radius::MD.into(),
                    },
                    ..container::Style::default()
                }
            },
        ))
        .on_press(Message::Focus(control))
        .into()
    }
}
