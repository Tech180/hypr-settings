use settings_domain::network::{
    VpnCandidate, VpnConfigKind, VpnDraft, VpnManager, VpnPeerDraft, VpnProfile, config_kind_for,
    draft_to_mirror, parse_wg_conf, peers_differ, render_wg_conf, validate_vpn_draft,
};

use super::NetworkCore;
use crate::error::AppError;
use crate::network_dto::VpnView;

fn rejected(text: impl Into<String>) -> AppError {
    AppError::Rejected(text.into())
}

impl NetworkCore {
    pub(crate) fn vpn_view(&self) -> Result<VpnView, AppError> {
        Ok(VpnView {
            profiles: self.ports.vpn.profiles()?,
        })
    }

    fn find_vpn(&self, id: &str) -> Result<VpnProfile, AppError> {
        self.ports
            .vpn
            .profiles()?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| rejected("That tunnel no longer exists."))
    }

    pub(crate) fn activate_vpn(&self, id: &str) -> Result<(), AppError> {
        let profile = self.find_vpn(id)?;
        let result = match profile.manager {
            VpnManager::Systemd => self.ports.units.start(&profile.systemd_unit),
            VpnManager::NetworkManager => self.ports.vpn.up(&profile.id),
        };
        match result {
            Ok(()) => {
                self.info(format!("{} connected.", profile.name));
                Ok(())
            }
            Err(err) => {
                self.error(format!("Failed to connect {}: {err}", profile.name));
                Err(rejected(format!("Failed to connect {}.", profile.name)))
            }
        }
    }

    pub(crate) fn deactivate_vpn(&self, id: &str) -> Result<(), AppError> {
        let profile = self.find_vpn(id)?;
        let result = match profile.manager {
            VpnManager::Systemd => self.ports.units.stop(&profile.systemd_unit),
            VpnManager::NetworkManager => self.ports.vpn.down(&profile.id),
        };
        match result {
            Ok(()) => {
                self.warn(format!("{} disconnected.", profile.name));
                Ok(())
            }
            Err(err) => {
                self.error(format!("Failed to disconnect {}: {err}", profile.name));
                Err(rejected(format!("Failed to disconnect {}.", profile.name)))
            }
        }
    }

    pub(crate) fn vpn_candidates(&self) -> Vec<VpnCandidate> {
        self.ports.files.candidates()
    }

    /// The form's starting point for editing a NetworkManager tunnel.
    pub(crate) fn load_vpn_draft(&self, id: &str) -> Result<VpnDraft, AppError> {
        let profile = self.find_vpn(id)?;
        if !profile.editable || profile.manager != VpnManager::NetworkManager {
            return Err(rejected("System tunnels cannot be edited here."));
        }
        let private_key = self.ports.vpn.private_key(&profile.id).unwrap_or_default();
        let mirror = if profile.uuid.is_empty() {
            None
        } else {
            self.ports
                .mirror
                .load()
                .ok()
                .and_then(|store| store.profiles.get(&profile.uuid).cloned())
        };

        let pick = |mirrored: Option<&String>, fallback: String| {
            mirrored
                .filter(|v| !v.is_empty())
                .cloned()
                .unwrap_or(fallback)
        };
        let mut draft = VpnDraft {
            id: profile.id.clone(),
            uuid: profile.uuid.clone(),
            name: profile.name.clone(),
            interface_name: profile.interface_name.clone(),
            private_key,
            addresses: pick(
                mirror.as_ref().map(|m| &m.addresses),
                if profile.ipv4_address.is_empty() {
                    profile.ipv6_address.clone()
                } else {
                    profile.ipv4_address.clone()
                },
            ),
            dns: pick(mirror.as_ref().map(|m| &m.dns), profile.dns.join(", ")),
            mtu: pick(
                mirror.as_ref().map(|m| &m.mtu),
                profile.mtu.map(|m| m.to_string()).unwrap_or_default(),
            ),
            listen_port: pick(
                mirror.as_ref().map(|m| &m.listen_port),
                profile
                    .listen_port
                    .map(|p| p.to_string())
                    .unwrap_or_default(),
            ),
            autoconnect: mirror
                .as_ref()
                .map_or(profile.autoconnect, |m| m.autoconnect),
            peers: Vec::new(),
        };
        draft.peers = match &mirror {
            Some(m) if !m.peers.is_empty() => m
                .peers
                .iter()
                .map(|p| VpnPeerDraft {
                    public_key: p.public_key.clone(),
                    preshared_key: String::new(),
                    endpoint: p.endpoint.clone(),
                    allowed_ips: p.allowed_ips.clone(),
                    persistent_keepalive: p.persistent_keepalive.clone(),
                })
                .collect(),
            _ if !profile.peer_public_key.is_empty() => vec![VpnPeerDraft {
                public_key: profile.peer_public_key.clone(),
                ..VpnPeerDraft::default()
            }],
            _ => vec![VpnPeerDraft::default()],
        };
        Ok(draft)
    }

    fn mirror_upsert(&self, draft: &VpnDraft) {
        let Some(entry) = draft_to_mirror(draft) else {
            return;
        };
        let mut store = self.ports.mirror.load().unwrap_or_default();
        store.profiles.insert(entry.uuid.clone(), entry);
        if let Err(err) = self.ports.mirror.save(&store) {
            self.warn(format!("Could not save the VPN mirror: {err}"));
        }
    }

    fn mirror_remove(&self, uuid: &str) {
        if uuid.is_empty() {
            return;
        }
        let mut store = self.ports.mirror.load().unwrap_or_default();
        if store.profiles.remove(uuid).is_some()
            && let Err(err) = self.ports.mirror.save(&store)
        {
            self.warn(format!("Could not save the VPN mirror: {err}"));
        }
    }

    pub(crate) fn save_vpn_draft(&self, draft: VpnDraft) -> Result<(), AppError> {
        if let Some(problem) = validate_vpn_draft(&draft) {
            return Err(rejected(problem));
        }
        let _guard = self.vpn_lock.lock().expect("vpn lock");
        if draft.id.is_empty() {
            self.create_vpn(draft)
        } else {
            self.update_vpn(draft)
        }
    }

    /// Imports the rendered conf, then applies the form's basics and records the mirror.
    fn import_draft(&self, draft: &VpnDraft) -> Result<String, AppError> {
        let conf = render_wg_conf(draft);
        let imported = self.ports.vpn.import_wireguard_text(&conf).map_err(|e| {
            self.error(format!("VPN import failed: {e}"));
            rejected(format!("Failed to create VPN: {e}"))
        })?;
        let imported = if imported.is_empty() {
            draft.name.trim().to_owned()
        } else {
            imported
        };
        if let Err(err) = self.ports.vpn.finalize_import(&imported, draft) {
            self.warn(format!("Could not finish the import of {imported}: {err}"));
        }
        let final_name = if draft.name.trim().is_empty() {
            imported
        } else {
            draft.name.trim().to_owned()
        };
        if let Ok(uuid) = self.ports.vpn.uuid_of(&final_name)
            && !uuid.is_empty()
        {
            self.mirror_upsert(&VpnDraft {
                uuid,
                id: final_name.clone(),
                ..draft.clone()
            });
        }
        Ok(final_name)
    }

    fn create_vpn(&self, draft: VpnDraft) -> Result<(), AppError> {
        let name = self.import_draft(&draft)?;
        self.info(format!("VPN {name} saved."));
        Ok(())
    }

    fn update_vpn(&self, draft: VpnDraft) -> Result<(), AppError> {
        let existing = self.find_vpn(&draft.id)?;
        if !existing.editable {
            return Err(rejected("Cannot edit this tunnel."));
        }

        let mirror = if draft.uuid.is_empty() {
            None
        } else {
            self.ports
                .mirror
                .load()
                .ok()
                .and_then(|s| s.profiles.get(&draft.uuid).cloned())
        };
        let prior: Vec<VpnPeerDraft> = match mirror {
            Some(m) => m
                .peers
                .into_iter()
                .map(|p| VpnPeerDraft {
                    public_key: p.public_key,
                    preshared_key: String::new(),
                    endpoint: p.endpoint,
                    allowed_ips: p.allowed_ips,
                    persistent_keepalive: p.persistent_keepalive,
                })
                .collect(),
            None if !existing.peer_public_key.is_empty() => vec![VpnPeerDraft {
                public_key: existing.peer_public_key.clone(),
                ..VpnPeerDraft::default()
            }],
            None => Vec::new(),
        };

        if peers_differ(&prior, &draft.peers) {
            // nmcli has no peers property: replace the tunnel.
            let was_active = existing.active;
            self.ports.vpn.delete(&draft.id).map_err(|e| {
                self.error(format!("VPN replace failed: {e}"));
                rejected(format!("Failed to replace VPN {}.", draft.name))
            })?;
            self.mirror_remove(&draft.uuid);
            let name = self.import_draft(&draft)?;
            if was_active {
                let _ = self.ports.vpn.up(&name);
            }
            self.info(format!("VPN {name} updated."));
            return Ok(());
        }

        self.ports
            .vpn
            .modify_basics(&draft.id, &draft)
            .map_err(|e| {
                self.error(format!("VPN update failed: {e}"));
                rejected(format!("Failed to update {}.", draft.name))
            })?;
        let lookup = if draft.name.trim().is_empty() {
            draft.id.clone()
        } else {
            draft.name.trim().to_owned()
        };
        let uuid = if draft.uuid.is_empty() {
            self.ports.vpn.uuid_of(&lookup).unwrap_or_default()
        } else {
            draft.uuid.clone()
        };
        self.mirror_upsert(&VpnDraft {
            uuid,
            ..draft.clone()
        });
        self.info(format!("VPN {} updated.", draft.name));
        Ok(())
    }

    pub(crate) fn delete_vpn(&self, id: &str) -> Result<(), AppError> {
        let profile = self.find_vpn(id)?;
        if !profile.editable || profile.manager != VpnManager::NetworkManager {
            return Err(rejected("System tunnels cannot be deleted here."));
        }
        match self.ports.vpn.delete(&profile.id) {
            Ok(()) => {
                self.mirror_remove(&profile.uuid);
                self.warn(format!("VPN {} deleted.", profile.name));
                Ok(())
            }
            Err(err) => {
                self.error(format!("VPN delete failed: {err}"));
                Err(rejected(format!("Failed to delete {}.", profile.name)))
            }
        }
    }

    pub(crate) fn import_vpn(&self, path: &str) -> Result<String, AppError> {
        let path = path.trim();
        if !path.starts_with('/') {
            return Err(rejected("Enter an absolute path to a .conf or .ovpn file."));
        }
        let kind = config_kind_for(path)
            .ok_or_else(|| rejected("Unsupported file. Use .conf (WireGuard) or .ovpn."))?;
        let name = self.ports.vpn.import_file(kind, path).map_err(|e| {
            self.error(format!("VPN import failed: {e}"));
            rejected(format!("Import failed: {e}"))
        })?;

        if kind == VpnConfigKind::WireGuard
            && !name.is_empty()
            && let Ok(text) = self.ports.files.read_text(path)
        {
            let mut draft = parse_wg_conf(&text, &name);
            draft.name.clone_from(&name);
            if let Ok(uuid) = self.ports.vpn.uuid_of(&name) {
                draft.uuid = uuid;
                draft.id.clone_from(&name);
                self.mirror_upsert(&draft);
            }
        }
        self.info(format!(
            "Imported {}.",
            if name.is_empty() { "VPN" } else { &name }
        ));
        Ok(name)
    }
}
