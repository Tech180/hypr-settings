use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;
use gtk4::gio;

pub fn choose_vpn_config_file(anchor: &gtk::Widget, pick: Rc<dyn Fn(String)>) {
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("VPN configs"));
    filter.add_pattern("*.conf");
    filter.add_pattern("*.ovpn");
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    let dialog = gtk::FileDialog::builder()
        .title("Import VPN config")
        .filters(&filters)
        .build();
    let window = anchor.root().and_downcast::<gtk::Window>();
    dialog.open(window.as_ref(), None::<&gio::Cancellable>, move |result| {
        if let Ok(file) = result
            && let Some(path) = file.path()
        {
            pick(path.to_string_lossy().into_owned());
        }
    });
}
