# RustyPet — Plan 2a : squelette de rendu bout-en-bout (GNOME/D-Bus)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Afficher un pet animé à l'écran sous GNOME Shell 50/Wayland : un démon Rust `petd` fait vivre le pet (moteur du plan 1) et émet sa position par D-Bus ; une extension GNOME Shell reçoit ces signaux et déplace un acteur affichant la bonne tuile du spritesheet.

**Architecture:** `petd` charge un `animations.xml`, décode le spritesheet en PNG RGBA écrit dans `~/.cache/rustypet/`, fait tourner le moteur `pet-engine` à la cadence des animations, et émet un signal `PetState` (~15 Hz) sur le bus de session. L'extension GNOME (JS/ESM) crée un `St.Widget`, charge ce PNG en `background-image`, et à chaque signal déplace l'acteur et décale `background-position` sur la tuile courante. Seule la physique des bords d'écran est active ; la marche sur les fenêtres, l'audio, le menu et le multi-pets relèvent des plans suivants.

**Tech Stack:** Rust 1.97 (edition 2024), `zbus` 5 (features `tokio`), `tokio` 1, le moteur existant (`pet-engine`, `pet-format`). Extension GNOME Shell 50 en JavaScript ESM (GJS 1.88), `Gio.DBusProxy`, `St.Widget`.

## Global Constraints

- Spec de conception : `docs/superpowers/specs/2026-07-20-rustypet-design.md`.
- Note de référence GNOME/D-Bus (versions et API vérifiées) : `docs/reference/gnome50-dbus.md` — **à consulter systématiquement**.
- Référence du moteur d'origine : `docs/reference/esheep-engine.md`.
- Le code Rust est en anglais ; **les commentaires et les messages de commit sont en français**. Le code JS suit la même règle (identifiants anglais, commentaires français).
- Aucune mention d'assistant IA dans les commits (pas de `Co-Authored-By`, pas d'emoji robot). `git add` ciblé, jamais `-A`.
- **Aucun `unwrap()`, `expect()` ni `panic!()` dans le code de bibliothèque Rust.** Le binaire `petd` a droit au `?` dans `main` et à `expect` sur des invariants de démarrage documentés ; le code de test est libre.
- Le RNG du moteur est injecté via `PetRng` / `SeededRng` (déjà réexportés par `pet-engine`).
- Chaque tâche se termine par un commit qui bumpe la version SemVer dans le `Cargo.toml` du workspace **et** ajoute une entrée en tête de `CHANGELOG.md`. Le workspace est à `0.9.2` ; ce plan va de `0.10.0` à `0.15.0`.
- `cargo clippy --all-targets -- -D warnings` et `cargo fmt --all --check` doivent passer avant chaque commit Rust.
- UUID de l'extension : `rustypet@yrbane.dev`. Nom de bus : `dev.yrbane.RustyPet`. Chemin d'objet : `/dev/yrbane/RustyPet`. Interface : `dev.yrbane.RustyPet1`.
- Le workspace vit dans `~/Dev/rustypet`. Tous les chemins sont relatifs à cette racine.

### Contrat D-Bus (figé, partagé par toutes les tâches)

Interface `dev.yrbane.RustyPet1` sur l'objet `/dev/yrbane/RustyPet` :

| Membre | Type | Signature | Rôle |
|---|---|---|---|
| `Configure` | méthode | in `(iiiiii)` | `screen_w, screen_h, area_x, area_y, area_w, area_h` — l'extension fournit la géométrie ; le pet (ré)apparaît |
| `GetSprite` | méthode | out `(suuu)` | `sheet_path: s, tile_w: u, tile_h: u, columns: u` — l'extension apprend quoi afficher |
| `PetState` | signal | `(iiubu)` | `x: i, y: i, tile: u, flipped: b, opacity: u` (opacité sur 0–255) |

`columns` = nombre de tuiles par ligne du spritesheet (`tiles_x`), nécessaire à l'extension pour convertir un index de tuile en décalage `background-position`.

---

### Task 1: Crate `petd` et écriture du spritesheet en cache

**Files:**
- Create: `crates/petd/Cargo.toml`, `crates/petd/src/main.rs`, `crates/petd/src/cache.rs`
- Modify: `Cargo.toml` (membre + dépendances workspace)

**Interfaces:**
- Consumes: `pet_format::SpriteSheet` (champs `width`, `height`, `rgba`, `tile_w`, `tile_h`, `tiles_x`, `tiles_y`).
- Produces:
  - `pub fn cache_dir() -> std::path::PathBuf` — `~/.cache/rustypet` (respecte `XDG_CACHE_HOME`).
  - `pub fn slugify(name: &str) -> String` — nom de pet → composant de chemin sûr.
  - `pub fn write_sprite_png(pet_name: &str, sheet: &SpriteSheet) -> std::io::Result<std::path::PathBuf>` — écrit le RGBA du spritesheet en PNG dans `<cache>/<slug>/sheet.png` et retourne le chemin.

**Contexte :** le PNG d'origine embarqué dans le XML n'a pas forcément la couleur-clé convertie en alpha ; c'est `SpriteSheet.rgba` (produit par `decode_sheet` en tâche 5 du plan 1) qui porte l'alpha correct. `petd` doit donc écrire **ce** buffer, pas le PNG brut. L'extension chargera ce fichier.

- [ ] **Step 1: Créer le crate et l'inscrire au workspace**

`crates/petd/Cargo.toml` :
```toml
[package]
name = "petd"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
pet-engine = { path = "../pet-engine" }
pet-format = { path = "../pet-format" }
zbus = { workspace = true }
tokio = { workspace = true }
image.workspace = true
clap.workspace = true

[dev-dependencies]
tempfile = "3"
serial_test = "3"
```

`serial_test` sérialise les tests qui manipulent `XDG_CACHE_HOME` (variable d'environnement globale au process) : sans cela, l'exécution parallèle des tests provoque des courses. Les tests concernés portent l'attribut `#[serial]`.

Dans le `Cargo.toml` du workspace, ajouter `"crates/petd"` à `members`, et sous `[workspace.dependencies]` :
```toml
zbus = { version = "5", default-features = false, features = ["tokio"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync"] }
```

- [ ] **Step 2: Écrire les tests qui échouent**

`crates/petd/src/cache.rs` :
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pet_format::SpriteSheet;

    fn sheet() -> SpriteSheet {
        // 2×1 tuiles de 3×2 px, RGBA plein.
        SpriteSheet {
            width: 6,
            height: 2,
            rgba: vec![0u8; 6 * 2 * 4],
            tile_w: 3,
            tile_h: 2,
            tiles_x: 2,
            tiles_y: 1,
        }
    }

    use serial_test::serial;

    #[test]
    fn slugify_nettoie_les_caracteres_de_chemin() {
        assert_eq!(slugify("Neko"), "neko");
        assert_eq!(slugify("Blue Sheep"), "blue_sheep");
        assert_eq!(slugify("../evil/./x"), "evil_x");
        assert_eq!(slugify(""), "pet");
    }

    #[test]
    fn cache_dir_respecte_xdg() {
        // On ne dépend pas de l'environnement réel : on vérifie juste que le
        // chemin se termine par rustypet.
        assert!(cache_dir().ends_with("rustypet"));
    }

    #[test]
    #[serial]
    fn write_sprite_png_ecrit_un_png_relisible() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Force XDG_CACHE_HOME sur le tempdir pour ce test.
        // SAFETY : mono-thread dans ce test.
        unsafe { std::env::set_var("XDG_CACHE_HOME", dir.path()) };

        let path = write_sprite_png("Neko", &sheet()).expect("écriture");
        assert!(path.exists(), "le fichier doit exister");
        assert!(path.ends_with("neko/sheet.png"));

        let decoded = image::open(&path).expect("relecture PNG");
        assert_eq!(decoded.width(), 6);
        assert_eq!(decoded.height(), 2);
    }
}
```

- [ ] **Step 3: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p petd`
Expected: FAIL — `cache_dir`, `slugify`, `write_sprite_png` introuvables.

- [ ] **Step 4: Implémenter le cache**

Au début de `crates/petd/src/cache.rs` :
```rust
//! Écriture du spritesheet décodé (RGBA, couleur-clé déjà appliquée) dans le
//! cache utilisateur, pour que l'extension GNOME le charge par chemin de
//! fichier. Voir `docs/reference/gnome50-dbus.md` §1.4.

use pet_format::SpriteSheet;
use std::path::{Path, PathBuf};

/// Répertoire de cache de RustyPet, respectant `XDG_CACHE_HOME`.
pub fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
            home.join(".cache")
        });
    base.join("rustypet")
}

/// Transforme un nom de pet en composant de chemin sûr : minuscules, seuls
/// `[a-z0-9_]` conservés, le reste fusionné en `_`. Jamais vide.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_underscore = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_underscore = false;
        } else if !last_underscore && !out.is_empty() {
            out.push('_');
            last_underscore = true;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "pet".to_string()
    } else {
        trimmed
    }
}

/// Écrit le RGBA du spritesheet en PNG sous `<cache>/<slug>/sheet.png`.
pub fn write_sprite_png(pet_name: &str, sheet: &SpriteSheet) -> std::io::Result<PathBuf> {
    let dir = cache_dir().join(slugify(pet_name));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("sheet.png");
    write_png(&path, sheet.width, sheet.height, &sheet.rgba)?;
    Ok(path)
}

/// Encode un buffer RGBA en PNG. Erreur d'encodage remontée en `io::Error`.
fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    image::save_buffer(path, rgba, width, height, image::ColorType::Rgba8.into())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
```

- [ ] **Step 5: `main.rs` provisoire pour que le crate compile**

`crates/petd/src/main.rs` :
```rust
//! Démon RustyPet : fait vivre un pet et publie son état sur D-Bus.

mod cache;

fn main() {
    // Le vrai point d'entrée arrive en tâche 3.
    println!("petd — squelette");
}
```

- [ ] **Step 6: Lancer les tests**

Run: `cargo test -p petd`
Expected: PASS — 3 tests.

Si `image::ColorType::Rgba8.into()` ne convient pas à la version d'`image` verrouillée (l'API de `save_buffer` a varié), consulter la doc de la version dans `Cargo.lock` et utiliser la forme correcte (`image::ExtendedColorType::Rgba8` selon les versions). Adapter sans changer le comportement.

- [ ] **Step 7: Vérifier lint et commiter**

```bash
cargo clippy --all-targets -- -D warnings && cargo fmt --all
git add crates/petd Cargo.toml CHANGELOG.md Cargo.lock
git commit -m "Démon petd : écriture du spritesheet en cache

Nouveau crate petd. Écrit le spritesheet décodé (RGBA, couleur-clé
appliquée) en PNG dans ~/.cache/rustypet/<slug>/sheet.png pour que
l'extension GNOME le charge par chemin. Version 0.10.0."
```

Bump `0.10.0`, entrée `CHANGELOG.md` :
```markdown
## 0.10.0 — 2026-07-21 · « Démon petd : cache du spritesheet »

- Nouveau crate `petd` (démon).
- Écriture du spritesheet décodé en PNG dans le cache utilisateur (XDG).
- Slugification sûre du nom de pet pour le chemin de cache.
```

---

### Task 2: Pilote du moteur — `Engine`

**Files:**
- Create: `crates/petd/src/engine.rs`
- Modify: `crates/petd/src/main.rs` (déclarer le module)

**Interfaces:**
- Consumes: `pet_engine::{Pet, World, TickOutcome, SeededRng, Rect}`, `pet_format::{parse_pet, decode_sheet, PetDefinition, SpriteSheet}`, `crate::cache::write_sprite_png`.
- Produces:
  - `pub struct PetFrame { pub x: i32, pub y: i32, pub tile: u32, pub flipped: bool, pub opacity: u32 }`
  - `pub struct SpriteInfo { pub sheet_path: String, pub tile_w: u32, pub tile_h: u32, pub columns: u32 }`
  - `pub struct Engine { … }` avec :
    - `pub fn load(xml_path: &str, seed: u64) -> Result<Engine, EngineError>` — parse le XML, décode et écrit le spritesheet en cache, prépare le moteur (mais ne fait pas apparaître le pet : il faut d'abord une géométrie).
    - `pub fn sprite_info(&self) -> SpriteInfo`
    - `pub fn configure(&mut self, world: Rect, area: Rect)` — (ré)initialise le monde et fait apparaître le pet.
    - `pub fn advance(&mut self) -> PetFrame` — avance d'un pas, gère la réapparition, retourne l'image à afficher.
    - `pub fn interval_ms(&self) -> u64` — délai avant le prochain pas.
  - `pub enum EngineError { Io(std::io::Error), Format(pet_format::FormatError) }` (via `thiserror` ou `From` manuel).

**Contexte :** ce module isole toute la logique du démon **hors D-Bus**, pour la rendre testable sans bus. `configure` prend deux rectangles : `world` = écran complet (bounds), `area` = zone de travail. Il construit un `pet_engine::World { bounds, area, windows: vec![] }` (pas de fenêtres dans ce plan). `advance` mappe `SpriteDraw` (f64 d'opacité) vers `PetFrame` (opacité 0–255 : `(opacity.clamp(0.0,1.0) * 255.0).round() as u32`), et relance `spawn` sur `TickOutcome::Respawn`.

Note API : `pet_engine::World` a des champs publics `bounds: Rect`, `area: Rect`, `windows: Vec<Rect>` (voir `crates/pet-engine/src/geometry.rs`). `Rect::new(x, y, w, h)` existe. `Pet::new(Arc<PetDefinition>, (i32, i32), &World)`, `Pet::spawn(&World, &mut dyn PetRng)`, `Pet::tick(&World, &mut dyn PetRng) -> TickOutcome`, `Pet::draw() -> SpriteDraw`, `Pet::interval_ms() -> i32`.

- [ ] **Step 1: Écrire les tests qui échouent**

`crates/petd/src/engine.rs` :
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn neko_path() -> String {
        // Fixture réelle du crate pet-format.
        format!(
            "{}/../pet-format/tests/fixtures/neko.xml",
            env!("CARGO_MANIFEST_DIR")
        )
    }

    fn with_temp_cache<T>(f: impl FnOnce() -> T) -> T {
        let dir = tempfile::tempdir().expect("tempdir");
        // SAFETY : les tests de ce module tournent en série (mono-thread par test).
        unsafe { std::env::set_var("XDG_CACHE_HOME", dir.path()) };
        f()
    }

    #[test]
    #[serial]
    fn load_prepare_le_sprite_et_expose_ses_dimensions() {
        with_temp_cache(|| {
            let engine = Engine::load(&neko_path(), 42).expect("chargement");
            let info = engine.sprite_info();
            assert!(info.tile_w > 0 && info.tile_h > 0);
            assert!(info.columns > 0);
            assert!(info.sheet_path.ends_with("sheet.png"));
            assert!(std::path::Path::new(&info.sheet_path).exists());
        });
    }

    #[test]
    #[serial]
    fn advance_produit_des_images_valides() {
        with_temp_cache(|| {
            let mut engine = Engine::load(&neko_path(), 42).expect("chargement");
            engine.configure(Rect::new(0, 0, 1920, 1080), Rect::new(0, 0, 1920, 1050));
            for _ in 0..500 {
                let frame = engine.advance();
                // Opacité toujours dans l'échelle Clutter, cadence jamais nulle.
                assert!((0..=255).contains(&frame.opacity));
                assert!(engine.interval_ms() >= 1);
            }
        });
    }

    #[test]
    #[serial]
    fn la_simulation_du_demon_est_deterministe() {
        with_temp_cache(|| {
            let trace = |seed| {
                let mut e = Engine::load(&neko_path(), seed).expect("chargement");
                e.configure(Rect::new(0, 0, 1920, 1080), Rect::new(0, 0, 1920, 1050));
                (0..300)
                    .map(|_| {
                        let f = e.advance();
                        (f.x, f.y, f.tile, f.flipped)
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(trace(7), trace(7));
        });
    }
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p petd engine`
Expected: FAIL — `Engine` introuvable.

- [ ] **Step 3: Implémenter le pilote**

Au début de `crates/petd/src/engine.rs` :
```rust
//! Pilote du moteur, indépendant de D-Bus : charge un pet, le fait vivre pas à
//! pas, et convertit chaque pas en une image affichable (`PetFrame`).

use crate::cache::write_sprite_png;
use pet_engine::{Pet, Rect, SeededRng, TickOutcome, World};
use pet_format::{decode_sheet, parse_pet, PetDefinition};
use std::sync::Arc;

/// Image à afficher pour un pas donné. Opacité sur 0–255 (échelle Clutter).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetFrame {
    pub x: i32,
    pub y: i32,
    pub tile: u32,
    pub flipped: bool,
    pub opacity: u32,
}

/// Ce que l'extension doit savoir pour afficher : le PNG et sa grille.
#[derive(Debug, Clone)]
pub struct SpriteInfo {
    pub sheet_path: String,
    pub tile_w: u32,
    pub tile_h: u32,
    pub columns: u32,
}

/// Erreur de chargement d'un pet.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("entrée/sortie : {0}")]
    Io(#[from] std::io::Error),
    #[error("format du pet : {0}")]
    Format(#[from] pet_format::FormatError),
}

/// Pilote complet d'un pet vivant.
pub struct Engine {
    definition: Arc<PetDefinition>,
    tile: (i32, i32),
    columns: u32,
    sheet_path: String,
    rng: SeededRng,
    world: World,
    pet: Option<Pet>,
}

impl Engine {
    /// Charge le pet et prépare son spritesheet en cache. Le pet n'apparaît
    /// qu'au premier `configure`.
    pub fn load(xml_path: &str, seed: u64) -> Result<Engine, EngineError> {
        let xml = std::fs::read_to_string(xml_path)?;
        let definition = Arc::new(parse_pet(&xml)?);
        let sheet = decode_sheet(&definition.image)?;
        let path = write_sprite_png(&definition.header.petname, &sheet)?;

        Ok(Engine {
            definition,
            tile: (sheet.tile_w as i32, sheet.tile_h as i32),
            columns: sheet.tiles_x,
            sheet_path: path.to_string_lossy().into_owned(),
            rng: SeededRng::new(seed),
            // Monde provisoire, remplacé au premier configure.
            world: World::simple(1, 1),
            pet: None,
        })
    }

    /// Informations d'affichage du spritesheet.
    pub fn sprite_info(&self) -> SpriteInfo {
        SpriteInfo {
            sheet_path: self.sheet_path.clone(),
            tile_w: self.tile.0 as u32,
            tile_h: self.tile.1 as u32,
            columns: self.columns,
        }
    }

    /// (Ré)initialise le monde à partir de la géométrie de l'écran, et fait
    /// (ré)apparaître le pet.
    pub fn configure(&mut self, bounds: Rect, area: Rect) {
        self.world = World { bounds, area, windows: Vec::new() };
        let mut pet = Pet::new(Arc::clone(&self.definition), self.tile, &self.world);
        pet.spawn(&self.world, &mut self.rng);
        self.pet = Some(pet);
    }

    /// Avance d'un pas et retourne l'image à afficher. Réapparition gérée.
    pub fn advance(&mut self) -> PetFrame {
        let Some(pet) = self.pet.as_mut() else {
            // Pas encore configuré : image neutre invisible.
            return PetFrame { x: 0, y: 0, tile: 0, flipped: false, opacity: 0 };
        };
        if pet.tick(&self.world, &mut self.rng) == TickOutcome::Respawn {
            pet.spawn(&self.world, &mut self.rng);
        }
        let draw = pet.draw();
        PetFrame {
            x: draw.x,
            y: draw.y,
            tile: draw.frame.max(0) as u32,
            flipped: draw.flipped,
            opacity: (draw.opacity.clamp(0.0, 1.0) * 255.0).round() as u32,
        }
    }

    /// Délai avant le prochain pas, en millisecondes (au moins 1).
    pub fn interval_ms(&self) -> u64 {
        self.pet.as_ref().map(|p| p.interval_ms().max(1) as u64).unwrap_or(100)
    }
}
```

Déclarer le module dans `crates/petd/src/main.rs` : ajouter `mod engine;` sous `mod cache;`.

Le crate `thiserror` est déjà une dépendance workspace : l'ajouter à `crates/petd/Cargo.toml` sous `[dependencies]` avec `thiserror.workspace = true`. Le dev-dependency `serial_test` a été ajouté en tâche 1 ; il sert ici aussi (l'attribut `#[serial]` sérialise les tests touchant `XDG_CACHE_HOME`).

- [ ] **Step 4: Lancer les tests**

Run: `cargo test -p petd`
Expected: PASS — les 3 tests de `cache` + les 3 de `engine`.

- [ ] **Step 5: Vérifier lint et commiter**

```bash
cargo clippy --all-targets -- -D warnings && cargo fmt --all
git add crates/petd Cargo.toml CHANGELOG.md
git commit -m "Démon petd : pilote du moteur

Structure Engine isolant la logique du démon hors D-Bus : chargement du
pet, apparition sur géométrie fournie, avance pas à pas convertie en
image affichable (PetFrame, opacité 0-255). Testable sans bus, et
déterministe à graine égale. Version 0.11.0."
```

Bump `0.11.0`, entrée `CHANGELOG.md` :
```markdown
## 0.11.0 — 2026-07-21 · « Démon petd : pilote du moteur »

- `Engine` : chargement d'un pet, apparition sur géométrie fournie.
- Avance pas à pas convertie en `PetFrame` (opacité échelle Clutter).
- Logique du démon entièrement testable sans D-Bus, déterministe.
```

---

### Task 3: Service D-Bus et boucle temps de `petd`

**Files:**
- Create: `crates/petd/src/service.rs`
- Modify: `crates/petd/src/main.rs` (point d'entrée réel, tokio, CLI)
- Create: `crates/petd/tests/dbus_smoke.sh` (test d'intégration scriptable, sans GNOME)

**Interfaces:**
- Consumes: `crate::engine::{Engine, PetFrame, SpriteInfo}`.
- Produces: le binaire `petd <chemin-animations.xml> [--seed N]`, qui possède le nom de bus `dev.yrbane.RustyPet`, expose `Configure`/`GetSprite` et émet `PetState`.

**Contexte (voir `docs/reference/gnome50-dbus.md` §2.2) :** zbus 5, macro `#[interface(name = "dev.yrbane.RustyPet1")]`, signal via méthode sans corps annotée `#[zbus(signal)]` prenant `&SignalEmitter<'_>`, runtime tokio. L'état (`Engine`) est partagé entre les méthodes D-Bus (`Configure` le reconfigure) et la boucle d'émission via `Arc<tokio::sync::Mutex<Engine>>`. La boucle émet `PetState` puis dort `interval_ms`, en boucle. Avant `Configure`, le pet est invisible (opacité 0) et la boucle tourne à cadence fixe.

Concurrence : `Configure` verrouille brièvement le mutex pour reconfigurer ; la boucle le verrouille à chaque pas pour avancer et lire l'image, puis relâche **avant** de dormir (ne jamais tenir le mutex pendant `sleep`).

- [ ] **Step 1: Écrire le test d'intégration scriptable**

`crates/petd/tests/dbus_smoke.sh` :
```bash
#!/usr/bin/env bash
# Test d'intégration sans GNOME : lance petd sur un bus de session jetable,
# appelle Configure et GetSprite, capture quelques signaux PetState.
# Réussite = au moins 3 signaux PetState observés après Configure.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
XML="$ROOT/crates/pet-format/tests/fixtures/neko.xml"
BIN="$ROOT/target/debug/petd"

[ -x "$BIN" ] || { echo "petd non construit : cargo build -p petd"; exit 1; }

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
```

Rendre exécutable : `chmod +x crates/petd/tests/dbus_smoke.sh`.

- [ ] **Step 2: Lancer le test pour vérifier qu'il échoue**

Run: `cargo build -p petd && ./crates/petd/tests/dbus_smoke.sh`
Expected: FAIL — `petd` ne possède pas encore le nom de bus, aucun signal. (Selon l'environnement, `busctl`/`dbus-monitor`/`dbus-run-session` doivent être installés — ils font partie de `dbus`/`systemd`, présents sur GNOME.)

- [ ] **Step 3: Implémenter le service**

`crates/petd/src/service.rs` :
```rust
//! Service D-Bus : expose Configure/GetSprite et émet PetState.
//! Voir `docs/reference/gnome50-dbus.md` §2.2.

use crate::engine::Engine;
use std::sync::Arc;
use tokio::sync::Mutex;
use zbus::interface;
use zbus::object_server::SignalEmitter;

/// État partagé entre les méthodes D-Bus et la boucle d'émission.
pub struct PetService {
    pub engine: Arc<Mutex<Engine>>,
}

#[interface(name = "dev.yrbane.RustyPet1")]
impl PetService {
    /// L'extension fournit la géométrie de l'écran ; le pet (ré)apparaît.
    async fn configure(
        &self,
        screen_w: i32,
        screen_h: i32,
        area_x: i32,
        area_y: i32,
        area_w: i32,
        area_h: i32,
    ) {
        use pet_engine::Rect;
        let mut engine = self.engine.lock().await;
        engine.configure(
            Rect::new(0, 0, screen_w, screen_h),
            Rect::new(area_x, area_y, area_w, area_h),
        );
    }

    /// Renseigne l'extension sur le PNG à charger et sa grille de tuiles.
    async fn get_sprite(&self) -> (String, u32, u32, u32) {
        let engine = self.engine.lock().await;
        let info = engine.sprite_info();
        (info.sheet_path, info.tile_w, info.tile_h, info.columns)
    }

    /// Émis à chaque pas : position, tuile, miroir, opacité (0–255).
    #[zbus(signal)]
    async fn pet_state(
        emitter: &SignalEmitter<'_>,
        x: i32,
        y: i32,
        tile: u32,
        flipped: bool,
        opacity: u32,
    ) -> zbus::Result<()>;
}
```

- [ ] **Step 4: Écrire le point d'entrée**

Remplacer `crates/petd/src/main.rs` par :
```rust
//! Démon RustyPet : fait vivre un pet et publie son état sur D-Bus.

mod cache;
mod engine;
mod service;

use clap::Parser;
use engine::Engine;
use service::PetService;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[derive(Parser)]
#[command(name = "petd", about = "Démon d'animaux de bureau RustyPet")]
struct Args {
    /// Chemin du fichier animations.xml du pet à afficher.
    xml: String,
    /// Graine du générateur aléatoire.
    #[arg(long, default_value_t = 42)]
    seed: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let engine = Engine::load(&args.xml, args.seed)?;
    let shared = Arc::new(Mutex::new(engine));

    let service = PetService { engine: Arc::clone(&shared) };
    let conn = zbus::connection::Builder::session()?
        .name("dev.yrbane.RustyPet")?
        .serve_at("/dev/yrbane/RustyPet", service)?
        .build()
        .await?;

    // Référence vers l'interface, pour émettre les signaux.
    let iface = conn
        .object_server()
        .interface::<_, PetService>("/dev/yrbane/RustyPet")
        .await?;

    // Boucle temps : avance le pet et émet son état à la cadence des animations.
    loop {
        let (frame, wait) = {
            let mut engine = shared.lock().await;
            let frame = engine.advance();
            (frame, engine.interval_ms())
        }; // mutex relâché avant de dormir

        let emitter = iface.signal_emitter();
        PetService::pet_state(
            emitter,
            frame.x,
            frame.y,
            frame.tile,
            frame.flipped,
            frame.opacity,
        )
        .await?;

        sleep(Duration::from_millis(wait)).await;
    }
}
```

- [ ] **Step 5: Lancer le test d'intégration**

Run: `cargo build -p petd && ./crates/petd/tests/dbus_smoke.sh`
Expected: PASS — « signaux PetState observés : N » avec N ≥ 3, et `GetSprite` retourne un chemin et des dimensions non nulles.

Si `busctl --user` échoue dans le bus jetable de `dbus-run-session`, utiliser `busctl --address="$DBUS_SESSION_BUS_ADDRESS"` ou `gdbus call --session`. Adapter le script sans affaiblir l'assertion (≥ 3 signaux).

- [ ] **Step 6: Vérifier lint et commiter**

```bash
cargo clippy --all-targets -- -D warnings && cargo fmt --all
git add crates/petd Cargo.toml CHANGELOG.md
git commit -m "Démon petd : service D-Bus et boucle temps

petd possède dev.yrbane.RustyPet, expose Configure et GetSprite, et émet
le signal PetState à la cadence des animations. État partagé par mutex
tokio entre méthodes et boucle d'émission. Test d'intégration D-Bus
scriptable, sans GNOME. Version 0.12.0."
```

Bump `0.12.0`, entrée `CHANGELOG.md` :
```markdown
## 0.12.0 — 2026-07-21 · « Démon petd : service D-Bus »

- petd possède `dev.yrbane.RustyPet`, expose `Configure` et `GetSprite`.
- Émission du signal `PetState` à la cadence des animations.
- Test d'intégration D-Bus scriptable (dbus-run-session), sans GNOME.
```

---

### Task 4: Logique de tuile de l'extension (testable hors GNOME)

**Files:**
- Create: `extension/petMath.js`, `extension/tests/petMath.test.js`

**Interfaces:**
- Produces (module ESM `petMath.js`) :
  - `export function tileBackgroundPosition(tile, tileW, tileH, columns)` → `{ x, y }` — décalage `background-position` en pixels (négatif) pour la tuile d'index `tile`, feuille de `columns` tuiles par ligne. `x = -(tile % columns) * tileW`, `y = -Math.floor(tile / columns) * tileH`.
  - `export function clutterOpacity(opacity255)` → entier borné dans `[0, 255]`.

**Contexte :** l'extension GNOME ne peut pas être testée hors du Shell, **sauf** la logique pure, exécutable via `gjs-console` (voir `docs/reference/gnome50-dbus.md` §3.2). On isole donc le calcul du décalage de tuile dans un module sans dépendance à `St`/`Clutter`, et on le teste réellement. La découpe est en ligne d'abord, cohérente avec le moteur (`index = row * columns + col`).

Le dossier `extension/` (à la racine du dépôt) contient l'extension ; il sera copié vers `~/.local/share/gnome-shell/extensions/rustypet@yrbane.dev/` en tâche 6. Le sous-dossier `tests/` n'est pas copié (il ne gêne pas le Shell, mais on l'exclut proprement à l'installation).

- [ ] **Step 1: Écrire le module de logique**

`extension/petMath.js` :
```js
// Calculs purs de l'extension, sans dépendance à GNOME Shell — testables via
// gjs-console. Découpe des tuiles en ligne d'abord : index = row*columns + col.

/**
 * Décalage `background-position` (en pixels, négatif) affichant la tuile
 * d'index `tile` d'un spritesheet de `columns` tuiles par ligne.
 */
export function tileBackgroundPosition(tile, tileW, tileH, columns) {
    const cols = Math.max(1, columns | 0);
    const t = Math.max(0, tile | 0);
    const col = t % cols;
    const row = Math.floor(t / cols);
    return { x: -col * tileW, y: -row * tileH };
}

/** Borne une opacité 0–255 en entier valide pour Clutter. */
export function clutterOpacity(opacity255) {
    const v = Math.round(opacity255);
    if (v < 0) return 0;
    if (v > 255) return 255;
    return v;
}
```

- [ ] **Step 2: Écrire le test exécutable par gjs**

`extension/tests/petMath.test.js` :
```js
// Test autonome, lancé par gjs-console (pas besoin de GNOME Shell).
// Sortie : « OK » et code 0 si tout passe ; sinon lève et code non nul.

import { tileBackgroundPosition, clutterOpacity } from '../petMath.js';

function assertEq(actual, expected, label) {
    const a = JSON.stringify(actual);
    const e = JSON.stringify(expected);
    if (a !== e) throw new Error(`${label} : attendu ${e}, obtenu ${a}`);
}

// Tuile 0 → coin haut-gauche.
assertEq(tileBackgroundPosition(0, 64, 64, 4), { x: 0, y: 0 }, 'tuile 0');
// Tuile 1 → une colonne à droite.
assertEq(tileBackgroundPosition(1, 64, 64, 4), { x: -64, y: 0 }, 'tuile 1');
// Tuile 4 → ligne suivante (4 colonnes) : ligne d'abord.
assertEq(tileBackgroundPosition(4, 64, 64, 4), { x: 0, y: -64 }, 'tuile 4');
// Tuile 6 → ligne 1, colonne 2.
assertEq(tileBackgroundPosition(6, 32, 48, 4), { x: -64, y: -48 }, 'tuile 6');
// columns=0 protégé.
assertEq(tileBackgroundPosition(3, 10, 10, 0), { x: 0, y: -30 }, 'columns=0');

assertEq(clutterOpacity(300), 255, 'opacité haute');
assertEq(clutterOpacity(-5), 0, 'opacité basse');
assertEq(clutterOpacity(128), 128, 'opacité milieu');

print('OK');
```

- [ ] **Step 3: Lancer le test et constater qu'il passe**

Run: `gjs -m extension/tests/petMath.test.js`
Expected: affiche `OK`, code de sortie 0.

Note : `gjs -m` exécute un module ESM. Si l'option diffère sur la version installée, essayer `gjs --module` ou `gjs-console -m`. Vérifier d'abord que la commande échoue si on casse volontairement une assertion (mordant du test), puis rétablir.

- [ ] **Step 4: Commiter**

```bash
git add extension/petMath.js extension/tests/petMath.test.js CHANGELOG.md Cargo.toml
git commit -m "Extension : logique de tuile testable hors GNOME

Module petMath.js (calcul du décalage background-position d'une tuile,
bornage d'opacité), testé via gjs-console sans lancer le Shell.
Version 0.13.0."
```

Bump `0.13.0` dans le `Cargo.toml` du workspace (le versionnage suit le dépôt, même pour du JS), entrée `CHANGELOG.md` :
```markdown
## 0.13.0 — 2026-07-21 · « Extension : logique de tuile »

- Module `petMath.js` : décalage de tuile, bornage d'opacité.
- Testé hors GNOME via gjs-console (découpe en ligne d'abord).
```

---

### Task 5: Extension GNOME Shell — affichage et abonnement D-Bus

**Files:**
- Create: `extension/metadata.json`, `extension/extension.js`

**Interfaces:**
- Consumes: `petMath.js` (tâche 4), le service D-Bus de `petd` (tâche 3).
- Produces: une extension chargeable qui, une fois `petd` lancé, affiche le pet et le fait bouger.

**Contexte (voir `docs/reference/gnome50-dbus.md` §1 et §2.1) :** à `enable()`, l'extension (1) lance `petd` avec un pet fixé via `Gio.Subprocess`, (2) crée un proxy D-Bus, (3) appelle `Configure` avec la géométrie du moniteur primaire, (4) appelle `GetSprite`, crée le `St.Widget` avec le PNG en `background-image`, (5) s'abonne à `PetState` et déplace/actualise l'acteur à chaque signal. À `disable()`, tout est démonté et `petd` arrêté.

Le chemin du pet est fixé en dur pour ce plan (sélection ultérieure) : `~/Dev/desktopPet/Pets/neko/animations.xml`. Le chemin du binaire `petd` est fixé à `~/.local/bin/petd` (installé en tâche 6) avec repli sur `target/debug/petd` du dépôt pour le développement.

- [ ] **Step 1: Écrire `metadata.json`**

`extension/metadata.json` :
```json
{
    "uuid": "rustypet@yrbane.dev",
    "name": "RustyPet",
    "description": "Affiche un animal de bureau animé, piloté par le démon petd.",
    "shell-version": [ "50" ],
    "url": "https://github.com/yrbane/rustypet"
}
```

- [ ] **Step 2: Écrire `extension.js`**

`extension/extension.js` :
```js
import St from 'gi://St';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import { tileBackgroundPosition, clutterOpacity } from './petMath.js';

const BUS_NAME = 'dev.yrbane.RustyPet';
const OBJECT_PATH = '/dev/yrbane/RustyPet';
const IFACE = 'dev.yrbane.RustyPet1';

export default class RustyPetExtension extends Extension {
    enable() {
        this._actor = null;
        this._proxy = null;
        this._signalId = 0;
        this._subprocess = null;
        this._tileW = 0;
        this._tileH = 0;
        this._columns = 1;

        this._startDaemon();
        // Laisse au démon le temps de prendre le nom de bus, puis se connecte.
        this._connectId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 600, () => {
            this._connect().catch(e => logError(e, 'RustyPet: connexion'));
            this._connectId = 0;
            return GLib.SOURCE_REMOVE;
        });
    }

    _daemonPath() {
        // Binaire installé, sinon build de développement du dépôt.
        const installed = GLib.build_filenamev([GLib.get_home_dir(), '.local', 'bin', 'petd']);
        if (GLib.file_test(installed, GLib.FileTest.IS_EXECUTABLE)) return installed;
        return GLib.build_filenamev(
            [GLib.get_home_dir(), 'Dev', 'rustypet', 'target', 'debug', 'petd']);
    }

    _startDaemon() {
        const petXml = GLib.build_filenamev(
            [GLib.get_home_dir(), 'Dev', 'desktopPet', 'Pets', 'neko', 'animations.xml']);
        try {
            this._subprocess = Gio.Subprocess.new(
                [this._daemonPath(), petXml],
                Gio.SubprocessFlags.NONE);
        } catch (e) {
            logError(e, 'RustyPet: lancement de petd');
        }
    }

    async _connect() {
        this._proxy = await Gio.DBusProxy.new_for_bus(
            Gio.BusType.SESSION, Gio.DBusProxyFlags.NONE, null,
            BUS_NAME, OBJECT_PATH, IFACE, null);

        // Géométrie du moniteur primaire + zone de travail.
        const monitor = Main.layoutManager.primaryMonitor;
        const wa = Main.layoutManager.getWorkAreaForMonitor(Main.layoutManager.primaryIndex);
        this._proxy.call_sync(
            'Configure',
            new GLib.Variant('(iiiiii)',
                [monitor.width, monitor.height, wa.x, wa.y, wa.width, wa.height]),
            Gio.DBusCallFlags.NONE, -1, null);

        // Dimensions du sprite.
        const res = this._proxy.call_sync(
            'GetSprite', null, Gio.DBusCallFlags.NONE, -1, null);
        const [sheetPath, tileW, tileH, columns] = res.deepUnpack();
        this._tileW = tileW;
        this._tileH = tileH;
        this._columns = columns;

        const uri = GLib.filename_to_uri(sheetPath, null);
        this._actor = new St.Widget({ reactive: false, width: tileW, height: tileH });
        this._actor.set_pivot_point(0.5, 0.5);
        this._baseStyle =
            `background-image: url("${uri}"); background-repeat: no-repeat;`;
        this._actor.set_style(this._baseStyle);
        Main.layoutManager.uiGroup.add_child(this._actor);

        this._signalId = this._proxy.connectSignal(
            'PetState', (_p, _s, args) => this._onState(args));
    }

    _onState(args) {
        if (!this._actor) return;
        const [x, y, tile, flipped, opacity] = args;
        const pos = tileBackgroundPosition(tile, this._tileW, this._tileH, this._columns);
        this._actor.set_style(
            `${this._baseStyle} background-position: ${pos.x}px ${pos.y}px;`);
        this._actor.set_position(x, y);
        this._actor.scale_x = flipped ? -1 : 1;
        this._actor.opacity = clutterOpacity(opacity);
    }

    disable() {
        if (this._connectId) { GLib.source_remove(this._connectId); this._connectId = 0; }
        if (this._proxy && this._signalId) {
            this._proxy.disconnectSignal(this._signalId);
            this._signalId = 0;
        }
        this._proxy = null;
        this._actor?.destroy();
        this._actor = null;
        // Arrête le démon lancé par l'extension.
        this._subprocess?.force_exit();
        this._subprocess = null;
    }
}
```

- [ ] **Step 3: Vérifier la syntaxe hors Shell**

Run: `gjs -c "import('./extension/petMath.js').then(() => print('import ok'))"`
Expected: `import ok` — confirme que `petMath.js` (importé par l'extension) est un module ESM valide. `extension.js` lui-même ne peut pas être importé hors du Shell (il dépend de `resource:///…`) ; sa validation est manuelle en tâche 6.

Vérifier aussi que `metadata.json` est un JSON valide :
Run: `python3 -c "import json,sys; json.load(open('extension/metadata.json')); print('json ok')"`
Expected: `json ok`.

- [ ] **Step 4: Commiter**

```bash
git add extension/metadata.json extension/extension.js CHANGELOG.md Cargo.toml
git commit -m "Extension GNOME : affichage et abonnement D-Bus

L'extension lance petd, fournit la géométrie via Configure, charge le
spritesheet et déplace un St.Widget à chaque signal PetState (position,
tuile, miroir, opacité). Version 0.14.0."
```

Bump `0.14.0`, entrée `CHANGELOG.md` :
```markdown
## 0.14.0 — 2026-07-21 · « Extension GNOME : affichage »

- Extension GNOME Shell 50 (ESM) : lancement de petd, géométrie, rendu.
- Acteur St.Widget piloté par le signal PetState (position, tuile,
  miroir, opacité).
```

---

### Task 6: Outillage d'installation et vérification bout-en-bout

**Files:**
- Create: `scripts/install-extension.sh`, `scripts/dev-session.sh`
- Modify: `README.md` (section « Voir le pet à l'écran »)

**Interfaces:**
- Consumes: tout ce qui précède.
- Produces: de quoi installer l'extension, construire `petd`, lancer une session imbriquée de développement, et une check-list de vérification manuelle.

**Contexte :** la vérification finale à l'écran ne peut pas être automatisée par un sous-agent — elle exige une session GNOME et un regard humain. Cette tâche fournit les scripts et une check-list que **l'utilisateur** exécute. Le sous-agent implémente les scripts et vérifie ce qui est vérifiable sans compositeur (les scripts s'exécutent sans erreur de syntaxe, l'extension s'installe au bon endroit, `petd` se construit).

- [ ] **Step 1: Écrire le script d'installation**

`scripts/install-extension.sh` :
```bash
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
```

- [ ] **Step 2: Écrire le script de session de développement**

`scripts/dev-session.sh` :
```bash
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
```

Rendre les deux exécutables : `chmod +x scripts/install-extension.sh scripts/dev-session.sh`.

- [ ] **Step 3: Vérifier que les scripts sont sains**

Run: `bash -n scripts/install-extension.sh && bash -n scripts/dev-session.sh && echo "syntaxe ok"`
Expected: `syntaxe ok`.

Run: `./scripts/install-extension.sh`
Expected: construit `petd` en release, l'installe dans `~/.local/bin/petd`, copie les trois fichiers de l'extension dans `~/.local/share/gnome-shell/extensions/rustypet@yrbane.dev/`. Vérifier ensuite :
Run: `ls ~/.local/share/gnome-shell/extensions/rustypet@yrbane.dev/`
Expected: `extension.js  metadata.json  petMath.js`.

- [ ] **Step 4: Écrire la section README**

Ajouter à `README.md`, après la section existante :
```markdown
## Voir le pet à l'écran (GNOME Shell 50, Wayland)

Le rendu passe par une extension GNOME Shell pilotée par le démon `petd`.

### Installation

```bash
./scripts/install-extension.sh
gnome-extensions enable rustypet@yrbane.dev
```

Puis **déconnectez et reconnectez votre session** : sous Wayland, GNOME ne
recharge pas les extensions à chaud. Le pet apparaît alors sur le bureau.

### Développement sans toucher à sa session

```bash
./scripts/dev-session.sh
```

Ceci ouvre une session GNOME imbriquée (`gnome-shell --devkit --wayland`,
GNOME 49+) dans une fenêtre. Dans le terminal de cette session :

```bash
gnome-extensions enable rustypet@yrbane.dev
```

Les journaux s'affichent dans le terminal qui a lancé le script.

### Ce qui marche à ce stade

Un seul pet, la physique des bords d'écran (il marche, tombe, rebondit sur
les bords et la barre des tâches). La marche sur les fenêtres, l'audio, le
menu, le glisser-déposer et le multi-pets viennent dans les plans suivants.
```

- [ ] **Step 5: Check-list de vérification manuelle (exécutée par l'utilisateur)**

Cette étape n'est pas automatisable ; elle liste ce que l'utilisateur vérifie dans une session imbriquée. La consigner dans le rapport de tâche comme check-list à faire valider :

1. `./scripts/dev-session.sh` ouvre une fenêtre de session GNOME imbriquée.
2. `gnome-extensions enable rustypet@yrbane.dev` dans cette session ne produit pas d'erreur dans les logs.
3. Un pet (Neko) **apparaît** à l'écran.
4. Il **se déplace** et **s'anime** (les frames changent), il tombe et **s'arrête au bas** de la zone de travail.
5. Il ne **sort pas** par les bords latéraux.
6. Le sprite est **détouré** (fond transparent, pas de rectangle magenta).
7. `disable` (via l'outil d'extensions) le fait disparaître et arrête `petd` (`pgrep petd` ne retourne rien).

- [ ] **Step 6: Commiter**

```bash
git add scripts/install-extension.sh scripts/dev-session.sh README.md CHANGELOG.md Cargo.toml
git commit -m "Outillage d'installation et session de développement

Scripts d'installation de l'extension et de session imbriquée devkit,
section README pour voir le pet à l'écran, check-list de vérification
manuelle. Version 0.15.0."
```

Bump `0.15.0`, entrée `CHANGELOG.md` :
```markdown
## 0.15.0 — 2026-07-21 · « Rendu bout-en-bout sous GNOME »

- Scripts d'installation de l'extension et de session de développement.
- README : comment voir le pet à l'écran, en session réelle ou imbriquée.
- Squelette de rendu complet : un pet animé, physique des bords d'écran.
```

---

## Fin du plan 2a

À ce stade, **un pet vivant s'affiche et se déplace à l'écran** sous GNOME/Wayland, piloté par le moteur du plan 1 via D-Bus. Le squelette de rendu bout-en-bout est prouvé.

Restent pour les plans suivants : la marche sur les fenêtres (géométrie remontée par l'extension via `WindowsChanged`, atterrissage `FallDetect`), le glisser-déposer, l'audio (rodio), le menu du panneau, le téléchargement de pets, et le multi-pets avec enfants.
