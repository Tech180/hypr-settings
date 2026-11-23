use crate::output::{Output, OutputIdentity};
use crate::profile::interfaces::Criteria;

/// How to refer to a display in a profile: the connector for the built-in panel
/// and for displays that cannot be told apart, otherwise its stable
/// `Make Model Serial` (connectors change across docks and reboots).
pub fn criteria_for(output: &Output, all: &[OutputIdentity]) -> Criteria {
    let id = OutputIdentity::of(output);
    if output.connector.is_internal() || !id.has_description() {
        return Criteria::Named(output.connector.to_string());
    }
    let description = id.description();
    let shared = all
        .iter()
        .filter(|o| o.description() == description)
        .count()
        > 1;
    if shared {
        Criteria::Named(output.connector.to_string())
    } else {
        Criteria::Named(description)
    }
}
