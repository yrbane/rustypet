//! Le pet RustySheep embarqué dans le dépôt : les gags demandés existent,
//! sont atteignables en jeu, et la spritesheet étendue couvre leurs frames.

use pet_format::{decode_sheet, decode_sound, parse_pet};
use std::collections::HashSet;

/// Les gags inédits générés par `tools/make_rustysheep.py`.
const GAGS: [&str; 14] = [
    "dance",
    "smoke",
    "superman",
    "poop",
    "sunglasses",
    "flower_grow",
    "parachute",
    "rocket",
    "acid",
    "rain",
    "soaked",
    "umbrella",
    "love",
    "family_walk",
];

fn xml() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/rustysheep/animations.xml"
    );
    std::fs::read_to_string(path).expect("assets/rustysheep/animations.xml manquant")
}

#[test]
fn le_rustysheep_se_parse_et_contient_les_gags() {
    let pet = parse_pet(&xml()).expect("le XML RustySheep doit se parser");
    for name in GAGS {
        assert!(
            pet.animation_id_by_name(name).is_some(),
            "animation absente : {name}"
        );
    }
    // Les classiques hérités du mouton restent là (dormir, brouter, fleur).
    for name in ["sleep1a", "eat", "flower", "walk"] {
        assert!(
            pet.animation_id_by_name(name).is_some(),
            "animation héritée absente : {name}"
        );
    }
}

#[test]
fn chaque_gag_est_atteignable_depuis_un_spawn() {
    let pet = parse_pet(&xml()).expect("parse");
    // Parcours du graphe : spawns -> next de séquence/bord/gravité.
    let mut seen: HashSet<i32> = HashSet::new();
    let mut stack: Vec<i32> = pet.spawns.iter().map(|s| s.next).collect();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        if let Some(anim) = pet.animation(id) {
            for next in anim
                .sequence
                .next
                .iter()
                .chain(anim.border.iter())
                .chain(anim.gravity.iter())
            {
                stack.push(next.id);
            }
        }
    }
    for name in GAGS {
        let id = pet.animation_id_by_name(name).expect("gag présent");
        assert!(seen.contains(&id), "gag inatteignable en jeu : {name}");
    }
}

#[test]
fn la_moutonne_et_les_agneaux_sont_declares() {
    let pet = parse_pet(&xml()).expect("parse");
    let love = pet.animation_id_by_name("love").expect("love présent");
    let family = pet
        .animation_id_by_name("family_walk")
        .expect("family_walk présent");

    let ewes: Vec<_> = pet
        .childs
        .iter()
        .filter(|c| c.animation_id == love)
        .collect();
    assert_eq!(ewes.len(), 1, "le coup de foudre fait venir une moutonne");
    let lambs: Vec<_> = pet
        .childs
        .iter()
        .filter(|c| c.animation_id == family)
        .collect();
    assert_eq!(lambs.len(), 2, "la parade familiale compte deux agneaux");

    // Chaque enfant démarre sur une animation existante.
    for child in ewes.iter().chain(lambs.iter()) {
        assert!(
            pet.animation(child.next).is_some(),
            "animation d'enfant inconnue : {}",
            child.next
        );
    }
}

#[test]
fn le_mouton_a_des_bêlements_discrets() {
    let pet = parse_pet(&xml()).expect("parse");
    assert!(!pet.sounds.is_empty(), "le mouton doit avoir des sons");
    for sound in &pet.sounds {
        assert!(
            pet.animation(sound.animation_id).is_some(),
            "son accroché à l'animation inconnue {}",
            sound.animation_id
        );
        let bytes = decode_sound(sound).expect("le son doit se décoder");
        assert_eq!(&bytes[..4], b"RIFF", "le son doit être un WAV");

        // Discrétion demandée : le pic d'amplitude PCM 16 bits reste sous
        // ~30 % de l'échelle (l'original est proche de la saturation).
        let peak = bytes[44..]
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]).unsigned_abs())
            .max()
            .unwrap_or(0);
        assert!(peak < 10_000, "bêlement trop fort : pic {peak}");
    }
}

#[test]
fn la_sheet_etendue_se_decode_et_couvre_toutes_les_frames() {
    let pet = parse_pet(&xml()).expect("parse");
    let sheet = decode_sheet(&pet.image).expect("la sheet doit se décoder");
    let max_frame = pet
        .animations
        .iter()
        .flat_map(|a| a.sequence.frames.iter())
        .copied()
        .max()
        .expect("des frames existent");
    assert!(
        (max_frame as u32) < sheet.tile_count(),
        "frame {max_frame} hors de la sheet ({} tuiles)",
        sheet.tile_count()
    );
}

#[test]
fn chaque_gag_revient_a_une_animation_du_mouton() {
    // Aucun gag ne doit être un cul-de-sac : sa séquence pointe vers une
    // animation existante (le respawn couvre superman/rocket via la sortie
    // d'écran, mais un next de secours reste exigé).
    let pet = parse_pet(&xml()).expect("parse");
    for name in GAGS {
        let id = pet.animation_id_by_name(name).expect("gag présent");
        let anim = pet.animation(id).expect("animation");
        assert!(
            !anim.sequence.next.is_empty(),
            "{name} n'a pas de transition de sortie"
        );
        for next in &anim.sequence.next {
            assert!(
                pet.animation(next.id).is_some(),
                "{name} pointe vers l'animation inconnue {}",
                next.id
            );
        }
    }
}
