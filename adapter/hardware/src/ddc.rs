//! DDC/CI through `ddcutil`. Everything that knows ddcutil's text format lives here.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use settings_domain::PortError;
use settings_domain::controls::{
    DdcDisplay, InputSource, InputSources, VCP_INPUT_SOURCE, input_label, percent_from_raw,
    raw_from_percent,
};
use settings_domain::ports::Ddc;

use crate::exec::{Exec, SystemExec};

pub struct DdcutilDdc {
    exec: Arc<dyn Exec>,
    cache: Mutex<Cache>,
    /// ddcutil is serialised per bus; displays on different buses run in parallel.
    buses: Mutex<HashMap<u32, Arc<Mutex<()>>>>,
}

#[derive(Default)]
struct Cache {
    detected: Option<Vec<DdcDisplay>>,
    /// (bus, code) -> maximum raw value
    max: HashMap<(u32, u8), u32>,
    overdrive: HashMap<u32, Option<u8>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl DdcutilDdc {
    pub fn new() -> Self {
        Self::with_exec(Arc::new(SystemExec))
    }

    pub fn with_exec(exec: Arc<dyn Exec>) -> Self {
        Self {
            exec,
            cache: Mutex::default(),
            buses: Mutex::default(),
        }
    }

    fn bus_lock(&self, bus: u32) -> Arc<Mutex<()>> {
        lock(&self.buses).entry(bus).or_default().clone()
    }

    fn ddcutil(&self, bus: u32, args: &[&str]) -> Result<String, PortError> {
        let bus_arg = bus.to_string();
        let mut full = vec!["--bus", bus_arg.as_str()];
        full.extend_from_slice(args);
        self.exec
            .run("ddcutil", &full)
            .map_err(|e| PortError::Unavailable(e.to_string()))
    }

    /// Current and maximum raw value of a continuous feature.
    fn read_continuous(&self, bus: u32, code: u8) -> Option<(u32, u32)> {
        let hex = format!("{code:x}");
        let out = self.ddcutil(bus, &["getvcp", &hex, "--terse"]).ok()?;
        let (current, max) = parse_continuous(&out, code)?;
        lock(&self.cache).max.insert((bus, code), max);
        Some((current, max))
    }
}

impl Default for DdcutilDdc {
    fn default() -> Self {
        Self::new()
    }
}

/// `VCP 10 C 50 100` (terse) or the verbose `current value = 50, max value = 100`.
pub fn parse_continuous(text: &str, code: u8) -> Option<(u32, u32)> {
    let wanted = format!("{code:x}");
    for line in text.lines() {
        let mut words = line.split_whitespace();
        if words.next().is_some_and(|w| w.eq_ignore_ascii_case("VCP"))
            && words
                .next()
                .is_some_and(|w| w.eq_ignore_ascii_case(&wanted))
            && words.next() == Some("C")
        {
            let current = words.next()?.parse().ok()?;
            let max = words.next()?.parse().ok()?;
            return (max > 0).then_some((current, max));
        }
    }
    let after = |key: &str| -> Option<u32> {
        let lower = text.to_lowercase();
        let at = lower.find(key)? + key.len();
        let digits: String = lower[at..]
            .trim_start_matches([' ', '='])
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.parse().ok()
    };
    let (current, max) = (after("current value")?, after("max value")?);
    (max > 0).then_some((current, max))
}

/// `ddcutil detect`: one block per display, with its bus, DRM connector, model and serial.
pub fn parse_detect(text: &str) -> Vec<DdcDisplay> {
    let mut displays = Vec::new();
    let mut block: Vec<&str> = Vec::new();
    let mut flush = |block: &mut Vec<&str>| {
        if let Some(display) = parse_detect_block(block) {
            displays.push(display);
        }
        block.clear();
    };
    for line in text.lines() {
        if line.starts_with("Display ") || line.starts_with("Invalid display") {
            flush(&mut block);
        }
        block.push(line);
    }
    flush(&mut block);
    displays
}

fn parse_detect_block(block: &[&str]) -> Option<DdcDisplay> {
    // Only numbered displays are usable: laptops and invalid entries have no index.
    let header = block.first()?.strip_prefix("Display ")?;
    header
        .split_whitespace()
        .next()?
        .parse::<u32>()
        .ok()
        .filter(|n| *n > 0)?;
    let field = |name: &str| {
        block.iter().find_map(|line| {
            let (key, value) = line.trim().split_once(':')?;
            (key.trim() == name).then(|| value.trim().to_owned())
        })
    };
    let bus = field("I2C bus")?.strip_prefix("/dev/i2c-")?.parse().ok()?;
    // `card1-DP-3` -> `DP-3`
    let connector = field("DRM_connector").and_then(|value| {
        let (card, name) = value.split_once('-')?;
        card.starts_with("card").then(|| name.to_owned())
    });
    Some(DdcDisplay {
        bus,
        connector,
        model: field("Model").unwrap_or_default(),
        serial: field("Serial number").unwrap_or_default(),
    })
}

const OVERDRIVE_WORDS: [&str; 5] = [
    "overdrive",
    "response time",
    "pixel response",
    "od setting",
    "over drive",
];

/// The `capabilities` feature whose name says overdrive or response time.
pub fn parse_overdrive_code(capabilities: &str) -> Option<u8> {
    capabilities.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("Feature:")?.trim();
        let (code, name) = rest.split_once('(')?;
        let name = name.to_lowercase();
        OVERDRIVE_WORDS
            .iter()
            .any(|w| name.contains(w))
            .then(|| u8::from_str_radix(code.trim(), 16).ok())
            .flatten()
    })
}

/// The selected input of `getvcp 60`: `(sl=0x0f)`, `current value = 15` or terse `VCP 60 SNC x0f`.
pub fn parse_input_current(text: &str) -> Option<u8> {
    let lower = text.to_lowercase();
    if let Some(at) = lower.find("sl=0x") {
        return u8::from_str_radix(lower.get(at + 5..at + 7)?, 16).ok();
    }
    if let Some(at) = lower.find("current value") {
        let value: String = lower[at..]
            .split('=')
            .nth(1)?
            .trim_start()
            .trim_start_matches("0x")
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        return value.parse().ok();
    }
    let terse = lower.lines().find(|l| l.starts_with("vcp 60 snc"))?;
    let value = terse.split_whitespace().nth(3)?.trim_start_matches('x');
    u8::from_str_radix(value, 16).ok()
}

/// `Feature: 60 (Input Source)` followed by indented `0f: DisplayPort-1` value lines.
pub fn parse_input_options(capabilities: &str) -> Vec<InputSource> {
    let mut lines = capabilities.lines();
    let mut found = false;
    for line in lines.by_ref() {
        let feature = line.trim().to_lowercase();
        if feature.starts_with("feature: 60") && feature.contains("input source") {
            found = true;
            break;
        }
    }
    if !found {
        return Vec::new();
    }
    let mut options = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.to_lowercase().starts_with("feature:") {
            break;
        }
        let Some((code, label)) = line.split_once(':') else {
            continue;
        };
        let Ok(code) = u8::from_str_radix(code.trim(), 16) else {
            continue;
        };
        let label = label.trim();
        options.push(InputSource {
            label: if label.is_empty() || label.eq_ignore_ascii_case("unrecognized value") {
                input_label(code)
            } else {
                label.to_owned()
            },
            code,
        });
    }
    options
}

impl Ddc for DdcutilDdc {
    fn detect(&self, force: bool) -> Vec<DdcDisplay> {
        if !force && let Some(cached) = lock(&self.cache).detected.clone() {
            return cached;
        }
        let displays = match self.exec.run("ddcutil", &["detect"]) {
            Ok(text) => parse_detect(&text),
            Err(err) => {
                tracing::warn!(%err, "ddcutil detect failed");
                return Vec::new();
            }
        };
        let mut cache = lock(&self.cache);
        cache.detected = Some(displays.clone());
        if force {
            cache.max.clear();
            cache.overdrive.clear();
        }
        displays
    }

    fn read_percent(&self, bus: u32, code: u8) -> Option<u8> {
        let bus_lock = self.bus_lock(bus);
        let _serial = lock(&bus_lock);
        let (current, max) = self.read_continuous(bus, code)?;
        percent_from_raw(current, max)
    }

    fn write_percent(&self, bus: u32, code: u8, percent: u8) -> Result<(), PortError> {
        let bus_lock = self.bus_lock(bus);
        let _serial = lock(&bus_lock);
        let cached = lock(&self.cache).max.get(&(bus, code)).copied();
        let max = match cached {
            Some(max) => max,
            None => {
                self.read_continuous(bus, code)
                    .ok_or_else(|| PortError::Unavailable(format!("bus {bus} did not answer")))?
                    .1
            }
        };
        let raw = raw_from_percent(percent, max).to_string();
        let hex = format!("{code:x}");
        match self.ddcutil(bus, &["setvcp", &hex, &raw, "--noverify"]) {
            Ok(_) => Ok(()),
            // Some displays apply the value but fail ddcutil's read-back.
            Err(PortError::Unavailable(msg))
                if msg.contains("DDCRC_VERIFY") || msg.contains("Verification failed") =>
            {
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn overdrive_code(&self, bus: u32) -> Option<u8> {
        if let Some(known) = lock(&self.cache).overdrive.get(&bus) {
            return *known;
        }
        let bus_lock = self.bus_lock(bus);
        let _serial = lock(&bus_lock);
        let code = self
            .ddcutil(bus, &["capabilities"])
            .ok()
            .and_then(|text| parse_overdrive_code(&text));
        lock(&self.cache).overdrive.insert(bus, code);
        code
    }

    fn input_sources(&self, bus: u32) -> InputSources {
        let bus_lock = self.bus_lock(bus);
        let _serial = lock(&bus_lock);
        let hex = format!("{VCP_INPUT_SOURCE:x}");
        let current_code = self
            .ddcutil(bus, &["getvcp", &hex])
            .ok()
            .and_then(|out| parse_input_current(&out));
        let mut options = self
            .ddcutil(bus, &["capabilities"])
            .map(|text| parse_input_options(&text))
            .unwrap_or_default();
        if options.is_empty()
            && let Some(code) = current_code
        {
            options = [code, 0x0f, 0x11, 0x12]
                .into_iter()
                .fold(Vec::new(), |mut all: Vec<u8>, c| {
                    if !all.contains(&c) {
                        all.push(c);
                    }
                    all
                })
                .into_iter()
                .map(|code| InputSource {
                    label: input_label(code),
                    code,
                })
                .collect();
        }
        let current = current_code.map(|code| {
            options
                .iter()
                .find(|o| o.code == code)
                .cloned()
                .unwrap_or_else(|| InputSource {
                    label: input_label(code),
                    code,
                })
        });
        if let Some(current) = &current
            && !options.iter().any(|o| o.code == current.code)
        {
            options.insert(0, current.clone());
        }
        InputSources { current, options }
    }

    fn set_input_source(&self, bus: u32, code: u8) -> Result<(), PortError> {
        let bus_lock = self.bus_lock(bus);
        let _serial = lock(&bus_lock);
        let hex = format!("{VCP_INPUT_SOURCE:x}");
        let value = code.to_string();
        match self.ddcutil(bus, &["setvcp", &hex, &value, "--noverify"]) {
            Err(PortError::Unavailable(msg))
                if msg.contains("DDCRC_VERIFY") || msg.contains("Verification failed") =>
            {
                Ok(())
            }
            other => other.map(|_| ()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::exec::ExecError;

    const DETECT: &str = "\
Invalid display
   I2C bus:  /dev/i2c-1
   Reason: Laptop displays do not support DDC/CI

Display 1
   I2C bus:             /dev/i2c-5
   DRM_connector:       card1-DP-3
   Monitor:             ACR:Acme 34:ABC123
   Model:               Acme 34
   Serial number:       A1B2C3

Display 2
   I2C bus:             /dev/i2c-7
   Model:               Other 27
   Serial number:       ZZ9
";

    const CAPS: &str = "\
Model: Acme
VCP Features:
   Feature: 10 (Brightness)
   Feature: 60 (Input Source)
      Values:
         0f: DisplayPort-1
         11: HDMI-1
         12: Unrecognized value
   Feature: F0 (Overdrive Level)
      Values: 0-3
";

    #[test]
    fn detect_keeps_only_numbered_displays_and_strips_the_card() {
        let displays = parse_detect(DETECT);
        assert_eq!(displays.len(), 2);
        assert_eq!(
            (displays[0].bus, displays[0].connector.as_deref()),
            (5, Some("DP-3"))
        );
        assert_eq!(displays[0].model, "Acme 34");
        assert_eq!(displays[0].serial, "A1B2C3");
        assert_eq!((displays[1].bus, displays[1].connector.clone()), (7, None));
    }

    #[test]
    fn continuous_values_parse_in_both_forms() {
        assert_eq!(parse_continuous("VCP 10 C 50 100\n", 0x10), Some((50, 100)));
        assert_eq!(parse_continuous("VCP 12 C 5 0", 0x12), None);
        assert_eq!(parse_continuous("VCP 10 C 50 100", 0x12), None);
        assert_eq!(
            parse_continuous(
                "VCP code 0x10 (Brightness): current value =    35, max value =   100",
                0x10
            ),
            Some((35, 100))
        );
        assert_eq!(parse_continuous("Display not found", 0x10), None);
    }

    #[test]
    fn capabilities_name_the_overdrive_feature_and_the_inputs() {
        assert_eq!(parse_overdrive_code(CAPS), Some(0xf0));
        assert_eq!(parse_overdrive_code("Feature: 10 (Brightness)"), None);
        let inputs = parse_input_options(CAPS);
        assert_eq!(
            inputs
                .iter()
                .map(|i| (i.code, i.label.as_str()))
                .collect::<Vec<_>>(),
            [(0x0f, "DisplayPort-1"), (0x11, "HDMI-1"), (0x12, "HDMI 2")]
        );
        assert!(parse_input_options("Feature: 10 (Brightness)").is_empty());
    }

    #[test]
    fn the_current_input_reads_in_every_form() {
        assert_eq!(
            parse_input_current("VCP code 0x60 (Input Source): DisplayPort-1 (sl=0x0f)"),
            Some(0x0f)
        );
        assert_eq!(parse_input_current("current value = 17"), Some(17));
        assert_eq!(parse_input_current("VCP 60 SNC x11 x00"), Some(0x11));
        assert_eq!(parse_input_current("nothing"), None);
    }

    struct Scripted {
        calls: Mutex<Vec<String>>,
        brightness_max: u32,
    }

    impl Exec for Scripted {
        fn run(&self, program: &str, args: &[&str]) -> Result<String, ExecError> {
            let call = format!("{program} {}", args.join(" "));
            self.calls.lock().unwrap().push(call.clone());
            Ok(match args {
                ["detect"] => DETECT.to_owned(),
                ["--bus", "5", "getvcp", "10", "--terse"] => {
                    format!("VCP 10 C 25 {}\n", self.brightness_max)
                }
                ["--bus", "5", "getvcp", "60"] => {
                    "VCP code 0x60 (Input Source): DisplayPort-1 (sl=0x0f)".to_owned()
                }
                ["--bus", "5", "capabilities"] => CAPS.to_owned(),
                _ => String::new(),
            })
        }

        fn spawn_detached(&self, _: &str, _: &[&str]) -> Result<(), ExecError> {
            unreachable!()
        }
    }

    fn ddc(max: u32) -> (Arc<Scripted>, DdcutilDdc) {
        let exec = Arc::new(Scripted {
            calls: Mutex::default(),
            brightness_max: max,
        });
        (exec.clone(), DdcutilDdc::with_exec(exec))
    }

    #[test]
    fn detection_is_cached_until_forced() {
        let (exec, ddc) = ddc(100);
        assert_eq!(ddc.detect(false).len(), 2);
        assert_eq!(ddc.detect(false).len(), 2);
        assert_eq!(
            exec.calls
                .lock()
                .unwrap()
                .iter()
                .filter(|c| c.ends_with("detect"))
                .count(),
            1
        );
        ddc.detect(true);
        assert_eq!(
            exec.calls
                .lock()
                .unwrap()
                .iter()
                .filter(|c| c.ends_with("detect"))
                .count(),
            2
        );
    }

    #[test]
    fn writes_scale_to_the_displays_range_and_remember_it() {
        let (exec, ddc) = ddc(255);
        assert_eq!(ddc.read_percent(5, 0x10), Some(10));
        ddc.write_percent(5, 0x10, 50).unwrap();
        ddc.write_percent(5, 0x10, 100).unwrap();
        let calls = exec.calls.lock().unwrap();
        assert!(
            calls.contains(&"ddcutil --bus 5 setvcp 10 128 --noverify".to_owned()),
            "{calls:?}"
        );
        assert!(calls.contains(&"ddcutil --bus 5 setvcp 10 255 --noverify".to_owned()));
        assert_eq!(
            calls.iter().filter(|c| c.contains("getvcp 10")).count(),
            1,
            "max is cached"
        );
    }

    #[test]
    fn the_input_source_is_read_with_its_options() {
        let (_, ddc) = ddc(100);
        let inputs = ddc.input_sources(5);
        assert_eq!(inputs.current.unwrap().code, 0x0f);
        assert_eq!(inputs.options.len(), 3);
        assert_eq!(ddc.overdrive_code(5), Some(0xf0));
    }
}
