# RustyPet — Plan d'implémentation 1/2 : le cœur du moteur

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Construire le cœur pur et sans I/O du moteur eSheep en Rust — parsing des `animations.xml`, évaluateur d'expressions, machine à états d'animation, physique — livré avec une CLI headless qui simule un pet et dumpe sa trace de sprites.

**Architecture:** Trois crates sans dépendance système (`pet-expr`, `pet-format`, `pet-engine`) plus un backend `null` et un binaire de simulation. Le générateur aléatoire est injecté partout, ce qui rend la simulation entièrement déterministe et donc testable par instantanés. Aucun code GNOME, Wayland ou D-Bus dans ce plan.

**Tech Stack:** Rust 1.97 (edition 2024), `quick-xml`, `base64`, `image`, `rand` (`StdRng` seedé), `thiserror`, `clap`, `insta` pour les tests d'instantané.

## Global Constraints

- Spec de référence : `docs/superpowers/specs/2026-07-20-rustypet-design.md`
- Référence technique du moteur d'origine : `docs/reference/esheep-engine.md` — **à consulter systématiquement**, les numéros de section (§2.3, §4.3…) y renvoient.
- Toutes les coordonnées sont en **pixels entiers**, origine en haut-gauche, Y croissant vers le bas.
- Le code est en anglais ; **les commentaires et les messages de commit sont en français**.
- Aucun `unwrap()` ni `panic!()` dans le code de bibliothèque : les erreurs remontent en `Result` avec des types `thiserror`.
- Le RNG est **toujours** injecté via le trait `PetRng` — jamais de `rand::thread_rng()` dans la logique.
- Chaque tâche se termine par un commit qui bumpe la version SemVer dans le `Cargo.toml` du workspace **et** ajoute une entrée en tête de `CHANGELOG.md`.
- `cargo clippy --all-targets -- -D warnings` et `cargo fmt --check` doivent passer avant chaque commit.
- Le workspace vit dans `~/Dev/rustypet`. Tous les chemins ci-dessous sont relatifs à cette racine.

---

### Task 1: Squelette du workspace

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, `CHANGELOG.md`, `README.md`
- Create: `crates/pet-expr/Cargo.toml`, `crates/pet-expr/src/lib.rs`
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: rien
- Produces: le workspace Cargo `rustypet` en version `0.1.0`, avec le crate `pet-expr` vide qui compile.

- [ ] **Step 1: Créer le `Cargo.toml` du workspace**

```toml
[workspace]
resolver = "3"
members = ["crates/pet-expr"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.97"
license = "MIT"
repository = "https://github.com/yrbane/rustypet"

[workspace.dependencies]
thiserror = "2"
rand = "0.9"
quick-xml = "0.37"
base64 = "0.22"
image = { version = "0.25", default-features = false, features = ["png"] }
clap = { version = "4", features = ["derive"] }
insta = "1"
```

- [ ] **Step 2: Créer `rust-toolchain.toml` et `.gitignore`**

`rust-toolchain.toml` :
```toml
[toolchain]
channel = "1.97"
components = ["clippy", "rustfmt"]
```

`.gitignore` :
```
/target
**/*.rs.bk
.DS_Store
```

- [ ] **Step 3: Créer le crate `pet-expr`**

`crates/pet-expr/Cargo.toml` :
```toml
[package]
name = "pet-expr"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
thiserror.workspace = true
rand.workspace = true
```

`crates/pet-expr/src/lib.rs` :
```rust
//! Évaluateur des expressions `x`, `y`, `interval` et `repeat` des fichiers
//! `animations.xml` d'eSheep. Voir `docs/reference/esheep-engine.md` §3.
```

- [ ] **Step 4: Créer `CHANGELOG.md`**

```markdown
# Changelog

## 0.1.0 — 2026-07-20 · « Squelette du workspace »

- Mise en place du workspace Cargo `rustypet` (edition 2024, Rust 1.97).
- Crate `pet-expr` créé, vide.
- Intégration continue GitHub Actions : `fmt`, `clippy`, `test`.
```

- [ ] **Step 5: Créer la CI**

`.github/workflows/ci.yml` :
```yaml
name: CI
on: [push, pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.97
        with:
          components: clippy, rustfmt
      - run: cargo fmt --all --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all
```

- [ ] **Step 6: Vérifier que tout compile**

Run: `cargo test --all && cargo clippy --all-targets -- -D warnings && cargo fmt --all --check`
Expected: compilation réussie, `0 passed` (aucun test encore), aucun warning clippy.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml rust-toolchain.toml .gitignore CHANGELOG.md crates/pet-expr .github/workflows/ci.yml
git commit -m "Squelette du workspace Cargo et intégration continue

Version 0.1.0."
```

---

### Task 2: Évaluateur arithmétique

**Files:**
- Create: `crates/pet-expr/src/lexer.rs`, `crates/pet-expr/src/parser.rs`
- Modify: `crates/pet-expr/src/lib.rs`
- Test: tests unitaires en fin de `crates/pet-expr/src/parser.rs`

**Interfaces:**
- Consumes: rien
- Produces:
  - `pub enum ExprError { UnexpectedChar(char), UnexpectedEnd, UnbalancedParen, DivisionByZero }`
  - `pub fn eval_arithmetic(input: &str) -> Result<f64, ExprError>` — évalue une expression purement numérique (aucun jeton substitué).

**Contexte :** `DataTable.Compute` accepte `+ - * / %`, les parenthèses, l'unaire `-`, la priorité standard. Les XML de pets n'utilisent que cela (§3.3). Le résultat est en `f64` ; la troncature en `i32` est faite par l'appelant, **vers zéro** (comportement du cast C#).

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/pet-expr/src/parser.rs` :
```rust
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
        assert!(matches!(eval_arithmetic("2+"), Err(ExprError::UnexpectedEnd)));
    }

    #[test]
    fn refuse_une_parenthese_non_fermee() {
        assert!(matches!(eval_arithmetic("(2+3"), Err(ExprError::UnbalancedParen)));
    }

    #[test]
    fn refuse_la_division_par_zero() {
        assert!(matches!(eval_arithmetic("1/0"), Err(ExprError::DivisionByZero)));
    }
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p pet-expr`
Expected: FAIL — `cannot find function eval_arithmetic in this scope`.

- [ ] **Step 3: Écrire le lexer**

`crates/pet-expr/src/lexer.rs` :
```rust
//! Découpage d'une expression en jetons.

/// Un jeton d'expression arithmétique.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    LParen,
    RParen,
}

/// Découpe l'expression en jetons. Les espaces sont ignorés.
pub fn tokenize(input: &str) -> Result<Vec<Token>, crate::ExprError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\r' | '\n' => i += 1,
            '+' => { tokens.push(Token::Plus); i += 1; }
            '-' => { tokens.push(Token::Minus); i += 1; }
            '*' => { tokens.push(Token::Star); i += 1; }
            '/' => { tokens.push(Token::Slash); i += 1; }
            '%' => { tokens.push(Token::Percent); i += 1; }
            '(' => { tokens.push(Token::LParen); i += 1; }
            ')' => { tokens.push(Token::RParen); i += 1; }
            '0'..='9' | '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                let value = text.parse::<f64>().map_err(|_| crate::ExprError::UnexpectedChar(c))?;
                tokens.push(Token::Number(value));
            }
            other => return Err(crate::ExprError::UnexpectedChar(other)),
        }
    }

    Ok(tokens)
}
```

- [ ] **Step 4: Écrire le parseur**

Au début de `crates/pet-expr/src/parser.rs` (avant le module `tests`) :
```rust
//! Parseur d'expressions arithmétiques à descente récursive.

use crate::lexer::{tokenize, Token};
use crate::ExprError;

/// Évalue une expression arithmétique ne contenant plus aucun jeton symbolique.
pub fn eval_arithmetic(input: &str) -> Result<f64, ExprError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser { tokens: &tokens, pos: 0 };
    let value = parser.parse_sum()?;
    if parser.pos != parser.tokens.len() {
        return Err(ExprError::UnbalancedParen);
    }
    Ok(value)
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
                Token::Plus => { self.pos += 1; left += self.parse_product()?; }
                Token::Minus => { self.pos += 1; left -= self.parse_product()?; }
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
                Token::Star => { self.pos += 1; left *= self.parse_unary()?; }
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
            Some(Token::Minus) => { self.pos += 1; Ok(-self.parse_unary()?) }
            Some(Token::Plus) => { self.pos += 1; self.parse_unary() }
            _ => self.parse_atom(),
        }
    }

    /// Nombre ou expression parenthésée.
    fn parse_atom(&mut self) -> Result<f64, ExprError> {
        match self.peek().cloned() {
            Some(Token::Number(v)) => { self.pos += 1; Ok(v) }
            Some(Token::LParen) => {
                self.pos += 1;
                let value = self.parse_sum()?;
                match self.peek() {
                    Some(Token::RParen) => { self.pos += 1; Ok(value) }
                    _ => Err(ExprError::UnbalancedParen),
                }
            }
            Some(_) => Err(ExprError::UnexpectedEnd),
            None => Err(ExprError::UnexpectedEnd),
        }
    }
}
```

- [ ] **Step 5: Déclarer les modules et l'erreur**

`crates/pet-expr/src/lib.rs` :
```rust
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
```

- [ ] **Step 6: Lancer les tests**

Run: `cargo test -p pet-expr`
Expected: PASS — 9 tests.

- [ ] **Step 7: Vérifier le lint**

Run: `cargo clippy --all-targets -- -D warnings && cargo fmt --all`
Expected: aucun warning.

- [ ] **Step 8: Commit**

```bash
git add crates/pet-expr Cargo.toml CHANGELOG.md
git commit -m "Évaluateur d'expressions arithmétiques

Parseur à descente récursive reproduisant DataTable.Compute pour les
opérateurs réellement utilisés par les pets : + - * / %, parenthèses,
unaire. Version 0.2.0."
```

Avant le commit, passer `version = "0.2.0"` dans `Cargo.toml` et ajouter en tête de `CHANGELOG.md` :
```markdown
## 0.2.0 — 2026-07-20 · « Évaluateur arithmétique »

- Lexer et parseur à descente récursive pour les expressions des animations.
- Opérateurs `+ - * / %`, parenthèses, signe unaire, priorité standard.
- Erreurs typées : caractère inattendu, expression incomplète, parenthèses
  déséquilibrées, division par zéro.
```

---

### Task 3: Substitution des jetons et contexte d'évaluation

**Files:**
- Create: `crates/pet-expr/src/context.rs`, `crates/pet-expr/src/value.rs`
- Modify: `crates/pet-expr/src/lib.rs`

**Interfaces:**
- Consumes: `eval_arithmetic` (Task 2)
- Produces:
  - `pub trait PetRng { fn gen_range_i32(&mut self, low: i32, high: i32) -> i32; }`
  - `pub struct SeededRng(rand::rngs::StdRng)` avec `SeededRng::new(seed: u64)`, implémentant `PetRng`
  - `pub struct EvalContext { pub screen_w: i32, pub screen_h: i32, pub area_w: i32, pub area_h: i32, pub image_w: i32, pub image_h: i32, pub image_x: i32, pub image_y: i32, pub rand_spawn: i32, pub scale: i32 }`
  - `pub fn substitute(expr: &str, ctx: &EvalContext, rng: &mut dyn PetRng, parent_flipped: bool) -> String`
  - `pub fn eval(expr: &str, ctx: &EvalContext, rng: &mut dyn PetRng, parent_flipped: bool) -> i32`
  - `pub struct PetValue { pub compute: String, pub is_dynamic: bool, pub is_screen: bool, pub value: i32 }` avec `PetValue::new(compute: String) -> Self` et `PetValue::update(&mut self, ctx, rng, parent_flipped)` et `PetValue::get(&self) -> i32`

**Contexte critique (§3.1) :**
- `areaH` vaut `WorkingArea.Height + WorkingArea.Y`, c'est-à-dire le **bord bas** de la zone de travail. Le champ `EvalContext::area_h` porte déjà cette valeur pré-calculée.
- `random` est tiré dans `[0, 100]` et **toutes ses occurrences dans une même expression reçoivent la même valeur** (le C# fait un `Replace` global).
- `randS` est tiré dans `[10, 90]` **une fois par chargement de XML** — il est donc porté par `EvalContext::rand_spawn`, pas retiré ici.
- L'ordre de substitution doit remplacer `random` **avant** `randS`, et les jetons longs avant les courts pour éviter les préfixes.
- Un échec de parsing retourne `0` (le C# logge et retourne 0).
- La conversion `f64 → i32` est une **troncature vers zéro**, pas un arrondi.
- Miroir de l'enfant (§3.2) : si `parent_flipped`, on remplace `-imageW` par `+imageW` si présent, sinon `imageW` par `(-imageW)`.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/pet-expr/src/context.rs` :
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EvalContext {
        EvalContext {
            screen_w: 1920,
            screen_h: 1080,
            area_w: 1920,
            area_h: 1050,
            image_w: 64,
            image_h: 64,
            image_x: -1,
            image_y: -1,
            rand_spawn: 42,
            scale: 1,
        }
    }

    #[test]
    fn substitue_les_dimensions_d_ecran() {
        let mut rng = SeededRng::new(1);
        assert_eq!(eval("screenW/2-imageW/2", &ctx(), &mut rng, false), 928);
    }

    #[test]
    fn substitue_le_bord_bas_de_la_zone_de_travail() {
        let mut rng = SeededRng::new(1);
        assert_eq!(eval("areaH-imageH", &ctx(), &mut rng, false), 986);
    }

    #[test]
    fn randS_vient_du_contexte() {
        let mut rng = SeededRng::new(1);
        assert_eq!(eval("randS", &ctx(), &mut rng, false), 42);
    }

    #[test]
    fn toutes_les_occurrences_de_random_ont_la_meme_valeur() {
        let mut rng = SeededRng::new(7);
        // random-random vaut toujours 0 si la substitution est globale et unique.
        assert_eq!(eval("random-random", &ctx(), &mut rng, false), 0);
    }

    #[test]
    fn random_reste_dans_ses_bornes() {
        let mut rng = SeededRng::new(3);
        for _ in 0..100 {
            let v = eval("random", &ctx(), &mut rng, false);
            assert!((0..=100).contains(&v), "valeur hors bornes : {v}");
        }
    }

    #[test]
    fn tronque_vers_zero() {
        let mut rng = SeededRng::new(1);
        assert_eq!(eval("7/2", &ctx(), &mut rng, false), 3);
        assert_eq!(eval("0-7/2", &ctx(), &mut rng, false), -3);
    }

    #[test]
    fn retourne_zero_si_l_expression_est_invalide() {
        let mut rng = SeededRng::new(1);
        assert_eq!(eval("2+", &ctx(), &mut rng, false), 0);
    }

    #[test]
    fn miroir_enfant_inverse_imageW() {
        let mut rng = SeededRng::new(1);
        // parent retourné : imageW devient (-imageW)
        assert_eq!(eval("imageW", &ctx(), &mut rng, true), -64);
        // et -imageW devient +imageW
        assert_eq!(eval("0-imageW", &ctx(), &mut rng, true), 64);
    }

    #[test]
    fn le_rng_seede_est_reproductible() {
        let mut a = SeededRng::new(99);
        let mut b = SeededRng::new(99);
        let va: Vec<i32> = (0..10).map(|_| eval("random", &ctx(), &mut a, false)).collect();
        let vb: Vec<i32> = (0..10).map(|_| eval("random", &ctx(), &mut b, false)).collect();
        assert_eq!(va, vb);
    }
}
```

Dans `crates/pet-expr/src/value.rs` :
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{EvalContext, SeededRng};

    fn ctx() -> EvalContext {
        EvalContext {
            screen_w: 1920, screen_h: 1080, area_w: 1920, area_h: 1050,
            image_w: 64, image_h: 64, image_x: -1, image_y: -1,
            rand_spawn: 42, scale: 1,
        }
    }

    #[test]
    fn classe_les_expressions_dynamiques() {
        assert!(PetValue::new("random*2".into()).is_dynamic);
        assert!(PetValue::new("randS".into()).is_dynamic);
        assert!(PetValue::new("imageX".into()).is_dynamic);
        assert!(PetValue::new("imageY".into()).is_dynamic);
        assert!(!PetValue::new("imageW".into()).is_dynamic);
        assert!(!PetValue::new("12".into()).is_dynamic);
    }

    #[test]
    fn classe_les_expressions_dependant_de_l_ecran() {
        assert!(PetValue::new("screenW".into()).is_screen);
        assert!(PetValue::new("areaH".into()).is_screen);
        assert!(!PetValue::new("imageW".into()).is_screen);
    }

    #[test]
    fn update_puis_get_renvoie_la_valeur_calculee() {
        let mut rng = SeededRng::new(1);
        let mut v = PetValue::new("screenW/2".into());
        v.update(&ctx(), &mut rng, false);
        assert_eq!(v.get(), 960);
    }
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p pet-expr`
Expected: FAIL — `EvalContext` et `PetValue` introuvables.

- [ ] **Step 3: Implémenter le contexte et la substitution**

Au début de `crates/pet-expr/src/context.rs` :
```rust
//! Substitution des jetons symboliques et évaluation contextuelle.
//! Voir `docs/reference/esheep-engine.md` §3.1 et §3.2.

use crate::eval_arithmetic;
use rand::{Rng, SeedableRng};

/// Source d'aléa du moteur. Injectée partout pour rendre la simulation
/// déterministe en test.
pub trait PetRng {
    /// Tire un entier dans `[low, high]`, bornes incluses.
    fn gen_range_i32(&mut self, low: i32, high: i32) -> i32;
}

/// Générateur seedé, reproductible d'une exécution à l'autre.
pub struct SeededRng(rand::rngs::StdRng);

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        Self(rand::rngs::StdRng::seed_from_u64(seed))
    }
}

impl PetRng for SeededRng {
    fn gen_range_i32(&mut self, low: i32, high: i32) -> i32 {
        if low >= high {
            return low;
        }
        self.0.random_range(low..=high)
    }
}

/// Valeurs substituées dans les expressions des animations.
#[derive(Debug, Clone, Copy)]
pub struct EvalContext {
    /// Largeur totale de l'écran.
    pub screen_w: i32,
    /// Hauteur totale de l'écran.
    pub screen_h: i32,
    /// Largeur de la zone de travail.
    pub area_w: i32,
    /// Bord bas de la zone de travail (hauteur + décalage Y). Voir §3.1.
    pub area_h: i32,
    /// Largeur d'une frame.
    pub image_w: i32,
    /// Hauteur d'une frame.
    pub image_h: i32,
    /// X du parent, -1 si le pet n'est pas un enfant.
    pub image_x: i32,
    /// Y du parent, -1 si le pet n'est pas un enfant.
    pub image_y: i32,
    /// Tirage figé au chargement du XML, dans [10, 90].
    pub rand_spawn: i32,
    /// Facteur d'échelle HiDPI.
    pub scale: i32,
}

/// Remplace les jetons symboliques par leurs valeurs.
///
/// L'ordre importe : `random` est substitué avant `randS`, et toutes les
/// occurrences de `random` dans une même expression reçoivent la même valeur.
pub fn substitute(
    expr: &str,
    ctx: &EvalContext,
    rng: &mut dyn PetRng,
    parent_flipped: bool,
) -> String {
    let mut out = expr.to_string();

    // Miroir horizontal du placement d'un enfant sous parent retourné (§3.2).
    if parent_flipped {
        if out.contains("-imageW") {
            out = out.replace("-imageW", "+imageW");
        } else {
            out = out.replace("imageW", "(0-imageW)");
        }
    }

    // Une seule valeur de `random` pour toute l'expression.
    let random_value = rng.gen_range_i32(0, 100);
    out = out.replace("random", &random_value.to_string());
    out = out.replace("randS", &ctx.rand_spawn.to_string());

    out = out.replace("screenW", &ctx.screen_w.to_string());
    out = out.replace("screenH", &ctx.screen_h.to_string());
    out = out.replace("areaW", &ctx.area_w.to_string());
    out = out.replace("areaH", &ctx.area_h.to_string());
    out = out.replace("imageW", &ctx.image_w.to_string());
    out = out.replace("imageH", &ctx.image_h.to_string());
    out = out.replace("imageX", &ctx.image_x.to_string());
    out = out.replace("imageY", &ctx.image_y.to_string());
    out = out.replace("scale", &ctx.scale.to_string());

    out
}

/// Substitue puis évalue. Retourne 0 si l'expression est invalide, comme le
/// moteur d'origine.
pub fn eval(expr: &str, ctx: &EvalContext, rng: &mut dyn PetRng, parent_flipped: bool) -> i32 {
    let substituted = substitute(expr, ctx, rng, parent_flipped);
    match eval_arithmetic(&substituted) {
        // Troncature vers zéro, comme le cast (int) du C#.
        Ok(v) => v.trunc() as i32,
        Err(_) => 0,
    }
}
```

- [ ] **Step 4: Implémenter `PetValue`**

Au début de `crates/pet-expr/src/value.rs` :
```rust
//! Valeur d'animation : expression, classification statique, valeur calculée.
//! Voir `docs/reference/esheep-engine.md` §3.4.

use crate::context::{eval, EvalContext, PetRng};

/// Une valeur issue du XML : l'expression source, sa classification, et la
/// dernière valeur calculée.
#[derive(Debug, Clone)]
pub struct PetValue {
    /// Expression telle qu'écrite dans le XML.
    pub compute: String,
    /// Doit être réévaluée à chaque démarrage d'animation.
    pub is_dynamic: bool,
    /// Dépend des dimensions de l'écran : à recalculer au changement de moniteur.
    pub is_screen: bool,
    /// Dernière valeur calculée.
    pub value: i32,
}

impl PetValue {
    /// Construit la valeur et la classe, sans l'évaluer.
    pub fn new(compute: String) -> Self {
        let is_dynamic = compute.contains("random")
            || compute.contains("randS")
            || compute.contains("imageX")
            || compute.contains("imageY");
        let is_screen = compute.contains("screen") || compute.contains("area");
        Self { compute, is_dynamic, is_screen, value: 0 }
    }

    /// Recalcule la valeur à partir du contexte courant.
    pub fn update(&mut self, ctx: &EvalContext, rng: &mut dyn PetRng, parent_flipped: bool) {
        self.value = eval(&self.compute, ctx, rng, parent_flipped);
    }

    /// Dernière valeur calculée.
    pub fn get(&self) -> i32 {
        self.value
    }
}
```

- [ ] **Step 5: Déclarer les modules**

Dans `crates/pet-expr/src/lib.rs`, remplacer le bloc `mod` et `pub use` par :
```rust
mod context;
mod lexer;
mod parser;
mod value;

pub use context::{eval, substitute, EvalContext, PetRng, SeededRng};
pub use parser::eval_arithmetic;
pub use value::PetValue;
```

- [ ] **Step 6: Lancer les tests**

Run: `cargo test -p pet-expr`
Expected: PASS — 21 tests.

- [ ] **Step 7: Vérifier le lint et commiter**

```bash
cargo clippy --all-targets -- -D warnings && cargo fmt --all
git add crates/pet-expr Cargo.toml CHANGELOG.md
git commit -m "Substitution des jetons et valeurs d'animation

Contexte d'évaluation (screenW, areaH, imageW, random, randS, scale),
RNG injecté et seedé, miroir des enfants sous parent retourné,
classification statique dynamique/écran. Version 0.3.0."
```

Bump `version = "0.3.0"` et entrée `CHANGELOG.md` :
```markdown
## 0.3.0 — 2026-07-20 · « Substitution des jetons »

- Contexte d'évaluation portant les 11 jetons du moteur d'origine.
- RNG injecté via le trait `PetRng`, implémentation seedée reproductible.
- `random` uniforme sur toute l'expression, `randS` figé par chargement.
- Miroir horizontal du placement des enfants sous parent retourné.
- Classification `is_dynamic` / `is_screen` des valeurs.
```

---

### Task 4: Parsing du fichier `animations.xml`

**Files:**
- Create: `crates/pet-format/Cargo.toml`, `crates/pet-format/src/lib.rs`, `crates/pet-format/src/model.rs`, `crates/pet-format/src/parse.rs`
- Create: `crates/pet-format/tests/fixtures/` (copie de 3 pets)
- Modify: `Cargo.toml` (ajouter le membre)
- Test: `crates/pet-format/tests/parse_corpus.rs`

**Interfaces:**
- Consumes: `pet_expr::PetValue`
- Produces:
  - `pub struct PetDefinition { pub header: Header, pub image: ImageDef, pub spawns: Vec<Spawn>, pub animations: Vec<Animation>, pub childs: Vec<Child>, pub sounds: Vec<Sound> }`
  - `pub struct Header { pub author: String, pub title: String, pub petname: String, pub version: String, pub info: String, pub application: i32, pub icon: String }`
  - `pub struct ImageDef { pub tiles_x: u32, pub tiles_y: u32, pub png_base64: String, pub transparency: String }`
  - `pub struct Movement { pub x: PetValue, pub y: PetValue, pub interval: PetValue, pub offset_y: i32, pub opacity: f64 }`
  - `pub struct Sequence { pub repeat: PetValue, pub repeat_from: i32, pub frames: Vec<i32>, pub action: Option<String>, pub next: Vec<NextAnimation> }`
  - `pub struct Animation { pub id: i32, pub name: String, pub start: Movement, pub end: Option<Movement>, pub sequence: Sequence, pub border: Vec<NextAnimation>, pub gravity: Vec<NextAnimation> }`
  - `pub struct NextAnimation { pub id: i32, pub probability: i32, pub only: OnlyFlags }`
  - `pub struct OnlyFlags(pub u8)` avec les constantes `OnlyFlags::NONE` (`0x7F`), `TASKBAR` (`0x01`), `WINDOW` (`0x02`), `HORIZONTAL` (`0x04`), `VERTICAL` (`0x08`)
  - `pub struct Spawn { pub id: i32, pub probability: i32, pub x: PetValue, pub y: PetValue, pub next: i32 }`
  - `pub struct Child { pub animation_id: i32, pub x: PetValue, pub y: PetValue, pub next: i32 }`
  - `pub struct Sound { pub animation_id: i32, pub probability: i32, pub loop_count: i32, pub base64: String }`
  - `pub fn parse_pet(xml: &str) -> Result<PetDefinition, FormatError>`

**Contexte critique (§1) :**
- Les sous-éléments suivent le modèle `xsd:all` : **l'ordre est libre**, ne jamais s'appuyer sur un ordre séquentiel.
- Valeurs par défaut : `offsety` = `0`, `opacity` = `1.0`, `transparency` = `"Magenta"`.
- `<end>` est optionnel ; absent, il vaut `<start>`.
- `<sequence>` mêle librement `frame`, `action` et `next` dans un `xsd:choice` répété.
- `repeat` est une **expression**, pas un entier.
- `only` accepte `none | taskbar | window | horizontal | horizontal+ | vertical`. `horizontal+` est traité comme `HORIZONTAL`.
- Le parsing doit être **tolérant** : un élément inconnu est ignoré, un champ absent prend son défaut.

- [ ] **Step 1: Préparer les fixtures**

```bash
mkdir -p crates/pet-format/tests/fixtures
for p in neko esheep64 pingus; do
  cp ~/Dev/desktopPet/Pets/$p/animations.xml crates/pet-format/tests/fixtures/$p.xml
done
ls -la crates/pet-format/tests/fixtures
```
Expected: trois fichiers `.xml` présents.

- [ ] **Step 2: Écrire le test qui échoue**

`crates/pet-format/tests/parse_corpus.rs` :
```rust
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
        assert!(pet.image.tiles_x > 0 && pet.image.tiles_y > 0, "{name} : tuiles");
        assert!(!pet.image.png_base64.is_empty(), "{name} : png vide");
        assert!(!pet.animations.is_empty(), "{name} : aucune animation");
        assert!(!pet.spawns.is_empty(), "{name} : aucun spawn");

        // Toute animation a au moins une frame.
        for anim in &pet.animations {
            assert!(!anim.sequence.frames.is_empty(), "{name} : animation {} sans frame", anim.id);
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
```

Et dans `crates/pet-format/src/parse.rs`, les tests unitaires ciblant les pièges :
```rust
#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
    <animations>
      <header><author>a</author><title>t</title><petname>p</petname>
        <version>1</version><info>i</info><application>1</application><icon>x</icon></header>
      <image><tilesx>4</tilesx><tilesy>2</tilesy><png>AAAA</png></image>
      <spawns><spawn id="1" probability="100"><x>0</x><y>0</y><next>1</next></spawn></spawns>
      <animations>
        <animation id="1">
          <name>walk</name>
          <start><x>2</x><y>0</y><interval>100</interval></start>
          <sequence repeat="3" repeatfrom="1"><frame>0</frame><frame>1</frame>
            <action>flip</action><next probability="50" only="taskbar">2</next></sequence>
          <border><next probability="100">3</next></border>
        </animation>
      </animations>
      <childs/>
    </animations>"#;

    #[test]
    fn applique_les_valeurs_par_defaut() {
        let pet = parse_pet(MINIMAL).unwrap();
        let anim = &pet.animations[0];
        assert_eq!(anim.start.offset_y, 0);
        assert_eq!(anim.start.opacity, 1.0);
        assert_eq!(pet.image.transparency, "Magenta");
    }

    #[test]
    fn end_absent_vaut_start() {
        let pet = parse_pet(MINIMAL).unwrap();
        assert!(pet.animations[0].end.is_none());
    }

    #[test]
    fn lit_les_frames_dans_l_ordre() {
        let pet = parse_pet(MINIMAL).unwrap();
        assert_eq!(pet.animations[0].sequence.frames, vec![0, 1]);
    }

    #[test]
    fn lit_l_action_flip() {
        let pet = parse_pet(MINIMAL).unwrap();
        assert_eq!(pet.animations[0].sequence.action.as_deref(), Some("flip"));
    }

    #[test]
    fn lit_repeat_comme_expression() {
        let pet = parse_pet(MINIMAL).unwrap();
        assert_eq!(pet.animations[0].sequence.repeat.compute, "3");
        assert_eq!(pet.animations[0].sequence.repeat_from, 1);
    }

    #[test]
    fn lit_les_drapeaux_only() {
        let pet = parse_pet(MINIMAL).unwrap();
        let next = &pet.animations[0].sequence.next[0];
        assert_eq!(next.id, 2);
        assert_eq!(next.probability, 50);
        assert_eq!(next.only, OnlyFlags::TASKBAR);
    }

    #[test]
    fn separe_border_de_sequence_next() {
        let pet = parse_pet(MINIMAL).unwrap();
        assert_eq!(pet.animations[0].border.len(), 1);
        assert_eq!(pet.animations[0].border[0].id, 3);
        assert!(pet.animations[0].gravity.is_empty());
    }

    #[test]
    fn accepte_l_ordre_libre_des_sous_elements() {
        let inverse = MINIMAL.replace(
            "<start><x>2</x><y>0</y><interval>100</interval></start>",
            "<start><interval>100</interval><y>0</y><x>2</x></start>",
        );
        let pet = parse_pet(&inverse).unwrap();
        assert_eq!(pet.animations[0].start.x.compute, "2");
        assert_eq!(pet.animations[0].start.interval.compute, "100");
    }

    #[test]
    fn horizontal_plus_vaut_horizontal() {
        assert_eq!(OnlyFlags::parse("horizontal+"), OnlyFlags::HORIZONTAL);
        assert_eq!(OnlyFlags::parse("none"), OnlyFlags::NONE);
        assert_eq!(OnlyFlags::parse(""), OnlyFlags::NONE);
    }
}
```

- [ ] **Step 3: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p pet-format`
Expected: FAIL — le crate `pet-format` n'existe pas encore.

- [ ] **Step 4: Créer le crate**

`crates/pet-format/Cargo.toml` :
```toml
[package]
name = "pet-format"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
pet-expr = { path = "../pet-expr" }
quick-xml.workspace = true
thiserror.workspace = true
```

Dans le `Cargo.toml` du workspace, passer `members` à :
```toml
members = ["crates/pet-expr", "crates/pet-format"]
```

- [ ] **Step 5: Écrire le modèle de données**

`crates/pet-format/src/model.rs` :
```rust
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
        self.animations.iter().find(|a| a.name == name).map(|a| a.id)
    }
}
```

- [ ] **Step 6: Écrire le parseur**

`crates/pet-format/src/parse.rs`, avant le module `tests` :
```rust
//! Lecture tolérante d'un `animations.xml`.
//!
//! Le schéma utilise `xsd:all` : l'ordre des sous-éléments est libre. Le
//! parseur construit donc un arbre générique puis lit les champs par nom.

use crate::model::*;
use crate::FormatError;
use pet_expr::PetValue;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;

/// Nœud générique : texte, attributs, enfants par nom.
#[derive(Debug, Default, Clone)]
pub(crate) struct Node {
    pub text: String,
    pub attrs: HashMap<String, String>,
    pub children: Vec<(String, Node)>,
}

impl Node {
    /// Premier enfant portant ce nom.
    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|(n, _)| n == name).map(|(_, c)| c)
    }

    /// Tous les enfants portant ce nom.
    fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children.iter().filter(move |(n, _)| n == name).map(|(_, c)| c)
    }

    /// Texte d'un enfant, ou la valeur par défaut.
    fn text_of(&self, name: &str, default: &str) -> String {
        self.child(name).map(|c| c.text.trim().to_string()).filter(|s| !s.is_empty())
            .unwrap_or_else(|| default.to_string())
    }

    /// Entier d'un enfant, ou la valeur par défaut.
    fn int_of(&self, name: &str, default: i32) -> i32 {
        self.text_of(name, "").parse().unwrap_or(default)
    }

    /// Attribut entier, ou la valeur par défaut.
    fn attr_int(&self, name: &str, default: i32) -> i32 {
        self.attrs.get(name).and_then(|v| v.parse().ok()).unwrap_or(default)
    }
}

/// Construit l'arbre générique à partir du XML.
///
/// Les préfixes de namespace sont retirés : les pets déclarent un namespace
/// par défaut que l'on ignore volontairement.
pub(crate) fn build_tree(xml: &str) -> Result<Node, FormatError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut stack: Vec<(String, Node)> = vec![(String::from("#root"), Node::default())];

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = local_name(e.name().as_ref());
                let mut node = Node::default();
                for attr in e.attributes().flatten() {
                    let key = local_name(attr.key.as_ref());
                    let value = attr.unescape_value().unwrap_or_default().to_string();
                    node.attrs.insert(key, value);
                }
                stack.push((name, node));
            }
            Ok(Event::Empty(e)) => {
                let name = local_name(e.name().as_ref());
                let mut node = Node::default();
                for attr in e.attributes().flatten() {
                    let key = local_name(attr.key.as_ref());
                    let value = attr.unescape_value().unwrap_or_default().to_string();
                    node.attrs.insert(key, value);
                }
                if let Some((_, parent)) = stack.last_mut() {
                    parent.children.push((name, node));
                }
            }
            Ok(Event::End(_)) => {
                if stack.len() > 1 {
                    let (name, node) = stack.pop().expect("pile non vide");
                    if let Some((_, parent)) = stack.last_mut() {
                        parent.children.push((name, node));
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if let Some((_, node)) = stack.last_mut() {
                    node.text.push_str(&text);
                }
            }
            Ok(Event::CData(e)) => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                if let Some((_, node)) = stack.last_mut() {
                    node.text.push_str(&text);
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(FormatError::Xml(e.to_string())),
        }
    }

    let (_, root) = stack.pop().expect("racine présente");
    Ok(root)
}

/// Retire le préfixe de namespace d'un nom qualifié.
fn local_name(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    match s.rsplit_once(':') {
        Some((_, local)) => local.to_string(),
        None => s.to_string(),
    }
}

/// Lit un `<next>` : contenu = identifiant, attributs `probability` et `only`.
fn parse_next(node: &Node) -> NextAnimation {
    NextAnimation {
        id: node.text.trim().parse().unwrap_or(-1),
        probability: node.attr_int("probability", 100),
        only: OnlyFlags::parse(node.attrs.get("only").map(String::as_str).unwrap_or("")),
    }
}

/// Lit un groupe `step` (`<start>` ou `<end>`).
fn parse_movement(node: &Node) -> Movement {
    Movement {
        x: PetValue::new(node.text_of("x", "0")),
        y: PetValue::new(node.text_of("y", "0")),
        interval: PetValue::new(node.text_of("interval", "100")),
        offset_y: node.int_of("offsety", 0),
        opacity: node.text_of("opacity", "1.0").parse().unwrap_or(1.0),
    }
}

/// Lit une `<sequence>` : `frame`, `action` et `next` sont mêlés librement.
fn parse_sequence(node: &Node) -> Sequence {
    let mut frames = Vec::new();
    let mut action = None;
    let mut next = Vec::new();

    for (name, child) in &node.children {
        match name.as_str() {
            "frame" => {
                if let Ok(v) = child.text.trim().parse::<i32>() {
                    frames.push(v);
                }
            }
            "action" => {
                let value = child.text.trim();
                if !value.is_empty() {
                    action = Some(value.to_string());
                }
            }
            "next" => next.push(parse_next(child)),
            _ => {}
        }
    }

    Sequence {
        repeat: PetValue::new(
            node.attrs.get("repeat").cloned().unwrap_or_else(|| "0".to_string()),
        ),
        repeat_from: node.attr_int("repeatfrom", 0),
        frames,
        action,
        next,
    }
}

/// Lit tous les `<next>` d'un conteneur `<border>` ou `<gravity>`.
fn parse_next_list(node: Option<&Node>) -> Vec<NextAnimation> {
    node.map(|n| n.children_named("next").map(parse_next).collect()).unwrap_or_default()
}

/// Lit un `animations.xml` complet.
pub fn parse_pet(xml: &str) -> Result<PetDefinition, FormatError> {
    let tree = build_tree(xml)?;
    let root = tree.child("animations").ok_or(FormatError::MissingRoot)?;

    let header_node = root.child("header").ok_or(FormatError::MissingElement("header"))?;
    let header = Header {
        author: header_node.text_of("author", ""),
        title: header_node.text_of("title", ""),
        petname: header_node.text_of("petname", "Pet"),
        version: header_node.text_of("version", "0"),
        info: header_node.text_of("info", ""),
        application: header_node.int_of("application", 1),
        icon: header_node.text_of("icon", ""),
    };

    let image_node = root.child("image").ok_or(FormatError::MissingElement("image"))?;
    let image = ImageDef {
        tiles_x: image_node.int_of("tilesx", 1).max(1) as u32,
        tiles_y: image_node.int_of("tilesy", 1).max(1) as u32,
        png_base64: image_node.text_of("png", "").split_whitespace().collect(),
        transparency: image_node.text_of("transparency", "Magenta"),
    };

    let spawns = root
        .child("spawns")
        .map(|n| {
            n.children_named("spawn")
                .map(|s| Spawn {
                    id: s.attr_int("id", 0),
                    probability: s.attr_int("probability", 100),
                    x: PetValue::new(s.text_of("x", "0")),
                    y: PetValue::new(s.text_of("y", "0")),
                    next: s.int_of("next", 1),
                })
                .collect()
        })
        .unwrap_or_default();

    // Le conteneur d'animations porte le même nom que la racine.
    let animations = root
        .child("animations")
        .map(|n| {
            n.children_named("animation")
                .map(|a| Animation {
                    id: a.attr_int("id", 0),
                    name: a.text_of("name", ""),
                    start: a.child("start").map(parse_movement).unwrap_or_else(|| {
                        parse_movement(&Node::default())
                    }),
                    end: a.child("end").map(parse_movement),
                    sequence: a
                        .child("sequence")
                        .map(parse_sequence)
                        .unwrap_or_else(|| parse_sequence(&Node::default())),
                    border: parse_next_list(a.child("border")),
                    gravity: parse_next_list(a.child("gravity")),
                })
                .collect()
        })
        .unwrap_or_default();

    let childs = root
        .child("childs")
        .map(|n| {
            n.children_named("child")
                .map(|c| Child {
                    animation_id: c.attr_int("animationid", 0),
                    x: PetValue::new(c.text_of("x", "0")),
                    y: PetValue::new(c.text_of("y", "0")),
                    next: c.int_of("next", 1),
                })
                .collect()
        })
        .unwrap_or_default();

    let sounds = root
        .child("sounds")
        .map(|n| {
            n.children_named("sound")
                .map(|s| Sound {
                    animation_id: s.attr_int("animationid", 0),
                    probability: s.int_of("probability", 100),
                    loop_count: s.int_of("loop", 0),
                    base64: s.text_of("base64", ""),
                })
                .collect()
        })
        .unwrap_or_default();

    if animations.is_empty() {
        return Err(FormatError::MissingElement("animations"));
    }

    Ok(PetDefinition { header, image, spawns, animations, childs, sounds })
}
```

- [ ] **Step 7: Écrire `lib.rs` et le type d'erreur**

`crates/pet-format/src/lib.rs` :
```rust
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
```

- [ ] **Step 8: Lancer les tests**

Run: `cargo test -p pet-format`
Expected: PASS — 9 tests unitaires + 2 tests d'intégration.

- [ ] **Step 9: Lancer le corpus complet**

Run: `RUSTYPET_CORPUS=$HOME/Dev/desktopPet/Pets cargo test -p pet-format -- --nocapture parse_le_corpus_complet`
Expected: PASS — plus de 10 pets parsés sans erreur. **Si un pet échoue, corriger le parseur avant de continuer** : la tolérance du parsing est une exigence.

- [ ] **Step 10: Commit**

```bash
cargo clippy --all-targets -- -D warnings && cargo fmt --all
git add crates/pet-format Cargo.toml CHANGELOG.md
git commit -m "Lecture des fichiers animations.xml

Parseur tolérant à l'ordre libre des sous-éléments (xsd:all), avec les
valeurs par défaut du moteur d'origine et les drapeaux de contexte only.
Corpus complet des pets amont validé. Version 0.4.0."
```

Bump `0.4.0` et entrée `CHANGELOG.md` :
```markdown
## 0.4.0 — 2026-07-20 · « Lecture des animations.xml »

- Crate `pet-format` : modèle de données complet du format eSheep.
- Parsing tolérant à l'ordre libre des sous-éléments et aux champs absents.
- Drapeaux de contexte `only`, `horizontal+` traité comme `horizontal`.
- Les 20+ pets du dépôt amont parsent sans erreur.
```

---

### Task 5: Décodage du spritesheet

**Files:**
- Create: `crates/pet-format/src/sprites.rs`
- Modify: `crates/pet-format/src/lib.rs`, `crates/pet-format/Cargo.toml`

**Interfaces:**
- Consumes: `ImageDef` (Task 4)
- Produces:
  - `pub struct SpriteSheet { pub width: u32, pub height: u32, pub rgba: Vec<u8>, pub tile_w: u32, pub tile_h: u32, pub tiles_x: u32, pub tiles_y: u32 }`
  - `pub fn decode_sheet(image: &ImageDef) -> Result<SpriteSheet, FormatError>`
  - `pub fn pad_base64(input: &str) -> String`
  - `pub fn parse_color(name: &str) -> [u8; 3]`
  - `impl SpriteSheet { pub fn tile_rect(&self, index: u32) -> (u32, u32, u32, u32) }` — retourne `(x, y, w, h)` de la tuile

**Contexte critique (§6) :**
- Le base64 est **souvent non padé** : ajouter `4 - (len % 4)` caractères `=` si nécessaire.
- La découpe est **en ligne d'abord** : `index = row * tiles_x + col`.
- `tile_w = width / tiles_x`, `tile_h = height / tiles_y`.
- Transparence : si le PNG a déjà un canal alpha **non trivial** (au moins un pixel avec `a < 255`), ne rien faire. Sinon, appliquer la couleur clé (`Magenta` = `#FF00FF` par défaut) en mettant `alpha = 0`.
- La limite Win32 de 255 px et le facteur `iScale` du C# ne sont **pas** portés : les sprites restent à leur taille native (§8.4).

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/pet-format/src/sprites.rs` :
```rust
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
        img.write_to(&mut buf, image::ImageFormat::Png).expect("encodage");
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(buf.into_inner())
    }

    fn image_def(png: String) -> ImageDef {
        ImageDef { tiles_x: 2, tiles_y: 2, png_base64: png, transparency: "Magenta".into() }
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
        img.write_to(&mut buf, image::ImageFormat::Png).expect("encodage");
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
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p pet-format sprites`
Expected: FAIL — module `sprites` inexistant.

- [ ] **Step 3: Ajouter les dépendances**

Dans `crates/pet-format/Cargo.toml`, ajouter sous `[dependencies]` :
```toml
base64.workspace = true
image.workspace = true
```

- [ ] **Step 4: Implémenter le décodage**

Au début de `crates/pet-format/src/sprites.rs` :
```rust
//! Décodage du spritesheet : base64 → PNG → RGBA avec couleur clé.
//! Voir `docs/reference/esheep-engine.md` §6.

use crate::model::ImageDef;
use crate::FormatError;
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
        (col * self.tile_w, row * self.tile_h, self.tile_w, self.tile_h)
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
    if let Some(hex) = trimmed.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16);
            let g = u8::from_str_radix(&hex[2..4], 16);
            let b = u8::from_str_radix(&hex[4..6], 16);
            if let (Ok(r), Ok(g), Ok(b)) = (r, g, b) {
                return [r, g, b];
            }
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

    let decoded = image::load_from_memory(&bytes)
        .map_err(|e| FormatError::Image(e.to_string()))?;
    let mut rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();

    if width == 0 || height == 0 {
        return Err(FormatError::Image("image de dimension nulle".into()));
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
```

- [ ] **Step 5: Étendre le type d'erreur et déclarer le module**

Dans `crates/pet-format/src/lib.rs`, ajouter `mod sprites;`, `pub use sprites::*;` et deux variantes :
```rust
    #[error("base64 invalide : {0}")]
    Base64(String),
    #[error("image invalide : {0}")]
    Image(String),
```

- [ ] **Step 6: Lancer les tests**

Run: `cargo test -p pet-format`
Expected: PASS — 7 nouveaux tests, tous les précédents toujours verts.

- [ ] **Step 7: Vérifier sur un vrai pet**

Ajouter dans `crates/pet-format/tests/parse_corpus.rs` :
```rust
/// Le spritesheet de chaque fixture doit se décoder et produire des tuiles
/// de dimensions plausibles.
#[test]
fn decode_les_spritesheets_des_fixtures() {
    for name in ["neko", "esheep64", "pingus"] {
        let path = format!("{}/tests/fixtures/{name}.xml", env!("CARGO_MANIFEST_DIR"));
        let xml = std::fs::read_to_string(&path).expect("fixture lisible");
        let pet = pet_format::parse_pet(&xml).expect("parsing");
        let sheet = pet_format::decode_sheet(&pet.image)
            .unwrap_or_else(|e| panic!("{name} : {e}"));

        assert!(sheet.tile_w > 0 && sheet.tile_h > 0, "{name} : tuile de taille nulle");
        assert_eq!(sheet.rgba.len(), (sheet.width * sheet.height * 4) as usize);

        // Toute frame référencée doit exister dans la feuille.
        for anim in &pet.animations {
            for &frame in &anim.sequence.frames {
                assert!(
                    (frame as u32) < sheet.tile_count(),
                    "{name} : animation {} référence la frame {frame}, hors feuille ({} tuiles)",
                    anim.id, sheet.tile_count()
                );
            }
        }
    }
}
```

Run: `cargo test -p pet-format`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
cargo clippy --all-targets -- -D warnings && cargo fmt --all
git add crates/pet-format Cargo.toml CHANGELOG.md
git commit -m "Décodage des spritesheets

Padding du base64 non padé, conversion de la couleur clé en alpha zéro
avec préservation d'un alpha existant, découpe des tuiles en ligne
d'abord. Version 0.5.0."
```

Bump `0.5.0`, entrée `CHANGELOG.md` :
```markdown
## 0.5.0 — 2026-07-20 · « Décodage des spritesheets »

- Décodage base64 avec padding automatique des flux non padés.
- Couleur clé convertie en alpha 0, alpha existant préservé.
- Découpe des tuiles en ligne d'abord, conforme aux index `<frame>`.
- La limite Win32 de 255 px et le facteur d'échelle entier ne sont pas portés.
```

---

### Task 6: État d'animation et interpolation

**Files:**
- Create: `crates/pet-engine/Cargo.toml`, `crates/pet-engine/src/lib.rs`, `crates/pet-engine/src/anim_state.rs`
- Modify: `Cargo.toml` (membre)

**Interfaces:**
- Consumes: `pet_format::{PetDefinition, Animation, Movement}`, `pet_expr::{EvalContext, PetRng, PetValue}`
- Produces:
  - `pub struct AnimState { pub animation_id: i32, pub step: i32, pub total_steps: i32 }`
  - `pub fn total_steps(seq: &Sequence) -> i32`
  - `pub fn pick_frame(seq: &Sequence, step: i32) -> i32`
  - `pub struct StepValues { pub x: i32, pub y: i32, pub interval: i32, pub opacity: f64, pub offset_y: i32 }`
  - `pub fn interpolate(anim: &Animation, step: i32, total: i32) -> StepValues`

**Contexte critique (§2.2, §2.3, §2.5) :**
- `total_steps = frames.len() + (frames.len() - repeat_from) * repeat`
- Interpolation avec **deux dénominateurs** : `T` pour `interval`, `opacity`, `offset_y` ; `T - 1` pour `x` et `y`. Si `T <= 1`, `x` et `y` valent leurs valeurs de départ.
- Choix de frame : si `step < frames.len()`, `frames[step]` ; sinon `frames[((step - len + repeat_from) % (len - repeat_from)) + repeat_from]`.
- `<end>` absent ⇒ les valeurs de fin égalent celles de début, donc aucune interpolation.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/pet-engine/src/anim_state.rs` :
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pet_expr::PetValue;
    use pet_format::{Animation, Movement, Sequence};

    fn movement(x: i32, y: i32, interval: i32, opacity: f64, offset_y: i32) -> Movement {
        let mut m = Movement {
            x: PetValue::new(x.to_string()),
            y: PetValue::new(y.to_string()),
            interval: PetValue::new(interval.to_string()),
            offset_y,
            opacity,
        };
        m.x.value = x;
        m.y.value = y;
        m.interval.value = interval;
        m
    }

    fn sequence(frames: Vec<i32>, repeat: i32, repeat_from: i32) -> Sequence {
        let mut r = PetValue::new(repeat.to_string());
        r.value = repeat;
        Sequence { repeat: r, repeat_from, frames, action: None, next: Vec::new() }
    }

    fn animation(start: Movement, end: Option<Movement>, seq: Sequence) -> Animation {
        Animation {
            id: 1,
            name: "test".into(),
            start,
            end,
            sequence: seq,
            border: Vec::new(),
            gravity: Vec::new(),
        }
    }

    #[test]
    fn calcule_le_nombre_total_de_pas() {
        // 4 frames, pas de répétition
        assert_eq!(total_steps(&sequence(vec![0, 1, 2, 3], 0, 0)), 4);
        // 4 frames répétées 2 fois de plus depuis le début : 4 + 4*2
        assert_eq!(total_steps(&sequence(vec![0, 1, 2, 3], 2, 0)), 12);
        // répétition partielle depuis l'index 2 : 4 + (4-2)*3
        assert_eq!(total_steps(&sequence(vec![0, 1, 2, 3], 3, 2)), 10);
    }

    #[test]
    fn choisit_la_frame_pendant_la_premiere_passe() {
        let seq = sequence(vec![10, 11, 12], 2, 1);
        assert_eq!(pick_frame(&seq, 0), 10);
        assert_eq!(pick_frame(&seq, 1), 11);
        assert_eq!(pick_frame(&seq, 2), 12);
    }

    #[test]
    fn choisit_la_frame_pendant_les_repetitions() {
        // frames [10,11,12], repeat_from=1 -> on boucle sur [11,12]
        let seq = sequence(vec![10, 11, 12], 2, 1);
        assert_eq!(pick_frame(&seq, 3), 11);
        assert_eq!(pick_frame(&seq, 4), 12);
        assert_eq!(pick_frame(&seq, 5), 11);
        assert_eq!(pick_frame(&seq, 6), 12);
    }

    #[test]
    fn interpole_avec_deux_denominateurs() {
        let start = movement(0, 0, 100, 0.0, 0);
        let end = movement(10, 20, 200, 1.0, 40);
        let anim = animation(start, Some(end), sequence(vec![0; 5], 0, 0));
        let total = 5;

        // interval, opacity, offset_y : dénominateur T = 5
        let at0 = interpolate(&anim, 0, total);
        assert_eq!(at0.interval, 100);
        assert_eq!(at0.opacity, 0.0);
        assert_eq!(at0.offset_y, 0);

        let at1 = interpolate(&anim, 1, total);
        assert_eq!(at1.interval, 120); // 100 + 100*1/5
        assert_eq!(at1.opacity, 0.2);
        assert_eq!(at1.offset_y, 8);

        // x, y : dénominateur T-1 = 4
        assert_eq!(at1.x, 2); // 0 + 10*1/4
        assert_eq!(at1.y, 5); // 0 + 20*1/4

        let at4 = interpolate(&anim, 4, total);
        assert_eq!(at4.x, 10); // valeur de fin atteinte au dernier pas
        assert_eq!(at4.y, 20);
    }

    #[test]
    fn ne_divise_pas_par_zero_si_un_seul_pas() {
        let start = movement(3, 4, 100, 1.0, 0);
        let end = movement(30, 40, 500, 0.0, 10);
        let anim = animation(start, Some(end), sequence(vec![0], 0, 0));
        let values = interpolate(&anim, 0, 1);
        assert_eq!(values.x, 3);
        assert_eq!(values.y, 4);
    }

    #[test]
    fn sans_end_les_valeurs_restent_celles_de_start() {
        let anim = animation(movement(5, 6, 80, 0.5, 3), None, sequence(vec![0; 4], 0, 0));
        for step in 0..4 {
            let v = interpolate(&anim, step, 4);
            assert_eq!(v.x, 5);
            assert_eq!(v.y, 6);
            assert_eq!(v.interval, 80);
            assert_eq!(v.opacity, 0.5);
            assert_eq!(v.offset_y, 3);
        }
    }
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p pet-engine`
Expected: FAIL — le crate `pet-engine` n'existe pas.

- [ ] **Step 3: Créer le crate**

`crates/pet-engine/Cargo.toml` :
```toml
[package]
name = "pet-engine"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
pet-expr = { path = "../pet-expr" }
pet-format = { path = "../pet-format" }
```

Dans le `Cargo.toml` du workspace :
```toml
members = ["crates/pet-expr", "crates/pet-format", "crates/pet-engine"]
```

- [ ] **Step 4: Implémenter l'état d'animation**

Au début de `crates/pet-engine/src/anim_state.rs` :
```rust
//! Nombre de pas, choix de frame et interpolation.
//! Voir `docs/reference/esheep-engine.md` §2.2, §2.3 et §2.5.

use pet_format::{Animation, Sequence};

/// Position dans l'animation courante.
#[derive(Debug, Clone, Copy)]
pub struct AnimState {
    pub animation_id: i32,
    /// Pas courant, 0-based. Vaut -1 avant le premier tick.
    pub step: i32,
    pub total_steps: i32,
}

/// Valeurs interpolées pour un pas donné.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepValues {
    /// Vitesse horizontale de ce pas, en pixels.
    pub x: i32,
    /// Vitesse verticale de ce pas, en pixels.
    pub y: i32,
    /// Durée de la frame, en millisecondes.
    pub interval: i32,
    pub opacity: f64,
    pub offset_y: i32,
}

/// Nombre total de pas de la séquence, répétitions comprises.
pub fn total_steps(seq: &Sequence) -> i32 {
    let len = seq.frames.len() as i32;
    if len == 0 {
        return 0;
    }
    let from = seq.repeat_from.clamp(0, len);
    len + (len - from) * seq.repeat.get().max(0)
}

/// Frame affichée au pas donné.
pub fn pick_frame(seq: &Sequence, step: i32) -> i32 {
    let len = seq.frames.len() as i32;
    if len == 0 {
        return 0;
    }
    let step = step.max(0);
    if step < len {
        return seq.frames[step as usize];
    }
    let from = seq.repeat_from.clamp(0, len - 1);
    let span = len - from;
    if span <= 0 {
        return seq.frames[(len - 1) as usize];
    }
    let idx = ((step - len + from) % span) + from;
    seq.frames[idx as usize]
}

/// Interpole les valeurs entre le début et la fin de l'animation.
///
/// Attention : `x` et `y` utilisent le dénominateur `total - 1`, les autres
/// valeurs utilisent `total` (§2.3).
pub fn interpolate(anim: &Animation, step: i32, total: i32) -> StepValues {
    let start = &anim.start;
    let end = anim.end.as_ref().unwrap_or(start);
    let step_f = step as f64;

    let (interval, opacity, offset_y) = if total > 0 {
        let ratio = step_f / total as f64;
        (
            start.interval.get() as f64
                + (end.interval.get() - start.interval.get()) as f64 * ratio,
            start.opacity + (end.opacity - start.opacity) * ratio,
            start.offset_y as f64 + (end.offset_y - start.offset_y) as f64 * ratio,
        )
    } else {
        (start.interval.get() as f64, start.opacity, start.offset_y as f64)
    };

    let (x, y) = if total > 1 {
        let ratio = step_f / (total - 1) as f64;
        (
            start.x.get() as f64 + (end.x.get() - start.x.get()) as f64 * ratio,
            start.y.get() as f64 + (end.y.get() - start.y.get()) as f64 * ratio,
        )
    } else {
        (start.x.get() as f64, start.y.get() as f64)
    };

    StepValues {
        x: x.trunc() as i32,
        y: y.trunc() as i32,
        interval: interval.trunc() as i32,
        opacity,
        offset_y: offset_y.trunc() as i32,
    }
}
```

- [ ] **Step 5: Écrire `lib.rs`**

`crates/pet-engine/src/lib.rs` :
```rust
//! Machine à états du pet : animations, physique, spawns et enfants.
//! Voir `docs/reference/esheep-engine.md` §2, §4 et §5.

mod anim_state;

pub use anim_state::{interpolate, pick_frame, total_steps, AnimState, StepValues};
```

- [ ] **Step 6: Lancer les tests**

Run: `cargo test -p pet-engine`
Expected: PASS — 6 tests.

- [ ] **Step 7: Commit**

```bash
cargo clippy --all-targets -- -D warnings && cargo fmt --all
git add crates/pet-engine Cargo.toml CHANGELOG.md
git commit -m "État d'animation et interpolation

Nombre total de pas avec répétitions partielles, choix de frame en
boucle depuis repeatfrom, et interpolation à deux dénominateurs
distincts pour x/y et pour interval/opacity/offsety. Version 0.6.0."
```

Bump `0.6.0`, entrée `CHANGELOG.md` :
```markdown
## 0.6.0 — 2026-07-20 · « État d'animation »

- Crate `pet-engine` : calcul du nombre de pas, répétitions partielles.
- Choix de la frame affichée, y compris pendant les boucles.
- Interpolation fidèle au moteur d'origine, avec ses deux dénominateurs.
```

---

### Task 7: Choix de l'animation suivante

**Files:**
- Create: `crates/pet-engine/src/transitions.rs`
- Modify: `crates/pet-engine/src/lib.rs`, `crates/pet-engine/Cargo.toml`

**Interfaces:**
- Consumes: `pet_format::{NextAnimation, OnlyFlags}`, `pet_expr::PetRng`
- Produces:
  - `pub fn pick_next(candidates: &[NextAnimation], context: OnlyFlags, rng: &mut dyn PetRng) -> Option<i32>`
  - `pub fn pick_spawn(spawns: &[Spawn], rng: &mut dyn PetRng) -> Option<usize>`

**Contexte critique (§2.4, §5.1) :**
1. Filtrer : sauter toute entrée telle que `only != NONE && (only & context) == 0`.
2. Sommer les `probability` retenues → `rand_max`.
3. Tirer `val = rng(0, rand_max)`.
4. Parcourir en accumulant ; la **première** entrée dont le cumul `>= val` est choisie.
5. Liste vide (ou tout filtré) → `None`, ce qui signifie « respawn » pour le pet principal et « fermeture » pour un enfant.

Le tirage des spawns suit le même algorithme, sans filtrage contextuel. Si aucun spawn n'est défini, l'appelant fabrique un spawn de secours (`x = "0"`, `y = "0"`, `interval = "1000"`, `next` = première animation connue, `probability = 100`).

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/pet-engine/src/transitions.rs` :
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pet_expr::SeededRng;
    use pet_format::{NextAnimation, OnlyFlags};

    fn next(id: i32, probability: i32, only: OnlyFlags) -> NextAnimation {
        NextAnimation { id, probability, only }
    }

    #[test]
    fn retourne_none_si_aucun_candidat() {
        let mut rng = SeededRng::new(1);
        assert_eq!(pick_next(&[], OnlyFlags::NONE, &mut rng), None);
    }

    #[test]
    fn choisit_l_unique_candidat() {
        let mut rng = SeededRng::new(1);
        let list = [next(7, 100, OnlyFlags::NONE)];
        assert_eq!(pick_next(&list, OnlyFlags::TASKBAR, &mut rng), Some(7));
    }

    #[test]
    fn filtre_selon_le_contexte() {
        let mut rng = SeededRng::new(1);
        let list = [
            next(1, 100, OnlyFlags::WINDOW),
            next(2, 100, OnlyFlags::TASKBAR),
        ];
        // Contexte barre des tâches : seul l'identifiant 2 est éligible.
        for _ in 0..20 {
            assert_eq!(pick_next(&list, OnlyFlags::TASKBAR, &mut rng), Some(2));
        }
    }

    #[test]
    fn retourne_none_si_tout_est_filtre() {
        let mut rng = SeededRng::new(1);
        let list = [next(1, 100, OnlyFlags::WINDOW)];
        assert_eq!(pick_next(&list, OnlyFlags::TASKBAR, &mut rng), None);
    }

    #[test]
    fn only_none_passe_dans_tous_les_contextes() {
        let mut rng = SeededRng::new(1);
        let list = [next(5, 100, OnlyFlags::NONE)];
        for ctx in [OnlyFlags::TASKBAR, OnlyFlags::WINDOW, OnlyFlags::HORIZONTAL, OnlyFlags::VERTICAL] {
            assert_eq!(pick_next(&list, ctx, &mut rng), Some(5));
        }
    }

    #[test]
    fn respecte_grossierement_les_poids() {
        let mut rng = SeededRng::new(12345);
        let list = [
            next(1, 90, OnlyFlags::NONE),
            next(2, 10, OnlyFlags::NONE),
        ];
        let mut ones = 0;
        let total = 2000;
        for _ in 0..total {
            if pick_next(&list, OnlyFlags::NONE, &mut rng) == Some(1) {
                ones += 1;
            }
        }
        // Le poids 90/100 doit dominer largement, sans exiger une précision fine.
        assert!(ones > total * 3 / 4, "l'identifiant 1 est sorti {ones} fois sur {total}");
    }

    #[test]
    fn le_tirage_est_reproductible_a_seed_egale() {
        let list = [
            next(1, 50, OnlyFlags::NONE),
            next(2, 50, OnlyFlags::NONE),
        ];
        let mut a = SeededRng::new(42);
        let mut b = SeededRng::new(42);
        let sa: Vec<_> = (0..50).map(|_| pick_next(&list, OnlyFlags::NONE, &mut a)).collect();
        let sb: Vec<_> = (0..50).map(|_| pick_next(&list, OnlyFlags::NONE, &mut b)).collect();
        assert_eq!(sa, sb);
    }
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p pet-engine transitions`
Expected: FAIL — module `transitions` inexistant.

- [ ] **Step 3: Ajouter la dépendance `pet-expr` si absente**

Vérifier que `crates/pet-engine/Cargo.toml` contient bien `pet-expr = { path = "../pet-expr" }` (ajouté en Task 6).

- [ ] **Step 4: Implémenter**

Au début de `crates/pet-engine/src/transitions.rs` :
```rust
//! Tirage de l'animation suivante et du point d'apparition.
//! Voir `docs/reference/esheep-engine.md` §2.4 et §5.1.

use pet_expr::PetRng;
use pet_format::{NextAnimation, OnlyFlags, Spawn};

/// Choisit la prochaine animation parmi les candidates éligibles au contexte.
///
/// Retourne `None` si aucune n'est éligible : le pet principal respawne, un
/// enfant se ferme.
pub fn pick_next(
    candidates: &[NextAnimation],
    context: OnlyFlags,
    rng: &mut dyn PetRng,
) -> Option<i32> {
    let eligible: Vec<&NextAnimation> =
        candidates.iter().filter(|c| c.only.allows(context)).collect();

    if eligible.is_empty() {
        return None;
    }

    let total: i32 = eligible.iter().map(|c| c.probability.max(0)).sum();
    if total <= 0 {
        return eligible.first().map(|c| c.id);
    }

    let draw = rng.gen_range_i32(0, total);
    let mut cumulative = 0;
    for candidate in &eligible {
        cumulative += candidate.probability.max(0);
        if cumulative >= draw {
            return Some(candidate.id);
        }
    }

    eligible.last().map(|c| c.id)
}

/// Choisit un point d'apparition, pondéré par les probabilités.
pub fn pick_spawn(spawns: &[Spawn], rng: &mut dyn PetRng) -> Option<usize> {
    if spawns.is_empty() {
        return None;
    }

    let total: i32 = spawns.iter().map(|s| s.probability.max(0)).sum();
    if total <= 0 {
        return Some(0);
    }

    let draw = rng.gen_range_i32(0, total);
    let mut cumulative = 0;
    for (index, spawn) in spawns.iter().enumerate() {
        cumulative += spawn.probability.max(0);
        if cumulative >= draw {
            return Some(index);
        }
    }

    Some(spawns.len() - 1)
}
```

- [ ] **Step 5: Déclarer le module**

Dans `crates/pet-engine/src/lib.rs`, ajouter :
```rust
mod transitions;

pub use transitions::{pick_next, pick_spawn};
```

- [ ] **Step 6: Lancer les tests**

Run: `cargo test -p pet-engine`
Expected: PASS — 13 tests au total.

- [ ] **Step 7: Commit**

```bash
cargo clippy --all-targets -- -D warnings && cargo fmt --all
git add crates/pet-engine Cargo.toml CHANGELOG.md
git commit -m "Tirage de l'animation suivante et du spawn

Filtrage par contexte (barre des tâches, fenêtre, horizontal, vertical),
tirage pondéré cumulatif fidèle au moteur d'origine, reproductible à
graine égale. Version 0.7.0."
```

Bump `0.7.0`, entrée `CHANGELOG.md` :
```markdown
## 0.7.0 — 2026-07-20 · « Transitions d'animation »

- Filtrage contextuel des transitions par drapeaux `only`.
- Tirage pondéré cumulatif, identique à l'algorithme d'origine.
- Tirage du point d'apparition, pondéré par les probabilités de spawn.
```

---

### Task 8: Le pet et sa physique

**Files:**
- Create: `crates/pet-engine/src/pet.rs`, `crates/pet-engine/src/geometry.rs`
- Modify: `crates/pet-engine/src/lib.rs`

**Interfaces:**
- Consumes: tout ce qui précède
- Produces:
  - `pub struct Rect { pub x: i32, pub y: i32, pub w: i32, pub h: i32 }` avec `right()`, `bottom()`
  - `pub struct World { pub bounds: Rect, pub area: Rect, pub windows: Vec<Rect> }`
  - `pub struct SpriteDraw { pub frame: i32, pub x: i32, pub y: i32, pub opacity: f64, pub flipped: bool }`
  - `pub enum TickOutcome { Continue, Respawn, Close }`
  - `pub struct Pet { … }` avec :
    - `pub fn new(definition: Arc<PetDefinition>, tile: (i32, i32), world: &World) -> Self`
    - `pub fn position(&self) -> (i32, i32)`, `pub fn set_position(&mut self, x: i32, y: i32)`, `pub fn set_flipped(&mut self, flipped: bool)`, `pub fn set_child(&mut self, is_child: bool)`
    - `pub fn spawn(&mut self, world: &World, rng: &mut dyn PetRng)`
    - `pub fn tick(&mut self, world: &World, rng: &mut dyn PetRng) -> TickOutcome`
    - `pub fn draw(&self) -> SpriteDraw`
    - `pub fn interval_ms(&self) -> i32`
    - `pub fn begin_drag(&mut self, rng: &mut dyn PetRng)`, `pub fn drag_to(&mut self, x: i32, y: i32)`, `pub fn end_drag(&mut self, rng: &mut dyn PetRng)`
    - `pub fn pending_children(&mut self) -> Vec<i32>` — identifiants d'animation dont les enfants restent à créer

**Contexte critique (§4.2) — ordre des opérations dans `tick` :**
1. Si `is_dragging` : position asservie au curseur, **retour immédiat**, aucune physique.
2. Interpoler `interval`, `opacity`, `offset_y`, puis `x` et `y`.
3. Nier `x` si `!is_moving_left`.
4. Détections dans cet ordre : bord gauche (`x < 0`), bord droit (`x > 0`), bas (`y > 0`), haut (`y < 0`).
5. Gravité si l'animation en a.
6. Fin de séquence (`step >= total_steps`) : `flip` éventuel, puis choix de l'animation suivante.
7. `position += (x, y)`.

**Détections de bord (§4.3)** — chaque détection appelle `pick_next(anim.border, contexte, rng)` ; si une animation est trouvée, la position est **clampée** et la vitesse concernée mise à `0` :

| Situation | Condition | Contexte | Clamp |
|---|---|---|---|
| Bord gauche | `x < 0 && pos_x + x < area.x` | `VERTICAL` | `pos_x = area.x` |
| Bord droit | `x > 0 && pos_x + x + tile_w > area.right()` | `VERTICAL` | `pos_x = area.right() - tile_w` |
| Bas | `y > 0 && pos_y + y > area.bottom() - tile_h` | `TASKBAR` | `pos_y = area.bottom() - tile_h` ; `offset_y = 0` |
| Haut | `y < 0 && pos_y + y < area.y` | `HORIZONTAL` | `pos_y = area.y` |

**Gravité (§4.4)** — si l'animation a une liste `gravity` non vide et que le pet n'est pas au sol : tolérance de 3 px (si `pos_y + y + 3 >= sol`, coller au sol), sinon déclencher `pick_next(anim.gravity, …)`.

**La marche sur les fenêtres n'est pas implémentée dans ce plan** : `World::windows` existe et le contexte `WINDOW` est câblé, mais la détection d'atterrissage sur fenêtre arrivera avec le backend GNOME (plan 2). Un `TODO` explicite le note dans le code.

- [ ] **Step 1: Écrire la géométrie**

`crates/pet-engine/src/geometry.rs` :
```rust
//! Rectangles et description du bureau.

/// Rectangle en pixels, origine en haut-gauche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    /// Bord droit, exclu.
    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    /// Bord bas, exclu.
    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }
}

/// Le bureau tel que le voit le moteur.
#[derive(Debug, Clone)]
pub struct World {
    /// Écran complet.
    pub bounds: Rect,
    /// Zone de travail, barres exclues.
    pub area: Rect,
    /// Fenêtres sur lesquelles le pet peut marcher. Vide en mode dégradé.
    pub windows: Vec<Rect>,
}

impl World {
    /// Un bureau simple sans fenêtre, pour les tests et le mode dégradé.
    pub fn simple(width: i32, height: i32) -> Self {
        let bounds = Rect::new(0, 0, width, height);
        Self { bounds, area: bounds, windows: Vec::new() }
    }
}
```

- [ ] **Step 2: Écrire les tests qui échouent**

Dans `crates/pet-engine/src/pet.rs` :
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pet_expr::SeededRng;
    use pet_format::parse_pet;
    use std::sync::Arc;

    /// Un pet minimal : marche vers la droite, rebondit sur les bords.
    const XML: &str = r#"
    <animations>
      <header><author>a</author><title>t</title><petname>p</petname>
        <version>1</version><info>i</info><application>1</application><icon>x</icon></header>
      <image><tilesx>2</tilesx><tilesy>1</tilesy><png>AAAA</png></image>
      <spawns><spawn id="1" probability="100"><x>100</x><y>200</y><next>1</next></spawn></spawns>
      <animations>
        <animation id="1">
          <name>walk</name>
          <start><x>5</x><y>0</y><interval>100</interval></start>
          <sequence repeat="0" repeatfrom="0"><frame>0</frame><frame>1</frame>
            <next probability="100">1</next></sequence>
          <border><next probability="100">2</next></border>
        </animation>
        <animation id="2">
          <name>turn</name>
          <start><x>0</x><y>0</y><interval>100</interval></start>
          <sequence repeat="0" repeatfrom="0"><frame>0</frame><action>flip</action>
            <next probability="100">1</next></sequence>
        </animation>
      </animations>
      <childs/>
    </animations>"#;

    fn make_pet() -> (Pet, World, SeededRng) {
        let def = Arc::new(parse_pet(XML).expect("parsing"));
        let world = World::simple(800, 600);
        let pet = Pet::new(def, (32, 32), &world);
        (pet, world, SeededRng::new(7))
    }

    #[test]
    fn le_spawn_positionne_le_pet() {
        let (mut pet, world, mut rng) = make_pet();
        pet.spawn(&world, &mut rng);
        assert_eq!(pet.position(), (100, 200));
        assert_eq!(pet.draw().frame, 0);
    }

    #[test]
    fn le_pet_avance_a_chaque_tick() {
        let (mut pet, world, mut rng) = make_pet();
        pet.spawn(&world, &mut rng);
        let (x0, _) = pet.position();
        pet.tick(&world, &mut rng);
        let (x1, _) = pet.position();
        assert_eq!(x1 - x0, 5, "le pet doit avancer de 5 px par pas");
    }

    #[test]
    fn le_pet_s_arrete_au_bord_droit() {
        let (mut pet, world, mut rng) = make_pet();
        pet.spawn(&world, &mut rng);
        pet.set_position(790, 200); // proche du bord droit (800 - 32)
        for _ in 0..10 {
            pet.tick(&world, &mut rng);
        }
        let (x, _) = pet.position();
        assert!(x <= world.area.right() - 32, "le pet est sorti à droite : x = {x}");
    }

    #[test]
    fn le_pet_ne_sort_pas_par_la_gauche() {
        let (mut pet, world, mut rng) = make_pet();
        pet.spawn(&world, &mut rng);
        pet.set_flipped(true); // se déplace vers la gauche
        pet.set_position(2, 200);
        for _ in 0..10 {
            pet.tick(&world, &mut rng);
        }
        let (x, _) = pet.position();
        assert!(x >= world.area.x, "le pet est sorti à gauche : x = {x}");
    }

    #[test]
    fn le_drag_suspend_la_physique() {
        let (mut pet, world, mut rng) = make_pet();
        pet.spawn(&world, &mut rng);
        pet.begin_drag(&mut rng);
        pet.drag_to(400, 300);
        pet.tick(&world, &mut rng);
        // La position reste celle du curseur, à l'ancrage près.
        let (x, y) = pet.position();
        assert_eq!(x, 400 - 16); // curseur - largeur/2
        assert_eq!(y, 300 - 2);
    }

    #[test]
    fn la_simulation_est_deterministe_a_graine_egale() {
        let trace = |seed: u64| {
            let def = Arc::new(parse_pet(XML).expect("parsing"));
            let world = World::simple(800, 600);
            let mut pet = Pet::new(def, (32, 32), &world);
            let mut rng = SeededRng::new(seed);
            pet.spawn(&world, &mut rng);
            (0..200)
                .map(|_| {
                    pet.tick(&world, &mut rng);
                    let d = pet.draw();
                    (d.frame, d.x, d.y, d.flipped)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(trace(1234), trace(1234));
    }

    #[test]
    fn le_pet_reste_dans_l_ecran_sur_une_longue_simulation() {
        let (mut pet, world, mut rng) = make_pet();
        pet.spawn(&world, &mut rng);
        for step in 0..2000 {
            pet.tick(&world, &mut rng);
            let (x, y) = pet.position();
            assert!(
                x >= world.area.x - 32 && x <= world.area.right(),
                "sorti horizontalement au pas {step} : x = {x}"
            );
            assert!(
                y >= world.area.y - 32 && y <= world.area.bottom(),
                "sorti verticalement au pas {step} : y = {y}"
            );
        }
    }
}
```

- [ ] **Step 3: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p pet-engine pet::`
Expected: FAIL — `Pet` introuvable.

- [ ] **Step 4: Implémenter le pet**

Au début de `crates/pet-engine/src/pet.rs` :
```rust
//! Le pet : état, physique et enchaînement des animations.
//! Voir `docs/reference/esheep-engine.md` §4.

use crate::anim_state::{interpolate, pick_frame, total_steps};
use crate::geometry::{Rect, World};
use crate::transitions::{pick_next, pick_spawn};
use pet_expr::{EvalContext, PetRng};
use pet_format::{OnlyFlags, PetDefinition};
use std::sync::Arc;

/// Ce que le moteur demande d'afficher pour ce pet, à cet instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteDraw {
    /// Index de tuile dans le spritesheet.
    pub frame: i32,
    pub x: i32,
    pub y: i32,
    pub opacity: f64,
    /// Le sprite doit être affiché en miroir horizontal.
    pub flipped: bool,
}

/// Ce qu'il faut faire du pet après un tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickOutcome {
    /// Le pet continue de vivre.
    Continue,
    /// Aucune animation suivante : le pet principal réapparaît.
    Respawn,
    /// Aucune animation suivante : un enfant disparaît.
    Close,
}

/// Un pet vivant.
pub struct Pet {
    definition: Arc<PetDefinition>,
    /// Dimensions d'une tuile, en pixels.
    tile_w: i32,
    tile_h: i32,
    ctx: EvalContext,

    animation_id: i32,
    step: i32,
    total: i32,

    position_x: i32,
    position_y: i32,
    offset_y: i32,
    opacity: f64,
    interval: i32,
    frame: i32,

    /// Faux quand le pet se déplace vers la droite ; l'axe X est alors nié.
    moving_left: bool,
    dragging: bool,
    drag_x: i32,
    drag_y: i32,

    /// Vrai pour un enfant : il se ferme au lieu de réapparaître.
    is_child: bool,
    /// Animations dont les enfants restent à créer.
    pending_children: Vec<i32>,
    /// Animation courante, expressions déjà évaluées.
    cache: Option<pet_format::Animation>,
}

impl Pet {
    /// Crée un pet à partir de sa définition et de la taille de ses tuiles.
    pub fn new(definition: Arc<PetDefinition>, tile: (i32, i32), world: &World) -> Self {
        let (tile_w, tile_h) = tile;
        let ctx = EvalContext {
            screen_w: world.bounds.w,
            screen_h: world.bounds.h,
            area_w: world.area.w,
            // areaH est le bord bas de la zone de travail (§3.1).
            area_h: world.area.bottom(),
            image_w: tile_w,
            image_h: tile_h,
            image_x: -1,
            image_y: -1,
            rand_spawn: 50,
            scale: 1,
        };

        Self {
            definition,
            tile_w,
            tile_h,
            ctx,
            animation_id: -1,
            step: -1,
            total: 0,
            position_x: 0,
            position_y: 0,
            offset_y: 0,
            opacity: 1.0,
            interval: 100,
            frame: 0,
            moving_left: true,
            dragging: false,
            drag_x: 0,
            drag_y: 0,
            is_child: false,
            pending_children: Vec::new(),
            cache: None,
        }
    }

    /// Marque ce pet comme enfant : il se ferme au lieu de réapparaître.
    pub fn set_child(&mut self, is_child: bool) {
        self.is_child = is_child;
    }

    /// Position courante, coin haut-gauche.
    pub fn position(&self) -> (i32, i32) {
        (self.position_x, self.position_y)
    }

    /// Force la position. Réservé aux tests et au placement des enfants.
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.position_x = x;
        self.position_y = y;
    }

    /// Force l'orientation. Réservé aux tests.
    pub fn set_flipped(&mut self, flipped: bool) {
        self.moving_left = !flipped;
    }

    /// Durée à attendre avant le prochain tick, en millisecondes.
    pub fn interval_ms(&self) -> i32 {
        self.interval.max(1)
    }

    /// Ce qu'il faut afficher maintenant.
    pub fn draw(&self) -> SpriteDraw {
        SpriteDraw {
            frame: self.frame,
            x: self.position_x,
            y: self.position_y + self.offset_y,
            opacity: self.opacity,
            flipped: !self.moving_left,
        }
    }

    /// Récupère et vide la liste des enfants à créer.
    pub fn pending_children(&mut self) -> Vec<i32> {
        std::mem::take(&mut self.pending_children)
    }

    /// Fait apparaître le pet à un point d'apparition tiré au sort.
    pub fn spawn(&mut self, world: &World, rng: &mut dyn PetRng) {
        // randS est figé pour toute la durée de vie du pet (§3.1).
        self.ctx.rand_spawn = rng.gen_range_i32(10, 90);
        self.refresh_context(world);

        let spawns = &self.definition.spawns;
        let (x, y, next) = match pick_spawn(spawns, rng) {
            Some(index) => {
                let mut spawn = spawns[index].clone();
                spawn.x.update(&self.ctx, rng, false);
                spawn.y.update(&self.ctx, rng, false);
                (spawn.x.get(), spawn.y.get(), spawn.next)
            }
            // Spawn de secours quand le XML n'en définit aucun (§5.1).
            None => {
                let first = self.definition.animations.first().map(|a| a.id).unwrap_or(1);
                (0, 0, first)
            }
        };

        // Miroir horizontal de la position d'apparition si le pet est retourné.
        self.position_x = if self.moving_left {
            world.bounds.x + x
        } else {
            world.bounds.x - (x - world.bounds.w) - self.tile_w
        };
        self.position_y = world.bounds.y + y;
        self.offset_y = 0;
        self.opacity = 1.0;

        self.set_animation(next, world, rng);
    }

    /// Démarre une animation. Le premier tick portera le pas 0.
    fn set_animation(&mut self, id: i32, world: &World, rng: &mut dyn PetRng) {
        self.refresh_context(world);

        let Some(animation) = self.definition.animation(id) else {
            return;
        };
        let mut animation = animation.clone();

        // Réévalue les expressions dynamiques au démarrage (§3.4).
        animation.sequence.repeat.update(&self.ctx, rng, false);
        animation.start.x.update(&self.ctx, rng, false);
        animation.start.y.update(&self.ctx, rng, false);
        animation.start.interval.update(&self.ctx, rng, false);
        if let Some(end) = animation.end.as_mut() {
            end.x.update(&self.ctx, rng, false);
            end.y.update(&self.ctx, rng, false);
            end.interval.update(&self.ctx, rng, false);
        }

        self.total = total_steps(&animation.sequence);
        self.animation_id = id;
        self.step = 0;
        self.interval = animation.start.interval.get().max(1);
        self.frame = pick_frame(&animation.sequence, 0);

        // Cette animation crée-t-elle des enfants ?
        if self.definition.childs.iter().any(|c| c.animation_id == id) {
            self.pending_children.push(id);
        }

        self.cache = Some(animation);
    }

    /// Met à jour le contexte d'évaluation avec la géométrie courante.
    fn refresh_context(&mut self, world: &World) {
        self.ctx.screen_w = world.bounds.w;
        self.ctx.screen_h = world.bounds.h;
        self.ctx.area_w = world.area.w;
        self.ctx.area_h = world.area.bottom();
        self.ctx.image_w = self.tile_w;
        self.ctx.image_h = self.tile_h;
    }

    /// Le pet est attrapé à la souris.
    pub fn begin_drag(&mut self, rng: &mut dyn PetRng) {
        self.dragging = true;
        if let Some(id) = self.definition.animation_id_by_name("drag") {
            let world = World::simple(self.ctx.screen_w, self.ctx.screen_h);
            self.set_animation(id, &world, rng);
        }
    }

    /// Le curseur a bougé pendant le glisser.
    pub fn drag_to(&mut self, x: i32, y: i32) {
        self.drag_x = x;
        self.drag_y = y;
    }

    /// Le pet est relâché : il tombe.
    pub fn end_drag(&mut self, rng: &mut dyn PetRng) {
        self.dragging = false;
        if let Some(id) = self.definition.animation_id_by_name("fall") {
            let world = World::simple(self.ctx.screen_w, self.ctx.screen_h);
            self.set_animation(id, &world, rng);
        }
    }

    /// Avance d'un pas. Voir l'ordre des opérations en §4.2.
    pub fn tick(&mut self, world: &World, rng: &mut dyn PetRng) -> TickOutcome {
        let Some(animation) = self.cache.clone() else {
            return TickOutcome::Continue;
        };

        self.frame = pick_frame(&animation.sequence, self.step);

        // 1. Pendant le glisser, la position suit le curseur, sans physique.
        if self.dragging {
            self.position_x = self.drag_x - self.tile_w / 2;
            self.position_y = self.drag_y - 2;
            self.step += 1;
            return TickOutcome::Continue;
        }

        // 2. Interpolation des valeurs du pas courant.
        let values = interpolate(&animation, self.step, self.total);
        self.interval = values.interval.max(1);
        self.opacity = values.opacity;
        self.offset_y = values.offset_y;

        let mut dx = values.x;
        let mut dy = values.y;

        // 3. L'axe X est nié quand le pet est retourné.
        if !self.moving_left {
            dx = -dx;
        }

        // 4. Détections de bord, dans l'ordre du moteur d'origine.
        let mut next_animation: Option<i32> = None;

        if dx < 0 && self.position_x + dx < world.area.x {
            if let Some(id) = pick_next(&animation.border, OnlyFlags::VERTICAL, rng) {
                self.position_x = world.area.x;
                dx = 0;
                next_animation = Some(id);
            }
        } else if dx > 0 && self.position_x + dx + self.tile_w > world.area.right() {
            if let Some(id) = pick_next(&animation.border, OnlyFlags::VERTICAL, rng) {
                self.position_x = world.area.right() - self.tile_w;
                dx = 0;
                next_animation = Some(id);
            }
        }

        let floor = world.area.bottom() - self.tile_h;
        if next_animation.is_none() {
            if dy > 0 && self.position_y + dy > floor {
                if let Some(id) = pick_next(&animation.border, OnlyFlags::TASKBAR, rng) {
                    self.position_y = floor;
                    self.offset_y = 0;
                    dy = 0;
                    next_animation = Some(id);
                }
            } else if dy < 0 && self.position_y + dy < world.area.y {
                if let Some(id) = pick_next(&animation.border, OnlyFlags::HORIZONTAL, rng) {
                    self.position_y = world.area.y;
                    dy = 0;
                    next_animation = Some(id);
                }
            }
        }

        // TODO(plan 2) : atterrissage sur les fenêtres de `world.windows`,
        // avec le contexte OnlyFlags::WINDOW.

        // 5. Gravité : le pet tombe s'il n'est pas au sol (§4.4).
        if next_animation.is_none() && animation.has_gravity() && self.position_y + dy < floor {
            // Tolérance de 3 px : on colle au sol plutôt que de déclencher une chute.
            if self.position_y + dy + 3 >= floor {
                dy = floor - self.position_y;
            } else if let Some(id) = pick_next(&animation.gravity, OnlyFlags::NONE, rng) {
                next_animation = Some(id);
            }
        }

        // 6. Fin de séquence.
        let mut outcome = TickOutcome::Continue;
        if next_animation.is_none() && self.step >= self.total {
            if animation.sequence.action.as_deref() == Some("flip") {
                self.moving_left = !self.moving_left;
            }
            match pick_next(&animation.sequence.next, OnlyFlags::NONE, rng) {
                Some(id) => next_animation = Some(id),
                None => {
                    outcome = if self.is_child {
                        TickOutcome::Close
                    } else {
                        TickOutcome::Respawn
                    };
                }
            }
        }

        // 7. Application du déplacement.
        self.position_x += dx;
        self.position_y += dy;
        self.step += 1;

        if let Some(id) = next_animation {
            self.set_animation(id, world, rng);
            // Le moteur d'origine affiche la première frame quasi immédiatement.
            self.interval = 1;
        }

        outcome
    }
}
```

- [ ] **Step 5: Déclarer les modules**

Dans `crates/pet-engine/src/lib.rs`, ajouter :
```rust
mod geometry;
mod pet;

pub use geometry::{Rect, World};
pub use pet::{Pet, SpriteDraw, TickOutcome};
```

- [ ] **Step 6: Lancer les tests**

Run: `cargo test -p pet-engine`
Expected: PASS — 20 tests. **Le test `le_pet_reste_dans_l_ecran_sur_une_longue_simulation` est le garde-fou principal** : s'il échoue, la logique de bord est fausse, ne pas le contourner en élargissant les bornes.

- [ ] **Step 7: Commit**

```bash
cargo clippy --all-targets -- -D warnings && cargo fmt --all
git add crates/pet-engine Cargo.toml CHANGELOG.md
git commit -m "Le pet et sa physique

Machine à états complète : spawn, enchaînement d'animations, détection
des bords d'écran, gravité avec tolérance de 3 px, glisser-déposer,
retournement. Simulation déterministe à graine égale. Version 0.8.0."
```

Bump `0.8.0`, entrée `CHANGELOG.md` :
```markdown
## 0.8.0 — 2026-07-20 · « Physique du pet »

- Apparition pondérée, avec miroir de la position si le pet est retourné.
- Détection des quatre bords de la zone de travail, avec clampage.
- Gravité et tolérance de 3 px avant déclenchement d'une chute.
- Glisser-déposer suspendant la physique, animations `drag` et `fall`.
- Retournement par l'action `flip`, appliqué au rendu et non aux pixels.
- Simulation reproductible à graine égale.
```

---

### Task 9: Simulateur headless

**Files:**
- Create: `crates/petsim/Cargo.toml`, `crates/petsim/src/main.rs`
- Create: `crates/petsim/tests/snapshots.rs`
- Modify: `Cargo.toml` (membre), `README.md`

**Interfaces:**
- Consumes: tous les crates précédents
- Produces: le binaire `petsim`, qui charge un `animations.xml`, simule N ticks et écrit la trace sur la sortie standard.

Le simulateur est le **backend `null`** de la spec : il prouve que le moteur tourne sans GNOME, sans Wayland et sans écran.

- [ ] **Step 1: Créer le crate**

`crates/petsim/Cargo.toml` :
```toml
[package]
name = "petsim"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
pet-expr = { path = "../pet-expr" }
pet-format = { path = "../pet-format" }
pet-engine = { path = "../pet-engine" }
clap.workspace = true

[dev-dependencies]
insta.workspace = true
```

Dans le `Cargo.toml` du workspace :
```toml
members = ["crates/pet-expr", "crates/pet-format", "crates/pet-engine", "crates/petsim"]
```

- [ ] **Step 2: Écrire le simulateur**

`crates/petsim/src/main.rs` :
```rust
//! Simulateur headless : fait vivre un pet sans écran et dumpe sa trace.
//!
//! Sert de backend « null » et d'outil de non-régression : la trace est
//! reproductible à graine égale.

use clap::Parser;
use pet_engine::{Pet, TickOutcome, World};
use pet_expr::SeededRng;
use pet_format::{decode_sheet, parse_pet};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "petsim", about = "Simule un pet eSheep sans affichage")]
struct Args {
    /// Chemin du fichier animations.xml
    xml: String,
    /// Nombre de pas à simuler
    #[arg(long, default_value_t = 100)]
    ticks: u32,
    /// Graine du générateur aléatoire
    #[arg(long, default_value_t = 42)]
    seed: u64,
    /// Largeur de l'écran simulé
    #[arg(long, default_value_t = 1920)]
    width: i32,
    /// Hauteur de l'écran simulé
    #[arg(long, default_value_t = 1080)]
    height: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let xml = std::fs::read_to_string(&args.xml)?;
    let definition = Arc::new(parse_pet(&xml)?);
    let sheet = decode_sheet(&definition.image)?;

    println!(
        "# pet={} tuiles={}x{} taille={}x{} animations={}",
        definition.header.petname,
        sheet.tiles_x,
        sheet.tiles_y,
        sheet.tile_w,
        sheet.tile_h,
        definition.animations.len()
    );

    let world = World::simple(args.width, args.height);
    let mut rng = SeededRng::new(args.seed);
    let mut pet = Pet::new(definition, (sheet.tile_w as i32, sheet.tile_h as i32), &world);
    pet.spawn(&world, &mut rng);

    let mut respawns = 0;
    for tick in 0..args.ticks {
        let outcome = pet.tick(&world, &mut rng);
        let draw = pet.draw();
        println!(
            "{tick:04} frame={:3} x={:5} y={:5} opacite={:.2} miroir={} attente={}ms",
            draw.frame,
            draw.x,
            draw.y,
            draw.opacity,
            draw.flipped,
            pet.interval_ms()
        );
        if outcome == TickOutcome::Respawn {
            respawns += 1;
            pet.spawn(&world, &mut rng);
        }
    }

    println!("# réapparitions={respawns}");
    Ok(())
}
```

- [ ] **Step 3: Lancer le simulateur sur un vrai pet**

Run: `cargo run -p petsim -- ~/Dev/desktopPet/Pets/neko/animations.xml --ticks 50`
Expected: 50 lignes de trace, des positions qui évoluent, aucune panique, `x` et `y` restant dans `[0, 1920]` et `[0, 1080]`.

- [ ] **Step 4: Écrire le test d'instantané**

`crates/petsim/tests/snapshots.rs` :
```rust
//! La trace d'un pet doit rester identique d'une exécution à l'autre, et
//! d'une version à l'autre tant que le moteur n'évolue pas volontairement.

use pet_engine::{Pet, TickOutcome, World};
use pet_expr::SeededRng;
use pet_format::{decode_sheet, parse_pet};
use std::sync::Arc;

/// Simule un pet et retourne sa trace sous forme de texte.
fn trace(name: &str, ticks: u32, seed: u64) -> String {
    let path = format!(
        "{}/../pet-format/tests/fixtures/{name}.xml",
        env!("CARGO_MANIFEST_DIR")
    );
    let xml = std::fs::read_to_string(&path).expect("fixture lisible");
    let definition = Arc::new(parse_pet(&xml).expect("parsing"));
    let sheet = decode_sheet(&definition.image).expect("spritesheet");

    let world = World::simple(1920, 1080);
    let mut rng = SeededRng::new(seed);
    let mut pet = Pet::new(definition, (sheet.tile_w as i32, sheet.tile_h as i32), &world);
    pet.spawn(&world, &mut rng);

    let mut out = String::new();
    for tick in 0..ticks {
        if pet.tick(&world, &mut rng) == TickOutcome::Respawn {
            pet.spawn(&world, &mut rng);
        }
        let d = pet.draw();
        out.push_str(&format!(
            "{tick:04} frame={} x={} y={} miroir={}\n",
            d.frame, d.x, d.y, d.flipped
        ));
    }
    out
}

#[test]
fn trace_de_neko() {
    insta::assert_snapshot!(trace("neko", 300, 42));
}

#[test]
fn trace_d_esheep() {
    insta::assert_snapshot!(trace("esheep64", 300, 42));
}

#[test]
fn trace_de_pingus() {
    insta::assert_snapshot!(trace("pingus", 300, 42));
}

/// Aucun pet ne doit sortir de l'écran, quelle que soit la graine.
#[test]
fn aucun_pet_ne_sort_de_l_ecran() {
    for name in ["neko", "esheep64", "pingus"] {
        for seed in [1, 2, 3, 99, 12345] {
            let path = format!(
                "{}/../pet-format/tests/fixtures/{name}.xml",
                env!("CARGO_MANIFEST_DIR")
            );
            let xml = std::fs::read_to_string(&path).expect("fixture lisible");
            let definition = Arc::new(parse_pet(&xml).expect("parsing"));
            let sheet = decode_sheet(&definition.image).expect("spritesheet");

            let world = World::simple(1920, 1080);
            let mut rng = SeededRng::new(seed);
            let tile = (sheet.tile_w as i32, sheet.tile_h as i32);
            let mut pet = Pet::new(definition, tile, &world);
            pet.spawn(&world, &mut rng);

            for step in 0..1000 {
                if pet.tick(&world, &mut rng) == TickOutcome::Respawn {
                    pet.spawn(&world, &mut rng);
                }
                let (x, y) = pet.position();
                assert!(
                    x >= -tile.0 && x <= world.area.right() + tile.0,
                    "{name} graine {seed} pas {step} : x = {x}"
                );
                assert!(
                    y >= -tile.1 && y <= world.area.bottom() + tile.1,
                    "{name} graine {seed} pas {step} : y = {y}"
                );
            }
        }
    }
}
```

- [ ] **Step 5: Générer et relire les instantanés**

Run: `cargo insta test --accept -p petsim`

Puis **relire manuellement** les fichiers générés dans `crates/petsim/tests/snapshots/` : vérifier que les positions varient de façon plausible, que le pet ne reste pas figé sur une seule frame, et qu'il ne téléporte pas. Un instantané accepté sans relecture ne prouve rien.

Run: `cargo test -p petsim`
Expected: PASS — 4 tests.

- [ ] **Step 6: Écrire le README**

`README.md` :
```markdown
# RustyPet

Portage en Rust de [DesktopPet / eSheep](https://github.com/Adrianotiger/desktopPet),
pour GNOME Shell sous Wayland.

Les animaux de bureau sont décrits par les fichiers `animations.xml` du projet
d'origine : spritesheet encodé, machine à états d'animation, physique. Les 20+
pets existants fonctionnent sans modification.

## État

Le cœur du moteur est fonctionnel et testable sans écran. L'affichage sous
GNOME fait l'objet d'un second chantier.

| Crate | Rôle |
|---|---|
| `pet-expr` | Évaluateur des expressions `x`, `y`, `interval`, `repeat` |
| `pet-format` | Lecture des `animations.xml` et décodage des spritesheets |
| `pet-engine` | Machine à états : animations, transitions, physique |
| `petsim` | Simulateur headless, sans affichage |

## Essayer

```bash
cargo run -p petsim -- chemin/vers/animations.xml --ticks 100
```

## Développement

```bash
cargo test --all
cargo clippy --all-targets -- -D warnings
RUSTYPET_CORPUS=~/Dev/desktopPet/Pets cargo test -p pet-format
```

## Documentation

- Conception : `docs/superpowers/specs/2026-07-20-rustypet-design.md`
- Référence du moteur d'origine : `docs/reference/esheep-engine.md`

## Licence

MIT pour le code. Les pets restent la propriété de leurs auteurs respectifs.
```

- [ ] **Step 7: Vérification finale**

Run: `cargo test --all && cargo clippy --all-targets -- -D warnings && cargo fmt --all --check`
Expected: tout passe, aucun warning.

Run: `RUSTYPET_CORPUS=$HOME/Dev/desktopPet/Pets cargo test -p pet-format`
Expected: PASS sur le corpus complet.

- [ ] **Step 8: Commit**

```bash
cargo fmt --all
git add crates/petsim Cargo.toml CHANGELOG.md README.md
git commit -m "Simulateur headless et tests d'instantané

Binaire petsim faisant vivre un pet sans écran, servant de backend nul.
Traces figées en instantanés pour détecter toute régression du moteur.
Version 0.9.0."
```

Bump `0.9.0`, entrée `CHANGELOG.md` :
```markdown
## 0.9.0 — 2026-07-20 · « Simulateur headless »

- Binaire `petsim` : simulation d'un pet sans affichage ni GNOME.
- Tests d'instantané figeant la trace de trois pets de référence.
- Garde-fou : aucun pet ne sort de l'écran, toutes graines confondues.
- README décrivant l'état du portage et la façon de l'essayer.
```

---

## Fin du plan 1

À ce stade, le moteur est complet et prouvé sans écran : les pets du dépôt
d'origine se chargent, s'animent, rebondissent et tombent, de façon
reproductible.

Le **plan 2** couvrira l'intégration GNOME : extension Shell et acteurs
Clutter, service D-Bus, géométrie des fenêtres et marche sur les barres de
titre, audio, menu du panneau, téléchargement de pets, multi-pets et enfants.
