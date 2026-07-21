//! Le parseur doit accepter tous les pets du dépôt d'origine.

use pet_format::{decode_sheet, parse_pet};

/// Les trois pets embarqués doivent parser sans erreur et exposer un contenu
/// cohérent.
#[test]
fn parse_les_fixtures() {
    for name in ["neko", "esheep64", "pingus"] {
        let path = format!("{}/tests/fixtures/{name}.xml", env!("CARGO_MANIFEST_DIR"));
        let xml = std::fs::read_to_string(&path).expect("fixture lisible");
        let pet = parse_pet(&xml).unwrap_or_else(|e| panic!("{name} : {e}"));

        assert!(!pet.header.petname.is_empty(), "{name} : petname vide");
        assert_eq!(pet.header.application, 1, "{name} : version de format");
        assert!(
            pet.image.tiles_x > 0 && pet.image.tiles_y > 0,
            "{name} : tuiles"
        );
        assert!(!pet.image.png_base64.is_empty(), "{name} : png vide");
        assert!(!pet.animations.is_empty(), "{name} : aucune animation");
        assert!(!pet.spawns.is_empty(), "{name} : aucun spawn");

        // Toute animation a au moins une frame.
        for anim in &pet.animations {
            assert!(
                !anim.sequence.frames.is_empty(),
                "{name} : animation {} sans frame",
                anim.id
            );
        }
    }
}

/// Le corpus complet du dépôt amont, si disponible. Activé par la variable
/// d'environnement RUSTYPET_CORPUS.
#[test]
#[ignore = "Nécessite la variable RUSTYPET_CORPUS pointant vers le dépôt eSheep (yrbane/desktopPet)"]
fn parse_le_corpus_complet() {
    let Ok(dir) = std::env::var("RUSTYPET_CORPUS") else {
        eprintln!("RUSTYPET_CORPUS non défini, test ignoré");
        return;
    };
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("dossier corpus") {
        let path = entry.expect("entrée").path().join("animations.xml");
        if !path.exists() {
            continue;
        }
        let xml = std::fs::read_to_string(&path).expect("lecture");
        let pet = parse_pet(&xml).unwrap_or_else(|e| panic!("{} : {e}", path.display()));

        // Le spritesheet de chaque pet réel doit se décoder (base64 non
        // padé, couleur clé variable, alpha parfois déjà présent...).
        let sheet = decode_sheet(&pet.image)
            .unwrap_or_else(|e| panic!("{} : décodage du spritesheet : {e}", path.display()));
        assert!(
            sheet.tile_w > 0 && sheet.tile_h > 0,
            "{} : tuile de taille nulle",
            path.display()
        );

        count += 1;
    }
    assert!(count > 10, "corpus trop petit : {count} pets");
}

/// Le spritesheet de chaque fixture doit se décoder et produire des tuiles
/// de dimensions plausibles.
#[test]
fn decode_les_spritesheets_des_fixtures() {
    for name in ["neko", "esheep64", "pingus"] {
        let path = format!("{}/tests/fixtures/{name}.xml", env!("CARGO_MANIFEST_DIR"));
        let xml = std::fs::read_to_string(&path).expect("fixture lisible");
        let pet = pet_format::parse_pet(&xml).expect("parsing");
        let sheet = pet_format::decode_sheet(&pet.image).unwrap_or_else(|e| panic!("{name} : {e}"));

        assert!(
            sheet.tile_w > 0 && sheet.tile_h > 0,
            "{name} : tuile de taille nulle"
        );
        assert_eq!(sheet.rgba.len(), (sheet.width * sheet.height * 4) as usize);

        // Toute frame référencée doit exister dans la feuille.
        for anim in &pet.animations {
            for &frame in &anim.sequence.frames {
                assert!(
                    (frame as u32) < sheet.tile_count(),
                    "{name} : animation {} référence la frame {frame}, hors feuille ({} tuiles)",
                    anim.id,
                    sheet.tile_count()
                );
            }
        }
    }
}
