//! Counting single, double and triple clicks on a clock the caller passes in.
//!
//! iced's own click counter reads the real clock, so components that count
//! clicks use this one with the time from the app's clock instead. That lets
//! a test harness on a virtual clock time double clicks exactly.

use iced::mouse::Button;
use iced::time::{Duration, Instant};
use iced::{Point, Transformation};

use std::ops::Mul;

/// The longest gap between two presses that still counts them together.
pub const INTERVAL: Duration = Duration::from_millis(300);

/// How far apart, in logical pixels, two presses can be and still count
/// together.
pub const DISTANCE: f32 = 6.0;

/// A mouse click and how many came before it in a row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Click {
    kind: Kind,
    button: Button,
    position: Point,
    time: Instant,
}

/// The kind of click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Single,
    Double,
    Triple,
}

impl Kind {
    fn next(self) -> Kind {
        match self {
            Kind::Single => Kind::Double,
            Kind::Double => Kind::Triple,
            Kind::Triple => Kind::Double,
        }
    }
}

impl Click {
    /// A press of `button` at `position` at the instant `now`, counted
    /// against the `previous` click.
    pub fn new(position: Point, button: Button, previous: Option<Click>, now: Instant) -> Click {
        let kind = match previous {
            Some(previous)
                if previous.button == button && previous.is_consecutive(position, now) =>
            {
                previous.kind.next()
            }
            _ => Kind::Single,
        };
        Click {
            kind,
            button,
            position,
            time: now,
        }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn position(&self) -> Point {
        self.position
    }

    fn is_consecutive(&self, position: Point, now: Instant) -> bool {
        let Some(gap) = now.checked_duration_since(self.time) else {
            return false;
        };
        !gap.is_zero() && gap <= INTERVAL && self.position.distance(position) < DISTANCE
    }
}

impl Mul<Transformation> for Click {
    type Output = Click;

    fn mul(self, transformation: Transformation) -> Click {
        Click {
            position: self.position * transformation,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: Point = Point::new(10.0, 10.0);

    fn presses(start: Instant, gaps_ms: &[u64]) -> Vec<Kind> {
        let mut last = None;
        let mut now = start;
        let mut kinds = Vec::new();
        for gap in gaps_ms {
            now += Duration::from_millis(*gap);
            let click = Click::new(AT, Button::Left, last, now);
            kinds.push(click.kind());
            last = Some(click);
        }
        kinds
    }

    #[test]
    fn quick_presses_count_up_to_triple_and_then_alternate() {
        let kinds = presses(Instant::now(), &[0, 100, 100, 100]);
        assert_eq!(
            kinds,
            vec![Kind::Single, Kind::Double, Kind::Triple, Kind::Double]
        );
    }

    #[test]
    fn a_gap_longer_than_the_interval_starts_again() {
        let kinds = presses(Instant::now(), &[0, 300, 301]);
        assert_eq!(kinds, vec![Kind::Single, Kind::Double, Kind::Single]);
    }

    #[test]
    fn presses_at_the_same_instant_do_not_count_together() {
        let kinds = presses(Instant::now(), &[0, 0]);
        assert_eq!(kinds, vec![Kind::Single, Kind::Single]);
    }

    #[test]
    fn an_earlier_instant_does_not_count_together() {
        let start = Instant::now() + Duration::from_secs(1);
        let first = Click::new(AT, Button::Left, None, start);
        let earlier = start - Duration::from_millis(50);
        let second = Click::new(AT, Button::Left, Some(first), earlier);
        assert_eq!(second.kind(), Kind::Single);
    }

    #[test]
    fn a_far_press_or_another_button_starts_again() {
        let start = Instant::now();
        let first = Click::new(AT, Button::Left, None, start);
        let later = start + Duration::from_millis(100);
        let far = Click::new(Point::new(16.0, 10.0), Button::Left, Some(first), later);
        assert_eq!(far.kind(), Kind::Single);
        let near = Click::new(Point::new(15.0, 10.0), Button::Left, Some(first), later);
        assert_eq!(near.kind(), Kind::Double);
        let right = Click::new(AT, Button::Right, Some(first), later);
        assert_eq!(right.kind(), Kind::Single);
    }

    #[test]
    fn a_transformation_moves_the_position_and_keeps_the_count() {
        let start = Instant::now();
        let first = Click::new(AT, Button::Left, None, start);
        let second = Click::new(AT, Button::Left, Some(first), start + INTERVAL);
        let scaled = second * Transformation::scale(2.0);
        assert_eq!(scaled.position(), Point::new(20.0, 20.0));
        assert_eq!(scaled.kind(), Kind::Double);
    }
}
