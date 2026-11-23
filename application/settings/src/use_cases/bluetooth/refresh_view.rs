use settings_domain::bluetooth::{AdapterSnapshot, DeviceSnapshot, ProfileState, class_from_uuids};

use crate::bluetooth_ports::BluetoothPorts;
use crate::dto::{
    AdapterView, AdvancedView, BluetoothView, CodecView, DeviceProfileView, DeviceView,
    ProfileToggleView,
};
use crate::error::AppError;

pub fn refresh_bluetooth_view(ports: &BluetoothPorts) -> Result<BluetoothView, AppError> {
    let prefs = ports.prefs.load()?;
    let adapter_snap = ports.gateway.adapter()?;
    let mut devices = ports.gateway.devices()?;
    for device in &mut devices {
        device.class = prefs.class_for(&device.id, class_from_uuids(&device.uuids));
    }

    let adapters = ports
        .gateway
        .adapters()?
        .iter()
        .map(map_adapter)
        .collect::<Vec<_>>();
    let adapter = adapter_snap.as_ref().map(map_adapter);
    let missing_saved_adapter = prefs.selected_adapter.as_ref().and_then(|wanted| {
        let present = adapters
            .iter()
            .any(|a| a.address.eq_ignore_ascii_case(wanted));
        (!present).then(|| wanted.clone())
    });

    let alias = adapter_snap
        .as_ref()
        .map(|a| a.alias.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "this computer".into());

    let (paired, discovered): (Vec<_>, Vec<_>) =
        devices.into_iter().map(map_device).partition(|d| d.paired);

    let mut paired = paired;
    paired.sort_by(|a, b| b.connected.cmp(&a.connected).then(a.name.cmp(&b.name)));

    let capabilities = ports.gateway.capabilities();
    let advanced = AdvancedView {
        pan_enabled: ports.extras.pan_enabled().unwrap_or(false),
        avrcp_version: ports.extras.avrcp_version().ok().flatten(),
        avrcp_options: ports.extras.avrcp_versions().unwrap_or_default(),
        sample_rate_hz: ports.extras.sample_rate_hz().ok().flatten(),
        sample_rate_options: ports.extras.sample_rates().unwrap_or_default(),
        a2dp_offload_disabled: ports.extras.a2dp_offload_disabled().unwrap_or(false),
        hci_snoop: ports.extras.hci_snoop_enabled().unwrap_or(false),
        auracast_available: ports.extras.auracast_available(),
    };

    Ok(BluetoothView {
        powered: adapter_snap.as_ref().is_some_and(|a| a.powered),
        alias,
        discovering: adapter_snap.as_ref().is_some_and(|a| a.discovering),
        adapters,
        adapter,
        missing_saved_adapter,
        paired,
        discovered,
        capabilities,
        advanced,
    })
}

fn map_adapter(a: &AdapterSnapshot) -> AdapterView {
    AdapterView {
        name: a.name.clone(),
        address: a.address.clone(),
        alias: a.alias.clone(),
        powered: a.powered,
        lmp_version: a.lmp_version.clone(),
        modalias: a.modalias.clone(),
    }
}

fn map_device(device: DeviceSnapshot) -> DeviceView {
    DeviceView {
        id: device.id,
        name: device.name,
        connected: device.connected,
        paired: device.paired,
        battery_percent: device.battery_percent,
        class: device.class,
        profiles: map_profiles(&device.profiles),
        codec: device.codec.map(|c| CodecView {
            active: c.active,
            options: c.options,
        }),
    }
}

fn map_profiles(profiles: &settings_domain::ProfileSnapshot) -> DeviceProfileView {
    fn toggle(state: &ProfileState) -> ProfileToggleView {
        ProfileToggleView {
            available: state.available,
            enabled: state.enabled,
        }
    }
    DeviceProfileView {
        media_audio: toggle(&profiles.media_audio),
        phone_calls: toggle(&profiles.phone_calls),
        contact_sharing: toggle(&profiles.contact_sharing),
        absolute_volume: toggle(&profiles.absolute_volume),
    }
}
