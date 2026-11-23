use std::rc::Rc;

use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;
use settings_application::dto::DeviceProfileView;
use settings_domain::bluetooth::ProfileKind;

use crate::features::bluetooth::interfaces::AdvancedPageProps;
use crate::features::bluetooth::interfaces::DeviceDetailProps;
use crate::shared::badge::{BadgeProps, Tone};
use crate::shared::icon::Icon;
use crate::shared::icon_tile::IconTileProps;
use crate::shared::settings_group::{RowControl, SettingRowProps};
pub fn profile_rows(props: &DeviceDetailProps) -> Vec<SettingRowProps> {
    ProfileKind::DETAIL_ROWS
        .iter()
        .filter_map(|kind| profile_row(props, *kind))
        .collect()
}

fn profile_row(props: &DeviceDetailProps, kind: ProfileKind) -> Option<SettingRowProps> {
    let toggle = profile_toggle(&props.device.profiles, kind);
    if !toggle.available {
        return None;
    }
    let (title, subtitle) = profile_copy(kind);
    let id = props.device.id.clone();
    let on_toggle = match kind {
        ProfileKind::AbsoluteVolume => {
            let set = props.actions.set_absolute_volume.clone();
            Rc::new(move |on| set(id.clone(), on)) as Rc<dyn Fn(bool)>
        }
        _ => {
            let set = props.actions.set_profile.clone();
            Rc::new(move |on| set(id.clone(), kind, on))
        }
    };
    Some(SettingRowProps {
        title: title.into(),
        subtitle: subtitle.map(str::to_owned),
        sensitive: true,
        control: RowControl::Switch {
            active: toggle.enabled,
            on_toggle,
        },
        leading: None,
        badge: None,
    })
}

fn profile_toggle(
    profiles: &DeviceProfileView,
    kind: ProfileKind,
) -> settings_application::dto::ProfileToggleView {
    match kind {
        ProfileKind::MediaAudio => profiles.media_audio.clone(),
        ProfileKind::PhoneCalls => profiles.phone_calls.clone(),
        ProfileKind::ContactSharing => profiles.contact_sharing.clone(),
        ProfileKind::AbsoluteVolume => profiles.absolute_volume.clone(),
    }
}

fn profile_copy(kind: ProfileKind) -> (&'static str, Option<&'static str>) {
    match kind {
        ProfileKind::MediaAudio => ("Media Audio", Some("High-quality stereo (A2DP)")),
        ProfileKind::PhoneCalls => ("Phone Calls", Some("Headset profile (HFP)")),
        ProfileKind::ContactSharing => ("Contact Sharing", Some("Phone book access (PBAP)")),
        ProfileKind::AbsoluteVolume => ("Absolute Volume", Some("Sync volume with device")),
    }
}

pub fn present_forget_dialog(
    id: &settings_application::dto::DeviceId,
    forget: Rc<dyn Fn(settings_application::dto::DeviceId)>,
) {
    let dialog = adw::AlertDialog::builder()
        .heading("Forget this device?")
        .body("It will be removed from paired devices. You can pair again later.")
        .close_response("cancel")
        .default_response("forget")
        .build();
    dialog.add_responses(&[("cancel", "Cancel"), ("forget", "Forget")]);
    dialog.set_response_appearance("forget", adw::ResponseAppearance::Destructive);
    let id = id.clone();
    dialog.connect_response(None, move |_, response| {
        if response == "forget" {
            forget(id.clone());
        }
    });
    let parent: Option<&gtk::Widget> = None;
    dialog.present(parent);
}

pub fn pan_row(props: &AdvancedPageProps) -> SettingRowProps {
    let available = props.capabilities.pan_tethering;
    SettingRowProps {
        title: "Bluetooth Tethering (PAN)".into(),
        subtitle: Some(if available {
            "Share this computer's internet connection".into()
        } else {
            "Requires NetworkManager".into()
        }),
        sensitive: available,
        control: RowControl::Switch {
            active: props.advanced.pan_enabled,
            on_toggle: props.actions.set_pan.clone(),
        },
        leading: Some(IconTileProps::small(Icon::Globe, Tone::Neutral)),
        badge: unavailable(available),
    }
}

pub fn auracast_row(props: &AdvancedPageProps) -> SettingRowProps {
    let available = props.advanced.auracast_available;
    SettingRowProps {
        title: "LE Audio / Auracast".into(),
        subtitle: Some(if available {
            "Broadcast and receive LE Audio".into()
        } else {
            "Needs BlueZ experimental features".into()
        }),
        sensitive: available,
        control: RowControl::Chevron(Rc::new(move || present_auracast_dialog(available))),
        leading: Some(IconTileProps::small(Icon::RadioTower, Tone::Neutral)),
        badge: Some(if available {
            BadgeProps::new("Experimental", Tone::Warning)
        } else {
            BadgeProps::new("Unavailable", Tone::Neutral)
        }),
    }
}

fn unavailable(available: bool) -> Option<BadgeProps> {
    (!available).then(|| BadgeProps::new("Unavailable", Tone::Neutral))
}

fn present_auracast_dialog(available: bool) {
    let dialog = if available {
        adw::AlertDialog::builder()
            .heading("Auracast")
            .body("Configure LE Audio broadcast from your BlueZ experimental settings.")
            .build()
    } else {
        adw::AlertDialog::builder()
            .heading("Auracast unavailable")
            .body("Enable BlueZ experimental features and LE Audio support to use Auracast on this machine.")
            .build()
    };
    dialog.add_response("ok", "OK");
    dialog.connect_response(None, |_, _| {});
    let parent: Option<&gtk::Widget> = None;
    dialog.present(parent);
}

/// Every developer option is listed; the ones this system cannot honour stay visible but
/// inactive and say so, instead of vanishing.
pub fn developer_rows(props: &AdvancedPageProps) -> Vec<SettingRowProps> {
    vec![
        avrcp_row(props),
        sample_rate_row(props),
        offload_row(props),
        snoop_row(props),
        gatt_row(props),
    ]
}

fn avrcp_row(props: &AdvancedPageProps) -> SettingRowProps {
    let available = props.capabilities.avrcp_config && !props.advanced.avrcp_options.is_empty();
    let labels = if available {
        props.advanced.avrcp_options.clone()
    } else {
        vec!["Unavailable".to_owned()]
    };
    let selected = props
        .advanced
        .avrcp_version
        .as_ref()
        .and_then(|v| labels.iter().position(|o| o == v))
        .unwrap_or(0);
    let set = props.actions.set_avrcp.clone();
    SettingRowProps {
        title: "AVRCP Version".into(),
        subtitle: Some("Remote control profile for media".into()),
        sensitive: available,
        control: RowControl::Select {
            options: labels.clone(),
            selected,
            on_select: Rc::new(move |index| {
                if let Some(label) = labels.get(index) {
                    set(label.clone());
                }
            }),
        },
        leading: Some(IconTileProps::small(Icon::RefreshCw, Tone::Neutral)),
        badge: Some(BadgeProps::new("Restart required", Tone::Warning)),
    }
}

fn sample_rate_row(props: &AdvancedPageProps) -> SettingRowProps {
    let available =
        props.capabilities.sample_rate && !props.advanced.sample_rate_options.is_empty();
    let rates = if available {
        props.advanced.sample_rate_options.clone()
    } else {
        Vec::new()
    };
    let labels: Vec<String> = if available {
        rates.iter().map(|hz| format!("{hz} Hz")).collect()
    } else {
        vec!["Unavailable".to_owned()]
    };
    let selected = props
        .advanced
        .sample_rate_hz
        .and_then(|hz| rates.iter().position(|r| *r == hz))
        .unwrap_or(0);
    let set = props.actions.set_sample_rate.clone();
    SettingRowProps {
        title: "Default Sample Rate".into(),
        subtitle: Some("PipeWire preference for Bluetooth audio".into()),
        sensitive: available,
        control: RowControl::Select {
            options: labels,
            selected,
            on_select: Rc::new(move |index| {
                if let Some(hz) = rates.get(index) {
                    set(*hz);
                }
            }),
        },
        leading: Some(IconTileProps::small(Icon::ChartLine, Tone::Neutral)),
        badge: unavailable(available),
    }
}

fn offload_row(props: &AdvancedPageProps) -> SettingRowProps {
    let available = props.capabilities.a2dp_offload;
    SettingRowProps {
        title: "Disable A2DP Offload".into(),
        subtitle: Some("Force software encoding".into()),
        sensitive: available,
        control: RowControl::Switch {
            active: props.advanced.a2dp_offload_disabled,
            on_toggle: props.actions.set_offload_disabled.clone(),
        },
        leading: Some(IconTileProps::small(Icon::Cpu, Tone::Neutral)),
        badge: unavailable(available),
    }
}

fn snoop_row(props: &AdvancedPageProps) -> SettingRowProps {
    let available = props.capabilities.hci_snoop;
    SettingRowProps {
        title: "HCI Snoop Log".into(),
        subtitle: Some("Capture Bluetooth traffic for debugging".into()),
        sensitive: available,
        control: RowControl::Switch {
            active: props.advanced.hci_snoop,
            on_toggle: props.actions.set_hci_snoop.clone(),
        },
        leading: Some(IconTileProps::small(Icon::Terminal, Tone::Neutral)),
        badge: unavailable(available),
    }
}

fn gatt_row(props: &AdvancedPageProps) -> SettingRowProps {
    let available = props.capabilities.gatt_cache_clear;
    SettingRowProps {
        title: "Clear GATT Cache".into(),
        subtitle: Some("Reset cached GATT attributes".into()),
        sensitive: available,
        control: RowControl::Button {
            label: "Clear".into(),
            on_click: props.actions.clear_gatt.clone(),
        },
        leading: Some(IconTileProps::small(Icon::Trash, Tone::Neutral)),
        badge: unavailable(available),
    }
}
