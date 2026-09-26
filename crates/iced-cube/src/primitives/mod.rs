//! Basic controls: buttons, icon buttons, checkboxes, inputs, radios,
//! scroll areas, separators, sliders and switches.

#[cfg(feature = "button")]
pub mod button;
#[cfg(feature = "checkbox")]
pub mod checkbox;
#[cfg(feature = "icon-button")]
pub mod icon_button;
#[cfg(feature = "input")]
pub mod input;
#[cfg(feature = "radio")]
pub mod radio;
#[cfg(feature = "scroll-area")]
pub mod scroll_area;
#[cfg(feature = "separator")]
pub mod separator;
#[cfg(feature = "slider")]
pub mod slider;
#[cfg(feature = "switch")]
pub mod switch;

#[cfg(feature = "button")]
pub use button::{Button, button};
#[cfg(feature = "checkbox")]
pub use checkbox::{CheckState, Checkbox, checkbox};
#[cfg(feature = "icon-button")]
pub use icon_button::{IconButton, icon_button};
#[cfg(feature = "input")]
pub use input::{Input, input};
#[cfg(feature = "radio")]
pub use radio::{Radio, RadioGroup, radio, radio_group};
#[cfg(feature = "scroll-area")]
pub use scroll_area::{ScrollArea, scroll_area};
#[cfg(feature = "separator")]
pub use separator::{Separator, separator, vertical_separator};
#[cfg(feature = "slider")]
pub use slider::{Slider, slider};
#[cfg(feature = "switch")]
pub use switch::{Switch, switch};
