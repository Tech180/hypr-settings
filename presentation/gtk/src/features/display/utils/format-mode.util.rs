use settings_application::dto::TrialView;
use settings_domain::{DisplayKind, TrialKind};

pub fn format_mode(width: u32, height: u32, hz: u32) -> String {
    format!("{width}×{height} @ {hz} Hz")
}

pub fn resolution_label(width: u32, height: u32) -> String {
    format!("{width}×{height}")
}

pub fn kind_label(kind: DisplayKind) -> &'static str {
    match kind {
        DisplayKind::Laptop => "Laptop panel",
        DisplayKind::Monitor => "Monitor",
        DisplayKind::Tv => "Television",
        DisplayKind::Unknown => "Display",
    }
}

/// A layout position as shown in badges: `+1920, 0`, `0, 0`, `-1080, +300`.
pub fn coordinates(x: i32, y: i32) -> String {
    let axis = |n: i32| {
        if n > 0 {
            format!("+{n}")
        } else {
            n.to_string()
        }
    };
    format!("{}, {}", axis(x), axis(y))
}

fn trial_kind_label(kind: TrialKind) -> &'static str {
    match kind {
        TrialKind::Resolution => "Resolution",
        TrialKind::Refresh => "Refresh rate",
        TrialKind::Scale => "Scale",
        TrialKind::Transform => "Rotation",
        TrialKind::Position => "Position",
    }
}

pub fn trial_message(trial: &TrialView) -> String {
    format!(
        "{} changed on {}. Keep it? Reverting in {}s.",
        trial_kind_label(trial.kind),
        trial.connector,
        trial.remaining_secs
    )
}

#[cfg(test)]
mod tests {
    use settings_application::dto::{ApplyOutcome, TrialView};
    use settings_domain::TrialKind;

    use super::*;

    #[test]
    fn mode_and_resolution_use_the_multiplication_sign() {
        assert_eq!(format_mode(1920, 1080, 144), "1920×1080 @ 144 Hz");
        assert_eq!(resolution_label(2560, 1080), "2560×1080");
    }

    #[test]
    fn coordinates_mark_positive_offsets() {
        assert_eq!(coordinates(1920, 0), "+1920, 0");
        assert_eq!(coordinates(0, 0), "0, 0");
        assert_eq!(coordinates(-1080, 300), "-1080, +300");
    }

    #[test]
    fn trial_message_names_the_change_and_the_countdown() {
        let trial = TrialView {
            connector: "DP-3".into(),
            kind: TrialKind::Refresh,
            remaining_secs: 12,
            timeout_secs: 30,
            outcome: ApplyOutcome::Verified,
        };
        assert_eq!(
            trial_message(&trial),
            "Refresh rate changed on DP-3. Keep it? Reverting in 12s."
        );
    }
}
