//! La trace d'un pet doit rester identique d'une exécution à l'autre, et
//! d'une version à l'autre tant que le moteur n'évolue pas volontairement.

use pet_engine::{Pet, TickOutcome, World};
use pet_expr::SeededRng;
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

/// Aucun pet ne doit s'enfuir de façon absurde, quelle que soit la graine.
///
/// D'après `docs/reference/esheep-engine.md` §4.7 (« Sortie d'écran »), une
/// animation sans `<border>` laisse volontairement le pet continuer tout
/// droit : dans le moteur d'origine la fenêtre est alors rognée visuellement
/// (rendu, hors sujet ici) mais la position logique, elle, continue de
/// dériver. C'est le cas réel de `run_catchb` chez Neko, qui boucle sur
/// elle-même sans jamais redéfinir de bord : sur seed 1, le chat dérive
/// jusqu'à x = -6712 en 1000 pas, ce qui est correct et attendu.
///
/// Cette garde ne vérifie donc pas « toujours visible à l'écran » (faux par
/// conception), mais l'absence d'emballement anormal : une dérive au-delà de
/// quelques dizaines de milliers de pixels trahirait un vrai bug (boucle
/// d'accumulation, dépassement arithmétique), pas un comportement de fuite
/// documenté.
#[test]
fn aucun_pet_ne_sort_de_l_ecran() {
    const MARGE: i32 = 50_000;

    for name in ["neko", "esheep64", "pingus"] {
        for seed in [1, 2, 3, 99, 12345] {
            let path = format!(
                "{}/../pet-format/tests/fixtures/{name}.xml",
                env!("CARGO_MANIFEST_DIR")
            );
            let xml = std::fs::read_to_string(&path).expect("fixture lisible");
            let definition = Arc::new(parse_pet(&xml).expect("parsing"));
            let sheet = decode_sheet(&definition.image).expect("spritesheet");

            let world = World::simple(1920, 1080);
            let mut rng = SeededRng::new(seed);
            let tile = (sheet.tile_w as i32, sheet.tile_h as i32);
            let mut pet = Pet::new(definition, tile, &world);
            pet.spawn(&world, &mut rng);

            for step in 0..1000 {
                if pet.tick(&world, &mut rng) == TickOutcome::Respawn {
                    pet.spawn(&world, &mut rng);
                }
                let (x, y) = pet.position();
                assert!(
                    x >= -MARGE && x <= world.area.right() + MARGE,
                    "{name} graine {seed} pas {step} : emballement en x = {x}"
                );
                assert!(
                    y >= -MARGE && y <= world.area.bottom() + MARGE,
                    "{name} graine {seed} pas {step} : emballement en y = {y}"
                );
            }
        }
    }
}
