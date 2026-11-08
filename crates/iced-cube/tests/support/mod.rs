//! A small runtime for system tests that need widget state to survive
//! updates: it applies each batch's messages to the app state and rebuilds
//! the interface on top of the previous widget tree, as iced does.

// Each test file uses only some of these helpers.
#![allow(dead_code)]

use futures::executor::block_on;
use iced::keyboard::{self, Key, Modifiers, key};
use iced::{Element, Event, Font, Pixels, Size};
use iced_test::core::clipboard::{Clipboard, Kind};
use iced_test::core::renderer::Headless;
use iced_test::core::widget::operation::focusable;
use iced_test::renderer::Renderer;
use iced_test::runtime::user_interface::{Cache, UserInterface};

/// An in-memory clipboard, so copy, cut and paste can be checked.
#[derive(Debug, Default)]
pub struct Memory(pub Option<String>);

impl Clipboard for Memory {
    fn read(&self, _kind: Kind) -> Option<String> {
        self.0.clone()
    }

    fn write(&mut self, _kind: Kind, contents: String) {
        self.0 = Some(contents);
    }
}

pub struct Harness<S, M> {
    pub state: S,
    pub clipboard: Memory,
    renderer: Renderer,
    cache: Option<Cache>,
    view: for<'a> fn(&'a S) -> Element<'a, M>,
    update: fn(&mut S, M),
}

impl<S, M> Harness<S, M> {
    pub fn new(state: S, view: for<'a> fn(&'a S) -> Element<'a, M>, update: fn(&mut S, M)) -> Self {
        let Some(renderer) = block_on(<Renderer as Headless>::new(
            Font::with_name("Fira Sans"),
            Pixels(16.0),
            Some("tiny-skia"),
        )) else {
            panic!("a headless renderer");
        };
        Self {
            state,
            clipboard: Memory::default(),
            renderer,
            cache: Some(Cache::default()),
            view,
            update,
        }
    }

    /// Focuses the widget with `id`.
    pub fn focus(&mut self, id: impl Into<iced::widget::Id>) {
        let cache = self.cache.take().unwrap_or_default();
        let mut ui = build((self.view)(&self.state), cache, &mut self.renderer);
        ui.operate(&self.renderer, &mut focusable::focus(id.into()));
        self.cache = Some(ui.into_cache());
    }

    /// Delivers `events` as one batch, then applies the messages they
    /// produced. Returns whether each event was captured.
    pub fn batch(&mut self, events: impl IntoIterator<Item = Event>) -> Vec<bool> {
        let events: Vec<Event> = events.into_iter().collect();
        let mut messages = Vec::new();
        let cache = self.cache.take().unwrap_or_default();
        let mut ui = build((self.view)(&self.state), cache, &mut self.renderer);
        let (_, statuses) = ui.update(
            &events,
            iced::mouse::Cursor::Unavailable,
            &mut self.renderer,
            &mut self.clipboard,
            &mut messages,
        );
        self.cache = Some(ui.into_cache());
        for message in messages {
            (self.update)(&mut self.state, message);
        }
        statuses
            .into_iter()
            .map(|status| status == iced::event::Status::Captured)
            .collect()
    }

    /// Types `text` one key at a time, applying each key before the next.
    pub fn type_text(&mut self, text: &str) {
        for event in iced_test::simulator::typewrite(text) {
            self.batch([event]);
        }
    }

    /// Presses and releases `key` with `modifiers` held. Returns whether the
    /// press was captured.
    pub fn press(&mut self, modifiers: Modifiers, key: impl Into<Key>) -> bool {
        let key = key.into();
        self.batch([Event::Keyboard(keyboard::Event::ModifiersChanged(
            modifiers,
        ))]);
        let captured = self.batch([press(modifiers, key.clone())]);
        self.batch([
            release(modifiers, key),
            Event::Keyboard(keyboard::Event::ModifiersChanged(Modifiers::empty())),
        ]);
        captured.first().copied().unwrap_or(false)
    }

    /// Presses a letter that carries `text` even with modifiers held, as
    /// some platforms deliver Ctrl+1 or AltGr chords. Returns whether the
    /// press was captured.
    pub fn press_with_text(&mut self, modifiers: Modifiers, c: char) -> bool {
        let key = letter(c);
        self.batch([Event::Keyboard(keyboard::Event::ModifiersChanged(
            modifiers,
        ))]);
        let captured = self.batch([Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key.clone(),
            physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
            location: keyboard::Location::Standard,
            modifiers,
            repeat: false,
            text: Some(c.to_string().into()),
        })]);
        self.batch([
            release(modifiers, key),
            Event::Keyboard(keyboard::Event::ModifiersChanged(Modifiers::empty())),
        ]);
        captured.first().copied().unwrap_or(false)
    }

    pub fn tap(&mut self, key: impl Into<Key>) -> bool {
        self.press(Modifiers::empty(), key)
    }
}

fn build<'a, M>(
    element: Element<'a, M>,
    cache: Cache,
    renderer: &mut Renderer,
) -> UserInterface<'a, M, iced::Theme, Renderer> {
    UserInterface::build(element, Size::new(800.0, 600.0), cache, renderer)
}

/// A key press as winit delivers it: a letter typed with Ctrl or Cmd held
/// carries no printable text.
pub fn press(modifiers: Modifiers, key: Key) -> Event {
    let text = match &key {
        Key::Character(c) if !modifiers.command() => Some(c.clone()),
        _ => None,
    };
    Event::Keyboard(keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers,
        repeat: false,
        text,
    })
}

fn release(modifiers: Modifiers, key: Key) -> Event {
    Event::Keyboard(keyboard::Event::KeyReleased {
        key: key.clone(),
        modified_key: key,
        physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers,
    })
}

pub fn letter(c: char) -> Key {
    Key::Character(c.to_string().into())
}

/// Ctrl, or Cmd on macOS.
pub const COMMAND: Modifiers = Modifiers::COMMAND;

/// The modifier that moves and deletes by word: Alt on macOS, Ctrl elsewhere.
pub fn jump() -> Modifiers {
    if cfg!(target_os = "macos") {
        Modifiers::ALT
    } else {
        Modifiers::CTRL
    }
}
