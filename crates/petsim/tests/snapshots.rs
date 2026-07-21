//! La trace d'un pet doit rester identique d'une exécution à l'autre, et
//! d'une version à l'autre tant que le moteur n'évolue pas volontairement.

use pet_engine::{Pet, SeededRng, TickOutcome, World};
use pet_format::{decode_sheet, parse_pet};
use std::sync::Arc;

/// Simule un pet et retourne sa trace sous forme de texte.
fn trace(name: &str, ticks: u32, seed: u64) -> String {
    let path = format!(
        "{}/../pet-format/tests/fixtures/{name}.xml",
        env!("CARGO_MANIFEST_DIR")
    );
    let xml = std::fs::read_to_string(&path).expect("fixture lisible");
    let definition = Arc::new(parse_pet(&xml).expect("parsing"));
    let sheet = decode_sheet(&definition.image).expect("spritesheet");

    let world = World::simple(1920, 1080);
    let mut rng = SeededRng::new(seed);
    let mut pet = Pet::new(
        definition,
        (sheet.tile_w as i32, sheet.tile_h as i32),
        &world,
    );
    pet.spawn(&world, &mut rng);

    let mut out = String::new();
    for tick in 0..ticks {
        if pet.tick(&world, &mut rng) == TickOutcome::Respawn {
            pet.spawn(&world, &mut rng);
        }
        let d = pet.draw();
        out.push_str(&format!(
            "{tick:04} frame={} x={} y={} miroir={}\n",
            d.frame, d.x, d.y, d.flipped
        ));
    }
    out
}

#[test]
fn trace_de_neko() {
    insta::assert_snapshot!(trace("neko", 300, 42));
}

#[test]
fn trace_d_esheep() {
    insta::assert_snapshot!(trace("esheep64", 300, 42));
}

#[test]
fn trace_de_pingus() {
    insta::assert_snapshot!(trace("pingus", 300, 42));
}

/// Le confinement à l'écran n'est *pas* un invariant du moteur eSheep :
/// d'après `docs/reference/esheep-engine.md` §4.7 (« Sortie d'écran »), une
/// animation sans `<border>` laisse volontairement le pet continuer tout
/// droit (rognage purement visuel à l'affichage, hors sujet ici). C'est le
/// cas réel de `run_catchb` chez Neko. Vérifier une absence de sortie
/// d'écran ne peut donc que se faire au prix d'une tolérance énorme qui ne
/// détecte plus rien d'utile.
///
/// À la place, on vérifie ce qui reste réellement vrai, quel que soit le
/// pet, la graine ou l'animation en cours, et que casserait une régression
/// de physique ou d'indexation de sprite :
/// - la simulation tourne jusqu'au bout sans paniquer, sur un nombre de pas
///   conséquent ;
/// - l'opacité restituée par `Pet::draw()` reste dans `[0.0, 1.0]` ;
/// - la frame affichée est toujours un index valide du spritesheet décodé
///   pour ce pet.
#[test]
fn les_invariants_physiques_tiennent() {
    const STEPS: u32 = 1000;

    for name in ["neko", "esheep64", "pingus"] {
        let path = format!(
            "{}/../pet-format/tests/fixtures/{name}.xml",
            env!("CARGO_MANIFEST_DIR")
        );
        let xml = std::fs::read_to_string(&path).expect("fixture lisible");
        let definition = Arc::new(parse_pet(&xml).expect("parsing"));
        let sheet = decode_sheet(&definition.image).expect("spritesheet");
        let tile_count = sheet.tile_count();

        for seed in [1, 2, 3, 99, 12345] {
            let world = World::simple(1920, 1080);
            let mut rng = SeededRng::new(seed);
            let tile = (sheet.tile_w as i32, sheet.tile_h as i32);
            let mut pet = Pet::new(Arc::clone(&definition), tile, &world);
            pet.spawn(&world, &mut rng);

            for step in 0..STEPS {
                if pet.tick(&world, &mut rng) == TickOutcome::Respawn {
                    pet.spawn(&world, &mut rng);
                }
                let d = pet.draw();

                assert!(
                    (0.0..=1.0).contains(&d.opacity),
                    "{name} graine {seed} pas {step} : opacité hors bornes = {}",
                    d.opacity
                );
                assert!(
                    d.frame >= 0 && (d.frame as u32) < tile_count,
                    "{name} graine {seed} pas {step} : frame invalide = {} (tile_count = {tile_count})",
                    d.frame
                );
            }
        }
    }
}
