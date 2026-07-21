//! Rectangles et description du bureau.

/// Rectangle en pixels, origine en haut-gauche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    /// Bord droit, exclu.
    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    /// Bord bas, exclu.
    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }
}

/// Le bureau tel que le voit le moteur.
#[derive(Debug, Clone)]
pub struct World {
    /// Écran complet.
    pub bounds: Rect,
    /// Zone de travail, barres exclues.
    pub area: Rect,
    /// Fenêtres sur lesquelles le pet peut marcher. Vide en mode dégradé.
    pub windows: Vec<Rect>,
}

impl World {
    /// Un bureau simple sans fenêtre, pour les tests et le mode dégradé.
    pub fn simple(width: i32, height: i32) -> Self {
        let bounds = Rect::new(0, 0, width, height);
        Self {
            bounds,
            area: bounds,
            windows: Vec::new(),
        }
    }
}
