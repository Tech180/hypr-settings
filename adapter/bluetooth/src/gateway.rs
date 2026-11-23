use std::process::Command;
use std::sync::Mutex;

use bluer::agent::Agent;
use bluer::{Address, Session};
use futures::StreamExt;
use settings_domain::PortError;
use settings_domain::bluetooth::{
    AdapterSnapshot, BluetoothCapabilities, CodecId, DeviceId, DeviceSnapshot, ProfileKind,
    ProfileSnapshot, ProfileState, class_from_uuids, pick_adapter,
};
use settings_domain::ports::BluetoothGateway;

use crate::runtime::block_on;

const UUID_A2DP: &str = "0000110b-0000-1000-8000-00805f9b34fb";
const UUID_HFP: &str = "0000111e-0000-1000-8000-00805f9b34fb";
const UUID_PBAP: &str = "00001130-0000-1000-8000-00805f9b34fb";
const UUID_MAP: &str = "00001132-0000-1000-8000-00805f9b34fb";

struct Inner {
    session: Option<Session>,
    agent: Option<bluer::agent::AgentHandle>,
    discovery: Option<tokio::task::JoinHandle<()>>,
    /// MAC address of the adapter the user chose; `None` means the first BlueZ lists.
    wanted_address: Option<String>,
}

/// BlueZ gateway via `bluer`.
pub struct BluerGateway {
    inner: Mutex<Inner>,
    capabilities: BluetoothCapabilities,
}

impl Default for BluerGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl BluerGateway {
    pub fn new() -> Self {
        let pipewire = command_available("pw-cli");
        let bluetoothctl = command_available("bluetoothctl");
        Self {
            inner: Mutex::new(Inner {
                session: None,
                agent: None,
                discovery: None,
                wanted_address: None,
            }),
            capabilities: BluetoothCapabilities {
                codec_switch: pipewire,
                pan_tethering: probe_network_manager(),
                auracast: probe_auracast(),
                avrcp_config: true,
                sample_rate: pipewire,
                a2dp_offload: true,
                hci_snoop: true,
                gatt_cache_clear: bluetoothctl,
            },
        }
    }

    fn session(&self) -> Result<Session, PortError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| PortError::Unavailable("lock".into()))?;
        if inner.session.is_none() {
            let session = block_on(Session::new()).map_err(map_err)?;
            if inner.agent.is_none() {
                let agent = Agent {
                    request_default: true,
                    ..Default::default()
                };
                inner.agent = Some(block_on(session.register_agent(agent)).map_err(map_err)?);
            }
            inner.session = Some(session);
        }
        Ok(inner.session.as_ref().expect("session").clone())
    }

    fn active_adapter(&self) -> Result<bluer::Adapter, PortError> {
        let wanted = self
            .inner
            .lock()
            .map_err(|_| PortError::Unavailable("lock".into()))?
            .wanted_address
            .clone();
        let session = self.session()?;
        block_on(async {
            let names = session.adapter_names().await.map_err(map_err)?;
            if names.is_empty() {
                return Err(PortError::Unavailable("no bluetooth adapter".into()));
            }
            let mut snapshots = Vec::with_capacity(names.len());
            for name in &names {
                let adapter = session.adapter(name).map_err(map_err)?;
                snapshots.push(adapter_snapshot(&adapter).await?);
            }
            let index = pick_adapter(&snapshots, wanted.as_deref())
                .ok_or_else(|| PortError::Unavailable("no bluetooth adapter".into()))?;
            session.adapter(&snapshots[index].name).map_err(map_err)
        })
    }

    fn device(&self, id: &DeviceId) -> Result<bluer::Device, PortError> {
        let address: Address = id
            .as_str()
            .parse()
            .map_err(|_| PortError::Invalid(format!("bad device id {}", id)))?;
        let adapter = self.active_adapter()?;
        adapter.device(address).map_err(map_err)
    }
}

impl BluetoothGateway for BluerGateway {
    fn adapters(&self) -> Result<Vec<AdapterSnapshot>, PortError> {
        let session = self.session()?;
        block_on(async {
            let mut out = Vec::new();
            for name in session.adapter_names().await.map_err(map_err)? {
                let adapter = session.adapter(&name).map_err(map_err)?;
                out.push(adapter_snapshot(&adapter).await?);
            }
            Ok(out)
        })
    }

    fn set_active_adapter(&self, address: Option<&str>) -> Result<(), PortError> {
        self.stop_discovery()?;
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| PortError::Unavailable("lock".into()))?;
        inner.wanted_address = address.map(str::to_owned);
        Ok(())
    }

    fn adapter(&self) -> Result<Option<AdapterSnapshot>, PortError> {
        match self.active_adapter() {
            Ok(adapter) => block_on(async { Ok(Some(adapter_snapshot(&adapter).await?)) }),
            Err(PortError::Unavailable(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn devices(&self) -> Result<Vec<DeviceSnapshot>, PortError> {
        let adapter = self.active_adapter()?;
        block_on(async {
            let mut out = Vec::new();
            for address in adapter.device_addresses().await.map_err(map_err)? {
                let device = adapter.device(address).map_err(map_err)?;
                if let Ok(snapshot) = device_snapshot(&device).await {
                    out.push(snapshot);
                }
            }
            Ok(out)
        })
    }

    fn set_powered(&self, on: bool) -> Result<(), PortError> {
        let adapter = self.active_adapter()?;
        block_on(adapter.set_powered(on)).map_err(map_err)?;
        // Turning the radio on never starts a scan; the user asks for that explicitly.
        if !on {
            self.stop_discovery()?;
        }
        Ok(())
    }

    fn set_discoverable(&self, on: bool) -> Result<(), PortError> {
        let adapter = self.active_adapter()?;
        block_on(adapter.set_discoverable(on)).map_err(map_err)
    }

    fn start_discovery(&self) -> Result<(), PortError> {
        let adapter = self.active_adapter()?;
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| PortError::Unavailable("lock".into()))?;
        if inner.discovery.is_some() {
            return Ok(());
        }
        let handle = block_on(async {
            let mut stream = adapter.discover_devices().await.map_err(map_err)?;
            Ok(tokio::spawn(async move {
                while stream.next().await.is_some() {}
            }))
        })?;
        inner.discovery = Some(handle);
        Ok(())
    }

    fn stop_discovery(&self) -> Result<(), PortError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| PortError::Unavailable("lock".into()))?;
        if let Some(handle) = inner.discovery.take() {
            handle.abort();
        }
        Ok(())
    }

    fn pair(&self, id: &DeviceId) -> Result<(), PortError> {
        let device = self.device(id)?;
        block_on(async {
            device.set_trusted(true).await.map_err(map_err)?;
            device.pair().await.map_err(map_err)
        })
    }

    fn trust(&self, id: &DeviceId, trust: bool) -> Result<(), PortError> {
        let device = self.device(id)?;
        block_on(device.set_trusted(trust)).map_err(map_err)
    }

    fn connect(&self, id: &DeviceId) -> Result<(), PortError> {
        let device = self.device(id)?;
        block_on(device.connect()).map_err(map_err)
    }

    fn disconnect(&self, id: &DeviceId) -> Result<(), PortError> {
        let device = self.device(id)?;
        block_on(device.disconnect()).map_err(map_err)
    }

    fn remove(&self, id: &DeviceId) -> Result<(), PortError> {
        let address: Address = id
            .as_str()
            .parse()
            .map_err(|_| PortError::Invalid(format!("bad device id {}", id)))?;
        let adapter = self.active_adapter()?;
        block_on(async {
            if let Ok(device) = adapter.device(address)
                && device.is_connected().await.unwrap_or(false)
            {
                let _ = device.disconnect().await;
            }
            adapter.remove_device(address).await.map_err(map_err)
        })
    }

    fn set_profile(
        &self,
        id: &DeviceId,
        profile: ProfileKind,
        enable: bool,
    ) -> Result<(), PortError> {
        let device = self.device(id)?;
        block_on(async {
            let uuid = profile_uuid(profile);
            if enable {
                device.connect_profile(&uuid).await.map_err(map_err)
            } else {
                device.disconnect_profile(&uuid).await.map_err(map_err)
            }
        })
    }

    fn set_codec(&self, _id: &DeviceId, _codec: &CodecId) -> Result<(), PortError> {
        Err(PortError::Unavailable("codec switch not configured".into()))
    }

    fn set_absolute_volume(&self, _id: &DeviceId, _on: bool) -> Result<(), PortError> {
        Ok(())
    }

    fn capabilities(&self) -> BluetoothCapabilities {
        self.capabilities.clone()
    }
}

fn profile_uuid(profile: ProfileKind) -> uuid::Uuid {
    use settings_domain::bluetooth::ProfileKind::*;
    let s = match profile {
        MediaAudio => UUID_A2DP,
        PhoneCalls => UUID_HFP,
        ContactSharing => UUID_PBAP,
        AbsoluteVolume => UUID_A2DP,
    };
    s.parse().expect("uuid")
}

async fn adapter_snapshot(adapter: &bluer::Adapter) -> Result<AdapterSnapshot, PortError> {
    Ok(AdapterSnapshot {
        name: adapter.name().to_owned(),
        address: adapter.address().await.map_err(map_err)?.to_string(),
        alias: adapter.alias().await.map_err(map_err)?,
        powered: adapter.is_powered().await.map_err(map_err)?,
        discoverable: adapter.is_discoverable().await.map_err(map_err)?,
        discovering: adapter.is_discovering().await.map_err(map_err)?,
        lmp_version: None,
        modalias: None,
    })
}

async fn device_snapshot(device: &bluer::Device) -> Result<DeviceSnapshot, bluer::Error> {
    let address = device.address().to_string();
    let uuids: Vec<String> = device
        .uuids()
        .await?
        .unwrap_or_default()
        .into_iter()
        .map(|u| u.to_string())
        .collect();
    let uuids_lower: Vec<_> = uuids.iter().map(|u| u.to_ascii_lowercase()).collect();
    let has = |uuid: &str| uuids_lower.iter().any(|u| u == uuid);
    let connected = device.is_connected().await?;
    let profiles = ProfileSnapshot {
        media_audio: ProfileState {
            available: has(UUID_A2DP),
            enabled: connected && has(UUID_A2DP),
        },
        phone_calls: ProfileState {
            available: has(UUID_HFP),
            enabled: connected && has(UUID_HFP),
        },
        contact_sharing: ProfileState {
            available: has(UUID_PBAP) || has(UUID_MAP),
            enabled: false,
        },
        absolute_volume: ProfileState {
            available: has(UUID_A2DP),
            enabled: false,
        },
    };
    Ok(DeviceSnapshot {
        id: DeviceId::new(address.clone()),
        name: device.name().await?.unwrap_or_else(|| address.clone()),
        address,
        paired: device.is_paired().await?,
        connected,
        trusted: device.is_trusted().await?,
        battery_percent: device.battery_percentage().await?,
        class: class_from_uuids(&uuids),
        uuids,
        profiles,
        codec: None,
    })
}

fn map_err(err: bluer::Error) -> PortError {
    PortError::Unavailable(err.to_string())
}

fn probe_network_manager() -> bool {
    Command::new("nmcli")
        .args(["-t", "-f", "RUNNING", "general"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Whether `program` is on PATH and runs, without printing its version banner.
fn command_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn probe_auracast() -> bool {
    std::fs::read_to_string("/etc/bluetooth/main.conf")
        .map(|text| text.contains("Experimental = true") || text.contains("Experimental=true"))
        .unwrap_or(false)
}
