use crate::arrangement::{Layout, Rect};
use crate::error::LayoutError;
use crate::layout::interfaces::{
    LayoutPatch, LayoutPolicy, LayoutWarning, ModeRequest, ResolvedPatch,
};
use crate::output::{Topology, logical_size};
use crate::value::{ConnectorName, Position, Scale, Transform};

impl LayoutPatch {
    pub fn new(connector: ConnectorName) -> Self {
        Self {
            connector,
            mode: None,
            scale: None,
            transform: None,
            enable: None,
            position: None,
        }
    }

    pub fn mode(mut self, request: ModeRequest) -> Self {
        self.mode = Some(request);
        self
    }

    pub fn scale(mut self, scale: Scale) -> Self {
        self.scale = Some(scale);
        self
    }

    pub fn transform(mut self, transform: Transform) -> Self {
        self.transform = Some(transform);
        self
    }

    pub fn enable(mut self, enable: bool) -> Self {
        self.enable = Some(enable);
        self
    }

    pub fn position(mut self, position: Position) -> Self {
        self.position = Some(position);
        self
    }
}

impl ResolvedPatch {
    pub fn position_of(&self, connector: &ConnectorName) -> Option<Position> {
        self.positions
            .iter()
            .find(|(c, _)| c == connector)
            .map(|(_, p)| *p)
    }
}

impl LayoutPolicy {
    /// The bar lives on its host display; switching that display off would take the bar
    /// with it, so another display has to become the main one first.
    pub fn guard_bar_host(
        topology: &Topology,
        patch: &LayoutPatch,
        host: Option<&str>,
    ) -> Result<(), LayoutError> {
        let disabling = patch.enable == Some(false)
            && topology.find(&patch.connector).is_some_and(|o| o.enabled);
        if disabling && host == Some(patch.connector.as_str()) {
            return Err(LayoutError::BarHost(patch.connector.clone()));
        }
        Ok(())
    }

    pub fn validate(
        topology: &Topology,
        patch: &LayoutPatch,
    ) -> Result<ResolvedPatch, LayoutError> {
        let output = topology
            .find(&patch.connector)
            .ok_or_else(|| LayoutError::UnknownOutput(patch.connector.clone()))?;

        if patch.enable == Some(false) && output.enabled && topology.enabled_count() <= 1 {
            return Err(LayoutError::LastDisplay(patch.connector.clone()));
        }

        let unsupported = |request: String| LayoutError::UnsupportedMode {
            connector: patch.connector.clone(),
            request,
        };

        let mode = match patch.mode {
            None => None,
            Some(ModeRequest::Exact(mode)) => {
                if output.modes.iter().any(|m| *m == mode) {
                    Some(mode)
                } else {
                    return Err(unsupported(mode.to_string()));
                }
            }
            Some(ModeRequest::Refresh(hz)) => {
                let current = output
                    .mode
                    .ok_or_else(|| LayoutError::NoCurrentMode(patch.connector.clone()))?;
                let found = output.modes.exact_for(current.resolution, hz);
                Some(found.ok_or_else(|| unsupported(format!("{}@{hz}", current.resolution)))?)
            }
            Some(ModeRequest::Resolution(res)) => {
                let prefer = output.mode.map_or(60, |m| m.refresh.rounded_hz());
                let found = output.modes.token_for_resolution(res, prefer);
                Some(found.ok_or_else(|| unsupported(res.to_string()))?)
            }
        };

        // Resolution the output will run at, when anything knows it.
        let resolution = mode
            .map(|m| m.resolution)
            .or_else(|| output.mode.map(|m| m.resolution))
            .or_else(|| output.modes.resolutions().first().copied());

        // Hyprland silently swaps in another scale when the logical size would be
        // fractional, so refuse such requests and repair stale scales ourselves.
        let mut scale = patch.scale;
        if let Some(res) = resolution {
            match scale {
                Some(requested) if !requested.is_valid_for(res) => {
                    return Err(LayoutError::InvalidScale {
                        connector: patch.connector.clone(),
                        requested,
                        nearest: requested.nearest_valid_for(res),
                    });
                }
                None if mode.is_some() && !output.scale.is_valid_for(res) => {
                    scale = Some(output.scale.nearest_valid_for(res));
                }
                _ => {}
            }
        }

        let before = Layout::from_topology(topology);
        let mut layout = before.clone();
        let will_be_enabled = patch.enable.unwrap_or(output.enabled);
        let mut position = None;

        if !will_be_enabled {
            layout.remove(&patch.connector);
        } else if let Some(res) = resolution {
            let size = logical_size(
                res,
                scale.unwrap_or(output.scale),
                patch.transform.unwrap_or(output.transform),
            );
            match layout.get(&patch.connector) {
                Some(rect) if rect.size() != size => layout.reflow_resize(&patch.connector, size),
                Some(_) => {}
                None => {
                    let at = layout.place_new(size, Some(output.position));
                    layout.set(&patch.connector, Rect::at(at, size));
                }
            }
            if let Some(wanted) = patch.position {
                let at = layout.validated_drop(&patch.connector, size, wanted);
                layout.set(&patch.connector, Rect::at(at, size));
            }
        }

        // A change that strands part of a connected layout pulls it back in. A layout
        // the user left detached on purpose is only reconnected by an explicit move.
        let changed = layout != before;
        if changed && (before.is_connected() || patch.position.is_some()) {
            layout.reconnect(&patch.connector);
        }

        // Only complain about overlaps this patch could have caused.
        for (first, second) in layout.overlaps() {
            let unchanged = |c: &ConnectorName| before.get(c) == layout.get(c);
            if unchanged(&first) && unchanged(&second) {
                continue;
            }
            return Err(LayoutError::Overlap { first, second });
        }

        // Negative coordinates confuse XWayland and pointer warps, and a moved layout
        // should start at the origin like GNOME and KDE store it. Edits that leave the
        // geometry alone never shift anything.
        let negative = layout.bounds().is_some_and(|b| b.x < 0 || b.y < 0);
        if changed && (patch.position.is_some() || negative) {
            layout.normalise_origin();
        }
        if will_be_enabled {
            position = layout.get(&patch.connector).map(Rect::position);
        }

        let mut warnings = Vec::new();
        if !layout.is_connected() {
            warnings.push(LayoutWarning::Disconnected);
        }

        Ok(ResolvedPatch {
            connector: patch.connector.clone(),
            mode,
            scale,
            transform: patch.transform,
            enable: patch.enable,
            position,
            positions: layout
                .tiles()
                .iter()
                .map(|t| (t.connector.clone(), t.rect.position()))
                .collect(),
            warnings,
        })
    }
}
