use super::interfaces::Resolution;

impl Resolution {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn pixels(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn parse(text: &str) -> Option<Self> {
        let (w, h) = text.split_once('x')?;
        let res = Self::new(w.parse().ok()?, h.parse().ok()?);
        (!res.is_empty()).then_some(res)
    }
}
