# `nix run .#cargo -- test` runs Cargo with the GTK dev environment.
# The package itself is only a script; the build env is captured from the same
# inputs the dev shell uses, so pkg-config can see gtk4 and libadwaita.
{
  lib,
  stdenv,
  writeShellScriptBin,
  cargo,
  rustc,
  clippy,
  rustfmt,
  pkg-config,
  gtk4,
  libadwaita,
  glib,
  dbus,
  bluez,
}:
let
  env = stdenv.mkDerivation {
    name = "hypr-settings-cargo-env";
    nativeBuildInputs = [ pkg-config ];
    buildInputs = [
      gtk4
      libadwaita
      glib
      dbus
      bluez
    ];
    phases = [ "installPhase" ];
    installPhase = ''
      mkdir -p "$out"
      {
        printf 'export PKG_CONFIG_PATH=%q\n' "''${PKG_CONFIG_PATH-}"
        printf 'export LIBRARY_PATH=%q\n' "''${LIBRARY_PATH-}"
      } > "$out/env.sh"
    '';
  };
in
writeShellScriptBin "cargo" ''
  . ${env}/env.sh
  if [ -z "''${LD_LIBRARY_PATH-}" ] && [ -n "''${LIBRARY_PATH-}" ]; then
    export LD_LIBRARY_PATH="$LIBRARY_PATH"
  fi
  export PATH="${
    lib.makeBinPath [
      cargo
      rustc
      clippy
      rustfmt
      pkg-config
      stdenv.cc
    ]
  }:$PATH"
  exec cargo "$@"
''
