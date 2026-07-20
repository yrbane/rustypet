//! Lecture des fichiers `animations.xml` d'eSheep et décodage des spritesheets.
//! Voir `docs/reference/esheep-engine.md` §1 et §6.

mod model;
mod parse;

pub use model::*;
pub use parse::parse_pet;

/// Erreur de lecture d'un pet.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("XML invalide : {0}")]
    Xml(String),
    #[error("élément racine <animations> absent")]
    MissingRoot,
    #[error("élément obligatoire absent : <{0}>")]
    MissingElement(&'static str),
}
