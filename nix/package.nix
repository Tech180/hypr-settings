{
  lib,
  rustPlatform,
  pkg-config,
  wrapGAppsHook4,
  gtk4,
  libadwaita,
  glib,
  kanshi,
  ddcutil,
  brightnessctl,
  hyprsunset,
  systemd,
  procps,
  dbus,
  bluez,
  networkmanager,
}:

rustPlatform.buildRustPackage {
  pname = "hypr-settings";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ../.;
    fileset = ../.;
  };

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [
    pkg-config
    wrapGAppsHook4
  ];

  buildInputs = [
    gtk4
    libadwaita
    glib
    dbus
  ];

  # Domain, application and adapter tests run against fakes and fixtures; no compositor needed.
  doCheck = true;

  # kanshictl must match the kanshi the user session runs; hyprctl is deliberately
  # not wrapped so it always talks to the running compositor. The display controls call
  # ddcutil and brightnessctl, and the night light drives hyprsunset through systemctl
  # and pkill.
  preFixup = ''
    gappsWrapperArgs+=(--prefix PATH : ${
      lib.makeBinPath [
        kanshi
        ddcutil
        brightnessctl
        hyprsunset
        systemd
        procps
        bluez
        networkmanager
      ]
    })
  '';

  meta = {
    description = "Native Hyprland settings window (GTK4 + libadwaita)";
    mainProgram = "hypr-settings";
    platforms = lib.platforms.linux;
  };
}
