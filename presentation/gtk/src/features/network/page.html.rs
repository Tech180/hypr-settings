use gtk::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use super::components;
use super::constants::SLIDE_MS;
use super::interfaces::{
    ConfirmProps, DiagnosticsProps, EthernetProps, HomeProps, HomeWidgets, NetPage, NetworkActions,
    PageFrame, PropertiesProps, QrProps, VpnDetailProps, VpnFormProps, VpnProps,
    WifiProps,
};
use crate::shared::subpage_bar;

pub fn build_frame() -> PageFrame {
    let home = adw::Bin::new();
    let home_scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .vexpand(true)
        .child(&home)
        .build();
    let wifi = adw::Bin::new();
    let ethernet = adw::Bin::new();
    let vpn = adw::Bin::new();
    let vpn_detail = adw::Bin::new();
    let vpn_form = adw::Bin::new();
    let diagnostics = adw::Bin::new();
    let properties = adw::Bin::new();
    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::SlideLeftRight)
        .transition_duration(SLIDE_MS)
        .hexpand(true)
        .vexpand(true)
        .build();
    stack.add_named(&home_scroller, Some(NetPage::Home.stack_name()));
    stack.add_named(&wifi, Some(NetPage::Wifi.stack_name()));
    stack.add_named(&ethernet, Some(NetPage::Ethernet.stack_name()));
    stack.add_named(&vpn, Some(NetPage::Vpn.stack_name()));
    stack.add_named(&vpn_detail, Some(NetPage::VpnDetail.stack_name()));
    stack.add_named(&vpn_form, Some(NetPage::VpnForm.stack_name()));
    stack.add_named(&diagnostics, Some(NetPage::Diagnostics.stack_name()));
    stack.add_named(&properties, Some(NetPage::Properties.stack_name()));
    PageFrame {
        root: stack,
        home,
        wifi,
        ethernet,
        vpn,
        vpn_detail,
        vpn_form,
        diagnostics,
        properties,
    }
}

/// What a subpage shows until its first data arrives; Back works meanwhile.
pub fn build_loading(page: NetPage, actions: &NetworkActions) -> gtk::Widget {
    let spinner = gtk::Spinner::builder()
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .width_request(32)
        .height_request(32)
        .vexpand(true)
        .build();
    spinner.start();
    spinner.update_property(&[gtk::accessible::Property::Label("Loading")]);
    subpage_bar::frame(page.title(), actions.back.clone(), spinner.upcast_ref())
}

pub fn build_home(props: &HomeProps) -> HomeWidgets {
    components::home_page::build(props)
}

pub fn build_wifi(props: &WifiProps) -> (gtk::Widget, components::wifi_page::WifiListSurfaces) {
    components::wifi_page::build(props)
}

pub fn build_ethernet(props: &EthernetProps) -> gtk::Widget {
    components::ethernet_page::build(props)
}

pub fn build_vpn(props: &VpnProps) -> gtk::Widget {
    components::vpn_page::build(props)
}

pub fn build_vpn_detail(props: &VpnDetailProps) -> gtk::Widget {
    components::vpn_detail_page::build(props)
}

pub fn build_vpn_form(props: &VpnFormProps) -> gtk::Widget {
    components::dialogs::vpn_form::build(props)
}

pub fn build_diagnostics(props: &DiagnosticsProps) -> gtk::Widget {
    components::diagnostics_page::build(props)
}

pub fn build_properties(props: &PropertiesProps) -> gtk::Widget {
    components::properties_page::build(props)
}

pub fn confirm(parent: &gtk::Widget, props: &ConfirmProps) {
    components::dialogs::confirm::present(parent, props);
}

pub fn show_qr(parent: &gtk::Widget, props: &QrProps) {
    components::dialogs::qr::present(parent, props);
}

