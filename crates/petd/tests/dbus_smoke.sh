#!/usr/bin/env bash
# Test d'intégration sans GNOME : lance petd sur un bus de session jetable,
# appelle Configure et GetSprite, capture quelques signaux PetState.
# Réussite = au moins 3 signaux PetState observés après Configure.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
XML="$ROOT/crates/pet-format/tests/fixtures/neko.xml"
BIN="$ROOT/target/debug/petd"

[ -x "$BIN" ] || { echo "petd non construit : cargo build -p petd"; exit 1; }

# `dbus-run-session` exécute un tout nouveau processus bash : sans export,
# XML/BIN resteraient vides dans ce sous-processus (ce n'est pas un simple
# sous-shell qui hériterait des variables locales).
export XML BIN

run() {
  # Lance petd en fond.
  "$BIN" "$XML" --seed 42 &
  local pid=$!
  sleep 1

  # Capture les signaux PetState pendant 2 s.
  local mon
  mon=$(mktemp)
  dbus-monitor "interface='dev.yrbane.RustyPet1',member='PetState'" >"$mon" 2>/dev/null &
  local monpid=$!

  # Configure via busctl (géométrie 1920x1080, zone 1920x1050).
  busctl --user call dev.yrbane.RustyPet /dev/yrbane/RustyPet \
    dev.yrbane.RustyPet1 Configure iiiiii 1920 1080 0 0 1920 1050

  # Lit les dimensions du sprite.
  busctl --user call dev.yrbane.RustyPet /dev/yrbane/RustyPet \
    dev.yrbane.RustyPet1 GetSprite

  # Remonte deux fenêtres factices : le pet peut marcher dessus.
  busctl --user call dev.yrbane.RustyPet /dev/yrbane/RustyPet \
    dev.yrbane.RustyPet1 UpdateWindows "a(iiii)" 2 200 400 600 500 1000 600 700 400

  sleep 2
  kill "$monpid" 2>/dev/null || true
  kill "$pid" 2>/dev/null || true

  local count
  count=$(grep -c "member=PetState" "$mon" || true)
  echo "signaux PetState observés : $count"
  rm -f "$mon"
  [ "$count" -ge 3 ]
}

# Bus de session jetable pour ne pas polluer la session réelle.
dbus-run-session -- bash -c "$(declare -f run); run"
