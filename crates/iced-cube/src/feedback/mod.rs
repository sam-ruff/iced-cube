//! Status and progress: alerts, badges, progress bars and spinners.

#[cfg(feature = "alert")]
pub mod alert;
#[cfg(feature = "badge")]
pub mod badge;
#[cfg(feature = "progress")]
pub mod progress;
#[cfg(feature = "spinner")]
pub mod spinner;

#[cfg(feature = "alert")]
pub use alert::{Alert, alert};
#[cfg(feature = "badge")]
pub use badge::{Badge, badge};
#[cfg(feature = "progress")]
pub use progress::{Progress, progress};
#[cfg(feature = "spinner")]
pub use spinner::{Spinner, spinner};
