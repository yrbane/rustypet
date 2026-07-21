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
    // `pub` nécessaire : le brief appelle `PetService::pet_state` depuis
    // `main.rs`, un module distinct de `service.rs`.
    #[zbus(signal)]
    pub async fn pet_state(
        emitter: &SignalEmitter<'_>,
        x: i32,
        y: i32,
        tile: u32,
        flipped: bool,
        opacity: u32,
    ) -> zbus::Result<()>;
}
