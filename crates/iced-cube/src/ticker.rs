//! A repeating timer that works with or without an async runtime.

use iced::Subscription;
use iced::time::Duration;

/// Emits `()` every `period`.
#[cfg(any(target_arch = "wasm32", feature = "tokio"))]
pub(crate) fn every(period: Duration) -> Subscription<()> {
    iced::time::every(period).map(|_| ())
}

/// A sleeping thread feeds a channel, so no async runtime timer is needed.
#[cfg(not(any(target_arch = "wasm32", feature = "tokio")))]
pub(crate) fn every(period: Duration) -> Subscription<()> {
    use iced::futures::channel::mpsc;

    Subscription::run_with(period, |period| {
        let period = *period;
        let (mut sender, receiver) = mpsc::channel(1);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(period);
                if sender
                    .try_send(())
                    .is_err_and(|error| error.is_disconnected())
                {
                    break;
                }
            }
        });
        receiver
    })
}
