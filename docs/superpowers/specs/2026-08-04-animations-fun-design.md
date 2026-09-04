# RustySheep — animations fun (design)

Date : 2026-08-04 · Branche : feat/rendu-gnome · Version cible : 0.17.0

## Demande

Donner au pet « pleins de nouvelles actions fun » : danser, fumer un joint,
voler en Superman, respirer une fleur, faire une petite crotte, se la péter
avec des lunettes de soleil, une fleur pousse et il la mange, arriver en
parachute, partir en fusée, dormir, brouter, être sous acide.

## Constat

Le moteur est **entièrement data-driven** : `pet-format` parse start/end
(vitesses, intervalle, opacité — expressions `pet-expr` avec `random`,
`screenW`, `areaH`, `imageH`…), `sequence` (repeat, action flip, `next`
probabilistes avec `only`), `border`, `gravity` et `spawns`. Le mouton
esheep64 (sheet 16×11 tuiles de 40×40, transparence Magenta) possède déjà
`sleep1-3` (dormir), `eat` (brouter), `flower` (respirer une fleur) et
`pissa/pissb`. **Aucune modification du moteur n'est nécessaire.**

## Approches envisagées

- **A (retenue)** : un nouveau pet « RustySheep » — XML esheep64 étendu par
  un script générateur committé (`tools/make_rustysheep.py`, PIL) qui
  compose de nouvelles tuiles 40×40 (accessoires pixel-art par-dessus les
  tuiles du mouton), agrandit la sheet (lignes supplémentaires), ajoute les
  animations id ≥ 100 et les branche par `next` probabilistes depuis `walk`
  + un spawn parachute. Avantages : respecte le format d'origine, réversible
  (le mouton pur reste dispo), testable par parsing + petsim.
- **B (rejetée)** : coder les gags en dur dans le moteur Rust — viole le
  format data-driven eSheep et ne profite à aucun autre pet.
- **C (rejetée)** : patcher chaque pet du corpus — hors périmètre, YAGNI.

## Nouvelles animations (id ≥ 100)

| id | nom | contenu |
|----|-----|---------|
| 100 | dance | pas chassés flip gauche/droite + notes de musique |
| 101 | smoke | assis, joint, volutes de fumée montantes |
| 102 | superman | cape rouge, décollage en diagonale, sortie d'écran haut → respawn |
| 103 | poop | accroupi, petite crotte, air soulagé |
| 104 | sunglasses | lunettes « deal with it » qui descendent, pose swag |
| 105 | flower_grow | une fleur pousse (3 stades) puis le mouton la mange |
| 106 | parachute | descente lente suspendu à un parachute, balancement |
| 107 | rocket | fusée sous le mouton, allumage, sortie d'écran haut → respawn |
| 108 | acid | teintes psychédéliques cyclées, yeux spirale, marche titubante (`random`) |

Branchement : `next` supplémentaires dans la `sequence` de `walk` (id 1)
avec probabilités faibles (2-6) pour garder un mouton crédible ; `dormir`,
`brouter`, `fleur` passent par les chaînes existantes. Nouveau
`<spawn probability="15">` : arrivée en parachute depuis le haut. `rocket`
et `superman` finissent hors écran : le respawn existant reprend la main.

## Livrables

- `tools/make_rustysheep.py` : générateur (PIL) — tuiles composées + XML.
- `assets/rustysheep/animations.xml` : pet généré, committé (base64).
- Tests (TDD) : `crates/pet-format/tests/rustysheep.rs` — le XML se parse,
  les 9 animations existent, chacune est atteignable depuis `walk` ou un
  spawn ; trace petsim longue où les nouveaux ids apparaissent.
- Publication 0.17.0 : CHANGELOG, README (section RustySheep), CI, tag,
  release, vitrines.

## Crédits

Sprites de base : eSheep 64bit (Adriano Petrucci, images LiL_Stenly),
projet desktopPet — attribution conservée dans le header du XML.
