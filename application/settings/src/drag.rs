//! Drag preview for the arrangement canvas. It runs the same rules the apply path
//! uses, on the plain data of a [`DisplayView`], so the ghost the user sees is the
//! position that gets applied.

use settings_domain::arrangement::{Layout, Rect, Tile};
use settings_domain::{ConnectorName, Position};

use crate::dto::OutputView;

/// The layout the canvas draws: enabled outputs with a known size.
pub fn layout_of(outputs: &[OutputView]) -> Layout {
    Layout::new(
        outputs
            .iter()
            .filter(|o| o.enabled && o.logical_width > 0 && o.logical_height > 0)
            .filter_map(|o| {
                Some(Tile {
                    connector: ConnectorName::try_from(o.connector.as_str()).ok()?,
                    rect: Rect::new(o.x, o.y, o.logical_width, o.logical_height),
                })
            })
            .collect(),
    )
}

/// Where a dragged output lands: flush against a neighbour, never overlapping,
/// aligned to neighbouring edges and centres within `threshold` logical pixels.
/// `None` for an output that is not part of the layout.
pub fn resolve_drop(
    outputs: &[OutputView],
    connector: &str,
    wanted: (i32, i32),
    threshold: i32,
) -> Option<(i32, i32)> {
    let layout = layout_of(outputs);
    let connector = ConnectorName::try_from(connector).ok()?;
    let size = layout.get(&connector)?.size();
    let at = layout.resolve_drop(
        &connector,
        size,
        Position::new(wanted.0, wanted.1),
        threshold,
    );
    Some((at.x, at.y))
}

/// The whole layout as it would be after dropping `connector` near `wanted`: the
/// dropped output where [`resolve_drop`] puts it, and any output the move strands
/// pulled back against the rest. Coordinates stay in the current layout space (no
/// origin shift), so the canvas can preview every tile without rescaling.
pub fn preview(
    outputs: &[OutputView],
    connector: &str,
    wanted: (i32, i32),
    threshold: i32,
) -> Option<Vec<(String, i32, i32)>> {
    let mut layout = layout_of(outputs);
    let name = ConnectorName::try_from(connector).ok()?;
    let size = layout.get(&name)?.size();
    let at = layout.resolve_drop(&name, size, Position::new(wanted.0, wanted.1), threshold);
    layout.set(&name, Rect::at(at, size));
    layout.reconnect(&name);
    Some(
        layout
            .tiles()
            .iter()
            .map(|t| (t.connector.to_string(), t.rect.x, t.rect.y))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::OutputView;
    use settings_domain::DisplayKind;

    fn view(connector: &str, x: i32, w: i32, h: i32) -> OutputView {
        OutputView {
            connector: connector.into(),
            description: String::new(),
            make: String::new(),
            model: String::new(),
            kind: DisplayKind::Monitor,
            enabled: true,
            focused: false,
            ultrawide: false,
            width: w as u32,
            height: h as u32,
            refresh_hz: 60,
            x,
            y: 0,
            logical_width: w,
            logical_height: h,
            flipped: false,
            suggested_x: x,
            suggested_y: 0,
            resolutions: Vec::new(),
            hz_options: Vec::new(),
            scale_options: Vec::new(),
            transform_options: Vec::new(),
        }
    }

    #[test]
    fn ghost_snaps_flush_and_aligned() {
        let outputs = [view("eDP-1", 0, 1920, 1080), view("DP-3", 1920, 2560, 1080)];
        assert_eq!(
            resolve_drop(&outputs, "DP-3", (1945, 18), 30),
            Some((1920, 0))
        );
    }

    #[test]
    fn preview_moves_the_stranded_output_with_the_drop() {
        let outputs = [
            view("A", 0, 1000, 1000),
            view("B", 1000, 1000, 1000),
            view("C", 2000, 1000, 1000),
        ];
        let moved = preview(&outputs, "B", (0, 1000), 30).unwrap();
        let at = |n: &str| {
            moved
                .iter()
                .find(|(c, ..)| c == n)
                .map(|(_, x, y)| (*x, *y))
        };
        assert_eq!(at("B"), Some((0, 1000)));
        assert_eq!(at("A"), Some((0, 0)));
        assert_eq!(at("C"), Some((1000, 0)), "C slides left to touch A");
        assert_eq!(preview(&outputs, "nope", (0, 0), 30), None);
    }

    #[test]
    fn unknown_or_disabled_outputs_have_no_ghost() {
        let mut outputs = [view("eDP-1", 0, 1920, 1080), view("DP-3", 1920, 2560, 1080)];
        outputs[1].enabled = false;
        assert_eq!(resolve_drop(&outputs, "DP-3", (0, 0), 30), None);
        assert_eq!(resolve_drop(&outputs, "DP-9", (0, 0), 30), None);
    }
}
