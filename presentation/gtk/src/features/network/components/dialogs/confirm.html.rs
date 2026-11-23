use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use crate::features::network::interfaces::ConfirmProps;

/// Asks before something that cannot be undone. Cancel is the default.
pub fn present(parent: &impl IsA<gtk::Widget>, props: &ConfirmProps) {
    let dialog = adw::AlertDialog::new(Some(&props.heading), Some(&props.body));
    dialog.add_responses(&[("cancel", "Cancel"), ("confirm", &props.confirm_label)]);
    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    let on_confirm = props.on_confirm.clone();
    dialog.connect_response(None, move |_, response| {
        if response == "confirm" {
            on_confirm();
        }
    });
    dialog.present(Some(parent));
}
