//! Substitution des jetons symboliques et évaluation contextuelle.
//! Voir `docs/reference/esheep-engine.md` §3.1 et §3.2.

use crate::eval_arithmetic;
use rand::{Rng, SeedableRng};

/// Source d'aléa du moteur. Injectée partout pour rendre la simulation
/// déterministe en test.
pub trait PetRng {
    /// Tire un entier dans `[low, high]`, bornes incluses.
    fn gen_range_i32(&mut self, low: i32, high: i32) -> i32;
}

/// Générateur seedé, reproductible d'une exécution à l'autre.
pub struct SeededRng(rand::rngs::StdRng);

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        Self(rand::rngs::StdRng::seed_from_u64(seed))
    }
}

impl PetRng for SeededRng {
    fn gen_range_i32(&mut self, low: i32, high: i32) -> i32 {
        if low >= high {
            return low;
        }
        self.0.random_range(low..=high)
    }
}

/// Valeurs substituées dans les expressions des animations.
#[derive(Debug, Clone, Copy)]
pub struct EvalContext {
    /// Largeur totale de l'écran.
    pub screen_w: i32,
    /// Hauteur totale de l'écran.
    pub screen_h: i32,
    /// Largeur de la zone de travail.
    pub area_w: i32,
    /// Bord bas de la zone de travail (hauteur + décalage Y). Voir §3.1.
    pub area_h: i32,
    /// Largeur d'une frame.
    pub image_w: i32,
    /// Hauteur d'une frame.
    pub image_h: i32,
    /// X du parent, -1 si le pet n'est pas un enfant.
    pub image_x: i32,
    /// Y du parent, -1 si le pet n'est pas un enfant.
    pub image_y: i32,
    /// Tirage figé au chargement du XML, dans [10, 90].
    pub rand_spawn: i32,
    /// Facteur d'échelle HiDPI.
    pub scale: i32,
}

/// Remplace les jetons symboliques par leurs valeurs.
///
/// L'ordre importe : `random` est substitué avant `randS`, et toutes les
/// occurrences de `random` dans une même expression reçoivent la même valeur.
pub fn substitute(
    expr: &str,
    ctx: &EvalContext,
    rng: &mut dyn PetRng,
    parent_flipped: bool,
) -> String {
    let mut out = expr.to_string();

    // Miroir horizontal du placement d'un enfant sous parent retourné (§3.2).
    if parent_flipped {
        if out.contains("-imageW") {
            out = out.replace("-imageW", "+imageW");
        } else {
            out = out.replace("imageW", "(0-imageW)");
        }
    }

    // Une seule valeur de `random` pour toute l'expression.
    let random_value = rng.gen_range_i32(0, 100);
    out = out.replace("random", &random_value.to_string());
    out = out.replace("randS", &ctx.rand_spawn.to_string());

    out = out.replace("screenW", &ctx.screen_w.to_string());
    out = out.replace("screenH", &ctx.screen_h.to_string());
    out = out.replace("areaW", &ctx.area_w.to_string());
    out = out.replace("areaH", &ctx.area_h.to_string());
    out = out.replace("imageW", &ctx.image_w.to_string());
    out = out.replace("imageH", &ctx.image_h.to_string());
    out = out.replace("imageX", &ctx.image_x.to_string());
    out = out.replace("imageY", &ctx.image_y.to_string());
    out = out.replace("scale", &ctx.scale.to_string());

    out
}

/// Substitue puis évalue. Retourne 0 si l'expression est invalide, comme le
/// moteur d'origine.
pub fn eval(expr: &str, ctx: &EvalContext, rng: &mut dyn PetRng, parent_flipped: bool) -> i32 {
    let substituted = substitute(expr, ctx, rng, parent_flipped);
    match eval_arithmetic(&substituted) {
        // Troncature vers zéro, comme le cast (int) du C#.
        Ok(v) => v.trunc() as i32,
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn substitue_les_dimensions_d_ecran() {
        let mut rng = SeededRng::new(1);
        assert_eq!(eval("screenW/2-imageW/2", &ctx(), &mut rng, false), 928);
    }

    #[test]
    fn substitue_le_bord_bas_de_la_zone_de_travail() {
        let mut rng = SeededRng::new(1);
        assert_eq!(eval("areaH-imageH", &ctx(), &mut rng, false), 986);
    }

    #[test]
    #[allow(non_snake_case)] // Nom repris verbatim du cahier des charges (task-3-brief.md).
    fn randS_vient_du_contexte() {
        let mut rng = SeededRng::new(1);
        assert_eq!(eval("randS", &ctx(), &mut rng, false), 42);
    }

    #[test]
    fn toutes_les_occurrences_de_random_ont_la_meme_valeur() {
        let mut rng = SeededRng::new(7);
        // random-random vaut toujours 0 si la substitution est globale et unique.
        assert_eq!(eval("random-random", &ctx(), &mut rng, false), 0);
    }

    #[test]
    fn random_reste_dans_ses_bornes() {
        let mut rng = SeededRng::new(3);
        for _ in 0..100 {
            let v = eval("random", &ctx(), &mut rng, false);
            assert!((0..=100).contains(&v), "valeur hors bornes : {v}");
        }
    }

    #[test]
    fn tronque_vers_zero() {
        let mut rng = SeededRng::new(1);
        assert_eq!(eval("7/2", &ctx(), &mut rng, false), 3);
        assert_eq!(eval("0-7/2", &ctx(), &mut rng, false), -3);
    }

    #[test]
    fn retourne_zero_si_l_expression_est_invalide() {
        let mut rng = SeededRng::new(1);
        assert_eq!(eval("2+", &ctx(), &mut rng, false), 0);
    }

    #[test]
    #[allow(non_snake_case)] // Nom repris verbatim du cahier des charges (task-3-brief.md).
    fn miroir_enfant_inverse_imageW() {
        let mut rng = SeededRng::new(1);
        // parent retourné : imageW devient (-imageW)
        assert_eq!(eval("imageW", &ctx(), &mut rng, true), -64);
        // et -imageW devient +imageW
        assert_eq!(eval("0-imageW", &ctx(), &mut rng, true), 64);
    }

    #[test]
    fn le_rng_seede_est_reproductible() {
        let mut a = SeededRng::new(99);
        let mut b = SeededRng::new(99);
        let va: Vec<i32> = (0..10)
            .map(|_| eval("random", &ctx(), &mut a, false))
            .collect();
        let vb: Vec<i32> = (0..10)
            .map(|_| eval("random", &ctx(), &mut b, false))
            .collect();
        assert_eq!(va, vb);
    }
}
