#!/usr/bin/env bash
# Lance une session GNOME imbriquée (devkit) pour tester l'extension sans
# toucher à la session courante. GNOME 49+ : --devkit (et non --nested).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UUID="rustypet@yrbane.dev"

cargo build -p petd --manifest-path "$ROOT/Cargo.toml"
"$ROOT/scripts/install-extension.sh"

export G_MESSAGES_DEBUG=all
echo "Dans la session imbriquée : gnome-extensions enable $UUID"
echo "Logs : cette sortie de terminal."
dbus-run-session -- gnome-shell --devkit --wayland
