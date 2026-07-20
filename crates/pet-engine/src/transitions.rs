//! Tirage de l'animation suivante et du point d'apparition.
//! Voir `docs/reference/esheep-engine.md` §2.4 et §5.1.

use pet_expr::PetRng;
use pet_format::{NextAnimation, OnlyFlags, Spawn};

/// Tire un index pondéré cumulatif parmi `weights`.
///
/// Effectue le tirage pondéré cumulatif commun à `pick_next` et `pick_spawn` :
/// on tire une valeur dans `[0, somme]` et on retient le premier index dont
/// le cumul atteint ou dépasse ce tirage.
///
/// Retourne `None` si `weights` est vide. Si la somme des poids (négatifs
/// ramenés à 0) est nulle, retourne `Some(0)` : c'est aux appelants d'adapter
/// ce cas selon leur sémantique propre (voir `pick_next` et `pick_spawn`).
fn weighted_index(weights: &[i32], rng: &mut dyn PetRng) -> Option<usize> {
    if weights.is_empty() {
        return None;
    }

    let total: i32 = weights.iter().map(|w| (*w).max(0)).sum();
    if total <= 0 {
        return Some(0);
    }

    let draw = rng.gen_range_i32(0, total);
    let mut cumulative = 0;
    for (index, weight) in weights.iter().enumerate() {
        cumulative += (*weight).max(0);
        if cumulative >= draw {
            return Some(index);
        }
    }

    Some(weights.len() - 1)
}

/// Choisit la prochaine animation parmi les candidates éligibles au contexte.
///
/// Filtre d'abord les candidates dont le drapeau `only` ne correspond pas au
/// contexte courant, puis effectue un tirage pondéré cumulatif sur les poids
/// (`probability`) restants : on tire une valeur dans `[0, somme]` et on
/// retient la première candidate dont le cumul atteint ou dépasse ce tirage.
///
/// Retourne `None` si aucune candidate n'est éligible : le pet principal
/// respawne, un enfant se ferme.
pub fn pick_next(
    candidates: &[NextAnimation],
    context: OnlyFlags,
    rng: &mut dyn PetRng,
) -> Option<i32> {
    let eligible: Vec<&NextAnimation> = candidates
        .iter()
        .filter(|c| c.only.allows(context))
        .collect();

    if eligible.is_empty() {
        return None;
    }

    let weights: Vec<i32> = eligible.iter().map(|c| c.probability).collect();
    // Poids total nul : `weighted_index` renvoie `Some(0)`, on garde donc la
    // première candidate, comme avant la factorisation.
    weighted_index(&weights, rng).map(|index| eligible[index].id)
}

/// Choisit un point d'apparition, pondéré par les probabilités.
///
/// Même algorithme que `pick_next`, sans filtrage contextuel : les points
/// d'apparition ne sont pas soumis aux drapeaux `only`.
pub fn pick_spawn(spawns: &[Spawn], rng: &mut dyn PetRng) -> Option<usize> {
    let weights: Vec<i32> = spawns.iter().map(|s| s.probability).collect();
    weighted_index(&weights, rng)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pet_expr::SeededRng;
    use pet_format::{NextAnimation, OnlyFlags};

    fn next(id: i32, probability: i32, only: OnlyFlags) -> NextAnimation {
        NextAnimation {
            id,
            probability,
            only,
        }
    }

    #[test]
    fn retourne_none_si_aucun_candidat() {
        let mut rng = SeededRng::new(1);
        assert_eq!(pick_next(&[], OnlyFlags::NONE, &mut rng), None);
    }

    #[test]
    fn choisit_l_unique_candidat() {
        let mut rng = SeededRng::new(1);
        let list = [next(7, 100, OnlyFlags::NONE)];
        assert_eq!(pick_next(&list, OnlyFlags::TASKBAR, &mut rng), Some(7));
    }

    #[test]
    fn filtre_selon_le_contexte() {
        let mut rng = SeededRng::new(1);
        let list = [
            next(1, 100, OnlyFlags::WINDOW),
            next(2, 100, OnlyFlags::TASKBAR),
        ];
        // Contexte barre des tâches : seul l'identifiant 2 est éligible.
        for _ in 0..20 {
            assert_eq!(pick_next(&list, OnlyFlags::TASKBAR, &mut rng), Some(2));
        }
    }

    #[test]
    fn retourne_none_si_tout_est_filtre() {
        let mut rng = SeededRng::new(1);
        let list = [next(1, 100, OnlyFlags::WINDOW)];
        assert_eq!(pick_next(&list, OnlyFlags::TASKBAR, &mut rng), None);
    }

    #[test]
    fn only_none_passe_dans_tous_les_contextes() {
        let mut rng = SeededRng::new(1);
        let list = [next(5, 100, OnlyFlags::NONE)];
        for ctx in [
            OnlyFlags::TASKBAR,
            OnlyFlags::WINDOW,
            OnlyFlags::HORIZONTAL,
            OnlyFlags::VERTICAL,
        ] {
            assert_eq!(pick_next(&list, ctx, &mut rng), Some(5));
        }
    }

    #[test]
    fn respecte_grossierement_les_poids() {
        let mut rng = SeededRng::new(12345);
        let list = [next(1, 90, OnlyFlags::NONE), next(2, 10, OnlyFlags::NONE)];
        let mut ones = 0;
        let total = 2000;
        for _ in 0..total {
            if pick_next(&list, OnlyFlags::NONE, &mut rng) == Some(1) {
                ones += 1;
            }
        }
        // Le poids 90/100 doit dominer largement, sans exiger une précision fine.
        assert!(
            ones > total * 3 / 4,
            "l'identifiant 1 est sorti {ones} fois sur {total}"
        );
    }

    #[test]
    fn le_tirage_est_reproductible_a_seed_egale() {
        let list = [next(1, 50, OnlyFlags::NONE), next(2, 50, OnlyFlags::NONE)];
        let mut a = SeededRng::new(42);
        let mut b = SeededRng::new(42);
        let sa: Vec<_> = (0..50)
            .map(|_| pick_next(&list, OnlyFlags::NONE, &mut a))
            .collect();
        let sb: Vec<_> = (0..50)
            .map(|_| pick_next(&list, OnlyFlags::NONE, &mut b))
            .collect();
        assert_eq!(sa, sb);
    }
}
