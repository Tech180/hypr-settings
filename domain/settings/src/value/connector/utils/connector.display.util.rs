use std::fmt;

use crate::value::connector::interfaces::ConnectorName;

impl fmt::Display for ConnectorName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
