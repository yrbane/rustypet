#!/usr/bin/env bash
# Construit petd et installe l'extension GNOME dans le répertoire utilisateur.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UUID="rustypet@yrbane.dev"
DEST="$HOME/.local/share/gnome-shell/extensions/$UUID"

echo "== Construction de petd =="
cargo build --release -p petd --manifest-path "$ROOT/Cargo.toml"
mkdir -p "$HOME/.local/bin"
install -m 755 "$ROOT/target/release/petd" "$HOME/.local/bin/petd"
echo "petd installé dans ~/.local/bin/petd"

echo "== Installation de l'extension =="
mkdir -p "$DEST"
# On copie les fichiers du Shell, pas le dossier de tests.
install -m 644 "$ROOT/extension/metadata.json" "$DEST/metadata.json"
install -m 644 "$ROOT/extension/extension.js" "$DEST/extension.js"
install -m 644 "$ROOT/extension/petMath.js" "$DEST/petMath.js"
echo "Extension installée dans $DEST"

echo
echo "Activez-la :  gnome-extensions enable $UUID"
echo "Puis déconnectez/reconnectez la session (Wayland ne recharge pas à chaud)."
