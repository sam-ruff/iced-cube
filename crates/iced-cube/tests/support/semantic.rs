//! Records what components report to icedwright, as the semantic tree and
//! its YAML form. Included by system tests under the `icedwright` feature.

// Each test file uses only some of these helpers.
#![allow(dead_code)]

use futures::executor::block_on;
use iced::{Element, Event, Font, Pixels, Size, mouse};
use iced_test::core::clipboard;
use iced_test::core::renderer::Headless;
use iced_test::renderer::Renderer;
use iced_test::runtime::user_interface::{Cache, UserInterface};
use icedwright_protocol::{RawTree, RawWindowInfo};
use icedwright_semantics::yaml::{self, YamlOptions};
use icedwright_semantics::{SemanticTree, WindowId, collect, record};
use icedwright_types::ScaleFactor;

const WINDOW: Size = Size::new(800.0, 600.0);

/// Keeps widget state between frames, so a test can open an overlay with
/// events and then record the tree.
pub struct Recorder {
    renderer: Renderer,
    cache: Option<Cache>,
}

impl Default for Recorder {
    fn default() -> Self {
        let Some(renderer) = block_on(<Renderer as Headless>::new(
            Font::with_name("Fira Sans"),
            Pixels(16.0),
            Some("tiny-skia"),
        )) else {
            panic!("a headless renderer");
        };
        Self {
            renderer,
            cache: Some(Cache::default()),
        }
    }
}

impl Recorder {
    /// Delivers `events` with the pointer at `cursor` and returns the
    /// messages they produced.
    pub fn events<'a, M>(
        &mut self,
        element: impl Into<Element<'a, M>>,
        cursor: mouse::Cursor,
        events: &[Event],
    ) -> Vec<M> {
        let cache = self.cache.take().unwrap_or_default();
        let mut ui = UserInterface::build(element.into(), WINDOW, cache, &mut self.renderer);
        let mut messages = Vec::new();
        let _ = ui.update(
            events,
            cursor,
            &mut self.renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        self.cache = Some(ui.into_cache());
        messages
    }

    /// The semantic tree of `element`, built on the widget state so far.
    pub fn tree<'a, M>(&mut self, element: impl Into<Element<'a, M>>) -> SemanticTree {
        let cache = self.cache.take().unwrap_or_default();
        let mut ui = UserInterface::build(element.into(), WINDOW, cache, &mut self.renderer);
        let tree = collect(&mut ui, &self.renderer, window());
        self.cache = Some(ui.into_cache());
        tree
    }

    /// The raw operation calls `element` makes, before any inference: the
    /// exact shape every operation sees.
    pub fn raw<'a, M>(&mut self, element: impl Into<Element<'a, M>>) -> RawTree {
        let cache = self.cache.take().unwrap_or_default();
        let mut ui = UserInterface::build(element.into(), WINDOW, cache, &mut self.renderer);
        let raw = record(&mut ui, &self.renderer, window());
        self.cache = Some(ui.into_cache());
        raw
    }

    /// The YAML of `element`'s tree, without bounds.
    pub fn yaml<'a, M>(&mut self, element: impl Into<Element<'a, M>>) -> String {
        write(&self.tree(element))
    }
}

fn window() -> RawWindowInfo {
    RawWindowInfo {
        id: WindowId(0),
        title: String::new(),
        size: icedwright_types::Size::new(WINDOW.width, WINDOW.height),
        scale_factor: ScaleFactor::default(),
    }
}

/// The YAML of a whole tree, without bounds.
pub fn write(tree: &SemanticTree) -> String {
    let Some(root) = tree.roots().first() else {
        return String::new();
    };
    yaml::write(tree, *root, &YamlOptions::default())
}

/// The YAML of `element`'s tree, recorded in a fresh interface.
pub fn yaml<'a, M>(element: impl Into<Element<'a, M>>) -> String {
    Recorder::default().yaml(element)
}
