//! Bounds and scale steps used by [`crate::value::scale::Scale`].

/// Minimum scale (0.5×), stored in hundredths.
pub const MIN_HUNDREDTHS: u16 = 50;

/// Maximum scale (4.0×), stored in hundredths.
pub const MAX_HUNDREDTHS: u16 = 400;

/// Upper bound when enumerating every valid scale for a resolution (3.0×).
pub const ALL_VALID_MAX_HUNDREDTHS: u16 = 300;

/// Common UI scale steps (hundredths); only those valid for the current mode are shown.
pub(crate) const COMMON_SCALES: [u16; 8] = [75, 100, 125, 150, 175, 200, 250, 300];
