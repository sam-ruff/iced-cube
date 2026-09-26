//! An indeterminate loading spinner.
//!
//! The spinner has no clock of its own. The app keeps a phase in its state,
//! advances it with [`advance`] on each frame while loading (for example from
//! `iced::window::frames()`), and stops subscribing when it is done.
//!
//! The track and the arc are small SVG images drawn through iced's image
//! path, which behaves the same on every backend. The arc comes from a fixed
//! set of [`FRAMES`] pre-rotated images, so spinners at the same phase share
//! one cached image.

use std::f32::consts::{FRAC_PI_2, TAU};
use std::sync::LazyLock;
use std::time::Duration;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::svg::{self as advanced_svg, Renderer as _};
use iced::advanced::widget::{Tree, Widget};
use iced::widget::svg::Handle;
use iced::{Color, Element, Length, Radians, Rectangle, Renderer, Theme, mouse};

use crate::icon::opaque;
use crate::theme::{Tokens, fade};

/// Time for one full turn.
pub const PERIOD: Duration = Duration::from_millis(900);

/// Share of the circle covered by the moving arc.
pub const ARC: f32 = 0.3;

/// Number of distinct arc positions in one turn.
pub const FRAMES: usize = 60;

/// Spinner diameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
}

impl Size {
    pub const ALL: [Size; 3] = [Size::Sm, Size::Md, Size::Lg];

    /// Diameter in logical pixels.
    pub const fn diameter(self) -> f32 {
        match self {
            Size::Sm => 16.0,
            Size::Md => 24.0,
            Size::Lg => 32.0,
        }
    }

    /// Stroke width in logical pixels.
    pub const fn stroke(self) -> f32 {
        match self {
            Size::Sm => 2.0,
            Size::Md => 2.5,
            Size::Lg => 3.0,
        }
    }

    const fn index(self) -> usize {
        match self {
            Size::Sm => 0,
            Size::Md => 1,
            Size::Lg => 2,
        }
    }
}

/// A spinner builder. Convert it into an [`Element`] to render.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spinner {
    phase: f32,
    size: Size,
}

/// Creates a spinner at `phase`, measured in turns (0.0 to 1.0).
pub fn spinner(phase: f32) -> Spinner {
    Spinner {
        phase: wrap(phase),
        size: Size::default(),
    }
}

impl Spinner {
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// The normalised phase, from 0.0 up to but not including 1.0.
    pub fn phase(&self) -> f32 {
        self.phase
    }
}

/// Advances `phase` by the share of [`PERIOD`] that `elapsed` covers.
pub fn advance(phase: f32, elapsed: Duration) -> f32 {
    wrap(phase + elapsed.as_secs_f32() / PERIOD.as_secs_f32())
}

/// Wraps a phase into 0.0..1.0. Non-finite phases become 0.0.
pub fn wrap(phase: f32) -> f32 {
    if !phase.is_finite() {
        return 0.0;
    }
    let wrapped = phase.rem_euclid(1.0);
    // rem_euclid can round up to exactly 1.0 for tiny negative inputs.
    if wrapped >= 1.0 { 0.0 } else { wrapped }
}

/// Start and end angles of the moving arc, clockwise from the top.
pub fn arc_angles(phase: f32) -> (Radians, Radians) {
    let start = wrap(phase) * TAU - FRAC_PI_2;
    (Radians(start), Radians(start + ARC * TAU))
}

/// The arc image drawn for `phase`: the nearest of the [`FRAMES`] positions.
pub fn frame(phase: f32) -> usize {
    let step = (wrap(phase) * FRAMES as f32).round() as usize;
    step % FRAMES
}

/// SVG markup for the full circle behind the arc.
pub fn track_svg(size: Size) -> String {
    let (centre, radius, stroke) = geometry(size);
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{d}" height="{d}" viewBox="0 0 {d} {d}"><circle cx="{centre}" cy="{centre}" r="{radius}" fill="none" stroke="black" stroke-width="{stroke}"/></svg>"#,
        d = size.diameter(),
    )
}

/// SVG markup for the arc at `phase`, running clockwise over [`ARC`] of the
/// circle from the angle [`arc_angles`] gives.
pub fn arc_svg(size: Size, phase: f32) -> String {
    let (centre, radius, stroke) = geometry(size);
    let (start, end) = arc_angles(phase);
    let point = |angle: Radians| {
        let x = centre + radius * angle.0.cos();
        let y = centre + radius * angle.0.sin();
        format!("{x:.3} {y:.3}")
    };
    let large_arc = u8::from(ARC > 0.5);
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{d}" height="{d}" viewBox="0 0 {d} {d}"><path d="M {from} A {radius} {radius} 0 {large_arc} 1 {to}" fill="none" stroke="black" stroke-width="{stroke}" stroke-linecap="round"/></svg>"#,
        d = size.diameter(),
        from = point(start),
        to = point(end),
    )
}

/// Centre, radius and stroke width, in the SVG's own units.
fn geometry(size: Size) -> (f32, f32, f32) {
    let stroke = size.stroke();
    let centre = size.diameter() / 2.0;
    (centre, centre - stroke / 2.0, stroke)
}

struct Images {
    track: [Handle; 3],
    arc: [Vec<Handle>; 3],
}

static IMAGES: LazyLock<Images> = LazyLock::new(|| Images {
    track: Size::ALL.map(|size| Handle::from_memory(track_svg(size).into_bytes())),
    arc: Size::ALL.map(|size| {
        (0..FRAMES)
            .map(|step| {
                let phase = step as f32 / FRAMES as f32;
                Handle::from_memory(arc_svg(size, phase).into_bytes())
            })
            .collect()
    }),
});

/// Colours of the track and the moving arc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colours {
    pub track: Color,
    pub arc: Color,
}

/// Resolves the spinner colours.
pub fn colours(tokens: &Tokens) -> Colours {
    Colours {
        track: fade(tokens.foreground, if tokens.is_dark { 0.18 } else { 0.12 }),
        arc: tokens.foreground,
    }
}

/// An svg image tinted with `colour`. iced ignores the alpha of a tint, so
/// the alpha becomes the image opacity.
fn image(handle: &Handle, colour: Color) -> advanced_svg::Svg {
    advanced_svg::Svg::new(handle.clone())
        .color(opaque(colour))
        .opacity(colour.a)
}

impl<Message> Widget<Message, Theme, Renderer> for Spinner {
    fn size(&self) -> iced::Size<Length> {
        let diameter = Length::Fixed(self.size.diameter());
        iced::Size::new(diameter, diameter)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let diameter = Length::Fixed(self.size.diameter());
        layout::atomic(limits, diameter, diameter)
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let colours = colours(&Tokens::of(theme));
        let index = self.size.index();
        renderer.draw_svg(image(&IMAGES.track[index], colours.track), bounds, bounds);
        let Some(arc) = IMAGES.arc[index].get(frame(self.phase)) else {
            return;
        };
        renderer.draw_svg(image(arc, colours.arc), bounds, bounds);
    }
}

impl<'a, Message: 'a> From<Spinner> for Element<'a, Message> {
    fn from(spinner: Spinner) -> Self {
        Element::new(spinner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    const EPSILON: f32 = 1e-5;

    #[test]
    fn default_builder_is_medium() {
        let s = spinner(0.25);
        assert_eq!(s.size, Size::Md);
        assert!((s.phase() - 0.25).abs() < EPSILON);
    }

    #[test]
    fn wrap_keeps_phase_in_one_turn() {
        assert_eq!(wrap(0.0), 0.0);
        assert!((wrap(1.25) - 0.25).abs() < EPSILON);
        assert!((wrap(-0.25) - 0.75).abs() < EPSILON);
        assert_eq!(wrap(1.0), 0.0);
        assert!(wrap(-1e-9) < 1.0);
    }

    #[test]
    fn wrap_maps_non_finite_to_zero() {
        assert_eq!(wrap(f32::NAN), 0.0);
        assert_eq!(wrap(f32::INFINITY), 0.0);
        assert_eq!(spinner(f32::NEG_INFINITY).phase(), 0.0);
    }

    #[test]
    fn advance_moves_by_share_of_period() {
        let half = PERIOD / 2;
        assert!((advance(0.0, half) - 0.5).abs() < EPSILON);
        assert!((advance(0.75, half) - 0.25).abs() < EPSILON);
        assert_eq!(advance(0.4, Duration::ZERO), 0.4);
    }

    #[test]
    fn a_full_period_returns_to_the_same_phase() {
        let phase = advance(0.3, PERIOD);
        assert!((phase - 0.3).abs() < EPSILON);
    }

    #[test]
    fn arc_starts_at_the_top_and_spans_arc_share() {
        let (start, end) = arc_angles(0.0);
        assert!((start.0 + FRAC_PI_2).abs() < EPSILON);
        assert!((end.0 - start.0 - ARC * TAU).abs() < EPSILON);

        let (quarter, _) = arc_angles(0.25);
        assert!(quarter.0.abs() < EPSILON);
    }

    #[test]
    fn frames_round_the_phase_to_the_nearest_step() {
        assert_eq!(frame(0.0), 0);
        assert_eq!(frame(0.5), FRAMES / 2);
        assert_eq!(frame(1.0 / FRAMES as f32 * 0.4), 0);
        assert_eq!(frame(1.0 / FRAMES as f32 * 0.6), 1);
        assert_eq!(frame(0.999), 0);
        assert_eq!(frame(f32::NAN), 0);
        assert!((0..1000).all(|i| frame(i as f32 / 997.0) < FRAMES));
    }

    #[test]
    fn svgs_fit_the_diameter_and_stroke() {
        for size in Size::ALL {
            let d = size.diameter();
            for markup in [track_svg(size), arc_svg(size, 0.4)] {
                assert!(
                    markup.contains(&format!(r#"viewBox="0 0 {d} {d}""#)),
                    "{markup}"
                );
                assert!(markup.contains(&format!(r#"stroke-width="{}""#, size.stroke())));
            }
            let (centre, radius, stroke) = geometry(size);
            assert!((centre + radius + stroke / 2.0 - d).abs() < EPSILON);
        }
    }

    #[test]
    fn arc_starts_at_the_top_at_phase_zero_and_runs_clockwise() {
        let markup = arc_svg(Size::Lg, 0.0);
        let (centre, radius, _) = geometry(Size::Lg);
        assert!(
            markup.contains(&format!("M {centre:.3} {:.3}", centre - radius)),
            "{markup}"
        );
        assert!(markup.contains(r#"stroke-linecap="round""#));
        assert!(markup.contains(" 0 0 1 "), "short clockwise arc: {markup}");
    }

    #[test]
    fn half_a_turn_starts_the_arc_at_the_bottom() {
        let markup = arc_svg(Size::Md, 0.5);
        let (centre, radius, _) = geometry(Size::Md);
        assert!(
            markup.contains(&format!("M {centre:.3} {:.3}", centre + radius)),
            "{markup}"
        );
    }

    #[test]
    fn every_frame_has_its_own_image() {
        for arcs in &IMAGES.arc {
            assert_eq!(arcs.len(), FRAMES);
            let ids: std::collections::HashSet<_> = arcs.iter().map(Handle::id).collect();
            assert_eq!(ids.len(), FRAMES);
        }
    }

    #[test]
    fn spinner_is_sized_to_its_diameter() {
        for size in Size::ALL {
            let s = spinner(0.0).size(size);
            let d = Length::Fixed(size.diameter());
            assert_eq!(
                Widget::<(), Theme, Renderer>::size(&s),
                iced::Size::new(d, d)
            );
        }
    }

    #[test]
    fn sizes_grow_monotonically() {
        let diameters = Size::ALL.map(Size::diameter);
        assert!(diameters.windows(2).all(|w| w[0] < w[1]));
        let strokes = Size::ALL.map(Size::stroke);
        assert!(strokes.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn track_is_fainter_than_arc() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let colours = colours(&tokens);
            assert_eq!(colours.arc, tokens.foreground);
            assert!(colours.track.a < colours.arc.a);
        }
    }
}
