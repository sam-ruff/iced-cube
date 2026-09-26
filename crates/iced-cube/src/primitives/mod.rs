//! Basic controls: buttons, checkboxes, inputs, radios, scroll areas,
//! separators, sliders and switches.

pub mod button;
pub mod checkbox;
pub mod input;
pub mod radio;
pub mod scroll_area;
pub mod separator;
pub mod slider;
pub mod switch;

pub use button::{Button, button, icon_button};
pub use checkbox::{CheckState, Checkbox, checkbox};
pub use input::{Input, input};
pub use radio::{Radio, RadioGroup, radio, radio_group};
pub use scroll_area::{ScrollArea, scroll_area};
pub use separator::{Separator, separator, vertical_separator};
pub use slider::{Slider, slider};
pub use switch::{Switch, switch};
