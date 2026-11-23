//! In-memory fakes for every driven port. `Fake` also plays kanshi: reloading
//! applies the matching profile to its scripted topology.

#![allow(dead_code, clippy::disallowed_methods)]

mod hardware;

#[allow(unused_imports)]
pub use hardware::{ddc_display, input};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use settings_application::Ports;
use settings_domain::controls::{
    BrightnessSnapshot, DdcDisplay, DrmProbe, InputSources, ProfileFlags, VCP_BRIGHTNESS,
};
use settings_domain::ports::{
    Clock, LiveOutputs, OutputConfig, PreferencesStore, ProfileController, ProfileStore,
    Subscription, TopologyEvent, TopologyEvents, TopologyReader, TrialJournal,
};
use settings_domain::{
    ConnectorName, Criteria, DisplayKind, DisplayPreferences, Kelvin, Mode, Output, PortError,
    Position, Profile, ProfileName, ProfileOutput, ProfileSet, Scale, Topology, Transform,
    TrialRecord,
};

pub struct World {
    pub topology: Topology,
    pub profiles: ProfileSet,
    pub journal: Option<TrialRecord>,
    pub log: Vec<String>,
    pub reload_fails: bool,
    pub reload_changes_nothing: bool,
    pub offset: Duration,
    pub backlight: Option<u8>,
    pub ddc_displays: Vec<DdcDisplay>,
    pub ddc_values: HashMap<(u32, u8), u32>,
    pub ddc_max: HashMap<(u32, u8), u32>,
    pub overdrive_code: Option<u8>,
    pub inputs: InputSources,
    pub night: Option<Kelvin>,
    pub drm: HashMap<String, DrmProbe>,
    pub flags: ProfileFlags,
    pub brightness_state: Option<BrightnessSnapshot>,
    pub reject_flags: bool,
    pub prefs: DisplayPreferences,
}

pub struct Fake {
    pub world: Mutex<World>,
    base: Instant,
}

pub fn conn(name: &str) -> ConnectorName {
    ConnectorName::try_from(name).unwrap()
}

pub fn mode(text: &str) -> Mode {
    text.parse().unwrap()
}

pub fn output(name: &str, advertised: &[&str], current: &str, x: i32) -> Output {
    let connector = conn(name);
    Output {
        kind: DisplayKind::infer(&connector, ""),
        connector,
        description: format!("fake {name}"),
        make: String::new(),
        model: String::new(),
        serial: String::new(),
        mode: Some(mode(current)),
        position: Position::new(x, 0),
        scale: Scale::clamped(1.0),
        transform: Transform::Normal,
        enabled: true,
        focused: false,
        modes: advertised.iter().map(|m| mode(m)).collect(),
        vrr: false,
        hdr: false,
        ten_bit: false,
    }
}

pub fn laptop() -> Output {
    output(
        "eDP-1",
        &["1920x1080@60.00Hz", "1920x1080@47.99Hz"],
        "1920x1080@59.999",
        0,
    )
}

pub fn ultrawide() -> Output {
    output(
        "DP-3",
        &[
            "2560x1080@60.00Hz",
            "2560x1080@75.00Hz",
            "2560x1080@100.00Hz",
            "1920x1080@60.00Hz",
        ],
        "2560x1080@60",
        1920,
    )
}

fn profile_output(name: &str, mode_text: &str, x: i32) -> ProfileOutput {
    ProfileOutput {
        criteria: Criteria::parse(name),
        enable: Some(true),
        mode: Some(mode(mode_text)),
        position: Some(Position::new(x, 0)),
        scale: Some(Scale::clamped(1.0)),
        transform: Some(Transform::Normal),
        rest_glob: false,
        extra: Vec::new(),
    }
}

pub fn seed_profiles() -> ProfileSet {
    let mut work = Profile::new(ProfileName::parse("work").unwrap());
    work.outputs
        .push(profile_output("eDP-1", "1920x1080@60", 0));
    work.outputs
        .push(profile_output("DP-3", "2560x1080@60", 1920));
    let mut default = Profile::new(ProfileName::parse("default").unwrap());
    default.outputs.push(profile_output("*", "1920x1080@60", 0));
    ProfileSet::new(vec![work, default])
}

impl Fake {
    pub fn new(outputs: Vec<Output>) -> Arc<Self> {
        Arc::new(Self {
            world: Mutex::new(World {
                topology: Topology::new(outputs),
                profiles: seed_profiles(),
                journal: None,
                log: Vec::new(),
                reload_fails: false,
                reload_changes_nothing: false,
                offset: Duration::ZERO,
                backlight: None,
                ddc_displays: Vec::new(),
                ddc_values: HashMap::new(),
                ddc_max: HashMap::from([((5, VCP_BRIGHTNESS), 100), ((5, 0x12), 100)]),
                overdrive_code: None,
                inputs: InputSources::default(),
                night: None,
                drm: HashMap::new(),
                flags: ProfileFlags::default(),
                brightness_state: None,
                reject_flags: false,
                prefs: DisplayPreferences::default(),
            }),
            base: Instant::now(),
        })
    }

    pub fn ports(self: &Arc<Self>) -> Ports {
        self.hardware_ports()
    }

    pub fn with<R>(&self, f: impl FnOnce(&mut World) -> R) -> R {
        f(&mut self.world.lock().unwrap())
    }

    pub fn advance(&self, by: Duration) {
        self.with(|w| w.offset += by);
    }

    pub fn log(&self) -> Vec<String> {
        self.with(|w| w.log.clone())
    }

    pub fn position_of(&self, entry: &str) -> Option<usize> {
        self.log().iter().position(|l| l == entry)
    }

    pub fn current_mode(&self, connector: &str) -> Mode {
        self.with(|w| w.topology.find(&conn(connector)).unwrap().mode.unwrap())
    }

    pub fn profiles(&self) -> ProfileSet {
        self.with(|w| w.profiles.clone())
    }
}

fn set_output(world: &mut World, connector: &ConnectorName, f: impl FnOnce(&mut Output)) {
    let mut outputs = world.topology.outputs().to_vec();
    if let Some(output) = outputs.iter_mut().find(|o| &o.connector == connector) {
        f(output);
    }
    world.topology = Topology::new(outputs);
}

impl TopologyReader for Fake {
    fn read(&self) -> Result<Topology, PortError> {
        Ok(self.with(|w| w.topology.clone()))
    }
}

impl TopologyEvents for Fake {
    fn subscribe(
        &self,
        _sink: Box<dyn Fn(TopologyEvent) + Send>,
    ) -> Result<Subscription, PortError> {
        Ok(Subscription::new(|| {}))
    }
}

impl ProfileStore for Fake {
    fn load(&self) -> Result<ProfileSet, PortError> {
        Ok(self.profiles())
    }

    fn save(&self, set: &ProfileSet) -> Result<(), PortError> {
        self.with(|w| {
            w.log.push("store.save".into());
            w.profiles = set.clone();
        });
        Ok(())
    }
}

impl ProfileController for Fake {
    fn reload(&self) -> Result<(), PortError> {
        self.with(|w| {
            if w.reload_fails {
                w.log.push("controller.reload(fail)".into());
                return Err(PortError::Unavailable("kanshi is down".into()));
            }
            w.log.push("controller.reload".into());
            if w.reload_changes_nothing {
                return Ok(());
            }
            let Some(active) = w.profiles.match_active(&w.topology.identities()) else {
                return Ok(());
            };
            let profile = w.profiles.get(&active).unwrap().clone();
            for entry in &profile.outputs {
                let Some(name) = entry.named() else { continue };
                let connector = conn(name);
                set_output(w, &connector, |o| {
                    o.enabled = entry.enable.unwrap_or(true);
                    if let Some(m) = entry.mode {
                        o.mode = Some(m);
                    }
                    if let Some(p) = entry.position {
                        o.position = p;
                    }
                    if let Some(s) = entry.scale {
                        o.scale = s;
                    }
                    if let Some(t) = entry.transform {
                        o.transform = t;
                    }
                });
            }
            Ok(())
        })
    }

    fn switch(&self, name: &ProfileName) -> Result<(), PortError> {
        self.with(|w| w.log.push(format!("controller.switch {name}")));
        Ok(())
    }
}

impl LiveOutputs for Fake {
    fn apply(&self, config: &OutputConfig) -> Result<(), PortError> {
        self.with(|w| {
            w.log.push("live.apply".into());
            let reject_flags = w.reject_flags;
            set_output(w, &config.connector, |o| {
                o.mode = config.mode;
                o.scale = config.scale;
                o.transform = config.transform;
                o.enabled = config.enabled;
                if reject_flags {
                    return;
                }
                if let Some(vrr) = config.flags.vrr {
                    o.vrr = vrr;
                }
                if let Some(hdr) = config.flags.hdr {
                    o.hdr = hdr;
                }
                if let Some(ten_bit) = config.flags.ten_bit {
                    o.ten_bit = ten_bit;
                }
            });
        });
        Ok(())
    }
}

impl PreferencesStore for Fake {
    fn load(&self) -> Result<DisplayPreferences, PortError> {
        Ok(self.with(|w| w.prefs.clone()))
    }

    fn save(&self, prefs: &DisplayPreferences) -> Result<(), PortError> {
        self.with(|w| w.prefs = prefs.clone());
        Ok(())
    }
}

pub fn absorbing_profile() -> Profile {
    let mut profile = Profile::new(ProfileName::parse("pair").unwrap());
    profile
        .outputs
        .push(profile_output("DP-1", "1920x1080@60", 0));
    profile.outputs.push(ProfileOutput {
        criteria: Criteria::Any,
        enable: Some(true),
        mode: Some(mode("1920x1080@60")),
        position: Some(Position::new(1920, 0)),
        scale: Some(Scale::clamped(1.0)),
        transform: Some(Transform::Normal),
        rest_glob: true,
        extra: Vec::new(),
    });
    profile
}

impl TrialJournal for Fake {
    fn write(&self, record: &TrialRecord) -> Result<(), PortError> {
        self.with(|w| {
            w.log.push("journal.write".into());
            w.journal = Some(record.clone());
        });
        Ok(())
    }

    fn read(&self) -> Result<Option<TrialRecord>, PortError> {
        Ok(self.with(|w| w.journal.clone()))
    }

    fn clear(&self) -> Result<(), PortError> {
        self.with(|w| {
            w.log.push("journal.clear".into());
            w.journal = None;
        });
        Ok(())
    }
}

impl Clock for Fake {
    fn now(&self) -> Instant {
        self.base + self.with(|w| w.offset)
    }

    fn unix_secs(&self) -> u64 {
        1_700_000_000 + self.with(|w| w.offset).as_secs()
    }

    fn sleep(&self, duration: Duration) {
        self.advance(duration);
    }
}
