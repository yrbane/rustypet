//! Structures issues du fichier `animations.xml`.
//! Voir `docs/reference/esheep-engine.md` §1.

use pet_expr::PetValue;

/// Filtre contextuel d'une transition. Champ de bits (§2.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnlyFlags(pub u8);

impl OnlyFlags {
    /// Masque « tous les contextes ».
    pub const NONE: OnlyFlags = OnlyFlags(0x7F);
    pub const TASKBAR: OnlyFlags = OnlyFlags(0x01);
    pub const WINDOW: OnlyFlags = OnlyFlags(0x02);
    pub const HORIZONTAL: OnlyFlags = OnlyFlags(0x04);
    pub const VERTICAL: OnlyFlags = OnlyFlags(0x08);

    /// Lit un attribut `only`. Toute valeur inconnue vaut `NONE`.
    pub fn parse(s: &str) -> OnlyFlags {
        match s {
            "taskbar" => Self::TASKBAR,
            "window" => Self::WINDOW,
            // `horizontal+` est une variante d'horizontal (§2.4).
            "horizontal" | "horizontal+" => Self::HORIZONTAL,
            "vertical" => Self::VERTICAL,
            _ => Self::NONE,
        }
    }

    /// Vrai si cette transition est jouable dans le contexte donné.
    pub fn allows(&self, context: OnlyFlags) -> bool {
        *self == Self::NONE || (self.0 & context.0) != 0
    }
}

/// En-tête descriptif du pet.
#[derive(Debug, Clone)]
pub struct Header {
    pub author: String,
    pub title: String,
    pub petname: String,
    pub version: String,
    pub info: String,
    /// Version du format ; vaut 1.
    pub application: i32,
    /// Icône ICO en base64.
    pub icon: String,
}

/// Spritesheet et son découpage.
#[derive(Debug, Clone)]
pub struct ImageDef {
    pub tiles_x: u32,
    pub tiles_y: u32,
    pub png_base64: String,
    /// Couleur clé de transparence, « Magenta » par défaut.
    pub transparency: String,
}

/// Un bout de mouvement : début ou fin d'animation.
#[derive(Debug, Clone)]
pub struct Movement {
    /// Vitesse horizontale par frame (expression).
    pub x: PetValue,
    /// Vitesse verticale par frame (expression).
    pub y: PetValue,
    /// Durée d'une frame en millisecondes (expression).
    pub interval: PetValue,
    pub offset_y: i32,
    pub opacity: f64,
}

/// Transition vers une autre animation.
#[derive(Debug, Clone)]
pub struct NextAnimation {
    pub id: i32,
    /// Poids relatif de tirage, pas un pourcentage.
    pub probability: i32,
    pub only: OnlyFlags,
}

/// Suite de frames d'une animation.
#[derive(Debug, Clone)]
pub struct Sequence {
    /// Nombre de répétitions supplémentaires (expression).
    pub repeat: PetValue,
    /// Index 0-based de la frame à partir de laquelle on répète.
    pub repeat_from: i32,
    pub frames: Vec<i32>,
    /// Seule valeur reconnue : « flip ».
    pub action: Option<String>,
    pub next: Vec<NextAnimation>,
}

/// Une animation complète.
#[derive(Debug, Clone)]
pub struct Animation {
    pub id: i32,
    pub name: String,
    pub start: Movement,
    /// Absent, il vaut `start`.
    pub end: Option<Movement>,
    pub sequence: Sequence,
    pub border: Vec<NextAnimation>,
    pub gravity: Vec<NextAnimation>,
}

impl Animation {
    /// Vrai si l'animation réagit aux bords.
    pub fn has_border(&self) -> bool {
        !self.border.is_empty()
    }

    /// Vrai si l'animation est soumise à la gravité.
    pub fn has_gravity(&self) -> bool {
        !self.gravity.is_empty()
    }
}

/// Point d'apparition. `x` et `y` sont des positions absolues, pas des vitesses.
#[derive(Debug, Clone)]
pub struct Spawn {
    pub id: i32,
    pub probability: i32,
    pub x: PetValue,
    pub y: PetValue,
    pub next: i32,
}

/// Pet enfant créé par une animation. `x` et `y` sont absolus.
#[derive(Debug, Clone)]
pub struct Child {
    /// Animation parente qui déclenche la création.
    pub animation_id: i32,
    pub x: PetValue,
    pub y: PetValue,
    pub next: i32,
}

/// Son associé à une animation.
#[derive(Debug, Clone)]
pub struct Sound {
    pub animation_id: i32,
    pub probability: i32,
    pub loop_count: i32,
    /// WAV encodé en base64.
    pub base64: String,
}

/// Un pet complet, tel que décrit par son XML.
#[derive(Debug, Clone)]
pub struct PetDefinition {
    pub header: Header,
    pub image: ImageDef,
    pub spawns: Vec<Spawn>,
    pub animations: Vec<Animation>,
    pub childs: Vec<Child>,
    pub sounds: Vec<Sound>,
}

impl PetDefinition {
    /// Cherche une animation par identifiant.
    pub fn animation(&self, id: i32) -> Option<&Animation> {
        self.animations.iter().find(|a| a.id == id)
    }

    /// Cherche l'identifiant d'une animation par son nom réservé (§2.6).
    pub fn animation_id_by_name(&self, name: &str) -> Option<i32> {
        self.animations
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.id)
    }
}
