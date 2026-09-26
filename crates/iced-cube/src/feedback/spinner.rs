//! An indeterminate loading spinner.
//!
//! The spinner has no clock of its own. The app keeps a phase in its state,
//! advances it with [`advance`] on each frame while loading (for example from
//! `iced::window::frames()`), and stops subscribing when it is done.

use std::f32::consts::{FRAC_PI_2, TAU};
use std::time::Duration;

use iced::widget::canvas::{self, Frame, Geometry, LineCap, Path, Stroke, path::Arc};
use iced::{Color, Element, Length, Radians, Rectangle, Renderer, Theme, mouse};

use crate::theme::{Tokens, fade};

/// Time for one full turn.
pub const PERIOD: Duration = Duration::from_millis(900);

/// Share of the circle covered by the moving arc.
pub const ARC: f32 = 0.3;

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

impl<Message> canvas::Program<Message> for Spinner {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let colours = colours(&Tokens::of(theme));
        let width = self.size.stroke();
        let mut frame = Frame::new(renderer, bounds.size());
        let center = frame.center();
        let radius = (bounds.width.min(bounds.height) - width) / 2.0;

        frame.stroke(
            &Path::circle(center, radius),
            Stroke::default()
                .with_color(colours.track)
                .with_width(width),
        );

        let (start_angle, end_angle) = arc_angles(self.phase);
        let arc = Path::new(|builder| {
            builder.arc(Arc {
                center,
                radius,
                start_angle,
                end_angle,
            });
        });
        frame.stroke(
            &arc,
            Stroke::default()
                .with_color(colours.arc)
                .with_width(width)
                .with_line_cap(LineCap::Round),
        );

        vec![frame.into_geometry()]
    }
}

impl<'a, Message: 'a> From<Spinner> for Element<'a, Message> {
    fn from(spinner: Spinner) -> Self {
        let diameter = Length::Fixed(spinner.size.diameter());
        canvas::Canvas::new(spinner)
            .width(diameter)
            .height(diameter)
            .into()
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
