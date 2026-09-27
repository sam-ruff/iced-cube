//! Application chrome: the menubar, toolbar, status bar and command palette.

#[cfg(feature = "command-palette")]
pub mod command_palette;
#[cfg(feature = "menubar")]
pub mod menubar;
#[cfg(feature = "status-bar")]
pub mod status_bar;
#[cfg(feature = "toolbar")]
pub mod toolbar;

#[cfg(feature = "command-palette")]
pub use command_palette::{CommandPalette, command_palette};
#[cfg(feature = "menubar")]
pub use menubar::{Menubar, menubar};
#[cfg(feature = "status-bar")]
pub use status_bar::{StatusBar, status_bar};
#[cfg(feature = "toolbar")]
pub use toolbar::{Toolbar, toolbar};

/// The one pixel line along the inner edge of a bar, in the border colour.
#[cfg(any(feature = "menubar", feature = "status-bar", feature = "toolbar"))]
pub(crate) fn edge<'a, Message: 'a>() -> iced::Element<'a, Message> {
    use iced::widget::{container, space};

    container(space())
        .width(iced::Length::Fill)
        .height(1)
        .style(|theme| edge_style(&crate::theme::Tokens::of(theme)))
        .into()
}

/// The style of a bar's edge line.
#[cfg(any(feature = "menubar", feature = "status-bar", feature = "toolbar"))]
pub(crate) fn edge_style(tokens: &crate::theme::Tokens) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(tokens.border)),
        ..iced::widget::container::Style::default()
    }
}
