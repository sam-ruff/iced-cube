//! The gallery program.

use iced::widget::{column, container, row, rule, scrollable, text};
use iced::{Element, Length, Subscription, Task, Theme};
use iced_cube::primitives::button::Variant;
use iced_cube::theme::{self, Tokens};
use iced_cube::{button, lucide};

use crate::bridge;
use crate::stories::{ALL, AnyMessage, AnyStory};

/// Space between the preview frame and a story that is not edge to edge.
const STORY_PADDING: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    #[default]
    Light,
    Dark,
}

impl ThemeChoice {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    pub fn theme(self) -> Theme {
        match self {
            Self::Light => theme::light(),
            Self::Dark => theme::dark(),
        }
    }
}

/// Whether the gallery shows its own navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// A sidebar of every story, for local development.
    Browser,
    /// One story filling the window, for embedding.
    Single,
}

#[derive(Debug, Clone)]
pub enum Message {
    Story(AnyMessage),
    Select(&'static str),
    ToggleTheme,
    Host(bridge::Event),
}

#[derive(Debug)]
pub struct Gallery {
    mode: Mode,
    theme: ThemeChoice,
    selected: &'static str,
    story: AnyStory,
}

impl Gallery {
    /// Opens `story`, or the first story when the id is unknown.
    pub fn new(mode: Mode, story: Option<&str>, theme: ThemeChoice) -> Self {
        let meta = story
            .and_then(|id| ALL.iter().find(|meta| meta.id == id))
            .unwrap_or(&ALL[0]);

        Self {
            mode,
            theme,
            selected: meta.id,
            story: open(meta.id),
        }
    }

    pub fn selected(&self) -> &'static str {
        self.selected
    }

    pub fn theme_choice(&self) -> ThemeChoice {
        self.theme
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Story(message) => self.story.update(message),
            Message::Select(id) => {
                self.selected = id;
                self.story = open(id);
            }
            Message::ToggleTheme => self.theme = self.theme.toggled(),
            Message::Host(bridge::Event::Theme(choice)) => self.theme = choice,
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let story = container(self.story.view().map(Message::Story))
            .padding(self.padding())
            .center(Length::Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(Tokens::of(theme).background.into()),
                ..container::Style::default()
            });

        if self.mode == Mode::Single {
            return story.into();
        }

        let list = column(ALL.iter().map(|meta| {
            let variant = if meta.id == self.selected {
                Variant::Secondary
            } else {
                Variant::Ghost
            };
            button(format!("{} / {}", meta.component, meta.title))
                .variant(variant)
                .width(Length::Fill)
                .on_press(Message::Select(meta.id))
                .into()
        }))
        .spacing(2);

        let toggle = button(match self.theme {
            ThemeChoice::Light => "Dark mode",
            ThemeChoice::Dark => "Light mode",
        })
        .icon(match self.theme {
            ThemeChoice::Light => lucide!(Moon),
            ThemeChoice::Dark => lucide!(Sun),
        })
        .variant(Variant::Outline)
        .width(Length::Fill)
        .on_press(Message::ToggleTheme);

        let sidebar = column![
            text("iced-cube").size(18),
            scrollable(list).height(Length::Fill),
            toggle
        ]
        .spacing(12)
        .padding(12)
        .width(240);

        row![sidebar, rule::vertical(1), story].into()
    }

    /// Space around the open story, or none for an edge-to-edge story.
    pub fn padding(&self) -> f32 {
        let edge = ALL.iter().any(|meta| meta.id == self.selected && meta.edge);
        if edge { 0.0 } else { STORY_PADDING }
    }

    /// The open story's own theme, or the light or dark gallery theme.
    pub fn theme(&self) -> Theme {
        self.story.theme().unwrap_or_else(|| self.theme.theme())
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            bridge::subscription().map(Message::Host),
            self.story.subscription().map(Message::Story),
        ])
    }
}

fn open(id: &str) -> AnyStory {
    AnyStory::new(id).unwrap_or_else(|| {
        // Ids come from ALL, so this only guards against a registry typo.
        AnyStory::new(ALL[0].id).unwrap_or_else(|| panic!("story registry is empty"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_story_falls_back_to_first() {
        let gallery = Gallery::new(Mode::Single, Some("nope"), ThemeChoice::Light);
        assert_eq!(gallery.selected(), ALL[0].id);
    }

    #[test]
    fn select_switches_story() {
        let mut gallery = Gallery::new(Mode::Browser, None, ThemeChoice::Light);
        let last = ALL[ALL.len() - 1].id;
        let _ = gallery.update(Message::Select(last));
        assert_eq!(gallery.selected(), last);
    }

    #[test]
    fn host_theme_event_overrides_toggle() {
        let mut gallery = Gallery::new(Mode::Single, None, ThemeChoice::Light);
        let _ = gallery.update(Message::ToggleTheme);
        assert_eq!(gallery.theme_choice(), ThemeChoice::Dark);
        let _ = gallery.update(Message::Host(bridge::Event::Theme(ThemeChoice::Light)));
        assert_eq!(gallery.theme_choice(), ThemeChoice::Light);
    }

    #[test]
    fn story_theme_overrides_the_gallery_theme() {
        let plain = Gallery::new(Mode::Single, Some("button/variants"), ThemeChoice::Dark);
        assert_eq!(plain.theme().to_string(), theme::dark().to_string());

        let custom = Gallery::new(Mode::Single, Some("theme/custom"), ThemeChoice::Dark);
        assert_ne!(custom.theme().to_string(), theme::dark().to_string());
        assert_ne!(custom.theme().to_string(), theme::light().to_string());
    }

    #[test]
    fn edge_stories_drop_the_gallery_padding() {
        for id in ["dialog/default", "dialog/form", "dialog/destructive"] {
            let gallery = Gallery::new(Mode::Single, Some(id), ThemeChoice::Light);
            assert_eq!(gallery.padding(), 0.0, "{id}");
        }
        let plain = Gallery::new(Mode::Single, Some("button/variants"), ThemeChoice::Light);
        assert_eq!(plain.padding(), STORY_PADDING);
    }

    #[test]
    fn every_story_has_a_usable_height() {
        for meta in ALL {
            assert!((120..=480).contains(&meta.height), "{}", meta.id);
        }
    }

    #[test]
    fn every_registered_story_can_be_opened() {
        for meta in ALL {
            assert!(AnyStory::new(meta.id).is_some(), "{}", meta.id);
        }
    }

    #[test]
    fn story_ids_are_unique_and_prefixed_by_component() {
        let mut ids: Vec<_> = ALL.iter().map(|meta| meta.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ALL.len());
        for meta in ALL {
            assert!(
                meta.id.starts_with(&format!("{}/", meta.component)),
                "{}",
                meta.id
            );
        }
    }
}
