#!/usr/bin/env bash
# Prépare le test de l'extension : construit petd, installe l'extension, puis
# tente une session GNOME imbriquée.
#
# La session imbriquée « devkit » (gnome-shell --devkit, qui remplace l'ancien
# --nested depuis GNOME 49) exige le binaire auxiliaire /usr/lib/mutter-devkit.
# Certaines distributions (dont Arch au moment de l'écriture) ne l'empaquettent
# pas : ce script le détecte et bascule alors sur la voie fiable — installation
# dans la session réelle puis reconnexion.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UUID="rustypet@yrbane.dev"
DEVKIT_HELPER="/usr/lib/mutter-devkit"

cargo build -p petd --manifest-path "$ROOT/Cargo.toml"
"$ROOT/scripts/install-extension.sh"

if [ -x "$DEVKIT_HELPER" ]; then
    export G_MESSAGES_DEBUG=all
    echo "Session imbriquée devkit. Dedans : gnome-extensions enable $UUID"
    echo "Logs : cette sortie de terminal."
    dbus-run-session -- gnome-shell --devkit --wayland
else
    cat <<EOF

── Session imbriquée indisponible ──
Le binaire $DEVKIT_HELPER est absent : « gnome-shell --devkit » ne peut pas
ouvrir de fenêtre imbriquée sur cette machine, et le mode « --headless »
n'expose pas la couche UI du Shell hors d'une vraie session logind.

Voie fiable — tester dans la session réelle :

  1. gnome-extensions enable $UUID
  2. Déconnexion puis reconnexion (Wayland ne recharge pas les extensions
     à chaud).
  3. Le pet apparaît sur le bureau. Journaux :
       journalctl --user -f -o cat /usr/bin/gnome-shell
  4. Pour l'arrêter : gnome-extensions disable $UUID

L'extension et petd viennent d'être (re)construits et installés : les
étapes ci-dessus suffisent.
EOF
fi
