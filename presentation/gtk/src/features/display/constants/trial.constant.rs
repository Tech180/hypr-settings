pub const TICK_SECS: u32 = 1;
pub const EXPIRED_MESSAGE: &str = "Change timed out. The previous layout was restored.";
pub const UNVERIFIED_MESSAGE: &str = "Applied, but the compositor has not confirmed it yet.";
pub const APPLIED_DIRECTLY_MESSAGE: &str = "Applied directly because kanshi is not running.";
/// How long a slider has to be still before its value is written to the hardware.
pub const SLIDER_DEBOUNCE_MS: u64 = 250;
