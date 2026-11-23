use std::rc::Rc;

use settings_domain::network::{IpConfig, IpVersion};

#[derive(Clone)]
pub struct IpSettingsProps {
    pub ipv4: IpConfig,
    pub ipv6: IpConfig,
    pub on_apply: Rc<dyn Fn(IpVersion, IpConfig)>,
}
