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

## Documentation

- Conception : `docs/superpowers/specs/2026-07-20-rustypet-design.md`
- Référence du moteur d'origine : `docs/reference/esheep-engine.md`

## Licence

MIT pour le code. Les pets restent la propriété de leurs auteurs respectifs.
