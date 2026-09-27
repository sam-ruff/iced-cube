//! Updates from the simulated platform, delivered through a channel.
//!
//! [`subscription`] owns the receiver and hands out the sender first. A
//! [`Worker`] holds clones of that sender and the toast sender, like a
//! background thread would, and waits with futures-timer so it also runs in
//! the browser.

use std::hash::{Hash, Hasher};
use std::time::Duration;

use futures_timer::Delay;
use iced::Subscription;
use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, Stream, StreamExt, stream};
use iced::time::Instant;
use iced_cube::overlay::toast::{self, Variant, toast};

use super::data::{CHATTER, JobId, Level, LogLine, SERVICES};

const CAPACITY: usize = 64;
const BATCH: usize = 32;
const TICK: Duration = Duration::from_millis(450);

pub type Sender = mpsc::Sender<Update>;

#[derive(Debug, Clone)]
pub enum Update {
    Progress(JobId, f32),
    Succeeded(JobId),
    Failed(JobId, f32),
    Log(LogLine),
    /// Someone submitted a job built from one of the templates.
    Submitted(usize),
}

#[derive(Debug, Clone)]
pub enum Event {
    Ready(Sender),
    Received(Vec<Update>),
}

pub fn subscription() -> Subscription<Event> {
    Subscription::run(stream)
}

fn stream() -> impl Stream<Item = Event> {
    let (sender, receiver) = mpsc::channel(CAPACITY);
    stream::once(async move { Event::Ready(sender) })
        .chain(receiver.ready_chunks(BATCH).map(Event::Received))
}

/// How often someone submits a new job. It has its own timer, so it keeps
/// going however often the worker restarts.
pub const SUBMIT_EVERY: Duration = Duration::from_secs(18);

/// Instants at `period`, for animating spinners and timing submissions.
pub fn ticks(period: Duration) -> Subscription<Instant> {
    Subscription::run_with(period, |period| {
        stream::unfold(*period, |period| async move {
            Delay::new(period).await;
            Some((Instant::now(), period))
        })
    })
}

/// A job the worker is running, with its progress when the worker started.
#[derive(Debug, Clone, PartialEq)]
pub struct Running {
    pub id: JobId,
    pub name: String,
    pub progress: f32,
    pub speed: f32,
    pub fails_at: Option<f32>,
}

/// Uninhabited: the worker runs until its subscription is dropped.
#[derive(Debug, Clone, Copy)]
pub enum Never {}

/// Simulated work. A new set of running jobs replaces the subscription,
/// which stops the old worker.
#[derive(Debug, Clone)]
pub struct Worker {
    pub jobs: Vec<Running>,
    pub feed: Sender,
    pub toasts: toast::Sender,
}

impl Hash for Worker {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for job in &self.jobs {
            job.id.hash(state);
        }
    }
}

impl Worker {
    pub fn run(&self) -> impl Stream<Item = Never> + use<> {
        stream::once(self.clone().work())
    }

    async fn work(mut self) -> Never {
        let seed = self.jobs.iter().fold(0x9E37_79B9_7F4A_7C15, |seed, job| {
            seed ^ u64::from(job.id.0).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        });
        let mut rng = Rng(seed | 1);

        loop {
            Delay::new(TICK).await;

            if !self.jobs.is_empty() {
                let index = rng.below(self.jobs.len());
                self.advance(index, &mut rng).await;
            }
            if rng.chance(0.55) {
                let (level, service, message) = CHATTER[rng.below(CHATTER.len())];
                self.send(Update::Log(LogLine::new(level, service, message)))
                    .await;
            }
        }
    }

    async fn advance(&mut self, index: usize, rng: &mut Rng) {
        let Some(job) = self.jobs.get_mut(index) else {
            return;
        };
        job.progress = (job.progress + job.speed * (0.5 + rng.unit())).min(1.0);
        let job = job.clone();

        // The app decides whether a failure is retried or reported, so only
        // successes toast from here.
        if job.fails_at.is_some_and(|at| job.progress >= at) {
            self.jobs.remove(index);
            self.send(Update::Failed(job.id, job.progress)).await;
            self.send(Update::Log(LogLine::new(
                Level::Error,
                SERVICES[0],
                format!("{} ({}) exited with status 137", job.name, job.id),
            )))
            .await;
        } else if job.progress >= 1.0 {
            self.jobs.remove(index);
            self.send(Update::Succeeded(job.id)).await;
            self.send(Update::Log(LogLine::new(
                Level::Info,
                SERVICES[0],
                format!("{} ({}) finished", job.name, job.id),
            )))
            .await;
            let _ = self
                .toasts
                .send(
                    toast(format!("{} finished", job.name))
                        .description(format!("{} completed without errors.", job.id))
                        .variant(Variant::Success),
                )
                .await;
        } else {
            self.send(Update::Progress(job.id, job.progress)).await;
        }
    }

    async fn send(&mut self, update: Update) {
        // A closed channel means the app has gone; there is nobody to tell.
        let _ = self.feed.send(update).await;
    }
}

/// A small xorshift generator, so the demo needs no extra crates.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn unit(&mut self) -> f32 {
        (self.next() % 10_000) as f32 / 10_000.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound.max(1) as u64) as usize
    }

    fn chance(&mut self, probability: f32) -> bool {
        self.unit() < probability
    }
}
