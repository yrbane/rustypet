//! Valeur d'animation : expression, classification statique, valeur calculée.
//! Voir `docs/reference/esheep-engine.md` §3.4.

use crate::context::{EvalContext, PetRng, eval};

/// Une valeur issue du XML : l'expression source, sa classification, et la
/// dernière valeur calculée.
#[derive(Debug, Clone)]
pub struct PetValue {
    /// Expression telle qu'écrite dans le XML.
    pub compute: String,
    /// Doit être réévaluée à chaque démarrage d'animation.
    pub is_dynamic: bool,
    /// Dépend des dimensions de l'écran : à recalculer au changement de moniteur.
    pub is_screen: bool,
    /// Dernière valeur calculée.
    pub value: i32,
}

impl PetValue {
    /// Construit la valeur et la classe, sans l'évaluer.
    pub fn new(compute: String) -> Self {
        let is_dynamic = compute.contains("random")
            || compute.contains("randS")
            || compute.contains("imageX")
            || compute.contains("imageY");
        let is_screen = compute.contains("screen") || compute.contains("area");
        Self {
            compute,
            is_dynamic,
            is_screen,
            value: 0,
        }
    }

    /// Recalcule la valeur à partir du contexte courant.
    pub fn update(&mut self, ctx: &EvalContext, rng: &mut dyn PetRng, parent_flipped: bool) {
        self.value = eval(&self.compute, ctx, rng, parent_flipped);
    }

    /// Dernière valeur calculée.
    pub fn get(&self) -> i32 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{EvalContext, SeededRng};

    fn ctx() -> EvalContext {
        EvalContext {
            screen_w: 1920,
            screen_h: 1080,
            area_w: 1920,
            area_h: 1050,
            image_w: 64,
            image_h: 64,
            image_x: -1,
            image_y: -1,
            rand_spawn: 42,
            scale: 1,
        }
    }

    #[test]
    fn classe_les_expressions_dynamiques() {
        assert!(PetValue::new("random*2".into()).is_dynamic);
        assert!(PetValue::new("randS".into()).is_dynamic);
        assert!(PetValue::new("imageX".into()).is_dynamic);
        assert!(PetValue::new("imageY".into()).is_dynamic);
        assert!(!PetValue::new("imageW".into()).is_dynamic);
        assert!(!PetValue::new("12".into()).is_dynamic);
    }

    #[test]
    fn classe_les_expressions_dependant_de_l_ecran() {
        assert!(PetValue::new("screenW".into()).is_screen);
        assert!(PetValue::new("areaH".into()).is_screen);
        assert!(!PetValue::new("imageW".into()).is_screen);
    }

    #[test]
    fn update_puis_get_renvoie_la_valeur_calculee() {
        let mut rng = SeededRng::new(1);
        let mut v = PetValue::new("screenW/2".into());
        v.update(&ctx(), &mut rng, false);
        assert_eq!(v.get(), 960);
    }
}
