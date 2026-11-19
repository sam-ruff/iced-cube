//! Dragging the edge between two column headers to resize the column
//! before it.

use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse::{Click, click};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Renderer as _, Shell};
use iced::{
    Background, Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    mouse, touch,
};

/// How far either side of a column's right edge a press takes hold of it.
pub(crate) const GRIP: f32 = 5.0;
const LINE: f32 = 2.0;

/// What the edge of one column may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Edge {
    pub resizable: bool,
    /// Whether the column shares the room left over, and so gives way to a
    /// column before it that grows.
    pub flexes: bool,
}

type OnResize<'a, Message> = Box<dyn Fn(usize, Vec<f32>) -> Message + 'a>;
type OnReset<'a, Message> = Box<dyn Fn(usize) -> Message + 'a>;

/// Wraps a row of header cells. Its children are `skip` leading cells and
/// then one cell per edge.
pub(crate) struct Resizer<'a, Message> {
    content: Element<'a, Message>,
    skip: usize,
    edges: Vec<Edge>,
    minimum: f32,
    line: fn(&Theme) -> Color,
    on_resize: Option<OnResize<'a, Message>>,
    on_reset: Option<OnReset<'a, Message>>,
}

pub(crate) fn resizer<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    skip: usize,
    edges: Vec<Edge>,
    minimum: f32,
    line: fn(&Theme) -> Color,
) -> Resizer<'a, Message> {
    Resizer {
        content: content.into(),
        skip,
        edges,
        minimum,
        line,
        on_resize: None,
        on_reset: None,
    }
}

impl<'a, Message> Resizer<'a, Message> {
    /// Called while an edge is dragged with the column's position and the
    /// widths of the columns up to and including it. Those before it are
    /// as they were when the drag began, so they hold still.
    pub(crate) fn on_resize(mut self, on_resize: impl Fn(usize, Vec<f32>) -> Message + 'a) -> Self {
        self.on_resize = Some(Box::new(on_resize));
        self
    }

    /// Called when an edge is double clicked.
    pub(crate) fn on_reset(mut self, on_reset: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_reset = Some(Box::new(on_reset));
        self
    }

    fn cells(&self, layout: Layout<'_>) -> Vec<Rectangle> {
        layout
            .children()
            .skip(self.skip)
            .take(self.edges.len())
            .map(|cell| cell.bounds())
            .collect()
    }

    /// The column whose edge is under `position`.
    fn edge_at(&self, layout: Layout<'_>, position: Point) -> Option<usize> {
        self.on_resize.as_ref()?;
        let cells = self.cells(layout);
        edge_at(&cells, &self.edges, position)
    }
}

fn edge_at(cells: &[Rectangle], edges: &[Edge], position: Point) -> Option<usize> {
    cells.iter().zip(edges).position(|(cell, edge)| {
        let right = cell.x + cell.width;
        edge.resizable
            && position.y >= cell.y
            && position.y <= cell.y + cell.height
            && (position.x - right).abs() <= GRIP
    })
}

/// The widest the column at `index` can be dragged: it takes the unused
/// room and squeezes the flexing columns after it down to `minimum`.
fn widest(cells: &[Rectangle], edges: &[Edge], index: usize, unused: f32, minimum: f32) -> f32 {
    let Some(cell) = cells.get(index) else {
        return minimum;
    };
    let give: f32 = cells
        .iter()
        .zip(edges)
        .skip(index + 1)
        .filter(|(_, edge)| edge.flexes)
        .map(|(cell, _)| (cell.width - minimum).max(0.0))
        .sum();
    (cell.width + unused.max(0.0) + give).max(minimum)
}

#[derive(Debug, Clone, PartialEq)]
struct Dragging {
    index: usize,
    origin: f32,
    /// Widths of the columns up to and including the dragged one.
    widths: Vec<f32>,
    widest: f32,
}

impl Dragging {
    fn widths_at(&self, x: f32, minimum: f32) -> Vec<f32> {
        let mut widths = self.widths.clone();
        if let Some(width) = widths.last_mut() {
            *width = (*width + x - self.origin).clamp(minimum, self.widest.max(minimum));
        }
        widths
    }
}

#[derive(Debug, Default)]
struct Grip {
    dragging: Option<Dragging>,
    hovered: Option<usize>,
    last_click: Option<Click>,
}

fn is_press(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. })
    )
}

fn is_move(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(touch::Event::FingerMoved { .. })
    )
}

fn is_release(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. })
    )
}

impl<Message> Widget<Message, Theme, Renderer> for Resizer<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Grip>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Grip::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        let grip = tree.state.downcast_ref::<Grip>();
        let marked = grip
            .dragging
            .as_ref()
            .map(|dragging| dragging.index)
            .or(grip.hovered);
        let Some(cell) = marked.and_then(|index| self.cells(layout).get(index).copied()) else {
            return;
        };
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x: cell.x + cell.width - LINE / 2.0,
                    width: LINE,
                    ..cell
                },
                snap: true,
                ..Quad::default()
            },
            Background::Color((self.line)(theme)),
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Tree {
            state, children, ..
        } = tree;
        let grip = state.downcast_mut::<Grip>();

        if let Some(dragging) = &grip.dragging {
            if is_release(event) {
                grip.dragging = None;
                shell.request_redraw();
                return;
            }
            if is_move(event)
                && let Some(position) = cursor.position()
                && let Some(on_resize) = &self.on_resize
            {
                let widths = dragging.widths_at(position.x, self.minimum);
                shell.publish(on_resize(dragging.index, widths));
                shell.capture_event();
            }
            return;
        }

        let over = cursor
            .position()
            .and_then(|position| self.edge_at(layout, position));
        if over != grip.hovered {
            grip.hovered = over;
            shell.request_redraw();
        }

        if is_press(event)
            && let Some(index) = over
            && let Some(position) = cursor.position()
        {
            let click = Click::new(position, mouse::Button::Left, grip.last_click);
            grip.last_click = Some(click);
            shell.capture_event();
            if click.kind() == click::Kind::Double {
                if let Some(on_reset) = &self.on_reset {
                    shell.publish(on_reset(index));
                }
                return;
            }
            let cells = self.cells(layout);
            let used: f32 = layout.children().map(|cell| cell.bounds().width).sum();
            let unused = layout.bounds().width - used;
            grip.dragging = Some(Dragging {
                index,
                origin: position.x,
                widths: cells
                    .iter()
                    .take(index + 1)
                    .map(|cell| cell.width)
                    .collect(),
                widest: widest(&cells, &self.edges, index, unused, self.minimum),
            });
            return;
        }

        self.content.as_widget_mut().update(
            &mut children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let grip = tree.state.downcast_ref::<Grip>();
        let over = cursor
            .position()
            .and_then(|position| self.edge_at(layout, position));
        if grip.dragging.is_some() || over.is_some() {
            return mouse::Interaction::ResizingHorizontally;
        }
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<Resizer<'a, Message>> for Element<'a, Message> {
    fn from(resizer: Resizer<'a, Message>) -> Self {
        Element::new(resizer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLEX: Edge = Edge {
        resizable: true,
        flexes: true,
    };
    const FIXED: Edge = Edge {
        resizable: true,
        flexes: false,
    };

    fn cells(widths: &[f32]) -> Vec<Rectangle> {
        let mut x = 0.0;
        widths
            .iter()
            .map(|&width| {
                let cell = Rectangle {
                    x,
                    y: 0.0,
                    width,
                    height: 40.0,
                };
                x += width;
                cell
            })
            .collect()
    }

    #[test]
    fn a_press_near_a_right_edge_finds_its_column() {
        let cells = cells(&[100.0, 200.0]);
        let edges = [FLEX, FLEX];
        assert_eq!(edge_at(&cells, &edges, Point::new(98.0, 20.0)), Some(0));
        assert_eq!(edge_at(&cells, &edges, Point::new(104.0, 20.0)), Some(0));
        assert_eq!(edge_at(&cells, &edges, Point::new(299.0, 20.0)), Some(1));
        assert_eq!(edge_at(&cells, &edges, Point::new(50.0, 20.0)), None);
        assert_eq!(edge_at(&cells, &edges, Point::new(100.0, 60.0)), None);
    }

    #[test]
    fn a_column_that_is_not_resizable_has_no_edge() {
        let cells = cells(&[100.0, 200.0]);
        let edges = [
            Edge {
                resizable: false,
                flexes: true,
            },
            FLEX,
        ];
        assert_eq!(edge_at(&cells, &edges, Point::new(100.0, 20.0)), None);
    }

    #[test]
    fn a_column_grows_into_the_flexing_columns_after_it() {
        let cells = cells(&[100.0, 200.0, 80.0, 120.0]);
        let edges = [FLEX, FLEX, FIXED, FLEX];
        // 200 - 40 from the second and 120 - 40 from the fourth.
        assert_eq!(widest(&cells, &edges, 0, 0.0, 40.0), 340.0);
        assert_eq!(widest(&cells, &edges, 1, 0.0, 40.0), 280.0);
        assert_eq!(widest(&cells, &edges, 3, 0.0, 40.0), 120.0);
    }

    #[test]
    fn a_column_grows_into_unused_room() {
        let cells = cells(&[100.0, 100.0]);
        assert_eq!(widest(&cells, &[FIXED, FIXED], 1, 100.0, 40.0), 200.0);
    }

    #[test]
    fn a_drag_keeps_the_columns_before_it_and_clamps_the_dragged_one() {
        let dragging = Dragging {
            index: 1,
            origin: 300.0,
            widths: vec![100.0, 200.0],
            widest: 260.0,
        };
        assert_eq!(dragging.widths_at(320.0, 40.0), vec![100.0, 220.0]);
        assert_eq!(dragging.widths_at(900.0, 40.0), vec![100.0, 260.0]);
        assert_eq!(dragging.widths_at(0.0, 40.0), vec![100.0, 40.0]);
    }
}
