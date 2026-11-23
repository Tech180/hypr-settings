//! Monitor arrangement rules in the compositor's logical pixel space.
//!
//! Hyprland (and wlroots compositors in general) place every enabled output as a
//! rectangle: its top-left corner is the position, its size is the mode divided by
//! the scale with axes swapped for quarter turns. Enabled rectangles must not
//! overlap, and pointers only cross between rectangles that share an edge. The
//! functions here keep a layout legal while it is edited.

use std::collections::VecDeque;

use crate::arrangement::constants::{MAX_RECONNECT_PASSES, MIN_SHARED_EDGE};
use crate::arrangement::interfaces::{Layout, Rect, Tile};
use crate::output::Topology;
use crate::value::{ConnectorName, Position};

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn at(position: Position, size: (i32, i32)) -> Self {
        Self::new(position.x, position.y, size.0, size.1)
    }

    pub fn right(self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(self) -> i32 {
        self.y + self.h
    }

    pub fn translated(self, dx: i32, dy: i32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.w, self.h)
    }

    pub fn position(self) -> Position {
        Position::new(self.x, self.y)
    }

    pub fn size(self) -> (i32, i32) {
        (self.w, self.h)
    }

    /// Strict overlap: touching edges are fine.
    pub fn overlaps(self, other: Self) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    fn shared_x(self, other: Self) -> i32 {
        self.right().min(other.right()) - self.x.max(other.x)
    }

    fn shared_y(self, other: Self) -> i32 {
        self.bottom().min(other.bottom()) - self.y.max(other.y)
    }

    /// True when the rectangles touch along an edge of positive length, which is
    /// what lets the pointer cross from one to the other.
    pub fn adjacent(self, other: Self) -> bool {
        let side = (self.right() == other.x || other.right() == self.x) && self.shared_y(other) > 0;
        let stacked =
            (self.bottom() == other.y || other.bottom() == self.y) && self.shared_x(other) > 0;
        side || stacked
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

impl Layout {
    pub fn new(tiles: Vec<Tile>) -> Self {
        Self { tiles }
    }

    /// Enabled outputs that have a mode. Disabled outputs are not part of the layout.
    pub fn from_topology(topology: &Topology) -> Self {
        let tiles = topology
            .outputs()
            .iter()
            .filter(|o| o.enabled)
            .filter_map(|o| {
                let size = o.logical_size()?;
                Some(Tile {
                    connector: o.connector.clone(),
                    rect: Rect::at(o.position, size),
                })
            })
            .collect();
        Self { tiles }
    }

    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    pub fn get(&self, connector: &ConnectorName) -> Option<Rect> {
        self.tiles
            .iter()
            .find(|t| &t.connector == connector)
            .map(|t| t.rect)
    }

    pub fn set(&mut self, connector: &ConnectorName, rect: Rect) {
        match self.tiles.iter_mut().find(|t| &t.connector == connector) {
            Some(tile) => tile.rect = rect,
            None => self.tiles.push(Tile {
                connector: connector.clone(),
                rect,
            }),
        }
    }

    pub fn remove(&mut self, connector: &ConnectorName) {
        self.tiles.retain(|t| &t.connector != connector);
    }

    pub fn bounds(&self) -> Option<Rect> {
        let first = self.tiles.first()?.rect;
        let (mut left, mut top, mut right, mut bottom) =
            (first.x, first.y, first.right(), first.bottom());
        for t in &self.tiles[1..] {
            left = left.min(t.rect.x);
            top = top.min(t.rect.y);
            right = right.max(t.rect.right());
            bottom = bottom.max(t.rect.bottom());
        }
        Some(Rect::new(left, top, right - left, bottom - top))
    }

    /// The first pair of overlapping tiles, in layout order.
    pub fn overlaps(&self) -> Vec<(ConnectorName, ConnectorName)> {
        let mut found = Vec::new();
        for (i, a) in self.tiles.iter().enumerate() {
            for b in &self.tiles[i + 1..] {
                if a.rect.overlaps(b.rect) {
                    found.push((a.connector.clone(), b.connector.clone()));
                }
            }
        }
        found
    }

    /// Every tile can be reached from every other through shared edges.
    pub fn is_connected(&self) -> bool {
        self.groups().len() <= 1
    }

    /// Tile indices grouped by which tiles the pointer can travel between.
    fn groups(&self) -> Vec<Vec<usize>> {
        let mut seen = vec![false; self.tiles.len()];
        let mut groups = Vec::new();
        for start in 0..self.tiles.len() {
            if seen[start] {
                continue;
            }
            seen[start] = true;
            let mut group = vec![start];
            let mut queue = VecDeque::from([start]);
            while let Some(i) = queue.pop_front() {
                for (j, other) in self.tiles.iter().enumerate() {
                    if !seen[j] && self.tiles[i].rect.adjacent(other.rect) {
                        seen[j] = true;
                        group.push(j);
                        queue.push_back(j);
                    }
                }
            }
            groups.push(group);
        }
        groups
    }

    /// Slides tiles that lost contact with the rest back against it, so moving the
    /// monitor that bridged two others does not strand one of them. The group holding
    /// `anchor` (the monitor the user just placed) stays put; every other group moves
    /// by the shortest translation that makes it flush without overlapping anything.
    pub fn reconnect(&mut self, anchor: &ConnectorName) {
        for _ in 0..MAX_RECONNECT_PASSES {
            let groups = self.groups();
            if groups.len() <= 1 {
                return;
            }
            let main = groups
                .iter()
                .position(|g| g.iter().any(|&i| &self.tiles[i].connector == anchor))
                .unwrap_or(0);
            let mut best: Option<(i64, usize, (i32, i32))> = None;
            for (g, members) in groups.iter().enumerate() {
                if g == main {
                    continue;
                }
                let blockers: Vec<Rect> = (0..self.tiles.len())
                    .filter(|i| !members.contains(i))
                    .map(|i| self.tiles[i].rect)
                    .collect();
                for &a in members {
                    for &m in &groups[main] {
                        for (dx, dy) in flush_translations(self.tiles[a].rect, self.tiles[m].rect) {
                            let fits = members.iter().all(|&i| {
                                let moved = self.tiles[i].rect.translated(dx, dy);
                                blockers.iter().all(|b| !b.overlaps(moved))
                            });
                            let cost = i64::from(dx).pow(2) + i64::from(dy).pow(2);
                            if fits && best.is_none_or(|(c, ..)| cost < c) {
                                best = Some((cost, g, (dx, dy)));
                            }
                        }
                    }
                }
            }
            let Some((_, g, (dx, dy))) = best else {
                return;
            };
            for &i in &groups[g] {
                self.tiles[i].rect = self.tiles[i].rect.translated(dx, dy);
            }
        }
    }

    /// Shifts everything so the top-left of the union sits at the origin.
    pub fn normalise_origin(&mut self) -> (i32, i32) {
        let Some(bounds) = self.bounds() else {
            return (0, 0);
        };
        for tile in &mut self.tiles {
            tile.rect.x -= bounds.x;
            tile.rect.y -= bounds.y;
        }
        (-bounds.x, -bounds.y)
    }

    fn others<'a>(&'a self, moving: &'a ConnectorName) -> impl Iterator<Item = Rect> + 'a {
        self.tiles
            .iter()
            .filter(move |t| &t.connector != moving)
            .map(|t| t.rect)
    }

    /// A position is acceptable when it overlaps nobody and touches a neighbour
    /// (or the monitor is alone).
    pub fn accepts(&self, moving: &ConnectorName, size: (i32, i32), at: Position) -> bool {
        let rect = Rect::at(at, size);
        let others: Vec<Rect> = self.others(moving).collect();
        others.iter().all(|o| !o.overlaps(rect))
            && (others.is_empty() || others.iter().any(|o| o.adjacent(rect)))
    }

    /// Turns a raw drag position into the nearest legal one: flush against a
    /// neighbour's edge, never overlapping, and snapped to aligned edges or
    /// centres within `threshold` logical pixels. With one neighbour that makes
    /// the monitor always stick to it, like GNOME.
    pub fn resolve_drop(
        &self,
        moving: &ConnectorName,
        size: (i32, i32),
        candidate: Position,
        threshold: i32,
    ) -> Position {
        let (w, h) = size;
        let others: Vec<Rect> = self.others(moving).collect();
        if others.is_empty() {
            return candidate;
        }

        // Alignment anchors from every other monitor, not only the one being touched,
        // so a tile can line up with a display across the layout.
        let x_anchors: Vec<i32> = others
            .iter()
            .flat_map(|o| [o.x, o.right() - w, o.x + (o.w - w) / 2])
            .collect();
        let y_anchors: Vec<i32> = others
            .iter()
            .flat_map(|o| [o.y, o.bottom() - h, o.y + (o.h - h) / 2])
            .collect();

        // `preferred` holds the side the tile's centre points at; `fallback` every
        // other side, used only when no preferred side is free.
        let mut preferred: Option<(i64, Position)> = None;
        let mut fallback: Option<(i64, Position)> = None;
        let centre = (
            f64::from(candidate.x) + f64::from(w) / 2.0,
            f64::from(candidate.y) + f64::from(h) / 2.0,
        );

        for o in &others {
            let shared_y = MIN_SHARED_EDGE.min(h).min(o.h);
            let (y_lo, y_hi) = (o.y - h + shared_y, o.bottom() - shared_y);
            let y = snap_axis(
                candidate.y.clamp(y_lo, y_hi),
                &y_anchors,
                threshold,
                (y_lo, y_hi),
            );
            let shared_x = MIN_SHARED_EDGE.min(w).min(o.w);
            let (x_lo, x_hi) = (o.x - w + shared_x, o.right() - shared_x);
            let x = snap_axis(
                candidate.x.clamp(x_lo, x_hi),
                &x_anchors,
                threshold,
                (x_lo, x_hi),
            );

            // Which way the tile leans from this neighbour, scaled by both sizes so
            // the boundary is the diagonal of their combined box.
            let lean_x = (centre.0 - (f64::from(o.x) + f64::from(o.w) / 2.0))
                / (f64::from(w + o.w) / 2.0).max(1.0);
            let lean_y = (centre.1 - (f64::from(o.y) + f64::from(o.h) / 2.0))
                / (f64::from(h + o.h) / 2.0).max(1.0);
            let sides = [
                (
                    Position::new(o.right(), y),
                    lean_x.abs() >= lean_y.abs() && lean_x >= 0.0,
                ),
                (
                    Position::new(o.x - w, y),
                    lean_x.abs() >= lean_y.abs() && lean_x < 0.0,
                ),
                (
                    Position::new(x, o.bottom()),
                    lean_y.abs() > lean_x.abs() && lean_y >= 0.0,
                ),
                (
                    Position::new(x, o.y - h),
                    lean_y.abs() > lean_x.abs() && lean_y < 0.0,
                ),
            ];
            for (p, is_preferred) in sides {
                let rect = Rect::at(p, size);
                if others.iter().any(|other| other.overlaps(rect)) {
                    continue;
                }
                let dx = i64::from(p.x - candidate.x);
                let dy = i64::from(p.y - candidate.y);
                let distance = dx * dx + dy * dy;
                let slot = if is_preferred {
                    &mut preferred
                } else {
                    &mut fallback
                };
                if slot.is_none_or(|(d, _)| distance < d) {
                    *slot = Some((distance, p));
                }
            }
        }

        preferred.or(fallback).map_or(candidate, |(_, p)| p)
    }

    /// Position to store for a drop: kept as is when already legal, otherwise
    /// the nearest legal one.
    pub fn validated_drop(
        &self,
        moving: &ConnectorName,
        size: (i32, i32),
        requested: Position,
    ) -> Position {
        if self.accepts(moving, size, requested) {
            requested
        } else {
            self.resolve_drop(moving, size, requested, 0)
        }
    }

    /// After `changed` takes a new logical size, tiles touching its right or bottom
    /// edge follow that edge (and tiles touching those follow in turn), so a
    /// connected layout stays connected. Leftover overlaps are pushed apart.
    pub fn reflow_resize(&mut self, changed: &ConnectorName, new_size: (i32, i32)) {
        let Some(old) = self.get(changed) else {
            return;
        };
        let (dw, dh) = (new_size.0 - old.w, new_size.1 - old.h);
        let before = self.tiles.clone();
        let Some(origin) = before.iter().position(|t| &t.connector == changed) else {
            return;
        };

        for (axis, delta) in [(Axis::X, dw), (Axis::Y, dh)] {
            if delta == 0 {
                continue;
            }
            for index in followers(&before, origin, axis) {
                match axis {
                    Axis::X => self.tiles[index].rect.x += delta,
                    Axis::Y => self.tiles[index].rect.y += delta,
                }
            }
        }
        self.tiles[origin].rect.w = new_size.0;
        self.tiles[origin].rect.h = new_size.1;
        self.separate(changed);
    }

    /// Pushes every tile that still overlaps `anchor` out along the shortest way.
    fn separate(&mut self, anchor: &ConnectorName) {
        for _ in 0..self.tiles.len().saturating_mul(4) {
            let Some(anchor_rect) = self.get(anchor) else {
                return;
            };
            let Some(index) = self
                .tiles
                .iter()
                .position(|t| &t.connector != anchor && t.rect.overlaps(anchor_rect))
            else {
                return;
            };
            let r = self.tiles[index].rect;
            let options = [
                (anchor_rect.right() - r.x, Axis::X, anchor_rect.right()),
                (r.right() - anchor_rect.x, Axis::X, anchor_rect.x - r.w),
                (anchor_rect.bottom() - r.y, Axis::Y, anchor_rect.bottom()),
                (r.bottom() - anchor_rect.y, Axis::Y, anchor_rect.y - r.h),
            ];
            if let Some(&(_, axis, to)) = options.iter().min_by_key(|o| o.0) {
                match axis {
                    Axis::X => self.tiles[index].rect.x = to,
                    Axis::Y => self.tiles[index].rect.y = to,
                }
            }
        }
    }

    /// Where a monitor with no usable saved position goes: its preferred position
    /// when that is legal, otherwise right of the layout with tops aligned.
    pub fn place_new(&self, size: (i32, i32), prefer: Option<Position>) -> Position {
        let Some(bounds) = self.bounds() else {
            return prefer.unwrap_or(Position::new(0, 0));
        };
        if let Some(prefer) = prefer {
            let rect = Rect::at(prefer, size);
            let legal = self.tiles.iter().all(|t| !t.rect.overlaps(rect))
                && self.tiles.iter().any(|t| t.rect.adjacent(rect));
            if legal {
                return prefer;
            }
        }
        Position::new(bounds.right(), bounds.y)
    }
}

/// Indices of tiles that touch the `axis` far edge of `origin`, transitively.
fn followers(tiles: &[Tile], origin: usize, axis: Axis) -> Vec<usize> {
    let touches = |a: Rect, b: Rect| match axis {
        Axis::X => b.x == a.right() && a.shared_y(b) > 0,
        Axis::Y => b.y == a.bottom() && a.shared_x(b) > 0,
    };
    let mut seen = vec![false; tiles.len()];
    seen[origin] = true;
    let mut queue = VecDeque::from([origin]);
    let mut moved = Vec::new();
    while let Some(i) = queue.pop_front() {
        for (j, other) in tiles.iter().enumerate() {
            if !seen[j] && touches(tiles[i].rect, other.rect) {
                seen[j] = true;
                moved.push(j);
                queue.push_back(j);
            }
        }
    }
    moved
}

/// Snaps `value` to the closest anchor inside `range` that is within `threshold`.
fn snap_axis(value: i32, anchors: &[i32], threshold: i32, range: (i32, i32)) -> i32 {
    anchors
        .iter()
        .copied()
        .filter(|a| (range.0..=range.1).contains(a))
        .map(|a| (a.abs_diff(value), a))
        .filter(|(d, _)| *d <= threshold.unsigned_abs())
        .min_by_key(|(d, _)| *d)
        .map_or(value, |(_, a)| a)
}

/// The four shortest moves that put `a` flush against a side of `m` with enough
/// shared edge for the pointer to cross.
fn flush_translations(a: Rect, m: Rect) -> [(i32, i32); 4] {
    let shared_y = MIN_SHARED_EDGE.min(a.h).min(m.h);
    let dy = a.y.clamp(m.y - a.h + shared_y, m.bottom() - shared_y) - a.y;
    let shared_x = MIN_SHARED_EDGE.min(a.w).min(m.w);
    let dx = a.x.clamp(m.x - a.w + shared_x, m.right() - shared_x) - a.x;
    [
        (m.right() - a.x, dy),
        (m.x - a.right(), dy),
        (dx, m.bottom() - a.y),
        (dx, m.y - a.bottom()),
    ]
}
