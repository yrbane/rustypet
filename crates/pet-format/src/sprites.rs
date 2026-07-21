//! Décodage du spritesheet : base64 → PNG → RGBA avec couleur clé.
//! Voir `docs/reference/esheep-engine.md` §6.

use crate::FormatError;
use crate::model::ImageDef;
use base64::Engine;

/// Spritesheet décodé, en RGBA prêt à l'affichage.
#[derive(Debug, Clone)]
pub struct SpriteSheet {
    pub width: u32,
    pub height: u32,
    /// Pixels RGBA, 4 octets par pixel, ligne par ligne.
    pub rgba: Vec<u8>,
    pub tile_w: u32,
    pub tile_h: u32,
    pub tiles_x: u32,
    pub tiles_y: u32,
}

impl SpriteSheet {
    /// Rectangle `(x, y, largeur, hauteur)` de la tuile d'index donné.
    ///
    /// Le parcours est en ligne d'abord : `index = row * tiles_x + col`.
    pub fn tile_rect(&self, index: u32) -> (u32, u32, u32, u32) {
        let col = index % self.tiles_x;
        let row = (index / self.tiles_x) % self.tiles_y;
        (
            col * self.tile_w,
            row * self.tile_h,
            self.tile_w,
            self.tile_h,
        )
    }

    /// Nombre total de tuiles.
    pub fn tile_count(&self) -> u32 {
        self.tiles_x * self.tiles_y
    }
}

/// Ajoute le padding `=` manquant : beaucoup de pets ont un base64 non padé.
pub fn pad_base64(input: &str) -> String {
    let mut s = input.to_string();
    let remainder = s.len() % 4;
    if remainder != 0 {
        s.push_str(&"=".repeat(4 - remainder));
    }
    s
}

/// Lit une couleur de transparence, par nom ou en hexadécimal.
/// Toute valeur inconnue vaut magenta, le défaut du format.
pub fn parse_color(name: &str) -> [u8; 3] {
    let trimmed = name.trim();
    if let Some(hex) = trimmed.strip_prefix('#')
        && hex.len() == 6
    {
        let r = u8::from_str_radix(&hex[0..2], 16);
        let g = u8::from_str_radix(&hex[2..4], 16);
        let b = u8::from_str_radix(&hex[4..6], 16);
        if let (Ok(r), Ok(g), Ok(b)) = (r, g, b) {
            return [r, g, b];
        }
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "black" => [0, 0, 0],
        "white" => [255, 255, 255],
        "red" => [255, 0, 0],
        "lime" => [0, 255, 0],
        "green" => [0, 128, 0],
        "blue" => [0, 0, 255],
        "cyan" | "aqua" => [0, 255, 255],
        "yellow" => [255, 255, 0],
        "fuchsia" | "magenta" => [255, 0, 255],
        _ => [255, 0, 255],
    }
}

/// Décode le spritesheet d'un pet.
pub fn decode_sheet(image: &ImageDef) -> Result<SpriteSheet, FormatError> {
    let padded = pad_base64(&image.png_base64);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(padded.as_bytes())
        .map_err(|e| FormatError::Base64(e.to_string()))?;

    let decoded = image::load_from_memory(&bytes).map_err(|e| FormatError::Image(e.to_string()))?;
    let mut rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();

    if width == 0 || height == 0 {
        return Err(FormatError::Image("image de dimension nulle".into()));
    }
    // Garde défensive : `ImageDef` est une structure publique, un appelant
    // pourrait fournir un découpage nul. `parse_pet` garantit déjà tiles_x/y
    // >= 1, mais on évite ici toute division par zéro (aucun panic autorisé
    // dans le code de bibliothèque).
    if image.tiles_x == 0 || image.tiles_y == 0 {
        return Err(FormatError::Image("découpage en tuiles nul".into()));
    }

    // Si le PNG porte déjà un alpha significatif, on le respecte. Sinon on
    // applique la couleur clé (§6.8).
    let has_alpha = rgba.pixels().any(|p| p.0[3] < 255);
    if !has_alpha {
        let key = parse_color(&image.transparency);
        for pixel in rgba.pixels_mut() {
            if pixel.0[0] == key[0] && pixel.0[1] == key[1] && pixel.0[2] == key[2] {
                pixel.0[3] = 0;
            }
        }
    }

    Ok(SpriteSheet {
        width,
        height,
        tile_w: width / image.tiles_x,
        tile_h: height / image.tiles_y,
        tiles_x: image.tiles_x,
        tiles_y: image.tiles_y,
        rgba: rgba.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construit un PNG RGB 4x2 sans alpha, avec un pixel magenta en (0,0).
    fn png_test_rgb() -> String {
        use image::{ImageBuffer, Rgb};
        let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(4, 2);
        for p in img.pixels_mut() {
            *p = Rgb([10, 20, 30]);
        }
        img.put_pixel(0, 0, Rgb([255, 0, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png)
            .expect("encodage");
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(buf.into_inner())
    }

    fn image_def(png: String) -> ImageDef {
        ImageDef {
            tiles_x: 2,
            tiles_y: 2,
            png_base64: png,
            transparency: "Magenta".into(),
        }
    }

    #[test]
    fn pad_le_base64_non_pade() {
        assert_eq!(pad_base64("QQ"), "QQ==");
        assert_eq!(pad_base64("QUJD"), "QUJD");
        assert_eq!(pad_base64("QUJDRA"), "QUJDRA==");
    }

    #[test]
    fn lit_les_couleurs_nommees_et_hexa() {
        assert_eq!(parse_color("Magenta"), [255, 0, 255]);
        assert_eq!(parse_color("magenta"), [255, 0, 255]);
        assert_eq!(parse_color("#FF00FF"), [255, 0, 255]);
        assert_eq!(parse_color("White"), [255, 255, 255]);
        // Valeur inconnue : magenta par défaut.
        assert_eq!(parse_color("chartreuse-fluo"), [255, 0, 255]);
    }

    #[test]
    fn decode_et_calcule_les_dimensions_de_tuile() {
        let sheet = decode_sheet(&image_def(png_test_rgb())).unwrap();
        assert_eq!((sheet.width, sheet.height), (4, 2));
        assert_eq!((sheet.tile_w, sheet.tile_h), (2, 1));
    }

    #[test]
    fn applique_la_couleur_cle_en_alpha_zero() {
        let sheet = decode_sheet(&image_def(png_test_rgb())).unwrap();
        // Pixel (0,0) magenta -> transparent.
        assert_eq!(sheet.rgba[3], 0);
        // Pixel (1,0) opaque.
        assert_eq!(sheet.rgba[7], 255);
    }

    #[test]
    fn decoupe_les_tuiles_en_ligne_d_abord() {
        let sheet = decode_sheet(&image_def(png_test_rgb())).unwrap();
        // index = row * tiles_x + col
        assert_eq!(sheet.tile_rect(0), (0, 0, 2, 1)); // ligne 0, colonne 0
        assert_eq!(sheet.tile_rect(1), (2, 0, 2, 1)); // ligne 0, colonne 1
        assert_eq!(sheet.tile_rect(2), (0, 1, 2, 1)); // ligne 1, colonne 0
        assert_eq!(sheet.tile_rect(3), (2, 1, 2, 1)); // ligne 1, colonne 1
    }

    #[test]
    fn preserve_l_alpha_existant() {
        use image::{ImageBuffer, Rgba};
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(4, 2);
        for p in img.pixels_mut() {
            *p = Rgba([255, 0, 255, 128]); // magenta mais semi-transparent
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png)
            .expect("encodage");
        use base64::Engine;
        let png = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());

        let sheet = decode_sheet(&image_def(png)).unwrap();
        // L'alpha d'origine est conservé : pas de color-key appliquée.
        assert_eq!(sheet.rgba[3], 128);
    }

    #[test]
    fn refuse_un_base64_invalide() {
        let def = image_def("!!!pas du base64!!!".into());
        assert!(decode_sheet(&def).is_err());
    }
}
