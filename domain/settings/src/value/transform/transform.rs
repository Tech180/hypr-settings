use super::constants;
use super::interfaces::Transform;

impl Transform {
    pub const ALL: [Transform; 8] = constants::ALL;

    /// The unmirrored rotations, in the order the UI offers them.
    pub const ROTATIONS: [Transform; 4] = constants::ROTATIONS;

    pub fn from_hypr(value: i32) -> Self {
        Self::ALL[(value & 7) as usize]
    }

    pub fn to_hypr(self) -> i32 {
        Self::ALL
            .iter()
            .position(|t| *t == self)
            .map_or(0, |i| i as i32)
    }

    /// Quarter turns of 90 and 270 swap a monitor's width and height.
    pub fn swaps_axes(self) -> bool {
        self.to_hypr() % 2 == 1
    }

    pub fn is_flipped(self) -> bool {
        self.to_hypr() >= 4
    }

    /// The same rotation without the mirror.
    pub fn rotation(self) -> Self {
        Self::ALL[(self.to_hypr() & 3) as usize]
    }

    /// Keeps the rotation and sets the mirror bit.
    pub fn with_flipped(self, flipped: bool) -> Self {
        Self::ALL[((self.to_hypr() & 3) | if flipped { 4 } else { 0 }) as usize]
    }

    /// Keeps the mirror bit and replaces the rotation by that of `rotation`.
    pub fn with_rotation(self, rotation: Self) -> Self {
        Self::ALL[((rotation.to_hypr() & 3) | (self.to_hypr() & 4)) as usize]
    }
}
