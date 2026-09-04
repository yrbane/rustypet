# Pluie, amour, troupeau, drag et sons (design)

Date : 2026-08-04 · Branche : feat/rendu-gnome · Versions cibles : 0.18 → 0.21

## Demande

1. Un petit nuage au-dessus du mouton : il se fait tremper, ou il sort un
   parapluie. 2. Il croise une moutonne et tombe amoureux. 3. Multi-pets :
   des agneaux. 4. Glisser-déposer à la souris. 5. Sons du mouton, discrets.

## Constat

- `Pet` (pet-engine) possède déjà le glisser (`begin_drag`/`drag_to`/
  `end_drag`, position suivie sans physique, `drag` puis `fall`) et la
  plomberie enfants (`pending_children`, `set_child`, `TickOutcome::Close`,
  `spawn` d'un enfant à fermer au lieu de réapparaître).
- `pet-format` parse déjà `<childs>` (x, y, next par animation déclencheuse)
  et `<sounds>` (WAV base64, probabilité, boucles). esheep64 a 3 childs ;
  les moutons colorés du corpus (blue_sheep…) embarquent des bêlements WAV.
- petd est mono-acteur : `advance() -> PetFrame`, signal `PetState(iiubu)`,
  extension à acteur unique non réactif.

## Plan par versions

### 0.18.0 — Pluie : trempé ou parapluie (data pur)

Six tuiles inédites (205-210) : nuage gris + gouttes, mouton trempé (laine
assombrie, gouttes, flaque), parapluie rouge ouvert sous la pluie.
Animations `rain` (109, nuage qui s'installe) → 50/50 `soaked` (110) ou
`umbrella` (111), retour à `walk`. Accroche depuis `walk` (probabilité 3).
Tests : mêmes invariants que les gags 0.17 (présence, atteignabilité,
frames couvertes). Aucun changement moteur.

### 0.19.0 — Multi-pets : la moutonne et les agneaux

- pet-engine : `Pet::spawn_child(&Child, world, rng)` — évalue x/y,
  marque enfant, démarre `child.next`.
- petd `Engine` : vecteur d'acteurs à cadence propre (échéancier en ms :
  `advance(elapsed) -> Vec<PetFrame>`, `interval_ms()` = min des échéances) ;
  les `pending_children` du pet principal créent des enfants (plafond : 6) ;
  `Close`/hors-écran retire l'enfant.
- D-Bus : `PetState` devient `a(iiubu)` (tableau d'acteurs, le principal en
  tête). Extension : un St.Widget par acteur, créés/détruits selon le
  tableau reçu.
- Contenu : `love` (112, cœurs au-dessus du mouton) déclenche l'enfant
  moutonne (tuiles : laine claire teintée rose, cils, nœud) qui traverse
  l'écran vers lui ; puis `family_walk` (113) déclenche deux agneaux
  (tuiles réduites ~60 %) qui trottinent derrière. Accroche depuis `walk`
  (probabilité 2).

### 0.20.0 — Glisser-déposer

Méthodes D-Bus `BeginDrag`, `DragTo(x,y)`, `EndDrag` → `Pet` (API déjà
prête). Extension : acteur principal `reactive: true`, `button-press` →
BeginDrag + grab, `motion` → DragTo (throttle au tick), `button-release` →
EndDrag. Le pet joue `drag` (rotation d'origine) puis retombe en `fall`.
Tests moteur : la position suit le curseur, la chute reprend au relâcher.

### 0.21.0 — Sons discrets

- pet-engine : au démarrage d'une animation, tirage des `<sound>` associés
  (probabilité) → `take_sound() -> Option<usize>`.
- petd : écrit chaque WAV en cache (`sound-<i>.wav`) au chargement ; signal
  `PetSound(s)` avec le chemin.
- Extension : lecture via `global.display.get_sound_player()
  .play_from_file(...)` (API GNOME native, respecte le mixeur).
- Contenu : bêlement emprunté à blue_sheep, **amplitude réduite à ~20 %**
  à la génération (module `wave` Python) — discrétion demandée. Attaché à
  `love` et à l'atterrissage, probabilités faibles.

## Approches rejetées

- Tout coder dans le moteur (gags en dur) : déjà rejeté en 0.17.
- Un thread/timer par acteur dans petd : l'échéancier min-délai dans la
  boucle unique est plus simple et déterministe (petsim reproductible).
- Lecture audio dans petd (rodio/pulse) : l'extension a déjà l'API GNOME
  idoine et le contexte session ; le démon reste sans dépendance audio.

## Publication

Une version par étape (commit = bump + CHANGELOG), CI verte par SHA, tags,
releases avec artefacts. Vitrines mises à jour après la dernière version.
