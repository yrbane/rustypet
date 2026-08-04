//! Pilote du moteur, indépendant de D-Bus : charge un pet, le fait vivre pas à
//! pas, et convertit chaque pas en une image affichable (`PetFrame`).

use crate::cache::write_sprite_png;
use pet_engine::{Flock, Rect, SeededRng, World};
use pet_format::{PetDefinition, decode_sheet, parse_pet};
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

/// Pilote complet d'un troupeau vivant (pet principal + enfants).
pub struct Engine {
    definition: Arc<PetDefinition>,
    tile: (i32, i32),
    columns: u32,
    sheet_path: String,
    rng: SeededRng,
    world: World,
    flock: Option<Flock>,
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
            flock: None,
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
    /// (ré)apparaître le troupeau.
    pub fn configure(&mut self, bounds: Rect, area: Rect) {
        self.world = World {
            bounds,
            area,
            windows: Vec::new(),
        };
        let mut flock = Flock::new(Arc::clone(&self.definition), self.tile, &self.world);
        flock.spawn(&self.world, &mut self.rng);
        self.flock = Some(flock);
    }

    /// Met à jour les fenêtres sur lesquelles le pet peut marcher.
    pub fn set_windows(&mut self, windows: Vec<Rect>) {
        self.world.windows = windows;
    }

    /// Fait passer `elapsed_ms` millisecondes et retourne les images à
    /// afficher, le pet principal en tête. Réapparitions et fermetures
    /// d'enfants gérées par le troupeau.
    pub fn advance(&mut self, elapsed_ms: i64) -> Vec<PetFrame> {
        let Some(flock) = self.flock.as_mut() else {
            // Pas encore configuré : aucun acteur.
            return Vec::new();
        };
        flock.advance(&self.world, &mut self.rng, elapsed_ms);
        flock
            .draws()
            .iter()
            .map(|draw| PetFrame {
                x: draw.x,
                y: draw.y,
                tile: draw.frame.max(0) as u32,
                flipped: draw.flipped,
                opacity: (draw.opacity.clamp(0.0, 1.0) * 255.0).round() as u32,
            })
            .collect()
    }

    /// Délai avant la prochaine échéance du troupeau, en millisecondes.
    pub fn interval_ms(&self) -> u64 {
        self.flock
            .as_ref()
            .map(|f| f.next_wait_ms().max(1) as u64)
            .unwrap_or(100)
    }
}

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
            let mut elapsed = 0;
            for _ in 0..500 {
                let frames = engine.advance(elapsed);
                assert!(!frames.is_empty(), "le principal est toujours affiché");
                for frame in &frames {
                    // Opacité toujours dans l'échelle Clutter.
                    assert!((0..=255).contains(&frame.opacity));
                }
                elapsed = engine.interval_ms() as i64;
                assert!(elapsed >= 1, "cadence jamais nulle");
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
                let mut elapsed = 0;
                (0..300)
                    .map(|_| {
                        let frames = e.advance(elapsed);
                        elapsed = e.interval_ms() as i64;
                        frames
                            .iter()
                            .map(|f| (f.x, f.y, f.tile, f.flipped))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(trace(7), trace(7));
        });
    }
}
