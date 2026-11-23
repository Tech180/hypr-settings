/// Shortest shared edge, in logical pixels, that counts as a usable neighbour
/// when sliding a dropped monitor along another one.
pub const MIN_SHARED_EDGE: i32 = 64;

/// Upper bound on how many stranded groups `Layout::reconnect` pulls back in.
pub const MAX_RECONNECT_PASSES: usize = 8;
