//! Évaluateur des expressions `x`, `y`, `interval` et `repeat` des fichiers
//! `animations.xml` d'eSheep. Voir `docs/reference/esheep-engine.md` §3.

mod lexer;
mod parser;

pub use parser::eval_arithmetic;

/// Erreur d'évaluation d'une expression.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ExprError {
    #[error("caractère inattendu : {0}")]
    UnexpectedChar(char),
    #[error("expression incomplète")]
    UnexpectedEnd,
    #[error("parenthèses déséquilibrées")]
    UnbalancedParen,
    #[error("division par zéro")]
    DivisionByZero,
}
