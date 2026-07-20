# Changelog

## 0.6.0 — 2026-07-20 · « État d'animation »

- Crate `pet-engine` : calcul du nombre de pas, répétitions partielles.
- Choix de la frame affichée, y compris pendant les boucles.
- Interpolation fidèle au moteur d'origine, avec ses deux dénominateurs.

## 0.5.0 — 2026-07-20 · « Décodage des spritesheets »

- Décodage base64 avec padding automatique des flux non padés.
- Couleur clé convertie en alpha 0, alpha existant préservé.
- Découpe des tuiles en ligne d'abord, conforme aux index `<frame>`.
- La limite Win32 de 255 px et le facteur d'échelle entier ne sont pas portés.

## 0.4.1 — 2026-07-20 · « Correctifs de revue »

- Marquage du test `parse_le_corpus_complet` comme ignoré (nécessite `RUSTYPET_CORPUS`).
- Renommage du test `end_absent_vaut_start` en `end_absent_reste_absent` pour clarifier son intention.
- Création du README.md avec section Développement.

## 0.4.0 — 2026-07-20 · « Lecture des animations.xml »

- Crate `pet-format` : modèle de données complet du format eSheep.
- Parsing tolérant à l'ordre libre des sous-éléments et aux champs absents.
- Drapeaux de contexte `only`, `horizontal+` traité comme `horizontal`.
- Les 22 pets du dépôt amont parsent sans erreur.

## 0.3.0 — 2026-07-20 · « Substitution des jetons »

- Contexte d'évaluation portant les 11 jetons du moteur d'origine.
- RNG injecté via le trait `PetRng`, implémentation seedée reproductible.
- `random` uniforme sur toute l'expression, `randS` figé par chargement.
- Miroir horizontal du placement des enfants sous parent retourné.
- Classification `is_dynamic` / `is_screen` des valeurs.

## 0.2.2 — 2026-07-20 · « Correctif de revue »

- Suppression du `.expect()` en code de bibliothèque : traitement explicite
  du cas `None` avec `match` dans `eval_arithmetic`.
- Correction de la coquille « inatendus » → « inattendus ».

## 0.2.1 — 2026-07-20 · « Correctifs de revue »

- Nouvelle variante d'erreur `UnexpectedToken` pour distinguer les jetons
  inattendu en milieu d'expression (ex. `*5`, `)`) du vrai cas
  d'expression incomplète (ex. `2+`).
- `eval_arithmetic` signale les jetons excédentaires comme `UnexpectedToken`
  au lieu de `UnbalancedParen`.
- `parse_atom` renvoie `UnexpectedToken` pour les jetons inattendus au lieu
  de la vague `UnexpectedEnd`.
- Tests ajoutés : associativité gauche des opérateurs non commutatifs
  (soustraction, division, modulo) et les deux nouveaux cas d'erreur.

## 0.2.0 — 2026-07-20 · « Évaluateur arithmétique »

- Lexer et parseur à descente récursive pour les expressions des animations.
- Opérateurs `+ - * / %`, parenthèses, signe unaire, priorité standard.
- Erreurs typées : caractère inattendu, expression incomplète, parenthèses
  déséquilibrées, division par zéro.

## 0.1.0 — 2026-07-20 · « Squelette du workspace »

- Mise en place du workspace Cargo `rustypet` (edition 2024, Rust 1.97).
- Crate `pet-expr` créé, vide.
- Intégration continue GitHub Actions : `fmt`, `clippy`, `test`.
