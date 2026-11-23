use gtk4 as gtk;
use libadwaita as adw;

use super::components;
use super::constants::SLIDE_MS;
use super::interfaces::{
    AdvancedPageProps, BluetoothMainProps, BluetoothScanProps, DeviceDetailProps, PageFrame,
};

pub fn build_frame() -> PageFrame {
    let main_body = adw::Bin::new();
    let main_scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .vexpand(true)
        .child(&main_body)
        .build();
    let device = adw::Bin::new();
    let advanced = adw::Bin::new();
    let scan = adw::Bin::new();
    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::SlideLeftRight)
        .transition_duration(SLIDE_MS)
        .hexpand(true)
        .vexpand(true)
        .build();
    stack.add_named(&main_scroller, Some("main"));
    stack.add_named(&device, Some("device"));
    stack.add_named(&advanced, Some("advanced"));
    stack.add_named(&scan, Some("scan"));
    PageFrame {
        root: stack,
        main_scroller,
        main_body,
        device,
        advanced,
        scan,
    }
}

pub fn build_main(props: &BluetoothMainProps) -> gtk::Widget {
    components::main_page::build(props)
}

pub fn build_device_detail(props: &DeviceDetailProps) -> gtk::Widget {
    components::device_detail::build(props)
}

pub fn build_advanced(props: &AdvancedPageProps) -> gtk::Widget {
    components::advanced_page::build(props)
}

pub fn build_scan(props: &BluetoothScanProps) -> gtk::Widget {
    components::scan_page::build(props)
}
