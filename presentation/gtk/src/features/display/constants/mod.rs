#[path = "canvas.constant.rs"]
pub mod canvas;
#[path = "page.constant.rs"]
pub mod page;
#[path = "trial.constant.rs"]
mod trial;

pub use page::{CONTENT_GAP, SLIDE_MS};
pub use trial::{
    APPLIED_DIRECTLY_MESSAGE, EXPIRED_MESSAGE, SLIDER_DEBOUNCE_MS, TICK_SECS, UNVERIFIED_MESSAGE,
};
