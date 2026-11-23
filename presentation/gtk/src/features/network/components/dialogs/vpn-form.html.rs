use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;
use settings_domain::network::{VpnDraft, VpnPeerDraft, validate_vpn_draft};

use super::frame::{entry, error_label, labelled, secret, show_error};
use crate::features::network::components::common;
use crate::features::network::constants::PAGE_GAP;
use crate::features::network::interfaces::VpnFormProps;
use crate::shared::button::ButtonKind;
use crate::shared::icon::Icon;
use crate::shared::subpage_bar;
use crate::shared::toggle::{self, ToggleProps};

struct PeerRow {
    public_key: gtk::Entry,
    preshared: gtk::PasswordEntry,
    endpoint: gtk::Entry,
    allowed_ips: gtk::Entry,
    keepalive: gtk::Entry,
}

impl PeerRow {
    fn read(&self) -> VpnPeerDraft {
        VpnPeerDraft {
            public_key: self.public_key.text().trim().to_owned(),
            preshared_key: self.preshared.text().trim().to_owned(),
            endpoint: self.endpoint.text().trim().to_owned(),
            allowed_ips: self.allowed_ips.text().trim().to_owned(),
            persistent_keepalive: self.keepalive.text().trim().to_owned(),
        }
    }
}

struct Form {
    name: gtk::Entry,
    interface: gtk::Entry,
    private_key: gtk::PasswordEntry,
    addresses: gtk::Entry,
    dns: gtk::Entry,
    mtu: gtk::Entry,
    listen_port: gtk::Entry,
    autoconnect: Rc<RefCell<bool>>,
    peers: gtk::Box,
    rows: RefCell<Vec<PeerRow>>,
}

impl Form {
    fn peers(&self) -> Vec<VpnPeerDraft> {
        self.rows.borrow().iter().map(PeerRow::read).collect()
    }

    fn draft(&self, base: &VpnDraft) -> VpnDraft {
        VpnDraft {
            id: base.id.clone(),
            uuid: base.uuid.clone(),
            name: self.name.text().trim().to_owned(),
            interface_name: self.interface.text().trim().to_owned(),
            private_key: self.private_key.text().trim().to_owned(),
            addresses: self.addresses.text().trim().to_owned(),
            dns: self.dns.text().trim().to_owned(),
            mtu: self.mtu.text().trim().to_owned(),
            listen_port: self.listen_port.text().trim().to_owned(),
            autoconnect: *self.autoconnect.borrow(),
            peers: self.peers(),
        }
    }
}

pub fn build(props: &VpnFormProps) -> gtk::Widget {
    let draft = &props.draft;
    let form = Rc::new(Form {
        name: entry(&draft.name, "Name"),
        interface: entry(&draft.interface_name, "wg0"),
        private_key: secret(&draft.private_key, "Private key"),
        addresses: entry(&draft.addresses, "10.0.0.2/32"),
        dns: entry(&draft.dns, "1.1.1.1"),
        mtu: entry(&draft.mtu, "1420"),
        listen_port: entry(&draft.listen_port, "Optional"),
        autoconnect: Rc::new(RefCell::new(draft.autoconnect)),
        peers: gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .build(),
        rows: RefCell::default(),
    });
    render_peers(&form, &draft.peers);

    let body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(14)
        .build();
    body.append(&labelled("Name", &form.name));
    body.append(&pair(
        labelled("Interface", &form.interface),
        labelled("MTU", &form.mtu),
    ));
    body.append(&labelled("Private key", &form.private_key));
    body.append(&pair(
        labelled("Addresses", &form.addresses),
        labelled("DNS", &form.dns),
    ));
    body.append(&pair(
        labelled("Listen port", &form.listen_port),
        autoconnect_switch(&form),
    ));
    body.append(&common::section(
        "Peers",
        Some(&add_peer_button(&form)),
        form.peers.upcast_ref(),
    ));
    let error = error_label();
    body.append(&error);

    let save = common::action("Save tunnel", ButtonKind::Primary, Some(Icon::Save), {
        let (form, error, base, on_save) = (
            form.clone(),
            error.clone(),
            props.draft.clone(),
            props.on_save.clone(),
        );
        Rc::new(move || {
            let draft = form.draft(&base);
            if let Some(problem) = validate_vpn_draft(&draft) {
                show_error(&error, problem);
                return;
            }
            on_save(draft);
        })
    });
    body.append(&save);

    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(PAGE_GAP)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .css_classes(["page-content", "page-content--sub", "network-page"])
        .build();
    column.append(&body);

    let title = if props.draft.id.is_empty() {
        "New WireGuard"
    } else {
        "Edit WireGuard"
    };
    subpage_bar::frame(title, props.actions.back.clone(), column.upcast_ref())
}

fn pair(left: gtk::Widget, right: gtk::Widget) -> gtk::Widget {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .homogeneous(true)
        .build();
    row.append(&left);
    row.append(&right);
    row.upcast()
}

fn autoconnect_switch(form: &Rc<Form>) -> gtk::Widget {
    let state = form.autoconnect.clone();
    let active = *state.borrow();
    let switch = toggle::build(&ToggleProps {
        active,
        on_toggle: Rc::new(move |on| *state.borrow_mut() = on),
    });
    switch.set_halign(gtk::Align::Start);
    labelled("Connect automatically", &switch)
}

fn add_peer_button(form: &Rc<Form>) -> gtk::Widget {
    let form = form.clone();
    common::action(
        "Add peer",
        ButtonKind::Hollow,
        Some(Icon::Plus),
        Rc::new(move || {
            let mut peers = form.peers();
            peers.push(VpnPeerDraft::default());
            render_peers(&form, &peers);
        }),
    )
}

/// Draws one block per peer from `peers`; add and remove redraw from what the fields hold.
fn render_peers(form: &Rc<Form>, peers: &[VpnPeerDraft]) {
    while let Some(child) = form.peers.first_child() {
        form.peers.remove(&child);
    }
    form.rows.borrow_mut().clear();
    for (index, peer) in peers.iter().enumerate() {
        let row = PeerRow {
            public_key: entry(&peer.public_key, "Peer public key"),
            preshared: secret(&peer.preshared_key, "Preshared key (optional)"),
            endpoint: entry(&peer.endpoint, "vpn.example.com:51820"),
            allowed_ips: entry(&peer.allowed_ips, "0.0.0.0/0, ::/0"),
            keepalive: entry(&peer.persistent_keepalive, "25"),
        };
        let card = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .css_classes(["net-card", "net-card__body"])
            .build();
        let head = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        head.append(
            &gtk::Label::builder()
                .label(format!("Peer {}", index + 1))
                .xalign(0.0)
                .hexpand(true)
                .css_classes(["net-card__title"])
                .build(),
        );
        if peers.len() > 1 {
            let target = Rc::downgrade(form);
            head.append(&common::action(
                "Remove",
                ButtonKind::Danger,
                Some(Icon::Trash),
                Rc::new(move || {
                    if let Some(form) = target.upgrade() {
                        let mut peers = form.peers();
                        if index < peers.len() {
                            peers.remove(index);
                        }
                        render_peers(&form, &peers);
                    }
                }),
            ));
        }
        card.append(&head);
        card.append(&labelled("Public key", &row.public_key));
        card.append(&labelled("Preshared key", &row.preshared));
        card.append(&labelled("Endpoint", &row.endpoint));
        card.append(&pair(
            labelled("Allowed IPs", &row.allowed_ips),
            labelled("Keepalive (s)", &row.keepalive),
        ));
        form.peers.append(&card);
        form.rows.borrow_mut().push(row);
    }
}
