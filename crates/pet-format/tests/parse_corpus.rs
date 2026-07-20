//! Le parseur doit accepter tous les pets du dépôt d'origine.

use pet_format::parse_pet;

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
        parse_pet(&xml).unwrap_or_else(|e| panic!("{} : {e}", path.display()));
        count += 1;
    }
    assert!(count > 10, "corpus trop petit : {count} pets");
}
