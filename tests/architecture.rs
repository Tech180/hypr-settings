//! Enforces the hexagonal dependency direction:
//! driving -> application -> domain <- driven adapters, wired only in this binary.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

struct Crate {
    dir: &'static str,
    package: &'static str,
    allowed: &'static [&'static str],
}

const CRATES: &[Crate] = &[
    Crate {
        dir: "domain/settings",
        package: "settings-domain",
        allowed: &["serde", "thiserror"],
    },
    Crate {
        dir: "application/settings",
        package: "settings-application",
        allowed: &["settings-domain", "serde", "thiserror", "tracing"],
    },
    Crate {
        dir: "adapter/hardware",
        package: "adapter-hardware",
        allowed: &["settings-domain", "thiserror", "tracing"],
    },
    Crate {
        dir: "adapter/hyprland",
        package: "adapter-hyprland",
        allowed: &[
            "settings-domain",
            "serde",
            "serde_json",
            "thiserror",
            "tracing",
        ],
    },
    Crate {
        dir: "adapter/kanshi",
        package: "adapter-kanshi",
        allowed: &["settings-domain", "thiserror", "tracing"],
    },
    Crate {
        dir: "adapter/system",
        package: "adapter-system",
        allowed: &[
            "settings-domain",
            "serde",
            "serde_json",
            "thiserror",
            "tracing",
        ],
    },
    Crate {
        dir: "adapter/bluetooth",
        package: "adapter-bluetooth",
        allowed: &[
            "settings-domain",
            "bluer",
            "tokio",
            "futures",
            "serde",
            "serde_json",
            "thiserror",
            "tracing",
            "uuid",
        ],
    },
    Crate {
        dir: "adapter/network",
        package: "adapter-network",
        allowed: &["settings-domain", "qrcode", "tracing"],
    },
    Crate {
        dir: "presentation/gtk",
        package: "presentation-gtk",
        allowed: &[
            "settings-application",
            "settings-domain",
            "gtk4",
            "libadwaita",
            "async-channel",
            "tracing",
            "resvg",
            "lucide-static-svg",
        ],
    },
    Crate {
        dir: "presentation/cli",
        package: "presentation-cli",
        allowed: &[
            "settings-application",
            "settings-domain",
            "clap",
            "serde_json",
        ],
    },
];

const TOOLKIT_CRATES: &[&str] = &["gtk4", "libadwaita", "gio", "glib", "gdk4"];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("binary crate lives inside the workspace")
        .to_path_buf()
}

fn manifest(krate: &Crate) -> toml::Table {
    let path = workspace_root().join(krate.dir).join("Cargo.toml");
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .parse()
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn dependencies(krate: &Crate) -> BTreeSet<String> {
    manifest(krate)
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .map(|t| t.keys().cloned().collect())
        .unwrap_or_default()
}

#[test]
fn package_names_match_the_plan() {
    for krate in CRATES {
        let name = manifest(krate)["package"]["name"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(name, krate.package, "{}", krate.dir);
    }
}

#[test]
fn dependencies_stay_within_each_allow_list() {
    for krate in CRATES {
        let extra: Vec<_> = dependencies(krate)
            .into_iter()
            .filter(|d| !krate.allowed.contains(&d.as_str()))
            .collect();
        assert!(
            extra.is_empty(),
            "{} depends on {extra:?}, which its allow-list forbids",
            krate.package
        );
    }
}

#[test]
fn adapters_never_depend_on_each_other_or_on_upper_layers() {
    for krate in CRATES.iter().filter(|c| c.dir.starts_with("adapter/")) {
        for dep in dependencies(krate) {
            assert!(
                !dep.starts_with("adapter-")
                    && !dep.starts_with("presentation-")
                    && dep != "settings-application",
                "{} must only implement domain ports but depends on {dep}",
                krate.package
            );
        }
    }
}

#[test]
fn only_the_gtk_presentation_crate_links_the_toolkit() {
    for krate in CRATES.iter().filter(|c| c.package != "presentation-gtk") {
        let deps = dependencies(krate);
        for toolkit in TOOLKIT_CRATES {
            assert!(
                !deps.contains(*toolkit),
                "{} must not depend on {toolkit}",
                krate.package
            );
        }
    }
}

fn collect_files(dir: &Path, suffix: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, suffix, out);
        } else if path.to_string_lossy().ends_with(suffix) {
            out.push(path);
        }
    }
}

#[test]
fn html_files_contain_markup_only() {
    let mut files = Vec::new();
    collect_files(
        &workspace_root().join("presentation/gtk/src"),
        ".html.rs",
        &mut files,
    );

    assert!(!files.is_empty(), "no *.html.rs files found");
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        for forbidden in [
            "settings_application",
            "DisplayApi",
            "spawn_blocking",
            "settings_domain::ports",
        ] {
            assert!(
                !text.contains(forbidden),
                "{} must stay markup-only but mentions {forbidden}",
                file.display()
            );
        }
    }
}
