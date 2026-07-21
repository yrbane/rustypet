//! Écriture du spritesheet décodé (RGBA, couleur-clé déjà appliquée) dans le
//! cache utilisateur, pour que l'extension GNOME le charge par chemin de
//! fichier. Voir `docs/reference/gnome50-dbus.md` §1.4.

use pet_format::SpriteSheet;
use std::path::{Path, PathBuf};

/// Répertoire de cache de RustyPet, respectant `XDG_CACHE_HOME`.
///
/// Toujours un chemin absolu : `XDG_CACHE_HOME` (si absolu), sinon
/// `HOME/.cache` (si `HOME` est absolu), sinon un repli garanti absolu sur
/// `std::env::temp_dir()`. Un démon ne doit jamais écrire à un chemin
/// relatif, dont la résolution dépendrait de son répertoire courant.
pub fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .map(|home| home.join(".cache"))
        })
        .unwrap_or_else(std::env::temp_dir);
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

    /// Garde RAII qui restaure une variable d'environnement à son état
    /// d'origine (présente ou absente) à la fin du test, pour ne pas
    /// polluer les autres tests du même binaire. Les tests qui l'utilisent
    /// doivent porter `#[serial]` : les variables d'environnement sont
    /// globales au processus.
    struct EnvVarGuard {
        key: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        /// Positionne `key` à `value` et mémorise l'état d'origine.
        fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
            let original = std::env::var_os(key);
            // SAFETY : test #[serial], pas d'accès concurrent à l'environnement.
            unsafe { std::env::set_var(key, value) };
            Self { key, original }
        }

        /// Retire `key` de l'environnement et mémorise l'état d'origine.
        fn remove(key: &'static str) -> Self {
            let original = std::env::var_os(key);
            // SAFETY : test #[serial], pas d'accès concurrent à l'environnement.
            unsafe { std::env::remove_var(key) };
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            // SAFETY : test #[serial], pas d'accès concurrent à l'environnement.
            unsafe {
                match &self.original {
                    Some(v) => std::env::set_var(self.key, v),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    #[test]
    fn slugify_nettoie_les_caracteres_de_chemin() {
        assert_eq!(slugify("Neko"), "neko");
        assert_eq!(slugify("Blue Sheep"), "blue_sheep");
        assert_eq!(slugify("../evil/./x"), "evil_x");
        assert_eq!(slugify(""), "pet");
    }

    #[test]
    #[serial]
    fn cache_dir_respecte_xdg() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = EnvVarGuard::set("XDG_CACHE_HOME", dir.path());

        let expected = dir.path().join("rustypet");
        assert_eq!(cache_dir(), expected);
        assert!(cache_dir().is_absolute());
    }

    #[test]
    #[serial]
    fn cache_dir_reste_absolu_sans_home_ni_xdg() {
        // Cas limite du point 1 : ni XDG_CACHE_HOME, ni HOME exploitables.
        // `cache_dir()` doit malgré tout retourner un chemin absolu (repli
        // sur `std::env::temp_dir()`), jamais un chemin relatif.
        let _guard_xdg = EnvVarGuard::remove("XDG_CACHE_HOME");
        let _guard_home = EnvVarGuard::remove("HOME");

        assert!(
            cache_dir().is_absolute(),
            "cache_dir() doit rester absolu même sans HOME ni XDG_CACHE_HOME"
        );
    }

    #[test]
    #[serial]
    fn write_sprite_png_ecrit_un_png_relisible() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = EnvVarGuard::set("XDG_CACHE_HOME", dir.path());

        let path = write_sprite_png("Neko", &sheet()).expect("écriture");
        assert!(path.exists(), "le fichier doit exister");
        assert!(path.ends_with("neko/sheet.png"));

        let decoded = image::open(&path).expect("relecture PNG");
        assert_eq!(decoded.width(), 6);
        assert_eq!(decoded.height(), 2);
    }
}
