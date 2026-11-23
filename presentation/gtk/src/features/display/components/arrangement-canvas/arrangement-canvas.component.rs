//! The interactive layout canvas. It is built once per page and kept in sync with
//! `set_props`, so tiles glide between arrangements instead of being rebuilt.
//!
//! One drag gesture lives on the stage (`gtk::Fixed`), which never moves, and finds the
//! tile under the pointer itself; a gesture on the tile would report offsets relative to
//! a widget that is being moved. Nothing is applied while dragging: the release reports
//! one drop to the page and the canvas waits, tiles already in place, for the new layout.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk::{gdk, glib};
use gtk4 as gtk;
use libadwaita as adw;

use super::html;
use super::props::{ArrangementCanvasProps, CanvasWidgets, TileProps, TileWidgets};
use crate::features::display::constants::canvas::{
    BADGE_MIN_HEIGHT, BADGE_MIN_WIDTH, CLICK_SLOP, KEY_STEP, KEY_STEP_LARGE, LABEL_MIN_HEIGHT,
    RESOLUTION_MIN_HEIGHT, RESOLUTION_MIN_WIDTH, SNAP_PX, TILE_ANIMATION_MS,
};
use crate::features::display::utils::canvas_geometry::{self, Fit};
use crate::features::display::utils::coordinates;

/// `(x, y, width, height)` in canvas pixels.
type Screen = (f64, f64, f64, f64);
type Preview = Vec<(String, i32, i32)>;

struct Drag {
    connector: String,
    origin: (i32, i32),
    size: (i32, i32),
    /// Where the press landed and how far that is from the tile's top-left corner.
    press: (f64, f64),
    grab: (f64, f64),
    /// Raw layout position under the pointer (or reached by key nudges).
    wanted: (i32, i32),
    /// The layout the drop would produce, from the same rules the apply path uses.
    preview: Option<Preview>,
    /// Disabled displays can be selected but not moved.
    movable: bool,
    moved: bool,
    keyboard: bool,
}

#[derive(Default)]
struct State {
    fit: Option<Fit>,
    size: (i32, i32),
    drag: Option<Drag>,
    /// A drop was reported and the page has not sent the resulting layout yet.
    pending: bool,
    /// Where each tile is drawn right now, the start of the next animation.
    shown: HashMap<String, Screen>,
    animation: Option<adw::TimedAnimation>,
}

struct Inner {
    this: Weak<Inner>,
    widgets: CanvasWidgets,
    tiles: RefCell<Vec<TileWidgets>>,
    props: RefCell<Option<ArrangementCanvasProps>>,
    state: RefCell<State>,
    /// Told when an interaction ends without a drop, so deferred updates can land.
    on_settle: Rc<dyn Fn()>,
}

pub struct ArrangementCanvas {
    inner: Rc<Inner>,
}

impl ArrangementCanvas {
    pub fn new(on_settle: Rc<dyn Fn()>) -> Self {
        let inner = Rc::new_cyclic(|this: &Weak<Inner>| Inner {
            this: this.clone(),
            widgets: html::build(),
            tiles: RefCell::default(),
            props: RefCell::default(),
            state: RefCell::default(),
            on_settle,
        });
        inner.install();
        Self { inner }
    }

    pub fn widget(&self) -> gtk::Widget {
        self.inner.widgets.root.clone().upcast()
    }

    /// True while a tile is being dragged or nudged; updates should wait until it ends.
    pub fn is_dragging(&self) -> bool {
        self.inner.state.borrow().drag.is_some()
    }

    pub fn set_props(&self, props: &ArrangementCanvasProps) {
        self.inner.set_props(props);
    }
}

fn names(props: Option<&ArrangementCanvasProps>) -> Vec<String> {
    let mut names: Vec<String> = props
        .map(|p| p.tiles.iter().map(|t| t.connector.clone()).collect())
        .unwrap_or_default();
    names.sort();
    names
}

fn layout_rect(tile: &TileProps) -> canvas_geometry::Rect {
    (tile.x, tile.y, tile.width, tile.height)
}

impl Inner {
    fn install(&self) {
        let weak = self.this.clone();
        self.widgets
            .backdrop
            .connect_resize(move |_, width, height| {
                if let Some(inner) = weak.upgrade() {
                    inner.resized(width, height);
                }
            });

        let drag = gtk::GestureDrag::builder()
            .button(gdk::BUTTON_PRIMARY)
            .build();
        let weak = self.this.clone();
        drag.connect_drag_begin(move |gesture, x, y| {
            if let Some(inner) = weak.upgrade() {
                inner.press(gesture, x, y);
            }
        });
        let weak = self.this.clone();
        drag.connect_drag_update(move |_, dx, dy| {
            if let Some(inner) = weak.upgrade() {
                inner.drag_by(dx, dy);
            }
        });
        let weak = self.this.clone();
        drag.connect_drag_end(move |_, _, _| {
            if let Some(inner) = weak.upgrade() {
                inner.finish(true);
            }
        });
        let weak = self.this.clone();
        drag.connect_cancel(move |_, _| {
            if let Some(inner) = weak.upgrade() {
                inner.finish(false);
            }
        });
        self.widgets.fixed.add_controller(drag);
    }

    fn props(&self) -> Option<ArrangementCanvasProps> {
        self.props.borrow().clone()
    }

    fn tile_props(&self, connector: &str) -> Option<TileProps> {
        self.props
            .borrow()
            .as_ref()?
            .tiles
            .iter()
            .find(|t| t.connector == connector)
            .cloned()
    }

    fn tile_root(&self, connector: &str) -> Option<gtk::Box> {
        self.tiles
            .borrow()
            .iter()
            .find(|t| t.connector == connector)
            .map(|t| t.root.clone())
    }

    fn set_props(&self, props: &ArrangementCanvasProps) {
        let same_set = names(self.props.borrow().as_ref()) == names(Some(props));
        *self.props.borrow_mut() = Some(props.clone());
        self.reconcile(props);
        if self.state.borrow().drag.is_some() {
            return;
        }
        let kept = {
            let mut state = self.state.borrow_mut();
            state.pending = false;
            same_set.then_some(state.fit).flatten().map(|f| f.scale)
        };
        self.widgets.root.set_cursor(None);
        self.hide_preview();
        self.relayout(kept, kept.is_some());
    }

    /// Adds, updates and removes tile widgets to match `props`; never rebuilds a tile.
    fn reconcile(&self, props: &ArrangementCanvasProps) {
        let fixed = &self.widgets.fixed;
        let mut added = Vec::new();
        {
            let mut tiles = self.tiles.borrow_mut();
            tiles.retain(|t| {
                let keep = props.tiles.iter().any(|p| p.connector == t.connector);
                if !keep {
                    fixed.remove(&t.root);
                }
                keep
            });
            for p in &props.tiles {
                match tiles.iter().find(|t| t.connector == p.connector) {
                    Some(widgets) => html::update_tile(widgets, p),
                    None => {
                        let widgets = html::tile(p);
                        fixed.put(&widgets.root, 0.0, 0.0);
                        added.push((widgets.connector.clone(), widgets.root.clone()));
                        tiles.push(widgets);
                    }
                }
            }
        }
        for (connector, root) in added {
            self.attach_keys(&connector, &root);
        }
        self.state
            .borrow_mut()
            .shown
            .retain(|name, _| props.tiles.iter().any(|p| &p.connector == name));
        // The selected tile is drawn above the others, the ghost above everything.
        if let Some(selected) = props.tiles.iter().find(|t| t.selected) {
            self.raise(&selected.connector);
        }
        self.widgets.ghost.insert_before(fixed, gtk::Widget::NONE);
    }

    fn raise(&self, connector: &str) {
        if let Some(root) = self.tile_root(connector) {
            root.insert_before(&self.widgets.fixed, gtk::Widget::NONE);
        }
    }

    fn resized(&self, width: i32, height: i32) {
        self.state.borrow_mut().size = (width, height);
        if self.state.borrow().drag.is_none() {
            self.relayout(None, false);
        }
    }

    /// Fits the props layout to the canvas (keeping `scale` while it still fits) and moves
    /// every tile there, gliding when `animate`.
    fn relayout(&self, scale: Option<f64>, animate: bool) {
        let Some(props) = self.props() else { return };
        let (width, height) = self.state.borrow().size;
        let rects: Vec<_> = props.tiles.iter().map(layout_rect).collect();
        let fit = canvas_geometry::fit_keeping(&rects, f64::from(width), f64::from(height), scale);
        self.state.borrow_mut().fit = Some(fit);
        let targets = props
            .tiles
            .iter()
            .map(|t| {
                (
                    t.connector.clone(),
                    canvas_geometry::to_screen(&fit, layout_rect(t)),
                )
            })
            .collect();
        if animate {
            self.animate_to(targets);
        } else {
            self.stop_animation();
            for (name, screen) in targets {
                self.place(&name, screen);
            }
        }
    }

    fn stop_animation(&self) {
        let running = self.state.borrow_mut().animation.take();
        if let Some(animation) = running {
            animation.pause();
        }
    }

    fn animate_to(&self, targets: Vec<(String, Screen)>) {
        self.stop_animation();
        let from = self.state.borrow().shown.clone();
        let weak = self.this.clone();
        let target = adw::CallbackAnimationTarget::new(move |t| {
            let Some(inner) = weak.upgrade() else { return };
            for (name, to) in &targets {
                let start = from.get(name).copied().unwrap_or(*to);
                inner.place(name, canvas_geometry::lerp(start, *to, t));
            }
        });
        let animation = adw::TimedAnimation::builder()
            .widget(&self.widgets.fixed)
            .value_from(0.0)
            .value_to(1.0)
            .duration(TILE_ANIMATION_MS)
            .easing(adw::Easing::EaseOutCubic)
            .target(&target)
            .build();
        self.state.borrow_mut().animation = Some(animation.clone());
        animation.play();
    }

    /// Draws one tile at a canvas rectangle. Edges are rounded on both sides so tiles that
    /// touch in the layout touch on screen, and text that would not fit is hidden.
    fn place(&self, connector: &str, screen: Screen) {
        let (x0, y0) = (screen.0.round(), screen.1.round());
        let (x1, y1) = ((screen.0 + screen.2).round(), (screen.1 + screen.3).round());
        let (w, h) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
        let focused = self.tile_props(connector).is_some_and(|t| t.focused);
        {
            let tiles = self.tiles.borrow();
            let Some(tile) = tiles.iter().find(|t| t.connector == connector) else {
                return;
            };
            tile.label.set_visible(h >= LABEL_MIN_HEIGHT);
            tile.resolution
                .set_visible(h >= RESOLUTION_MIN_HEIGHT && w >= RESOLUTION_MIN_WIDTH);
            tile.badge
                .set_visible(focused && h >= BADGE_MIN_HEIGHT && w >= BADGE_MIN_WIDTH);
            tile.root.set_size_request(w as i32, h as i32);
            self.widgets.fixed.move_(&tile.root, x0, y0);
        }
        self.state
            .borrow_mut()
            .shown
            .insert(connector.to_owned(), screen);
    }

    /// Topmost first: the order hit testing should try the tiles in.
    fn hit_order(&self) -> Vec<(String, Screen)> {
        let Some(props) = self.props() else {
            return Vec::new();
        };
        let state = self.state.borrow();
        let mut order: Vec<&TileProps> = props.tiles.iter().filter(|t| t.selected).collect();
        order.extend(props.tiles.iter().rev().filter(|t| !t.selected));
        order
            .into_iter()
            .filter_map(|t| {
                state
                    .shown
                    .get(&t.connector)
                    .map(|s| (t.connector.clone(), *s))
            })
            .collect()
    }

    fn press(&self, gesture: &gtk::GestureDrag, x: f64, y: f64) {
        let busy = {
            let state = self.state.borrow();
            state.pending || state.drag.is_some() || state.fit.is_none()
        };
        let order = self.hit_order();
        let rects: Vec<Screen> = order.iter().map(|(_, s)| *s).collect();
        let hit = canvas_geometry::hit(&rects, (x, y)).map(|i| order[i].clone());
        let (Some((connector, screen)), false) = (hit, busy) else {
            gesture.set_state(gtk::EventSequenceState::Denied);
            return;
        };
        let Some(tile) = self.tile_props(&connector) else {
            return;
        };
        self.stop_animation();
        if let Some(root) = self.tile_root(&connector) {
            root.grab_focus();
        }
        self.raise(&connector);
        self.widgets
            .ghost
            .insert_before(&self.widgets.fixed, gtk::Widget::NONE);
        self.state.borrow_mut().drag = Some(Drag {
            connector,
            origin: (tile.x, tile.y),
            size: (tile.width, tile.height),
            press: (x, y),
            grab: (x - screen.0, y - screen.1),
            wanted: (tile.x, tile.y),
            preview: None,
            movable: tile.enabled,
            moved: false,
            keyboard: false,
        });
    }

    fn begin_keyboard(&self, connector: &str, moved: bool) {
        let Some(tile) = self.tile_props(connector) else {
            return;
        };
        self.raise(connector);
        self.state.borrow_mut().drag = Some(Drag {
            connector: connector.to_owned(),
            origin: (tile.x, tile.y),
            size: (tile.width, tile.height),
            press: (0.0, 0.0),
            grab: (0.0, 0.0),
            wanted: (tile.x, tile.y),
            preview: None,
            movable: tile.enabled,
            moved,
            keyboard: true,
        });
    }

    fn drag_by(&self, dx: f64, dy: f64) {
        let request = {
            let mut state = self.state.borrow_mut();
            let Some(fit) = state.fit else { return };
            let Some(drag) = state.drag.as_mut() else {
                return;
            };
            if drag.keyboard || !drag.movable {
                return;
            }
            if !drag.moved && dx.hypot(dy) < CLICK_SLOP {
                return;
            }
            drag.moved = true;
            let top_left = (
                drag.press.0 + dx - drag.grab.0,
                drag.press.1 + dy - drag.grab.1,
            );
            drag.wanted = canvas_geometry::to_logical_point(&fit, top_left);
            (
                drag.connector.clone(),
                drag.wanted,
                canvas_geometry::to_logical(&fit, SNAP_PX),
                top_left,
            )
        };
        let (connector, wanted, threshold, top_left) = request;
        self.update_preview(&connector, wanted, threshold);
        self.show_drag(Some(top_left));
    }

    fn update_preview(&self, connector: &str, wanted: (i32, i32), threshold: i32) {
        let Some(props) = self.props() else { return };
        let preview = (props.preview)(connector, wanted, threshold);
        if let Some(drag) = self.state.borrow_mut().drag.as_mut() {
            drag.preview = preview;
        }
    }

    /// Draws a drag in progress: the dragged tile under the pointer (or at its nudged
    /// position), every other tile where the drop would leave it, and a dashed ghost
    /// where the dragged one will land.
    fn show_drag(&self, pointer: Option<(f64, f64)>) {
        let Some(props) = self.props() else { return };
        let (fit, connector, size, wanted, preview) = {
            let state = self.state.borrow();
            let (Some(fit), Some(drag)) = (state.fit, state.drag.as_ref()) else {
                return;
            };
            (
                fit,
                drag.connector.clone(),
                drag.size,
                drag.wanted,
                drag.preview.clone(),
            )
        };
        let planned = |name: &str| {
            preview
                .as_ref()
                .and_then(|p| p.iter().find(|(c, ..)| c == name))
                .map(|(_, x, y)| (*x, *y))
        };

        for tile in props.tiles.iter().filter(|t| t.connector != connector) {
            let (x, y) = planned(&tile.connector).unwrap_or((tile.x, tile.y));
            self.place(
                &tile.connector,
                canvas_geometry::to_screen(&fit, (x, y, tile.width, tile.height)),
            );
        }
        let landing = planned(&connector);
        let own = match pointer {
            Some((x, y)) => (
                x,
                y,
                f64::from(size.0) * fit.scale,
                f64::from(size.1) * fit.scale,
            ),
            None => canvas_geometry::to_screen(&fit, (wanted.0, wanted.1, size.0, size.1)),
        };
        self.place(&connector, own);

        let ghost = &self.widgets.ghost;
        match landing {
            Some((x, y)) => {
                let (gx, gy, gw, gh) = canvas_geometry::to_screen(&fit, (x, y, size.0, size.1));
                ghost.set_size_request(gw.round().max(1.0) as i32, gh.round().max(1.0) as i32);
                self.widgets.fixed.move_(ghost, gx.round(), gy.round());
                ghost.set_visible(true);
            }
            None => ghost.set_visible(false),
        }

        // A moved layout is stored from the origin, so show the coordinates it will get.
        let (min_x, min_y) = preview.as_ref().map_or((0, 0), |p| {
            (
                p.iter().map(|(_, x, _)| *x).min().unwrap_or(0),
                p.iter().map(|(_, _, y)| *y).min().unwrap_or(0),
            )
        });
        let at = landing.unwrap_or(wanted);
        self.widgets.hint.set_label(&format!(
            "{connector} → {}",
            coordinates(at.0 - min_x, at.1 - min_y)
        ));
        self.widgets.hint.set_visible(true);
    }

    fn hide_preview(&self) {
        self.widgets.ghost.set_visible(false);
        self.widgets.hint.set_visible(false);
    }

    /// Ends a gesture. A press without travel selects; a drag that lands somewhere new
    /// glides into place and reports the drop; anything else glides back.
    fn finish(&self, commit: bool) {
        let Some(drag) = self.state.borrow_mut().drag.take() else {
            return;
        };
        let Some(props) = self.props() else { return };
        let kept = self.state.borrow().fit.map(|f| f.scale);

        if !drag.moved {
            self.hide_preview();
            self.relayout(kept, false);
            let (select, settle, connector) = (
                Rc::clone(&props.on_select),
                Rc::clone(&self.on_settle),
                drag.connector,
            );
            // Out of the gesture handler: the page may re-render in response.
            glib::idle_add_local_once(move || {
                select(connector);
                settle();
            });
            return;
        }

        let landing = drag
            .preview
            .as_ref()
            .and_then(|p| p.iter().find(|(c, ..)| *c == drag.connector))
            .map(|(_, x, y)| (*x, *y))
            .filter(|at| commit && *at != drag.origin);
        let Some((x, y)) = landing else {
            self.hide_preview();
            self.relayout(kept, true);
            let settle = Rc::clone(&self.on_settle);
            glib::idle_add_local_once(move || settle());
            return;
        };

        let Some(fit) = self.state.borrow().fit else {
            return;
        };
        let preview = drag.preview.unwrap_or_default();
        let targets = props
            .tiles
            .iter()
            .map(|t| {
                let (px, py) = preview
                    .iter()
                    .find(|(c, ..)| *c == t.connector)
                    .map_or((t.x, t.y), |(_, px, py)| (*px, *py));
                (
                    t.connector.clone(),
                    canvas_geometry::to_screen(&fit, (px, py, t.width, t.height)),
                )
            })
            .collect();
        self.state.borrow_mut().pending = true;
        self.widgets.ghost.set_visible(false);
        self.widgets.hint.set_label("Applying…");
        self.widgets.hint.set_visible(true);
        self.widgets.root.set_cursor_from_name(Some("progress"));
        self.animate_to(targets);

        let (on_drop, connector) = (Rc::clone(&props.on_drop), drag.connector);
        glib::idle_add_local_once(move || on_drop(connector, x, y));
    }

    fn keyboard_active(&self) -> bool {
        self.state
            .borrow()
            .drag
            .as_ref()
            .is_some_and(|d| d.keyboard)
    }

    fn nudge(&self, connector: &str, dx: i32, dy: i32, step: i32) {
        if self.state.borrow().pending {
            return;
        }
        if self.state.borrow().drag.is_none() {
            self.begin_keyboard(connector, true);
        }
        let wanted = {
            let mut state = self.state.borrow_mut();
            let Some(drag) = state.drag.as_mut() else {
                return;
            };
            if !drag.keyboard || !drag.movable {
                return;
            }
            drag.wanted = (drag.wanted.0 + dx * step, drag.wanted.1 + dy * step);
            drag.wanted
        };
        // No alignment pull for single nudges, only the rule that tiles stay flush.
        self.update_preview(connector, wanted, 0);
        self.show_drag(None);
    }

    fn attach_keys(&self, connector: &str, root: &gtk::Box) {
        let keys = gtk::EventControllerKey::new();
        let (weak, name) = (self.this.clone(), connector.to_owned());
        keys.connect_key_pressed(move |_, key, _, modifiers| {
            let Some(inner) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            let step = if modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
                KEY_STEP_LARGE
            } else {
                KEY_STEP
            };
            let direction = match key {
                gdk::Key::Left => Some((-1, 0)),
                gdk::Key::Right => Some((1, 0)),
                gdk::Key::Up => Some((0, -1)),
                gdk::Key::Down => Some((0, 1)),
                _ => None,
            };
            if let Some((dx, dy)) = direction {
                inner.nudge(&name, dx, dy, step);
                return glib::Propagation::Stop;
            }
            match key {
                gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::space => {
                    if inner.state.borrow().pending {
                        return glib::Propagation::Stop;
                    }
                    if !inner.keyboard_active() {
                        inner.begin_keyboard(&name, false);
                    }
                    inner.finish(true);
                    glib::Propagation::Stop
                }
                gdk::Key::Escape if inner.keyboard_active() => {
                    inner.finish(false);
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        root.add_controller(keys);

        // Moving focus elsewhere abandons a half-done keyboard move.
        let focus = gtk::EventControllerFocus::new();
        let weak = self.this.clone();
        focus.connect_leave(move |_| {
            if let Some(inner) = weak.upgrade()
                && inner.keyboard_active()
            {
                inner.finish(false);
            }
        });
        root.add_controller(focus);
    }
}
