//! A repeating timer that works with or without an async runtime.

use iced::Subscription;
use iced::time::{Duration, Instant};

/// Emits the instant of each tick, every `period`.
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub(crate) fn every(period: Duration) -> Subscription<Instant> {
    iced::time::every(period)
}

/// The browser timer's instants are another type, so each tick is stamped
/// as it arrives.
#[cfg(target_arch = "wasm32")]
pub(crate) fn every(period: Duration) -> Subscription<Instant> {
    iced::time::every(period).map(|_| Instant::now())
}

/// A sleeping thread feeds a channel, so no async runtime timer is needed.
#[cfg(not(any(target_arch = "wasm32", feature = "tokio")))]
pub(crate) fn every(period: Duration) -> Subscription<Instant> {
    use iced::futures::channel::mpsc;

    Subscription::run_with(period, |period| {
        #[cfg(feature = "icedwright")]
        icedwright_probe::time::real_timer("iced_cube::ticker");
        let period = *period;
        let (mut sender, receiver) = mpsc::channel(1);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(period);
                if sender
                    .try_send(Instant::now())
                    .is_err_and(|error| error.is_disconnected())
                {
                    break;
                }
            }
        });
        receiver
    })
}
