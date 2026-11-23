# hypr-settings

A native settings app for this Hyprland desktop, written in Rust with GTK4 and libadwaita. The first section is Display: a drag-to-arrange canvas, resolution, refresh rate, scale, rotation and flip, enable or disable, the main display (the bar host), custom refresh rates, kanshi profiles, HDR / 10-bit / Adaptive Sync, brightness, DDC/CI contrast, overdrive and input source, a night light and a port badge (DP / HDMI standard and link rate). Layout changes run as a confirm-or-revert trial.

## Layout

The workspace is hexagonal and layer-first. Each layer is a folder, and each crate inside it is named after the feature it serves.

| Path | Role |
|------|------|
| `domain/settings` | Value objects, topology, the arrangement rules (logical size, snapping, reflow, overlap), layout policy, kanshi profile matching, the trial state machine, and the driven ports |
| `application/settings` | `DisplayApi` and `ControlsApi` (the driving ports, joined as `SettingsApi`) and `DisplayService`, which orchestrates apply, verify (position, scale and mode per output), keep, revert, and crash recovery; `drag` resolves the canvas drop preview |
| `adapter/hyprland` | `hyprctl` topology, the socket2 event stream, and live `hl.monitor` changes |
| `adapter/kanshi` | `kanshi.conf` parser and serializer, atomic store, and the `kanshictl` controller |
| `adapter/hardware` | `ddcutil` (DDC/CI brightness, contrast, overdrive, input), `brightnessctl` for the panel backlight, and `hyprsunset` for the night light |
| `adapter/system` | XDG paths, preferences, feature flags, the brightness snapshot, the DRM / EDID / DPCD probe, the trial journal, and the clock |
| `presentation/gtk` | The GTK4 and libadwaita window, split into html (markup), component (logic), and css files per component. UI icons come from the [`lucide-static-svg`](https://crates.io/crates/lucide-static-svg) crate; see **Icons** below |
| `presentation/cli` | Headless `display list`, `profiles`, `set`, `move`, `main`, `save`, `hz`, `flag`, `brightness`, `contrast`, `overdrive`, `input`, `night`, `controls`, `restore` and `revert-pending` |
| `hypr-settings` | The binary; the only place that wires adapters into the application |

`hypr-settings/tests/architecture.rs` enforces the dependency rules, for example that adapters never depend on each other and that only `presentation-gtk` touches the toolkit.

## Usage

```bash
hypr-settings                         # open the window (focuses it when already open)
hypr-settings open --section display
hypr-settings open --section bluetooth
hypr-settings open --section network
hypr-settings toggle
hypr-settings display list
hypr-settings display set --connector DP-3 --rotate 90 --flip --trial
hypr-settings display move DP-3 1920 0 --trial
hypr-settings display revert-pending  # run by Hyprland on start to recover an unconfirmed trial
hypr-settings display restore         # run at login: HDR / 10-bit / VRR of the active profile and a manual night light
hypr-settings display main DP-3       # host the bar on DP-3
hypr-settings display flag DP-3 hdr on
hypr-settings display brightness adjust -5   # what the XF86 keys and the bar scroll end up calling
hypr-settings display night set 3500
hypr-settings bluetooth list          # active adapter first, then devices
hypr-settings bluetooth adapters      # every controller; * marks the active one
hypr-settings bluetooth adapter AA:BB:CC:DD:EE:FF   # select by MAC (saved)
hypr-settings bluetooth power on
hypr-settings bluetooth connect AA:BB:CC:DD:EE:FF
hypr-settings network status             # link, connectivity, tunnel count
hypr-settings network wifi list          # * marks the joined network
hypr-settings network wifi connect SSID [--password PW]
hypr-settings network wifi on|off|scan|disconnect
hypr-settings network vpn list           # * marks tunnels that are up
hypr-settings network vpn up|down ID
```

The **Bluetooth** section talks to BlueZ over the session D-Bus (`bluer`). When more than one adapter is present, the main page offers a picker; power, scanning, devices, and Advanced all follow the **active** adapter. The choice is remembered by MAC in `bluetooth.json` (`selected_adapter`) and falls back to the first controller BlueZ lists if that adapter is gone. Pairing, discovery, device detail profiles, and advanced options (PAN when NetworkManager is present, AVRCP / developer prefs — system-wide across adapters) follow the [bluetooth preview](previews/bluetooth-display.html). Controls that need PipeWire or an experimental BlueZ build are shown but gated when the stack cannot drive them on this machine.

`Super + ,` toggles the window and the AGS bar display button opens the Display section.

## Arrangement rules

- Positions are logical pixels. An output's size is its mode divided by its scale (rounded as Hyprland does), with width and height swapped for 90° and 270° (also flipped).
- Enabled outputs never overlap and always touch another output along an edge, so the pointer can reach every display. A drop is snapped flush to the side of the neighbour its centre leans towards, and aligned to the edges or centres of any display within 30 canvas pixels.
- A move or disable that would strand displays pulls them back flush against the rest, closest first; the canvas previews this while dragging.
- Changing size, scale or rotation moves the neighbours that were touching its right or bottom edge, so a rotated laptop panel never leaves a gap or an overlap.
- After a move, or any change that would leave a display at negative coordinates, the layout is shifted so it starts at `0, 0`: moving a display to the left of the others stores it at the origin. Other edits leave the layout where it is.
- Only scales that divide the resolution evenly are offered; Hyprland would silently pick a different one otherwise.
- A drop, like every other change, is applied live as a trial. It is written to kanshi only after Keep.
- kanshi matching follows kanshi itself: the first profile in file order whose output count equals the connected heads (`...` absorbs extra heads), by connector or `Make Model Serial`. A change that touches a display absorbed by `*` is stored as a new profile placed before the catch-all.
- With the keyboard, focus a tile: arrows nudge along an edge (Ctrl for larger steps), Enter applies, Escape cancels.

## State

The running kanshi daemon reads `~/.local/state/hypr-settings/display/kanshi.conf`. When that file is missing it is seeded from the previous AGS location, then from the Home Manager seed in `home/programs/hyprland/kanshi.nix`; the old file is never deleted. Next to it:

- `preferences.json`: the main display (`barHostConnector`), custom refresh rates, sync brightness and the night light override. The bar watches it.
- `feature-flags.json`: HDR, 10-bit and Adaptive Sync per profile and connector, re-applied after a profile switch and at login.
- `brightness.json`: the brightness targets and the last percent. The AGS bar reads it with a file monitor, and `display brightness` keeps it current, so nothing waits for `ddcutil` on a key press.
- `trial-journal.json`: the pending trial.

The **Network** section (see the [network preview](previews/network-display.html)) drives NetworkManager through `nmcli` (`adapter/network`; argument arrays only, `LC_ALL=C`, live updates from `nmcli monitor`). Pages: Home (status, live Tx/Rx, entry rows), Wi-Fi (radio switch, adapter picker, search, scan policy Manual / On open / every 30s–5m, expandable network cards with password connect), Ethernet (adapter, link and IPv4 editor), VPN (WireGuard tunnels from NetworkManager and NixOS `wg-quick-*.service` units, create / edit / delete, import `.conf` / `.ovpn`) and Diagnostics (action log plus the NetworkManager journal). Connection Properties covers auto-connect, metered, MAC randomisation, IPv4 / IPv6 editing, telemetry, Wi-Fi QR sharing and Forget. Keys: Esc goes back, Ctrl+F searches Wi-Fi networks. Scan policy lives in `network/preferences.json`; the old AGS `network-preferences.json` and `vpn-profiles.json` seed it on first run.

Network preferences (`preferences.json`, `vpn-profiles.json`) sit next to the display state under `~/.local/state/hypr-settings/network/`.

Bluetooth preferences and developer toggles not stored in BlueZ live under `~/.local/state/hypr-settings/bluetooth/` (`bluetooth.json`).

Preferences and feature flags are seeded once from `~/.local/state/ags/display` when missing; the old files are not deleted.

HDR, 10-bit and Adaptive Sync apply immediately and are checked against `hyprctl monitors`, rolled back when the compositor does not confirm. They are not part of the Keep or Revert trial. Switching input asks first, because a wrong input can leave the display blank and DDC cannot undo it. Probing displays is slow, so the page fills in the hardware rows after it opens.

## Static preview

Single-file HTML/CSS design of the Display page. The GTK window is a port of it: same tokens, sidebar, arrangement canvas, bento dashboard (mode + global environment left; Hz hero, feature tiles, and DDC right), profile card grid, floating trial pill, and profile sub page. [`previews/display.html`](previews/display.html) is the reference to compare against; Appearance and Performance are placeholders in both.

## Development

Colors come from `~/.config/hypr-settings/tokens.css` when that file exists (this NixOS config writes it from nix-colors). Otherwise the built-in [`tokens.css`](presentation/gtk/src/style/tokens.css) is used. A scheme change is a home rebuild, not a Rust rebuild.

```bash
nix develop
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
nix build
```

## Icons (GTK)

Glyphs are **not** drawn by hand and are not loaded from the system icon theme. The type is [`lucide_static_svg::Icon`](https://docs.rs/lucide-static-svg), re-exported from [`shared::icon`](presentation/gtk/src/shared/icon/mod.rs). [`icon.html.rs`](presentation/gtk/src/shared/icon/icon.html.rs) loads SVG via `Icon::svg_str()`, rasterizes with **resvg**, and tints from CSS `color` (alpha mask, not per-pixel recolor).

To add an icon:

1. Pick a name on [lucide.dev](https://lucide.dev) and use `Icon::Variant` at the call site (`IconProps { icon, size }`).
2. Add the variant to [`used-icons.rs`](presentation/gtk/src/shared/icon/used-icons.rs) (`USED`) so the parse cache and tests cover it.
3. `cargo test -p presentation-gtk used_icons` — every listed icon must parse and render.

To bump Lucide: update `lucide-static-svg` in [`presentation/gtk/Cargo.toml`](presentation/gtk/Cargo.toml), fix any renamed variants at compile time, and re-run the tests above.

## Select (GTK)

Single-choice controls use the custom [`shared::select`](presentation/gtk/src/shared/select/mod.rs) widget (not `GtkDropDown`). It follows [`previews/select.html`](previews/select.html): trigger, chevron, popover list with a check on the active row. Colors are only `@hs_*` tokens from [`tokens.css`](presentation/gtk/src/style/tokens.css). [`shared::dropdown`](presentation/gtk/src/shared/dropdown/mod.rs) is a thin facade with the same `DropdownProps` API.
