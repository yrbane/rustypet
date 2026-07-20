//! Nombre de pas, choix de frame et interpolation.
//! Voir `docs/reference/esheep-engine.md` §2.2, §2.3 et §2.5.

use pet_format::{Animation, Sequence};

/// Position dans l'animation courante.
#[derive(Debug, Clone, Copy)]
pub struct AnimState {
    pub animation_id: i32,
    /// Pas courant, 0-based. Vaut -1 avant le premier tick.
    pub step: i32,
    pub total_steps: i32,
}

/// Valeurs interpolées pour un pas donné.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepValues {
    /// Vitesse horizontale de ce pas, en pixels.
    pub x: i32,
    /// Vitesse verticale de ce pas, en pixels.
    pub y: i32,
    /// Durée de la frame, en millisecondes.
    pub interval: i32,
    pub opacity: f64,
    pub offset_y: i32,
}

/// Nombre total de pas de la séquence, répétitions comprises.
pub fn total_steps(seq: &Sequence) -> i32 {
    let len = seq.frames.len() as i32;
    if len == 0 {
        return 0;
    }
    let from = seq.repeat_from.clamp(0, len);
    len + (len - from) * seq.repeat.get().max(0)
}

/// Frame affichée au pas donné.
///
/// Pendant la première passe (`step < frames.len()`), on lit directement
/// `frames[step]`. Au-delà, on boucle sur la portion `[repeat_from..len)`.
/// Toute séquence vide renvoie 0 : il n'existe alors aucune frame valide,
/// mais l'appelant ne doit jamais paniquer.
pub fn pick_frame(seq: &Sequence, step: i32) -> i32 {
    let len = seq.frames.len() as i32;
    if len == 0 {
        return 0;
    }
    let step = step.max(0);
    if step < len {
        return seq.frames[step as usize];
    }
    // Au-delà de `len - 1`, `repeat_from` doit rester un index valide de la
    // séquence pour que la portion bouclée soit non vide.
    let from = seq.repeat_from.clamp(0, len - 1);
    let span = len - from;
    if span <= 0 {
        return seq.frames[(len - 1) as usize];
    }
    // Le premier pas de répétition (step == len) doit retomber exactement sur
    // l'index `repeat_from` : on ne réinjecte donc pas `from` dans le calcul
    // du modulo, seulement en décalage final.
    let idx = ((step - len) % span) + from;
    seq.frames[idx as usize]
}

/// Interpole les valeurs entre le début et la fin de l'animation.
///
/// Attention : `x` et `y` utilisent le dénominateur `total - 1`, les autres
/// valeurs utilisent `total` (§2.3). C'est le comportement réel du moteur
/// d'origine, pas une coquille : les animations existantes en dépendent.
/// Un `<end>` absent équivaut à `start` : aucune interpolation n'a lieu.
pub fn interpolate(anim: &Animation, step: i32, total: i32) -> StepValues {
    let start = &anim.start;
    let end = anim.end.as_ref().unwrap_or(start);
    let step_f = step as f64;

    let (interval, opacity, offset_y) = if total > 0 {
        let ratio = step_f / total as f64;
        (
            start.interval.get() as f64
                + (end.interval.get() - start.interval.get()) as f64 * ratio,
            start.opacity + (end.opacity - start.opacity) * ratio,
            start.offset_y as f64 + (end.offset_y - start.offset_y) as f64 * ratio,
        )
    } else {
        (
            start.interval.get() as f64,
            start.opacity,
            start.offset_y as f64,
        )
    };

    let (x, y) = if total > 1 {
        let ratio = step_f / (total - 1) as f64;
        (
            start.x.get() as f64 + (end.x.get() - start.x.get()) as f64 * ratio,
            start.y.get() as f64 + (end.y.get() - start.y.get()) as f64 * ratio,
        )
    } else {
        (start.x.get() as f64, start.y.get() as f64)
    };

    StepValues {
        x: x.trunc() as i32,
        y: y.trunc() as i32,
        interval: interval.trunc() as i32,
        opacity,
        offset_y: offset_y.trunc() as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pet_expr::PetValue;
    use pet_format::{Animation, Movement, Sequence};

    fn movement(x: i32, y: i32, interval: i32, opacity: f64, offset_y: i32) -> Movement {
        let mut m = Movement {
            x: PetValue::new(x.to_string()),
            y: PetValue::new(y.to_string()),
            interval: PetValue::new(interval.to_string()),
            offset_y,
            opacity,
        };
        m.x.value = x;
        m.y.value = y;
        m.interval.value = interval;
        m
    }

    fn sequence(frames: Vec<i32>, repeat: i32, repeat_from: i32) -> Sequence {
        let mut r = PetValue::new(repeat.to_string());
        r.value = repeat;
        Sequence {
            repeat: r,
            repeat_from,
            frames,
            action: None,
            next: Vec::new(),
        }
    }

    fn animation(start: Movement, end: Option<Movement>, seq: Sequence) -> Animation {
        Animation {
            id: 1,
            name: "test".into(),
            start,
            end,
            sequence: seq,
            border: Vec::new(),
            gravity: Vec::new(),
        }
    }

    #[test]
    fn calcule_le_nombre_total_de_pas() {
        // 4 frames, pas de répétition
        assert_eq!(total_steps(&sequence(vec![0, 1, 2, 3], 0, 0)), 4);
        // 4 frames répétées 2 fois de plus depuis le début : 4 + 4*2
        assert_eq!(total_steps(&sequence(vec![0, 1, 2, 3], 2, 0)), 12);
        // répétition partielle depuis l'index 2 : 4 + (4-2)*3
        assert_eq!(total_steps(&sequence(vec![0, 1, 2, 3], 3, 2)), 10);
    }

    #[test]
    fn choisit_la_frame_pendant_la_premiere_passe() {
        let seq = sequence(vec![10, 11, 12], 2, 1);
        assert_eq!(pick_frame(&seq, 0), 10);
        assert_eq!(pick_frame(&seq, 1), 11);
        assert_eq!(pick_frame(&seq, 2), 12);
    }

    #[test]
    fn choisit_la_frame_pendant_les_repetitions() {
        // frames [10,11,12], repeat_from=1 -> on boucle sur [11,12]
        let seq = sequence(vec![10, 11, 12], 2, 1);
        assert_eq!(pick_frame(&seq, 3), 11);
        assert_eq!(pick_frame(&seq, 4), 12);
        assert_eq!(pick_frame(&seq, 5), 11);
        assert_eq!(pick_frame(&seq, 6), 12);
    }

    #[test]
    fn interpole_avec_deux_denominateurs() {
        let start = movement(0, 0, 100, 0.0, 0);
        let end = movement(10, 20, 200, 1.0, 40);
        let anim = animation(start, Some(end), sequence(vec![0; 5], 0, 0));
        let total = 5;

        // interval, opacity, offset_y : dénominateur T = 5
        let at0 = interpolate(&anim, 0, total);
        assert_eq!(at0.interval, 100);
        assert_eq!(at0.opacity, 0.0);
        assert_eq!(at0.offset_y, 0);

        let at1 = interpolate(&anim, 1, total);
        assert_eq!(at1.interval, 120); // 100 + 100*1/5
        assert_eq!(at1.opacity, 0.2);
        assert_eq!(at1.offset_y, 8);

        // x, y : dénominateur T-1 = 4
        assert_eq!(at1.x, 2); // 0 + 10*1/4
        assert_eq!(at1.y, 5); // 0 + 20*1/4

        let at4 = interpolate(&anim, 4, total);
        assert_eq!(at4.x, 10); // valeur de fin atteinte au dernier pas
        assert_eq!(at4.y, 20);
    }

    #[test]
    fn ne_divise_pas_par_zero_si_un_seul_pas() {
        let start = movement(3, 4, 100, 1.0, 0);
        let end = movement(30, 40, 500, 0.0, 10);
        let anim = animation(start, Some(end), sequence(vec![0], 0, 0));
        let values = interpolate(&anim, 0, 1);
        assert_eq!(values.x, 3);
        assert_eq!(values.y, 4);
    }

    #[test]
    fn sans_end_les_valeurs_restent_celles_de_start() {
        let anim = animation(movement(5, 6, 80, 0.5, 3), None, sequence(vec![0; 4], 0, 0));
        for step in 0..4 {
            let v = interpolate(&anim, step, 4);
            assert_eq!(v.x, 5);
            assert_eq!(v.y, 6);
            assert_eq!(v.interval, 80);
            assert_eq!(v.opacity, 0.5);
            assert_eq!(v.offset_y, 3);
        }
    }
}
