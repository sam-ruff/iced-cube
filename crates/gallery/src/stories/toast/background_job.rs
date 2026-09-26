use std::hash::{Hash, Hasher};
use std::time::Duration;

use futures_timer::Delay;
use iced::futures::{SinkExt, Stream, stream};
use iced::widget::{column, text};
use iced::{Element, Subscription};
use iced_cube::button;
use iced_cube::overlay::toast::{self, Variant, toast, toasts};

#[derive(Debug, Clone)]
pub enum Message {
    Toast(toast::Event),
    StartJob,
    JobFinished,
}

#[derive(Debug)]
pub struct Example {
    toasts: toast::State,
    sender: Option<toast::Sender>,
    job: Option<Job>,
    runs: u64,
}

impl Default for Example {
    fn default() -> Self {
        let mut toasts = toast::State::new();
        toasts.push(
            toast("Build finished")
                .description("All 214 tests passed.")
                .variant(Variant::Success),
        );
        toasts.push(
            toast("Upload failed")
                .description("The server did not respond.")
                .variant(Variant::Destructive),
        );

        Self {
            toasts,
            sender: None,
            job: None,
            runs: 0,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Toast(event) => {
                if let Some(toast::Output::Ready(sender)) = self.toasts.update(event) {
                    self.sender = Some(sender);
                }
            }
            Message::StartJob => {
                let Some(sender) = self.sender.clone() else {
                    return;
                };
                self.runs += 1;
                self.job = Some(Job {
                    run: self.runs,
                    sender,
                });
            }
            Message::JobFinished => self.job = None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status = if self.job.is_some() {
            "The job is sending progress through the channel."
        } else {
            "Producers send toasts through a channel."
        };
        let start = self.job.is_none().then_some(Message::StartJob);

        let content = column![
            text(status).size(14),
            button("Start background job").on_press_maybe(start),
        ]
        .spacing(12);

        toasts(&self.toasts, content)
            .on_event(Message::Toast)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let job = match &self.job {
            Some(job) => Subscription::run_with(job.clone(), Job::run),
            None => Subscription::none(),
        };

        Subscription::batch([
            toast::subscription().map(Message::Toast),
            toast::timer(&self.toasts).map(Message::Toast),
            job,
        ])
    }
}

/// Simulated work that only holds a sender, like a worker thread would.
#[derive(Debug, Clone)]
struct Job {
    run: u64,
    sender: toast::Sender,
}

impl Hash for Job {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.run.hash(state);
    }
}

impl Job {
    fn run(&self) -> impl Stream<Item = Message> + use<> {
        let mut sender = self.sender.clone();

        stream::once(async move {
            for step in 1..=3 {
                Delay::new(Duration::from_millis(900)).await;
                let _ = sender
                    .send(toast(format!("Step {step} of 3 complete")))
                    .await;
            }
            let _ = sender
                .send(toast("Job finished").variant(Variant::Success))
                .await;
            Message::JobFinished
        })
    }
}
