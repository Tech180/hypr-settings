//! kanshi.conf text <-> `ProfileSet`. Anything the model does not understand is
//! kept verbatim (output tokens, profile lines, top-level lines) so a rewrite never
//! destroys hand-edited content. Comments are not preserved.

use settings_domain::{
    Criteria, Mode, Position, Profile, ProfileName, ProfileOutput, ProfileSet, Scale, Transform,
};

const HEADER: &str =
    "# Managed by hypr-settings (layout + modes). Kanshi applies on match/reload.\n";

/// Splits on whitespace, keeping `"quoted strings"` together (quotes removed).
/// `None` for an unterminated quote.
fn tokenize(text: &str) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        let mut token = String::new();
        let mut in_quotes = false;
        while let Some(&c) = chars.peek() {
            if c == '"' {
                in_quotes = !in_quotes;
                chars.next();
            } else if c.is_whitespace() && !in_quotes {
                break;
            } else {
                token.push(c);
                chars.next();
            }
        }
        if in_quotes {
            return None;
        }
        tokens.push(token);
    }
    Some(tokens)
}

fn quote_if_needed(token: &str) -> String {
    if token.is_empty() || token.chars().any(char::is_whitespace) {
        format!("\"{token}\"")
    } else {
        token.to_owned()
    }
}

fn parse_output(line: &str) -> Option<ProfileOutput> {
    let (rest_glob, rest) = match line.strip_prefix("...output ") {
        Some(rest) => (true, rest),
        None => (false, line.strip_prefix("output ")?),
    };
    let tokens = tokenize(rest)?;
    let (criteria, options) = tokens.split_first()?;

    let mut out = ProfileOutput {
        criteria: Criteria::parse(criteria),
        enable: None,
        mode: None,
        position: None,
        scale: None,
        transform: None,
        rest_glob,
        extra: Vec::new(),
    };

    let mut i = 0;
    while i < options.len() {
        let key = options[i].as_str();
        let value = options.get(i + 1);
        match (key, value) {
            ("enable", _) => out.enable = Some(true),
            ("disable", _) => out.enable = Some(false),
            ("mode", Some(v)) if v.parse::<Mode>().is_ok() => {
                out.mode = v.parse().ok();
                i += 1;
            }
            ("position", Some(v)) if Position::from_kanshi(v).is_some() => {
                out.position = Position::from_kanshi(v);
                i += 1;
            }
            ("scale", Some(v)) if v.parse().is_ok_and(|s: f64| Scale::new(s).is_ok()) => {
                out.scale = v.parse().ok().and_then(|s: f64| Scale::new(s).ok());
                i += 1;
            }
            ("transform", Some(v)) if Transform::from_kanshi(v).is_some() => {
                out.transform = Transform::from_kanshi(v);
                i += 1;
            }
            _ => out.extra.push(options[i].clone()),
        }
        i += 1;
    }
    Some(out)
}

fn parse_profile_header(line: &str) -> Option<Option<ProfileName>> {
    let rest = line.strip_prefix("profile")?;
    if !rest.starts_with(char::is_whitespace) && !rest.starts_with('{') {
        return None;
    }
    let rest = rest.trim().trim_end_matches('{').trim();
    // `profile {` (anonymous) is recognised but cannot be modelled by name.
    Some(ProfileName::from_stored(rest).ok())
}

pub fn parse(text: &str) -> ProfileSet {
    let mut set = ProfileSet::default();
    let mut current: Option<Profile> = None;
    // Lines inside a block we cannot name are dropped until its closing brace.
    let mut skipping = false;

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if skipping {
            skipping = line != "}";
            continue;
        }
        if current.is_none() {
            if let Some(header) = parse_profile_header(line) {
                match header {
                    Some(name) => current = Some(Profile::new(name)),
                    None => skipping = true,
                }
            } else if line != "{" {
                set.preamble.push(line.to_owned());
            }
            continue;
        }
        if line == "}" {
            set.profiles.extend(current.take());
        } else if line != "{" {
            let profile = current.as_mut().expect("checked above");
            match parse_output(line) {
                Some(output) => profile.outputs.push(output),
                None => profile.extra_lines.push(line.to_owned()),
            }
        }
    }
    // A profile left open at EOF is still kept.
    set.profiles.extend(current);
    set
}

fn format_output(o: &ProfileOutput) -> String {
    let keyword = if o.rest_glob { "...output" } else { "output" };
    let mut parts = vec![format!("{keyword} \"{}\"", o.criteria.as_str())];
    match o.enable {
        Some(true) => parts.push("enable".into()),
        Some(false) => parts.push("disable".into()),
        None => {}
    }
    if let Some(mode) = o.mode {
        parts.push(format!("mode {}", mode.kanshi_token()));
    }
    if let Some(position) = o.position {
        parts.push(format!("position {}", position.kanshi_token()));
    }
    if let Some(scale) = o.scale {
        parts.push(format!("scale {scale}"));
    }
    if let Some(transform) = o.transform {
        parts.push(format!("transform {}", transform.kanshi_token()));
    }
    parts.extend(o.extra.iter().map(|t| quote_if_needed(t)));
    format!("  {}", parts.join(" "))
}

pub fn serialize(set: &ProfileSet) -> String {
    let mut out = String::from(HEADER);
    for line in &set.preamble {
        out.push_str(line);
        out.push('\n');
    }
    for profile in &set.profiles {
        out.push_str(&format!("profile {} {{\n", profile.name));
        for output in &profile.outputs {
            out.push_str(&format_output(output));
            out.push('\n');
        }
        for line in &profile.extra_lines {
            out.push_str(&format!("  {line}\n"));
        }
        out.push_str("}\n\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: &str = include_str!("../tests/fixtures/kanshi-seed.conf");
    const AGS_RUNTIME: &str = include_str!("../tests/fixtures/kanshi-ags-runtime.conf");

    fn name(s: &str) -> ProfileName {
        ProfileName::parse(s).unwrap()
    }

    fn head(
        connector: &str,
        make: &str,
        model: &str,
        serial: &str,
    ) -> settings_domain::OutputIdentity {
        settings_domain::OutputIdentity {
            connector: settings_domain::ConnectorName::try_from(connector).unwrap(),
            make: make.into(),
            model: model.into(),
            serial: serial.into(),
        }
    }

    fn active(heads: &[settings_domain::OutputIdentity]) -> Option<String> {
        parse(SEED).match_active(heads).map(|n| n.to_string())
    }

    #[test]
    fn seed_matches_like_kanshi_for_real_hardware() {
        let laptop = head("eDP-1", "BOE", "0x0A89", "");
        // One head: `portable` wins over the single-`*` default.
        assert_eq!(
            active(std::slice::from_ref(&laptop)).as_deref(),
            Some("portable")
        );
        // The two Dells match `home` by make, model and serial.
        let dell_a = head("DP-1", "Dell Inc.", "DELL P2422H", "B95JF04");
        let dell_b = head("DP-2", "Dell Inc.", "DELL P2422H", "9BWHF04");
        assert_eq!(
            active(&[laptop.clone(), dell_a, dell_b]).as_deref(),
            Some("home")
        );
        // The Sceptre on DP-3 matches `asia` by connector.
        let sceptre = head("DP-3", "Sceptre Tech Inc", "Sceptre P30", "");
        assert_eq!(active(&[laptop.clone(), sceptre]).as_deref(), Some("asia"));
        // An unknown second monitor falls to `docked`, which absorbs it.
        let other = head("HDMI-A-1", "Acme", "Panel", "1");
        assert_eq!(
            active(&[laptop.clone(), other.clone()]).as_deref(),
            Some("docked")
        );
        // Without the laptop only `default` matches; it is the fallback, never reported.
        assert_eq!(active(&[other]), None);
    }

    #[test]
    fn parses_the_seed_profiles() {
        let set = parse(SEED);
        let names: Vec<_> = set.profiles.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["home", "portable", "asia", "docked", "default"]);

        let home = set.get(&name("home")).unwrap();
        assert_eq!(home.outputs.len(), 3);
        assert_eq!(
            home.outputs[1].criteria,
            Criteria::Named("Dell Inc. DELL P2422H B95JF04".into())
        );
        assert_eq!(home.outputs[1].position, Some(Position::new(1920, 0)));
        assert_eq!(home.outputs[0].transform, Some(Transform::Normal));

        let docked = set.get(&name("docked")).unwrap();
        assert!(docked.has_rest_glob());
        assert_eq!(docked.named_connectors(), ["eDP-1"]);
        let glob = &docked.outputs[1];
        assert_eq!(glob.criteria, Criteria::Any);
        assert_eq!(glob.mode.unwrap().kanshi_token(), "1920x1080@60");

        let default = set.get(&name("default")).unwrap();
        assert_eq!(default.outputs[0].enable, None);
        assert_eq!(default.outputs[0].scale.unwrap().to_string(), "1");
    }

    #[test]
    fn round_trips_the_seed_and_the_live_runtime_conf() {
        for text in [SEED, AGS_RUNTIME] {
            let first = parse(text);
            let again = parse(&serialize(&first));
            assert_eq!(again, first);
        }
    }

    #[test]
    fn serializing_twice_is_stable() {
        let once = serialize(&parse(SEED));
        assert_eq!(serialize(&parse(&once)), once);
    }

    #[test]
    fn explicit_normal_transform_is_emitted() {
        let text = serialize(&parse(SEED));
        assert!(text.contains(
            r#"output "eDP-1" enable mode 1920x1080@60 position 0,0 scale 1 transform normal"#
        ));
    }

    #[test]
    fn unknown_content_survives_a_rewrite() {
        let src = r#"
include /etc/kanshi/extra
profile docked {
  output "eDP-1" enable mode 1920x1080@60 adaptive_sync on position 0,0
  output "HDMI-A-1" enable mode --custom 1280x720@50
  exec notify-send "docked"
}
"#;
        let set = parse(src);
        assert_eq!(set.preamble, ["include /etc/kanshi/extra"]);
        let profile = set.get(&name("docked")).unwrap();
        assert_eq!(profile.extra_lines, [r#"exec notify-send "docked""#]);
        assert_eq!(profile.outputs[0].extra, ["adaptive_sync", "on"]);
        assert_eq!(profile.outputs[1].mode, None);

        let text = serialize(&set);
        assert!(text.contains("include /etc/kanshi/extra"));
        assert!(text.contains("adaptive_sync on"));
        assert!(text.contains("mode --custom 1280x720@50"));
        assert!(text.contains(r#"exec notify-send "docked""#));
        assert_eq!(parse(&text), set);
    }

    #[test]
    fn fractional_modes_and_quoted_extras_round_trip() {
        let src = "profile a {\n  output \"X Y\" enable mode 2560x1080@164.99 scale 1.25 note \"two words\"\n}\n";
        let set = parse(src);
        let out = &set.profiles[0].outputs[0];
        assert_eq!(out.mode.unwrap().kanshi_token(), "2560x1080@164.99");
        assert_eq!(out.scale.unwrap().to_string(), "1.25");
        assert_eq!(out.extra, ["note", "two words"]);
        assert_eq!(parse(&serialize(&set)), set);
    }

    #[test]
    fn anonymous_profiles_and_garbage_do_not_panic() {
        let set = parse("profile {\n output \"x\"\n}\nprofile ok {\n output \"unterminated\n}\n");
        assert_eq!(set.profiles.len(), 1);
        assert_eq!(set.profiles[0].extra_lines, ["output \"unterminated"]);
    }

    #[test]
    fn seed_fixture_matches_kanshi_nix_when_the_repo_is_present() {
        let nix = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../home/programs/hyprland/kanshi.nix");
        let Ok(nix) = std::fs::read_to_string(nix) else {
            return; // built outside the repository (nix sandbox)
        };
        let start = nix
            .find("writeText \"kanshi-ags-seed.conf\" ''")
            .expect("seed block");
        let body = &nix[start..];
        let body = &body[body.find("''\n").unwrap() + 3..];
        let body = &body[..body.find("\n  '';").unwrap()];
        let dedented: String = body
            .lines()
            .map(|l| l.strip_prefix("    ").unwrap_or(l))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(parse(&dedented), parse(SEED));
    }
}
