//! Resizing a side panel by dragging its edge.
//!
//! A view that owns a resizable panel keeps a [`PanelResize`], passes the
//! panel's root element through [`PanelResize::attach`], which follows the
//! drag, and adds [`PanelResize::handle`] (GPUI Kit's resize handle, on the
//! panel's inner edge) as the root's last child. While
//! the edge is dragged the panel draws [`PanelResize::width`], the dragged
//! width (clamped); the owner hears the final width once, when the mouse
//! button comes up, and saves it. The chat panel uses it; the sidebar can
//! take the same thing for its trailing edge.

use gpui::{
    px, AppContext as _, Axis, Bounds, Context, DragMoveEvent, Empty, InteractiveElement as _,
    IntoElement, MouseButton, MouseUpEvent, Pixels, Render, Stateful, Styled as _, Window,
};
use gpui_kit::component::resizable::{resize_handle, HandleEdge};

/// The payload of a dragged panel edge. Every panel's handle drags the same
/// type, so it carries the id of the panel it belongs to.
#[derive(Clone, Debug)]
pub struct EdgeDrag {
    id: &'static str,
}

impl Render for EdgeDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // Nothing follows the pointer: the panel itself resizes.
        Empty
    }
}

/// A panel whose width follows a drag of one of its edges.
#[derive(Clone, Debug)]
pub struct PanelResize {
    /// Tells this panel's drags from other panels'; also the handle's id.
    id: &'static str,
    /// The edge of the panel the handle hugs: `Leading` (left) for a panel on
    /// the right of the window, `Trailing` (right) for one on the left.
    edge: HandleEdge,
    min: f32,
    max: f32,
    /// The width while the edge is dragged.
    dragged: Option<f32>,
}

impl PanelResize {
    /// A panel `min`..=`max` pixels wide whose `edge` can be dragged.
    pub const fn new(id: &'static str, edge: HandleEdge, min: f32, max: f32) -> Self {
        Self {
            id,
            edge,
            min,
            max,
            dragged: None,
        }
    }

    /// The width to draw: the dragged one while a drag runs, else `saved`,
    /// within the panel's limits.
    pub fn width(&self, saved: f32) -> f32 {
        self.dragged.unwrap_or(saved).clamp(self.min, self.max)
    }

    /// Whether the edge is being dragged.
    #[cfg(test)]
    pub fn is_dragging(&self) -> bool {
        self.dragged.is_some()
    }

    /// The pointer is at `x` with the panel at `bounds`. Returns whether the
    /// width changed.
    pub fn drag_to(&mut self, bounds: Bounds<Pixels>, x: Pixels) -> bool {
        let width = width_at(self.edge, bounds, x).clamp(self.min, self.max);
        let changed = self.dragged != Some(width);
        self.dragged = Some(width);
        changed
    }

    /// The drag ended: the width it left, if the edge was dragged.
    pub fn finish(&mut self) -> Option<f32> {
        self.dragged.take()
    }

    /// Make `panel` (the panel's root element) follow a drag of its
    /// [`Self::handle`], which the caller adds as its last child. `state`
    /// finds this `PanelResize` in the view; `resized` is called once per
    /// drag, with the final width.
    pub fn attach<V: 'static>(
        &self,
        panel: Stateful<gpui::Div>,
        state: fn(&mut V) -> &mut PanelResize,
        resized: fn(&mut V, f32, &mut Context<V>),
        cx: &mut Context<V>,
    ) -> Stateful<gpui::Div> {
        let id = self.id;
        // A mouse-up anywhere ends the drag: over the panel, or outside it
        // (the pointer is rarely still on the narrow handle).
        let finish = move |this: &mut V, cx: &mut Context<V>| {
            if let Some(width) = state(this).finish() {
                resized(this, width, cx);
                cx.notify();
            }
        };
        panel
            .relative()
            .on_drag_move(
                cx.listener(move |this, event: &DragMoveEvent<EdgeDrag>, _, cx| {
                    if event.drag(cx).id == id
                        && state(this).drag_to(event.bounds, event.event.position.x)
                    {
                        cx.notify();
                    }
                }),
            )
            .capture_any_mouse_up(
                cx.listener(move |this, _: &MouseUpEvent, _, cx| finish(this, cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseUpEvent, _, cx| finish(this, cx)),
            )
    }

    /// The handle on the panel's edge. Add it as the panel's last child, so
    /// it is on top of the panel's content.
    pub fn handle(&self) -> impl IntoElement {
        let id = self.id;
        resize_handle::<EdgeDrag, EdgeDrag>(id, Axis::Horizontal)
            .inside(self.edge)
            .on_drag(EdgeDrag { id }, |drag, _, _, cx| cx.new(|_| drag.clone()))
    }
}

/// The width a panel at `bounds` gets with its `edge` dragged to `x`: the
/// far edge stays where it is.
fn width_at(edge: HandleEdge, bounds: Bounds<Pixels>, x: Pixels) -> f32 {
    let width = match edge {
        HandleEdge::Leading => bounds.right() - x,
        HandleEdge::Trailing => x - bounds.left(),
    };
    f32::from(width.max(px(0.)))
}

#[cfg(test)]
mod tests {
    use gpui::{point, size};

    use super::*;

    fn bounds() -> Bounds<Pixels> {
        Bounds::new(point(px(700.), px(0.)), size(px(300.), px(500.)))
    }

    #[test]
    fn the_far_edge_stays_put() {
        assert_eq!(width_at(HandleEdge::Leading, bounds(), px(600.)), 400.);
        assert_eq!(width_at(HandleEdge::Trailing, bounds(), px(900.)), 200.);
        assert_eq!(width_at(HandleEdge::Leading, bounds(), px(1200.)), 0.);
    }

    #[test]
    fn a_drag_is_clamped_and_ends_once() {
        let mut resize = PanelResize::new("edge", HandleEdge::Leading, 240., 640.);
        assert_eq!(resize.width(320.), 320.);
        assert_eq!(resize.width(10_000.), 640., "a saved width is clamped too");

        assert!(resize.drag_to(bounds(), px(900.)));
        assert_eq!(resize.width(320.), 240.);
        assert!(!resize.drag_to(bounds(), px(950.)), "still the minimum");
        assert!(resize.drag_to(bounds(), px(100.)));
        assert_eq!(resize.width(320.), 640.);
        assert!(resize.drag_to(bounds(), px(600.)));

        assert_eq!(resize.finish(), Some(400.));
        assert_eq!(resize.finish(), None, "a mouse-up without a drag");
        assert_eq!(resize.width(320.), 320.);
    }
}
