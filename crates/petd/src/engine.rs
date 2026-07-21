//! Pilote du moteur, indépendant de D-Bus : charge un pet, le fait vivre pas à
//! pas, et convertit chaque pas en une image affichable (`PetFrame`).

use crate::cache::write_sprite_png;
use pet_engine::{Pet, Rect, SeededRng, TickOutcome, World};
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
        self.world = World {
            bounds,
            area,
            windows: Vec::new(),
        };
        let mut pet = Pet::new(Arc::clone(&self.definition), self.tile, &self.world);
        pet.spawn(&self.world, &mut self.rng);
        self.pet = Some(pet);
    }

    /// Avance d'un pas et retourne l'image à afficher. Réapparition gérée.
    pub fn advance(&mut self) -> PetFrame {
        let Some(pet) = self.pet.as_mut() else {
            // Pas encore configuré : image neutre invisible.
            return PetFrame {
                x: 0,
                y: 0,
                tile: 0,
                flipped: false,
                opacity: 0,
            };
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
        self.pet
            .as_ref()
            .map(|p| p.interval_ms().max(1) as u64)
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
