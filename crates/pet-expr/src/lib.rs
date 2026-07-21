//! Évaluateur des expressions `x`, `y`, `interval` et `repeat` des fichiers
//! `animations.xml` d'eSheep. Voir `docs/reference/esheep-engine.md` §3.

mod context;
mod lexer;
mod parser;
mod value;

pub use context::{EvalContext, PetRng, SeededRng, eval, substitute};
pub use parser::eval_arithmetic;
pub use value::PetValue;

/// Erreur d'évaluation d'une expression.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ExprError {
    #[error("caractère inattendu : {0}")]
    UnexpectedChar(char),
    #[error("jeton inattendu : {0}")]
    UnexpectedToken(String),
    #[error("expression incomplète")]
    UnexpectedEnd,
    #[error("parenthèses déséquilibrées")]
    UnbalancedParen,
    #[error("division par zéro")]
    DivisionByZero,
}
