//! Status and identity: alerts, avatars, badges, chips, progress bars and
//! spinners.

#[cfg(feature = "alert")]
pub mod alert;
#[cfg(feature = "avatar")]
pub mod avatar;
#[cfg(feature = "badge")]
pub mod badge;
#[cfg(feature = "chip")]
pub mod chip;
#[cfg(feature = "progress")]
pub mod progress;
#[cfg(feature = "spinner")]
pub mod spinner;

#[cfg(feature = "alert")]
pub use alert::{Alert, alert};
#[cfg(feature = "avatar")]
pub use avatar::{Avatar, AvatarGroup, avatar, avatar_group};
#[cfg(feature = "badge")]
pub use badge::{Badge, badge};
#[cfg(feature = "chip")]
pub use chip::{Chip, chip};
#[cfg(feature = "progress")]
pub use progress::{Progress, progress};
#[cfg(feature = "spinner")]
pub use spinner::{Spinner, spinner};
