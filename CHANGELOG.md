# Changelog

## 0.2.0 — 2026-07-20 · « Évaluateur arithmétique »

- Lexer et parseur à descente récursive pour les expressions des animations.
- Opérateurs `+ - * / %`, parenthèses, signe unaire, priorité standard.
- Erreurs typées : caractère inattendu, expression incomplète, parenthèses
  déséquilibrées, division par zéro.

## 0.1.0 — 2026-07-20 · « Squelette du workspace »

- Mise en place du workspace Cargo `rustypet` (edition 2024, Rust 1.97).
- Crate `pet-expr` créé, vide.
- Intégration continue GitHub Actions : `fmt`, `clippy`, `test`.
