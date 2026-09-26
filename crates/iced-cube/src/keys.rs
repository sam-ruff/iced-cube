//! Keyboard shortcuts that apps can rebind.
//!
//! iced has no focus model for most widgets, so shortcuts are app-wide: the
//! app subscribes with [`subscription`], stores each [`Event`] it receives,
//! and resolves it through a component's [`Keymap`] into that component's
//! own action. Every interactive component has an `Action` enum and a
//! `default_keymap()`, and an app can [`bind`](Keymap::bind),
//! [`unbind`](Keymap::unbind) or [`clear`](Keymap::clear) the defaults.
//!
//! ```no_run
//! # #[cfg(feature = "tabs")]
//! # mod example {
//! use iced::Subscription;
//! use iced::keyboard::key::Named;
//! use iced_cube::keys::{self, Chord, Keymap};
//! use iced_cube::navigation::tabs::{self, Action};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Key(keys::Event),
//!     Tabs(tabs::Event<u8>),
//! }
//!
//! struct App {
//!     tabs: tabs::State<u8>,
//!     keymap: Keymap<Action>,
//! }
//!
//! impl App {
//!     fn new() -> Self {
//!         let keymap = tabs::default_keymap()
//!             .unbind(&Chord::named(Named::ArrowLeft))
//!             .unbind(&Chord::named(Named::ArrowRight));
//!         Self { tabs: tabs::State::new([tabs::tab(1, "One"), tabs::tab(2, "Two")]), keymap }
//!     }
//!
//!     fn update(&mut self, message: Message) {
//!         let event = match message {
//!             Message::Key(key) => self
//!                 .keymap
//!                 .resolve_event(&key)
//!                 .and_then(|action| action.event(&self.tabs)),
//!             Message::Tabs(event) => Some(event),
//!         };
//!         if let Some(event) = event {
//!             let _ = self.tabs.update(event);
//!         }
//!     }
//!
//!     fn subscription(&self) -> Subscription<Message> {
//!         keys::subscription().map(Message::Key)
//!     }
//! }
//! # }
//! ```

use std::fmt;
use std::str::FromStr;

use iced::Subscription;
use iced::keyboard::{self, Key, Modifiers, key::Named};

/// A key press delivered by [`subscription`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub key: Key,
    pub modifiers: Modifiers,
}

/// Key presses that no widget captured, such as a focused text input.
pub fn subscription() -> Subscription<Event> {
    keyboard::listen().filter_map(|event| match event {
        keyboard::Event::KeyPressed { key, modifiers, .. } => Some(Event { key, modifiers }),
        _ => None,
    })
}

/// Named keys a [`Chord`] understands: the canonical name first, then aliases.
const NAMED: &[(Named, &[&str])] = &[
    (Named::Enter, &["Enter", "Return"]),
    (Named::Tab, &["Tab"]),
    (Named::Space, &["Space", "Spacebar"]),
    (Named::Escape, &["Escape", "Esc"]),
    (Named::Backspace, &["Backspace"]),
    (Named::Delete, &["Delete", "Del"]),
    (Named::Insert, &["Insert", "Ins"]),
    (Named::ArrowUp, &["ArrowUp", "Up"]),
    (Named::ArrowDown, &["ArrowDown", "Down"]),
    (Named::ArrowLeft, &["ArrowLeft", "Left"]),
    (Named::ArrowRight, &["ArrowRight", "Right"]),
    (Named::Home, &["Home"]),
    (Named::End, &["End"]),
    (Named::PageUp, &["PageUp", "PgUp"]),
    (Named::PageDown, &["PageDown", "PgDn"]),
    (Named::F1, &["F1"]),
    (Named::F2, &["F2"]),
    (Named::F3, &["F3"]),
    (Named::F4, &["F4"]),
    (Named::F5, &["F5"]),
    (Named::F6, &["F6"]),
    (Named::F7, &["F7"]),
    (Named::F8, &["F8"]),
    (Named::F9, &["F9"]),
    (Named::F10, &["F10"]),
    (Named::F11, &["F11"]),
    (Named::F12, &["F12"]),
];

/// Display name of the logo key on this platform.
const LOGO: &str = if cfg!(target_os = "macos") {
    "Cmd"
} else {
    "Super"
};

/// A key plus the modifiers held with it, such as `Ctrl+Shift+Tab`.
///
/// Parse one from a string: modifiers and the key are joined by `+`, and
/// case does not matter. Modifiers are `Ctrl`, `Shift`, `Alt` (or
/// `Option`), `Cmd` (or `Super`, `Meta`) and `Mod`, which is Cmd on macOS
/// and Ctrl elsewhere. Keys are a single character (`K`, `/`, `+`) or a
/// name: `Enter`, `Tab`, `Space`, `Escape`, `Backspace`, `Delete`,
/// `Insert`, `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`, `Home`,
/// `End`, `PageUp`, `PageDown` and `F1` to `F12`.
///
/// ```
/// use iced_cube::keys::Chord;
///
/// let chord: Chord = "ctrl+shift+tab".parse().unwrap();
/// assert_eq!(chord.to_string(), "Ctrl+Shift+Tab");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Chord {
    key: Key,
    modifiers: Modifiers,
}

impl Chord {
    /// A chord for `key`. Characters are stored in lower case.
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key: normalise(key),
            modifiers,
        }
    }

    /// A named key with no modifiers.
    pub fn named(key: Named) -> Self {
        Self::new(Key::Named(key), Modifiers::empty())
    }

    /// A character key with no modifiers.
    pub fn character(key: char) -> Self {
        Self::new(Key::Character(key.to_string().into()), Modifiers::empty())
    }

    pub fn ctrl(self) -> Self {
        self.with(Modifiers::CTRL)
    }

    pub fn shift(self) -> Self {
        self.with(Modifiers::SHIFT)
    }

    pub fn alt(self) -> Self {
        self.with(Modifiers::ALT)
    }

    /// Adds the logo key: Cmd on macOS, Super or the Windows key elsewhere.
    pub fn logo(self) -> Self {
        self.with(Modifiers::LOGO)
    }

    /// Adds Cmd on macOS and Ctrl elsewhere.
    pub fn command(self) -> Self {
        self.with(Modifiers::COMMAND)
    }

    fn with(mut self, modifiers: Modifiers) -> Self {
        self.modifiers |= modifiers;
        self
    }

    pub fn key(&self) -> &Key {
        &self.key
    }

    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Whether a key press with exactly these modifiers triggers the chord.
    pub fn matches(&self, key: &Key, modifiers: Modifiers) -> bool {
        self.modifiers == modifiers && same_key(&self.key, key)
    }
}

fn normalise(key: Key) -> Key {
    match key {
        Key::Character(text) => Key::Character(text.to_lowercase().into()),
        key => key,
    }
}

fn same_key(chord: &Key, pressed: &Key) -> bool {
    match (chord, pressed) {
        (Key::Character(a), Key::Character(b)) => a.as_str() == b.to_lowercase(),
        (a, b) => a == b,
    }
}

/// Why a string is not a valid [`Chord`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseChordError {
    Empty,
    /// A `+` with nothing on one side, such as `Ctrl+`.
    MissingKey,
    UnknownModifier(String),
    UnknownKey(String),
}

impl fmt::Display for ParseChordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty key chord"),
            Self::MissingKey => write!(f, "key chord has an empty part"),
            Self::UnknownModifier(name) => write!(f, "unknown modifier \"{name}\""),
            Self::UnknownKey(name) => write!(f, "unknown key \"{name}\""),
        }
    }
}

impl std::error::Error for ParseChordError {}

impl FromStr for Chord {
    type Err = ParseChordError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if value.is_empty() {
            return Err(ParseChordError::Empty);
        }

        // A trailing "++" means the key itself is "+".
        let (modifiers, key) = match value.strip_suffix("++") {
            Some(rest) => (Some(rest), "+"),
            None if value == "+" => (None, "+"),
            None => match value.rsplit_once('+') {
                Some((rest, key)) => (Some(rest), key),
                None => (None, value),
            },
        };

        let mut chord = Chord::new(parse_key(key.trim())?, Modifiers::empty());
        let Some(modifiers) = modifiers else {
            return Ok(chord);
        };
        for name in modifiers.split('+') {
            chord = chord.with(parse_modifier(name.trim())?);
        }
        Ok(chord)
    }
}

impl TryFrom<&str> for Chord {
    type Error = ParseChordError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

fn parse_key(name: &str) -> Result<Key, ParseChordError> {
    if name.is_empty() {
        return Err(ParseChordError::MissingKey);
    }
    if let Some((named, _)) = NAMED
        .iter()
        .find(|(_, names)| names.iter().any(|alias| alias.eq_ignore_ascii_case(name)))
    {
        return Ok(Key::Named(*named));
    }
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(_), None) => Ok(Key::Character(name.into())),
        _ => Err(ParseChordError::UnknownKey(name.to_owned())),
    }
}

fn parse_modifier(name: &str) -> Result<Modifiers, ParseChordError> {
    if name.is_empty() {
        return Err(ParseChordError::MissingKey);
    }
    let modifiers = match name.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Modifiers::CTRL,
        "shift" => Modifiers::SHIFT,
        "alt" | "option" | "opt" => Modifiers::ALT,
        "cmd" | "command" | "super" | "meta" | "logo" | "win" => Modifiers::LOGO,
        "mod" => Modifiers::COMMAND,
        _ => return Err(ParseChordError::UnknownModifier(name.to_owned())),
    };
    Ok(modifiers)
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (modifier, name) in [
            (Modifiers::CTRL, "Ctrl"),
            (Modifiers::ALT, "Alt"),
            (Modifiers::SHIFT, "Shift"),
            (Modifiers::LOGO, LOGO),
        ] {
            if self.modifiers.contains(modifier) {
                write!(f, "{name}+")?;
            }
        }
        match &self.key {
            Key::Named(named) => match NAMED.iter().find(|(key, _)| key == named) {
                Some((_, names)) => write!(f, "{}", names.first().copied().unwrap_or_default()),
                None => write!(f, "{named:?}"),
            },
            Key::Character(text) => write!(f, "{}", text.to_uppercase()),
            Key::Unidentified => write!(f, "Unidentified"),
        }
    }
}

/// A component action that can be bound to keys.
pub trait Action: Copy + PartialEq + fmt::Debug + 'static {
    /// Every action, in the order they are documented.
    const ALL: &'static [Self];

    /// The component's default shortcuts.
    fn defaults() -> Keymap<Self>;

    /// A short name, such as `"Next"`.
    fn name(self) -> &'static str;

    /// One sentence describing what the action does.
    fn description(self) -> &'static str;
}

/// Chords bound to actions. Each chord triggers at most one action; an
/// action may have several chords.
#[derive(Debug, Clone, PartialEq)]
pub struct Keymap<A> {
    bindings: Vec<(Chord, A)>,
}

impl<A> Default for Keymap<A> {
    fn default() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }
}

impl<A> Keymap<A> {
    /// An empty keymap.
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds `chord` to `action`, replacing whatever the chord did before.
    pub fn bind(mut self, chord: Chord, action: A) -> Self {
        match self.bindings.iter_mut().find(|(bound, _)| *bound == chord) {
            Some(binding) => binding.1 = action,
            None => self.bindings.push((chord, action)),
        }
        self
    }

    /// Removes the binding for `chord`, if there is one.
    pub fn unbind(mut self, chord: &Chord) -> Self {
        self.bindings.retain(|(bound, _)| bound != chord);
        self
    }

    /// Removes every binding.
    pub fn clear(mut self) -> Self {
        self.bindings.clear();
        self
    }

    /// Every binding, in the order it was added.
    pub fn bindings(&self) -> impl Iterator<Item = (&Chord, &A)> {
        self.bindings.iter().map(|(chord, action)| (chord, action))
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl<A: Clone> Keymap<A> {
    /// The action bound to a key press, if any.
    pub fn resolve(&self, key: &Key, modifiers: Modifiers) -> Option<A> {
        self.bindings
            .iter()
            .find(|(chord, _)| chord.matches(key, modifiers))
            .map(|(_, action)| action.clone())
    }

    /// The action bound to a key press from [`subscription`], if any.
    pub fn resolve_event(&self, event: &Event) -> Option<A> {
        self.resolve(&event.key, event.modifiers)
    }
}

impl<A: Clone + PartialEq> Keymap<A> {
    /// The chords bound to `action`.
    pub fn chords(&self, action: &A) -> Vec<&Chord> {
        self.bindings
            .iter()
            .filter(|(_, bound)| bound == action)
            .map(|(chord, _)| chord)
            .collect()
    }

    /// Removes every chord bound to `action`.
    pub fn unbind_action(mut self, action: &A) -> Self {
        self.bindings.retain(|(_, bound)| bound != action);
        self
    }

    /// Chords grouped by action, in the order each action first appears.
    pub fn grouped(&self) -> Vec<(A, Vec<Chord>)> {
        let mut groups: Vec<(A, Vec<Chord>)> = Vec::new();
        for (chord, action) in &self.bindings {
            match groups.iter_mut().find(|(bound, _)| bound == action) {
                Some((_, chords)) => chords.push(chord.clone()),
                None => groups.push((action.clone(), vec![chord.clone()])),
            }
        }
        groups
    }
}

impl<A: Action> Keymap<A> {
    /// The default shortcuts of the component that owns `A`.
    pub fn defaults() -> Self {
        A::defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(value: &str) -> Chord {
        value.parse().unwrap()
    }

    fn character(value: &str) -> Key {
        Key::Character(value.into())
    }

    #[test]
    fn parses_named_keys_with_modifiers() {
        let parsed = chord("Ctrl+Shift+Tab");
        assert_eq!(parsed.key(), &Key::Named(Named::Tab));
        assert_eq!(parsed.modifiers(), Modifiers::CTRL | Modifiers::SHIFT);
        assert_eq!(chord("Escape"), Chord::named(Named::Escape));
        assert_eq!(chord("ArrowRight"), Chord::named(Named::ArrowRight));
    }

    #[test]
    fn parsing_ignores_case_and_accepts_aliases() {
        assert_eq!(chord("ctrl+k"), chord("Ctrl+K"));
        assert_eq!(chord("CONTROL + K"), chord("Ctrl+K"));
        assert_eq!(chord("esc"), Chord::named(Named::Escape));
        assert_eq!(chord("right"), Chord::named(Named::ArrowRight));
        assert_eq!(chord("Option+PgDn"), Chord::named(Named::PageDown).alt());
        assert_eq!(chord("Meta+S"), Chord::character('s').logo());
    }

    #[test]
    fn mod_is_the_platform_command_key() {
        let parsed = chord("Mod+K");
        assert_eq!(parsed.modifiers(), Modifiers::COMMAND);
        if cfg!(target_os = "macos") {
            assert_eq!(parsed, chord("Cmd+K"));
        } else {
            assert_eq!(parsed, chord("Ctrl+K"));
        }
    }

    #[test]
    fn plus_can_be_the_key() {
        assert_eq!(chord("+").key(), &character("+"));
        let zoom = chord("Ctrl++");
        assert_eq!(zoom.key(), &character("+"));
        assert_eq!(zoom.modifiers(), Modifiers::CTRL);
    }

    #[test]
    fn rejects_invalid_chords() {
        assert_eq!("".parse::<Chord>(), Err(ParseChordError::Empty));
        assert_eq!("  ".parse::<Chord>(), Err(ParseChordError::Empty));
        assert_eq!("Ctrl+".parse::<Chord>(), Err(ParseChordError::MissingKey));
        assert_eq!("Ctrl++K".parse::<Chord>(), Err(ParseChordError::MissingKey));
        assert_eq!(
            "Hyper+K".parse::<Chord>(),
            Err(ParseChordError::UnknownModifier("Hyper".into()))
        );
        assert_eq!(
            "Ctrl+Banana".parse::<Chord>(),
            Err(ParseChordError::UnknownKey("Banana".into()))
        );
    }

    #[test]
    fn displays_in_canonical_form() {
        assert_eq!(chord("shift+ctrl+tab").to_string(), "Ctrl+Shift+Tab");
        assert_eq!(chord("ctrl+k").to_string(), "Ctrl+K");
        assert_eq!(chord("esc").to_string(), "Escape");
        assert_eq!(chord("alt+up").to_string(), "Alt+ArrowUp");
        assert_eq!(chord("Ctrl++").to_string(), "Ctrl++");
    }

    #[test]
    fn display_round_trips() {
        for value in ["Ctrl+Alt+Shift+Cmd+F5", "Space", "Mod+/", "Shift+End", "+"] {
            let parsed = chord(value);
            assert_eq!(chord(&parsed.to_string()), parsed, "{value}");
        }
    }

    #[test]
    fn matches_exact_modifiers_and_any_case() {
        let save = chord("Ctrl+S");
        assert!(save.matches(&character("s"), Modifiers::CTRL));
        assert!(save.matches(&character("S"), Modifiers::CTRL));
        assert!(!save.matches(&character("s"), Modifiers::empty()));
        assert!(!save.matches(&character("s"), Modifiers::CTRL | Modifiers::SHIFT));
        assert!(!save.matches(&Key::Named(Named::Tab), Modifiers::CTRL));
        assert!(chord("Tab").matches(&Key::Named(Named::Tab), Modifiers::empty()));
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Test {
        Open,
        Close,
    }

    fn keymap() -> Keymap<Test> {
        Keymap::new()
            .bind(chord("Ctrl+O"), Test::Open)
            .bind(chord("Enter"), Test::Open)
            .bind(chord("Escape"), Test::Close)
    }

    #[test]
    fn resolves_bound_chords_only() {
        let keymap = keymap();
        assert_eq!(
            keymap.resolve(&character("o"), Modifiers::CTRL),
            Some(Test::Open)
        );
        assert_eq!(
            keymap.resolve(&Key::Named(Named::Escape), Modifiers::empty()),
            Some(Test::Close)
        );
        assert_eq!(keymap.resolve(&character("o"), Modifiers::empty()), None);
        let event = Event {
            key: Key::Named(Named::Enter),
            modifiers: Modifiers::empty(),
        };
        assert_eq!(keymap.resolve_event(&event), Some(Test::Open));
    }

    #[test]
    fn bind_replaces_an_existing_chord() {
        let keymap = keymap().bind(chord("Enter"), Test::Close);
        assert_eq!(keymap.len(), 3);
        assert_eq!(
            keymap.resolve(&Key::Named(Named::Enter), Modifiers::empty()),
            Some(Test::Close)
        );
    }

    #[test]
    fn unbind_and_clear() {
        let keymap = keymap().unbind(&chord("Escape"));
        assert_eq!(
            keymap.resolve(&Key::Named(Named::Escape), Modifiers::empty()),
            None
        );
        assert_eq!(keymap.len(), 2);
        let unchanged = keymap.clone().unbind(&chord("F1"));
        assert_eq!(unchanged, keymap);
        assert!(keymap.clear().is_empty());
    }

    #[test]
    fn chords_grouped_and_unbound_by_action() {
        let keymap = keymap();
        assert_eq!(keymap.chords(&Test::Open).len(), 2);
        let grouped = keymap.grouped();
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].0, Test::Open);
        assert_eq!(grouped[0].1, vec![chord("Ctrl+O"), chord("Enter")]);
        assert_eq!(grouped[1], (Test::Close, vec![chord("Escape")]));
        let closed = keymap.unbind_action(&Test::Open);
        assert_eq!(closed.len(), 1);
    }
}
