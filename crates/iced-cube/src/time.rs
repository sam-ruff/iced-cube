//! The one place components read the clock, so a test harness can move it.
//!
//! With the `icedwright` feature these read icedwright's clock, which is the
//! real one unless a harness installs its virtual clock. Otherwise they read
//! the real clock.

use iced::time::Instant;

/// The current instant.
#[cfg(not(all(feature = "icedwright", not(target_arch = "wasm32"))))]
pub(crate) fn now() -> Instant {
    Instant::now()
}

/// The current instant.
#[cfg(all(feature = "icedwright", not(target_arch = "wasm32")))]
pub(crate) fn now() -> Instant {
    icedwright_probe::time::now()
}

/// The current time, for calendars.
#[cfg(feature = "calendar")]
pub(crate) fn timestamp() -> jiff::Timestamp {
    #[cfg(all(feature = "icedwright", not(target_arch = "wasm32")))]
    if let Ok(now) = jiff::Timestamp::try_from(icedwright_probe::time::system_now()) {
        return now;
    }
    jiff::Timestamp::now()
}
