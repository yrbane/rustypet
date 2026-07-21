//! Écriture du spritesheet décodé (RGBA, couleur-clé déjà appliquée) dans le
//! cache utilisateur, pour que l'extension GNOME le charge par chemin de
//! fichier. Voir `docs/reference/gnome50-dbus.md` §1.4.

use pet_format::SpriteSheet;
use std::path::{Path, PathBuf};

/// Répertoire de cache de RustyPet, respectant `XDG_CACHE_HOME`.
pub fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default();
            home.join(".cache")
        });
    base.join("rustypet")
}

/// Transforme un nom de pet en composant de chemin sûr : minuscules, seuls
/// `[a-z0-9_]` conservés, le reste fusionné en `_`. Jamais vide.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_underscore = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_underscore = false;
        } else if !last_underscore && !out.is_empty() {
            out.push('_');
            last_underscore = true;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "pet".to_string()
    } else {
        trimmed
    }
}

/// Écrit le RGBA du spritesheet en PNG sous `<cache>/<slug>/sheet.png`.
pub fn write_sprite_png(pet_name: &str, sheet: &SpriteSheet) -> std::io::Result<PathBuf> {
    let dir = cache_dir().join(slugify(pet_name));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("sheet.png");
    write_png(&path, sheet.width, sheet.height, &sheet.rgba)?;
    Ok(path)
}

/// Encode un buffer RGBA en PNG. Erreur d'encodage remontée en `io::Error`.
fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    image::save_buffer(path, rgba, width, height, image::ExtendedColorType::Rgba8)
        .map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pet_format::SpriteSheet;

    fn sheet() -> SpriteSheet {
        // 2×1 tuiles de 3×2 px, RGBA plein.
        SpriteSheet {
            width: 6,
            height: 2,
            rgba: vec![0u8; 6 * 2 * 4],
            tile_w: 3,
            tile_h: 2,
            tiles_x: 2,
            tiles_y: 1,
        }
    }

    use serial_test::serial;

    #[test]
    fn slugify_nettoie_les_caracteres_de_chemin() {
        assert_eq!(slugify("Neko"), "neko");
        assert_eq!(slugify("Blue Sheep"), "blue_sheep");
        assert_eq!(slugify("../evil/./x"), "evil_x");
        assert_eq!(slugify(""), "pet");
    }

    #[test]
    fn cache_dir_respecte_xdg() {
        // On ne dépend pas de l'environnement réel : on vérifie juste que le
        // chemin se termine par rustypet.
        assert!(cache_dir().ends_with("rustypet"));
    }

    #[test]
    #[serial]
    fn write_sprite_png_ecrit_un_png_relisible() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Force XDG_CACHE_HOME sur le tempdir pour ce test.
        // SAFETY : mono-thread dans ce test.
        unsafe { std::env::set_var("XDG_CACHE_HOME", dir.path()) };

        let path = write_sprite_png("Neko", &sheet()).expect("écriture");
        assert!(path.exists(), "le fichier doit exister");
        assert!(path.ends_with("neko/sheet.png"));

        let decoded = image::open(&path).expect("relecture PNG");
        assert_eq!(decoded.width(), 6);
        assert_eq!(decoded.height(), 2);
    }
}
