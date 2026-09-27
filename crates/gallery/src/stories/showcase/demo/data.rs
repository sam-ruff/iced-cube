//! Fake data for the demo: a data platform run by the fictional Fernhill Analytics.

use std::fmt;

use iced_cube::feedback::badge::Variant as BadgeVariant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(pub u32);

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JOB-{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Waiting for its schedule, such as a nightly run.
    Scheduled,
    Queued,
    Running,
    Paused,
    Succeeded,
    Failed,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Scheduled => "Scheduled",
            Status::Queued => "Queued",
            Status::Running => "Running",
            Status::Paused => "Paused",
            Status::Succeeded => "Succeeded",
            Status::Failed => "Failed",
        }
    }

    pub fn badge(self) -> BadgeVariant {
        match self {
            Status::Scheduled => BadgeVariant::Outline,
            Status::Queued => BadgeVariant::Secondary,
            Status::Running => BadgeVariant::Default,
            Status::Paused => BadgeVariant::Warning,
            Status::Succeeded => BadgeVariant::Success,
            Status::Failed => BadgeVariant::Destructive,
        }
    }

    pub fn is_finished(self) -> bool {
        matches!(self, Status::Succeeded | Status::Failed)
    }
}

/// The filter above the job list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    All,
    Only(Status),
}

impl StatusFilter {
    pub const ALL: [StatusFilter; 7] = [
        StatusFilter::All,
        StatusFilter::Only(Status::Running),
        StatusFilter::Only(Status::Queued),
        StatusFilter::Only(Status::Scheduled),
        StatusFilter::Only(Status::Paused),
        StatusFilter::Only(Status::Failed),
        StatusFilter::Only(Status::Succeeded),
    ];

    pub fn allows(self, status: Status) -> bool {
        match self {
            StatusFilter::All => true,
            StatusFilter::Only(only) => only == status,
        }
    }
}

impl fmt::Display for StatusFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StatusFilter::All => f.write_str("All statuses"),
            StatusFilter::Only(status) => f.write_str(status.label()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Queue {
    High,
    Normal,
    Low,
}

impl Queue {
    pub const ALL: [Queue; 3] = [Queue::High, Queue::Normal, Queue::Low];
}

impl fmt::Display for Queue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Queue::High => "High",
            Queue::Normal => "Normal",
            Queue::Low => "Low",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schedule {
    Once,
    Hourly,
    Nightly,
    Weekly,
}

impl Schedule {
    pub const ALL: [Schedule; 4] = [
        Schedule::Once,
        Schedule::Hourly,
        Schedule::Nightly,
        Schedule::Weekly,
    ];
}

impl fmt::Display for Schedule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Schedule::Once => "Run once",
            Schedule::Hourly => "Every hour",
            Schedule::Nightly => "Nightly at 02:00",
            Schedule::Weekly => "Mondays at 06:00",
        })
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: JobId,
    pub name: String,
    pub owner: &'static str,
    pub source: String,
    pub queue: Queue,
    pub status: Status,
    pub progress: f32,
    /// Progress at which the simulated run fails, if it does.
    pub fails_at: Option<f32>,
    /// Share of the work done per tick of the simulated worker.
    pub speed: f32,
    pub started: String,
    pub checked: bool,
    pub schedule: Schedule,
    pub notes: String,
    /// Automatic retries used since the job last started from the beginning.
    pub attempts: u8,
}

impl Job {
    pub fn new(
        id: JobId,
        name: impl Into<String>,
        owner: &'static str,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            owner,
            source: source.into(),
            queue: Queue::Normal,
            status: Status::Queued,
            progress: 0.0,
            fails_at: (id.0 % 5 == 2).then_some(0.5 + (id.0 % 4) as f32 * 0.1),
            speed: 0.015 + (id.0 % 3) as f32 * 0.01,
            started: String::from("-"),
            checked: false,
            schedule: Schedule::Once,
            notes: String::new(),
            attempts: 0,
        }
    }

    fn with(mut self, queue: Queue, status: Status, progress: f32, started: &str) -> Self {
        self.queue = queue;
        self.status = status;
        self.progress = progress;
        self.started = started.to_owned();
        self
    }

    /// What the last column of the job list says.
    pub fn detail(&self) -> String {
        match self.status {
            Status::Scheduled => format!("Next run: {}", self.schedule),
            Status::Queued => String::from("Waiting for a slot"),
            Status::Running | Status::Paused => format!("{}%", (self.progress * 100.0).round()),
            Status::Succeeded => format!("Finished, started {}", self.started),
            Status::Failed => format!("Failed at {}%", (self.progress * 100.0).round()),
        }
    }
}

pub const ME: &str = "Priya Shah";

pub const SOURCES: &[&str] = &[
    "billing.invoices",
    "crm.accounts",
    "events.clickstream",
    "orders.parquet",
    "s3://fernhill-raw/sensors",
    "warehouse.sessions",
    "warehouse.stock_levels",
    "postgres://ledger",
];

/// Jobs the simulated scheduler submits: name, owner and source.
pub const TEMPLATES: &[(&str, &str, &str)] = &[
    ("search-reindex", "Mei Lin", "crm.accounts"),
    ("sensor-rollup", "Jonas Berg", "s3://fernhill-raw/sensors"),
    ("churn-scores", "Aisha Karim", "warehouse.sessions"),
    ("stock-forecast", "Leo Martins", "warehouse.stock_levels"),
    ("invoice-sync", ME, "billing.invoices"),
    ("clickstream-sessionise", "Tom Okafor", "events.clickstream"),
];

pub fn jobs() -> Vec<Job> {
    use Queue::{High, Low, Normal};
    use Status::{Failed, Paused, Queued, Running, Succeeded};

    vec![
        Job::new(JobId(1041), "nightly-orders-etl", ME, "orders.parquet")
            .with(High, Running, 0.62, "09:12"),
        Job::new(
            JobId(1042),
            "ledger-reconcile",
            "Tom Okafor",
            "postgres://ledger",
        )
        .with(High, Running, 0.28, "09:20"),
        Job::new(
            JobId(1043),
            "ml-feature-refresh",
            "Mei Lin",
            "warehouse.sessions",
        )
        .with(Normal, Running, 0.81, "09:03"),
        Job::new(
            JobId(1044),
            "gdpr-erasure-sweep",
            "Aisha Karim",
            "crm.accounts",
        )
        .with(Normal, Paused, 0.45, "08:40"),
        Job::new(
            JobId(1045),
            "image-thumbnails",
            "Leo Martins",
            "s3://fernhill-raw/sensors",
        )
        .with(Low, Queued, 0.0, "-"),
        Job::new(JobId(1046), "billing-rollup", ME, "billing.invoices")
            .with(Normal, Queued, 0.0, "-"),
        Job::new(
            JobId(1047),
            "warehouse-backfill",
            "Jonas Berg",
            "warehouse.stock_levels",
        )
        .with(Low, Failed, 0.37, "07:55"),
        Job::new(
            JobId(1048),
            "events-compaction",
            "Tom Okafor",
            "events.clickstream",
        )
        .with(Normal, Succeeded, 1.0, "06:30"),
        Job::new(JobId(1049), "crm-dedupe", "Mei Lin", "crm.accounts")
            .with(Low, Succeeded, 1.0, "05:10"),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Level::Info => "Info",
            Level::Warning => "Warn",
            Level::Error => "Error",
        }
    }

    pub fn badge(self) -> BadgeVariant {
        match self {
            Level::Info => BadgeVariant::Outline,
            Level::Warning => BadgeVariant::Warning,
            Level::Error => BadgeVariant::Destructive,
        }
    }
}

/// Which log lines the activity page shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelFilter {
    All,
    Warnings,
    Errors,
}

impl LevelFilter {
    pub const ALL: [LevelFilter; 3] =
        [LevelFilter::All, LevelFilter::Warnings, LevelFilter::Errors];

    pub fn allows(self, level: Level) -> bool {
        match self {
            LevelFilter::All => true,
            LevelFilter::Warnings => level != Level::Info,
            LevelFilter::Errors => level == Level::Error,
        }
    }
}

impl fmt::Display for LevelFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LevelFilter::All => "All",
            LevelFilter::Warnings => "Warnings",
            LevelFilter::Errors => "Errors",
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogLine {
    pub level: Level,
    pub service: &'static str,
    pub message: String,
}

impl LogLine {
    pub fn new(level: Level, service: &'static str, message: impl Into<String>) -> Self {
        Self {
            level,
            service,
            message: message.into(),
        }
    }
}

pub const SERVICES: &[&str] = &[
    "scheduler",
    "ingest-api",
    "warehouse",
    "object-store",
    "auth",
    "notifier",
];

/// Background chatter from the platform: level, service and message.
pub const CHATTER: &[(Level, &str, &str)] = &[
    (Level::Info, "ingest-api", "Accepted batch of 2,048 events"),
    (Level::Info, "warehouse", "Vacuumed table sessions in 1.8s"),
    (
        Level::Info,
        "auth",
        "Rotated signing key for service accounts",
    ),
    (
        Level::Info,
        "object-store",
        "Uploaded 312 objects to fernhill-raw",
    ),
    (Level::Info, "scheduler", "Heartbeat from 4 workers"),
    (
        Level::Warning,
        "warehouse",
        "Query on stock_levels took 12.4s",
    ),
    (Level::Info, "notifier", "Sent daily digest to 38 members"),
    (
        Level::Warning,
        "ingest-api",
        "Client retried 3 times after a timeout",
    ),
    (Level::Info, "ingest-api", "Accepted batch of 1,024 events"),
    (
        Level::Error,
        "object-store",
        "Checksum mismatch on sensors/2026-09-26.csv",
    ),
    (
        Level::Info,
        "warehouse",
        "Refreshed materialised view daily_revenue",
    ),
    (
        Level::Warning,
        "auth",
        "Token for tom.okafor expires in 2 days",
    ),
];

pub fn logs() -> Vec<(u32, LogLine)> {
    let start = 9 * 3600 + 38 * 60;
    CHATTER
        .iter()
        .take(8)
        .enumerate()
        .map(|(index, (level, service, message))| {
            (
                start + index as u32 * 17,
                LogLine::new(*level, service, *message),
            )
        })
        .rev()
        .collect()
}

/// Formats seconds since midnight as a clock time.
pub fn clock(seconds: u32) -> String {
    let seconds = seconds % 86_400;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    London,
    Frankfurt,
    Dublin,
    Virginia,
}

impl Region {
    pub const ALL: [Region; 4] = [
        Region::London,
        Region::Frankfurt,
        Region::Dublin,
        Region::Virginia,
    ];

    /// Fake load for each region over a time range, as a share of capacity.
    pub fn load(self, range: Range) -> f32 {
        let base: f32 = match self {
            Region::London => 0.72,
            Region::Frankfurt => 0.48,
            Region::Dublin => 0.91,
            Region::Virginia => 0.33,
        };
        let shift = match range {
            Range::Day => 0.0,
            Range::Week => -0.08,
            Range::Month => -0.15,
        };
        (base + shift).clamp(0.0, 1.0)
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Region::London => "London",
            Region::Frankfurt => "Frankfurt",
            Region::Dublin => "Dublin",
            Region::Virginia => "Virginia",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Production,
    Staging,
    Development,
}

impl Environment {
    pub fn label(self) -> &'static str {
        match self {
            Environment::Production => "Production",
            Environment::Staging => "Staging",
            Environment::Development => "Development",
        }
    }

    /// A label short enough for the header on a phone.
    pub fn short(self) -> &'static str {
        match self {
            Environment::Production => "Prod",
            Environment::Staging => "Staging",
            Environment::Development => "Dev",
        }
    }

    pub fn badge(self) -> BadgeVariant {
        match self {
            Environment::Production => BadgeVariant::Destructive,
            Environment::Staging => BadgeVariant::Warning,
            Environment::Development => BadgeVariant::Secondary,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runbook {
    Degraded,
    Retry,
    Escalation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Email,
    Chat,
    Pager,
}

impl Channel {
    pub const ALL: [Channel; 3] = [Channel::Email, Channel::Chat, Channel::Pager];
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Channel::Email => "Email",
            Channel::Chat => "Team chat",
            Channel::Pager => "Pager",
        })
    }
}
