use crate::value::position::interfaces::Position;

impl Position {
    /// Kanshi form `x,y`.
    pub fn kanshi_token(self) -> String {
        format!("{},{}", self.x, self.y)
    }

    /// Hyprland form `XxY`.
    pub fn hypr_token(self) -> String {
        format!("{}x{}", self.x, self.y)
    }

    pub fn from_kanshi(token: &str) -> Option<Self> {
        let (x, y) = token.split_once(',')?;
        Some(Self::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
    }
}
