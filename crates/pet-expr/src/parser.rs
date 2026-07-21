//! Parseur d'expressions arithmétiques à descente récursive.

use crate::ExprError;
use crate::lexer::{Token, tokenize};

/// Crée une description textuelle d'un jeton pour les messages d'erreur.
fn token_description(token: &Token) -> String {
    match token {
        Token::Number(n) => format!("nombre {}", n),
        Token::Plus => "opérateur +".to_string(),
        Token::Minus => "opérateur -".to_string(),
        Token::Star => "opérateur *".to_string(),
        Token::Slash => "opérateur /".to_string(),
        Token::Percent => "opérateur %".to_string(),
        Token::LParen => "parenthèse ouvrante".to_string(),
        Token::RParen => "parenthèse fermante".to_string(),
    }
}

/// Évalue une expression arithmétique ne contenant plus aucun jeton symbolique.
pub fn eval_arithmetic(input: &str) -> Result<f64, ExprError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser {
        tokens: &tokens,
        pos: 0,
    };
    let value = parser.parse_sum()?;
    if parser.pos != parser.tokens.len() {
        match parser.peek() {
            Some(token) => Err(ExprError::UnexpectedToken(token_description(token))),
            None => Err(ExprError::UnexpectedEnd),
        }
    } else {
        Ok(value)
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    /// Somme et différence : priorité la plus faible.
    fn parse_sum(&mut self) -> Result<f64, ExprError> {
        let mut left = self.parse_product()?;
        while let Some(op) = self.peek().cloned() {
            match op {
                Token::Plus => {
                    self.pos += 1;
                    left += self.parse_product()?;
                }
                Token::Minus => {
                    self.pos += 1;
                    left -= self.parse_product()?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// Produit, quotient et reste.
    fn parse_product(&mut self) -> Result<f64, ExprError> {
        let mut left = self.parse_unary()?;
        while let Some(op) = self.peek().cloned() {
            match op {
                Token::Star => {
                    self.pos += 1;
                    left *= self.parse_unary()?;
                }
                Token::Slash => {
                    self.pos += 1;
                    let right = self.parse_unary()?;
                    if right == 0.0 {
                        return Err(ExprError::DivisionByZero);
                    }
                    left /= right;
                }
                Token::Percent => {
                    self.pos += 1;
                    let right = self.parse_unary()?;
                    if right == 0.0 {
                        return Err(ExprError::DivisionByZero);
                    }
                    left %= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// Signe unaire.
    fn parse_unary(&mut self) -> Result<f64, ExprError> {
        match self.peek() {
            Some(Token::Minus) => {
                self.pos += 1;
                Ok(-self.parse_unary()?)
            }
            Some(Token::Plus) => {
                self.pos += 1;
                self.parse_unary()
            }
            _ => self.parse_atom(),
        }
    }

    /// Nombre ou expression parenthésée.
    fn parse_atom(&mut self) -> Result<f64, ExprError> {
        match self.peek().cloned() {
            Some(Token::Number(v)) => {
                self.pos += 1;
                Ok(v)
            }
            Some(Token::LParen) => {
                self.pos += 1;
                let value = self.parse_sum()?;
                match self.peek() {
                    Some(Token::RParen) => {
                        self.pos += 1;
                        Ok(value)
                    }
                    _ => Err(ExprError::UnbalancedParen),
                }
            }
            Some(token) => Err(ExprError::UnexpectedToken(token_description(&token))),
            None => Err(ExprError::UnexpectedEnd),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evalue_un_entier() {
        assert_eq!(eval_arithmetic("42").unwrap(), 42.0);
    }

    #[test]
    fn respecte_la_priorite_des_operateurs() {
        assert_eq!(eval_arithmetic("2+3*4").unwrap(), 14.0);
    }

    #[test]
    fn respecte_les_parentheses() {
        assert_eq!(eval_arithmetic("(2+3)*4").unwrap(), 20.0);
    }

    #[test]
    fn gere_l_unaire_negatif() {
        assert_eq!(eval_arithmetic("-5+2").unwrap(), -3.0);
        assert_eq!(eval_arithmetic("3*-2").unwrap(), -6.0);
    }

    #[test]
    fn gere_le_modulo_et_la_division() {
        assert_eq!(eval_arithmetic("10%3").unwrap(), 1.0);
        assert_eq!(eval_arithmetic("10/4").unwrap(), 2.5);
    }

    #[test]
    fn ignore_les_espaces() {
        assert_eq!(eval_arithmetic(" 1920 / 2 - 32 ").unwrap(), 928.0);
    }

    #[test]
    fn refuse_une_expression_incomplete() {
        assert!(matches!(
            eval_arithmetic("2+"),
            Err(ExprError::UnexpectedEnd)
        ));
    }

    #[test]
    fn refuse_une_parenthese_non_fermee() {
        assert!(matches!(
            eval_arithmetic("(2+3"),
            Err(ExprError::UnbalancedParen)
        ));
    }

    #[test]
    fn refuse_la_division_par_zero() {
        assert!(matches!(
            eval_arithmetic("1/0"),
            Err(ExprError::DivisionByZero)
        ));
    }

    #[test]
    fn respecte_associativite_soustraction() {
        // (10 - 3) - 2 = 7 - 2 = 5 (pas 10 - (3 - 2) = 10 - 1 = 9)
        assert_eq!(eval_arithmetic("10-3-2").unwrap(), 5.0);
    }

    #[test]
    fn respecte_associativite_division() {
        // (100 / 5) / 2 = 20 / 2 = 10 (pas 100 / (5 / 2) = 100 / 2.5 = 40)
        assert_eq!(eval_arithmetic("100/5/2").unwrap(), 10.0);
    }

    #[test]
    fn respecte_associativite_modulo() {
        // (20 % 7) % 4 = 6 % 4 = 2 (pas 20 % (7 % 4) = 20 % 3 = 2, mais la precedence est importante)
        assert_eq!(eval_arithmetic("20%7%4").unwrap(), 2.0);
    }

    #[test]
    fn refuse_operateur_au_debut() {
        // Un opérateur au début : "*5"
        assert!(matches!(
            eval_arithmetic("*5"),
            Err(ExprError::UnexpectedToken(_))
        ));
    }

    #[test]
    fn refuse_parenthese_fermante_seule() {
        // Une parenthèse fermante sans ouvrante : ")"
        assert!(matches!(
            eval_arithmetic(")"),
            Err(ExprError::UnexpectedToken(_))
        ));
    }

    #[test]
    fn refuse_jetons_excedentaires() {
        // Deux nombres sans opérateur : "2 3"
        assert!(matches!(
            eval_arithmetic("2 3"),
            Err(ExprError::UnexpectedToken(_))
        ));
    }

    #[test]
    fn verifie_parenthese_non_fermee() {
        // Parenthèse non fermée : "(2+3"
        assert!(matches!(
            eval_arithmetic("(2+3"),
            Err(ExprError::UnbalancedParen)
        ));
    }
}
