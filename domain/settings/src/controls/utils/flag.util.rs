use crate::controls::interfaces::{Capability, FeatureCaps, Flag, OutputFlags};
use crate::output::Output;

impl Flag {
    pub const ALL: [Self; 3] = [Self::Hdr, Self::TenBit, Self::Vrr];

    pub fn label(self) -> &'static str {
        match self {
            Self::Hdr => "HDR",
            Self::TenBit => "10-bit color",
            Self::Vrr => "Adaptive Sync",
        }
    }

    /// Whether the compositor has the feature on for `output` right now.
    pub fn live(self, output: &Output) -> bool {
        match self {
            Self::Hdr => output.hdr,
            Self::TenBit => output.ten_bit,
            Self::Vrr => output.vrr,
        }
    }
}

impl OutputFlags {
    pub fn only(flag: Flag, value: bool) -> Self {
        let mut flags = Self::default();
        flags.set(flag, Some(value));
        flags
    }

    /// Everything the compositor has on or off right now.
    pub fn live(output: &Output) -> Self {
        Self {
            hdr: Some(output.hdr),
            ten_bit: Some(output.ten_bit),
            vrr: Some(output.vrr),
        }
    }

    pub fn get(self, flag: Flag) -> Option<bool> {
        match flag {
            Flag::Hdr => self.hdr,
            Flag::TenBit => self.ten_bit,
            Flag::Vrr => self.vrr,
        }
    }

    pub fn set(&mut self, flag: Flag, value: Option<bool>) {
        match flag {
            Flag::Hdr => self.hdr = value,
            Flag::TenBit => self.ten_bit = value,
            Flag::Vrr => self.vrr = value,
        }
    }

    pub fn is_empty(self) -> bool {
        self == Self::default()
    }
}

impl FeatureCaps {
    pub fn of(&self, flag: Flag) -> &Capability {
        match flag {
            Flag::Hdr => &self.hdr,
            Flag::TenBit => &self.ten_bit,
            Flag::Vrr => &self.vrr,
        }
    }
}
