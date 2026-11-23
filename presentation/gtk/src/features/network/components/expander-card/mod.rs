#[path = "expander-card.html.rs"]
mod html;
#[path = "interfaces/expander-card-props.interface.rs"]
mod props;

pub use html::build;
pub use props::CardProps;
