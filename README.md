# RustyPet

Portage en Rust de [DesktopPet / eSheep](https://github.com/Adrianotiger/desktopPet),
pour GNOME Shell sous Wayland.

Les animaux de bureau sont décrits par les fichiers `animations.xml` du projet
d'origine : spritesheet encodé, machine à états d'animation, physique. Les 20+
pets existants fonctionnent sans modification.

## État

Le cœur du moteur est fonctionnel et testable sans écran. L'affichage sous
GNOME fait l'objet d'un second chantier.

| Crate | Rôle |
|---|---|
| `pet-expr` | Évaluateur des expressions `x`, `y`, `interval`, `repeat` |
| `pet-format` | Lecture des `animations.xml` et décodage des spritesheets |
| `pet-engine` | Machine à états : animations, transitions, physique |
| `petsim` | Simulateur headless, sans affichage |

## Essayer

```bash
cargo run -p petsim -- chemin/vers/animations.xml --ticks 100
```

## Développement

Lancer la suite de tests :

```bash
cargo test --all
```

Linter et formateur :

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
```

Valider le corpus complet du dépôt amont (22 pets) :

```bash
RUSTYPET_CORPUS=~/Dev/desktopPet/Pets cargo test -p pet-format -- --ignored
```

## Voir le pet à l'écran (GNOME Shell 50, Wayland)

Le rendu passe par une extension GNOME Shell pilotée par le démon `petd`.

### Installation

```bash
./scripts/install-extension.sh
gnome-extensions enable rustypet@yrbane.dev
```

Les binaires peuvent aussi s'installer via cargo — le manifeste du workspace
est virtuel, il faut cibler chaque crate (`cargo install --path .` échoue) :

```bash
cargo install --path crates/petd --locked     # démon D-Bus
cargo install --path crates/petsim --locked   # simulateur headless
```

Puis **déconnectez et reconnectez votre session** : sous Wayland, GNOME ne
recharge pas les extensions à chaud. Le pet apparaît alors sur le bureau.

### Développement

```bash
./scripts/dev-session.sh
```

Ce script construit `petd`, installe l'extension, puis tente une session GNOME
imbriquée via `gnome-shell --devkit --wayland` (le `--devkit` remplace l'ancien
`--nested` depuis GNOME 49).

La session imbriquée exige le binaire auxiliaire `/usr/lib/mutter-devkit`, que
certaines distributions — dont Arch — n'empaquettent pas. Quand il est absent,
le script le détecte et affiche la marche à suivre pour tester dans la session
réelle (le mode `--headless` de mutter, lui, n'expose pas la couche UI du Shell
hors d'une vraie session logind). Les journaux du Shell se lisent avec :

```bash
journalctl --user -f -o cat /usr/bin/gnome-shell
```

### Vérifier le démon sans compositeur

Le démon se teste seul, sans GNOME : il émet la position et la tuile du pet sur
D-Bus. Le test d'intégration lance `petd` sur un bus jetable et compte les
signaux :

```bash
cargo build -p petd && ./crates/petd/tests/dbus_smoke.sh
```

Le spritesheet décodé (couleur-clé convertie en transparence) est écrit dans
`~/.cache/rustypet/<pet>/sheet.png` — ouvrable directement pour contrôle.

### Choisir son pet

Par défaut, le neko. Le dépôt embarque **RustySheep**, le mouton eSheep 64
enrichi de gags inédits (danse, joint, superman, crotte, lunettes de
soleil, fleur qui pousse et se fait manger, arrivée en parachute, départ en
fusée, trip sous acide, nuage de pluie — trempé ou parapluie) en plus des
classiques (dormir, brouter, respirer une fleur) :

```bash
mkdir -p ~/.config/rustypet
echo "$PWD/assets/rustysheep/animations.xml" > ~/.config/rustypet/pet
```

N'importe quel `animations.xml` du corpus eSheep fonctionne aussi
(`~/Dev/desktopPet/Pets/esheep64/animations.xml` pour le mouton d'origine).
Puis déconnexion/reconnexion de session. Supprimer le fichier revient au
neko. RustySheep est regénérable depuis les sprites d'origine avec
`python3 tools/make_rustysheep.py`.

### Ce qui marche à ce stade

La physique des bords d'écran (il marche, tombe, rebondit sur les bords et
la barre des tâches), **la marche sur les fenêtres** (atterrissage sur les
toits, chute quand la fenêtre se ferme ou bouge), **les gags de RustySheep**
(danse, joint, superman, crotte, lunettes, fleur, parachute, fusée, acide,
pluie, sommeil, broutage) et **le multi-pets** : le coup de foudre fait
entrer la moutonne, puis deux agneaux trottinent derrière le couple — les
scènes à deux moutons d'origine (bain, mouton noir) marchent aussi. **Le
glisser-déposer** : attrape le mouton à la souris, il gigote, lâche-le, il
tombe. Et **les sons** : des bêlements discrets (18 % d'amplitude, joués
par l'API sonore de GNOME) au coup de foudre, en parachute et à
l'atterrissage. Le menu vient dans les plans suivants.

## Documentation

- Conception : `docs/superpowers/specs/2026-07-20-rustypet-design.md`
- Référence du moteur d'origine : `docs/reference/esheep-engine.md`

## Licence

MIT pour le code. Les pets restent la propriété de leurs auteurs respectifs.
