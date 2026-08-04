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

    /// L'extension remonte les fenêtres visibles (x, y, largeur, hauteur) ;
    /// le pet peut atterrir dessus et arpenter leur toit.
    async fn update_windows(&self, windows: Vec<(i32, i32, i32, i32)>) {
        use pet_engine::Rect;
        let rects = windows
            .iter()
            .map(|&(x, y, w, h)| Rect::new(x, y, w, h))
            .collect();
        let mut engine = self.engine.lock().await;
        engine.set_windows(rects);
    }

    /// Le pet principal est attrapé à la souris.
    async fn begin_drag(&self) {
        self.engine.lock().await.begin_drag();
    }

    /// Le curseur a bougé pendant le glisser (coordonnées écran).
    async fn drag_to(&self, x: i32, y: i32) {
        self.engine.lock().await.drag_to(x, y);
    }

    /// Le pet principal est relâché : il retombe.
    async fn end_drag(&self) {
        self.engine.lock().await.end_drag();
    }

    /// Renseigne l'extension sur le PNG à charger et sa grille de tuiles.
    async fn get_sprite(&self) -> (String, u32, u32, u32) {
        let engine = self.engine.lock().await;
        let info = engine.sprite_info();
        (info.sheet_path, info.tile_w, info.tile_h, info.columns)
    }

    /// Émis à chaque pas : un tuple (x, y, tuile, miroir, opacité 0–255)
    /// par acteur, le pet principal en tête, les enfants ensuite.
    // `pub` nécessaire : le brief appelle `PetService::pet_state` depuis
    // `main.rs`, un module distinct de `service.rs`.
    #[zbus(signal)]
    pub async fn pet_state(
        emitter: &SignalEmitter<'_>,
        actors: Vec<(i32, i32, u32, bool, u32)>,
    ) -> zbus::Result<()>;

    /// Émis quand une animation tire son son : chemin d'un WAV en cache,
    /// que l'extension joue via l'API sonore de GNOME.
    #[zbus(signal)]
    pub async fn pet_sound(emitter: &SignalEmitter<'_>, path: String) -> zbus::Result<()>;
}
