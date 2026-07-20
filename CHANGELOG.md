# Changelog

## 0.9.1 — 2026-07-20 · « Invariants physiques »

- Remplacement du test `aucun_pet_ne_sort_de_l_ecran` (tolérance de 50 000 px
  vidée de son sens) par `les_invariants_physiques_tiennent` : opacité dans
  `[0.0, 1.0]` et frame toujours un index valide du spritesheet décodé,
  vérifiés sur 1000 pas, pour `neko`, `esheep64` et `pingus`, sur les graines
  1, 2, 3, 99 et 12345.
- Le confinement à l'écran n'était pas un invariant réel du moteur (cf.
  `docs/reference/esheep-engine.md` §4.7, sortie d'écran documentée de
  `run_catchb`) : la nouvelle garde ne dépend plus d'un plafond arbitraire.

## 0.9.0 — 2026-07-20 · « Simulateur headless »

- Binaire `petsim` : simulation d'un pet sans affichage ni GNOME.
- Tests d'instantané figeant la trace de trois pets de référence.
- Garde-fou : aucun pet ne dérive de façon absurde, toutes graines confondues
  (la sortie d'écran documentée en §4.7 reste autorisée).
- README décrivant l'état du portage et la façon de l'essayer.

## 0.8.1 — 2026-07-20 · « Durcissements de revue »

- Test anti-sortie d'écran : bornes ressserrées pour refléter le clampage
  réel de la physique, sans tolérance aux marges.
- Accès non paniquant dans le spawn : remplacement de l'indexation directe
  par `.get()` pour éliminer tout risque de panique en bibliothèque.

## 0.8.0 — 2026-07-20 · « Physique du pet »

- Apparition pondérée, avec miroir de la position si le pet est retourné.
- Détection des quatre bords de la zone de travail, avec clampage.
- Gravité et tolérance de 3 px avant déclenchement d'une chute.
- Glisser-déposer suspendant la physique, animations `drag` et `fall`.
- Retournement par l'action `flip`, appliqué au rendu et non aux pixels.
- Simulation reproductible à graine égale.

## 0.7.0 — 2026-07-20 · « Transitions d'animation »

- Filtrage contextuel des transitions par drapeaux `only`.
- Tirage pondéré cumulatif, identique à l'algorithme d'origine.
- Tirage du point d'apparition, pondéré par les probabilités de spawn.

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
