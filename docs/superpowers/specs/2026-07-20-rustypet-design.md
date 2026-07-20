# RustyPet — portage de DesktopPet/eSheep en Rust pour GNOME/Wayland

Date : 2026-07-20
Statut : design validé, prêt pour le plan d'implémentation

## 1. Objectif

Porter le moteur d'animaux de bureau **DesktopPet / eSheep** (C#/.NET WinForms,
~11 500 lignes, Windows uniquement) vers un programme Rust natif tournant sur
Arch Linux avec GNOME Shell 50 sous Wayland.

Le portage est **complet** : le moteur d'animation d'origine est reproduit dans
son intégralité, y compris la marche sur les barres de titre des fenêtres, les
sons, les apparitions (spawns), les animaux enfants (childs) et le
multi-instances.

La compatibilité avec le format `animations.xml` d'origine est une contrainte
forte : les 20+ pets existants du dépôt amont doivent fonctionner sans
modification.

La référence technique exhaustive du moteur d'origine (schéma XML, sémantique
des animations, physique, décodage des spritesheets) est en annexe :
[`docs/reference/esheep-engine.md`](../../reference/esheep-engine.md).

## 2. Contrainte structurante : Wayland

Sous GNOME/Wayland, aucune application ne peut :

- se positionner elle-même au pixel près à l'écran ;
- se maintenir de façon fiable au-dessus des autres fenêtres ;
- connaître la géométrie des fenêtres des autres applications.

Ces trois capacités sont pourtant le fondement d'un animal de bureau. GNOME ne
supporte pas `wlr-layer-shell`, qui les fournirait sur les compositeurs
wlroots.

**Conséquence retenue** : le rendu ne se fait pas dans une fenêtre applicative
mais **à l'intérieur du compositeur**, via une extension GNOME Shell qui dessine
les animaux comme acteurs Clutter. C'est la seule voie donnant un résultat
fidèle et, accessoirement, la seule qui rend possible la marche sur les
fenêtres.

Le repli par XWayland (fenêtre X11 `override-redirect`) a été écarté : il ne
verrait presque aucune fenêtre sous GNOME 50, où les clients sont Wayland
natifs.

## 3. Architecture

```
rustypet/                      (workspace Cargo)
├── crates/
│   ├── pet-format/    parse animations.xml → structures typées
│   │                  + décodage spritesheet (base64 → RGBA, color-key)
│   ├── pet-expr/      évaluateur des expressions x/y
│   ├── pet-engine/    machine à états : animations, séquences, branches
│   │                  probabilistes, gravité, bords, spawns, enfants
│   ├── pet-backend/   trait DesktopBackend → impls gnome / x11 / null
│   └── petd/          binaire : boucle temps, audio, D-Bus, téléchargement
└── gnome-extension/   extension GNOME Shell (JS) : acteurs Clutter,
                       géométrie des fenêtres, clics, menu du panneau
```

Le démon Rust est la source de vérité : il possède l'état de chaque animal et
calcule les positions. L'extension est un afficheur passif doublé d'un capteur
d'événements, sans logique métier.

### 3.1 `pet-format`

Parse `animations.xml` vers des structures typées, et décode le spritesheet.

Points sensibles relevés dans l'analyse du moteur d'origine :

- le base64 des images est fréquemment non padé → padding à rajouter avant
  décodage ;
- la transparence est une **couleur-clé** (magenta par défaut), à convertir
  explicitement en alpha 0 ;
- la découpe des tuiles se fait **en ligne d'abord** : `index = row * tilesx + col`.

Aucune dépendance système. Entièrement testable.

### 3.2 `pet-expr`

Les attributs `x` et `y` des animations ne sont pas des nombres mais des
**expressions arithmétiques**, évaluées côté C# par `DataTable.Compute` après
substitution textuelle de 11 jetons : `screenW`, `screenH`, `areaW`, `areaH`,
`imageW`, `imageH`, `imageX`, `imageY`, `random`, `randS`, `scale`.

Ce crate fournit un mini-évaluateur reproduisant cette sémantique. Pièges
documentés à respecter :

- `areaH` vaut `WorkingArea.Height + WorkingArea.Y`, pas la seule hauteur ;
- `random` est réévalué **à chaque évaluation** ;
- `randS` est figé **au chargement du XML** et reste constant ensuite.

Le générateur aléatoire est **injecté** (trait), ce qui rend l'évaluation
déterministe en test.

### 3.3 `pet-engine`

Machine à états pure : entrées = temps écoulé + géométrie (écran, fenêtres,
pointeur), sortie = liste de sprites à afficher. Aucune entrée/sortie.

Sémantique à reproduire :

- `x`/`y` sont des **vitesses par frame**, sauf dans `<spawn>` et `<child>` où
  ce sont des **positions absolues** ;
- l'interpolation start→end utilise **deux dénominateurs différents** :
  `total_steps` pour `interval`, `opacity` et `offsety` ; `total_steps - 1`
  pour `x` et `y` ;
- les séquences enchaînent par branches probabilistes (`next` pondérés),
  avec `repeat` et `repeatfrom` ;
- gravité, détection des bords (`border`), comportement au glisser-déposer et
  chute au relâchement.

**Pas de framerate fixe** : le moteur d'origine ré-arme un timer mono-coup dont
l'intervalle est recalculé à chaque tick. Le portage planifie sur des
**instants absolus** pour éviter la dérive cumulative.

### 3.4 `pet-backend`

```rust
trait DesktopBackend {
    fn screens(&self) -> Vec<Rect>;
    fn windows(&self) -> Vec<Rect>;
    fn present(&mut self, frame: &[SpriteDraw]);
    fn poll_events(&mut self) -> Vec<PointerEvent>;
}
```

Trois implémentations :

- **gnome** — dialogue D-Bus avec l'extension. Implémentation de référence,
  livrée en 0.1.0.
- **null** — sans affichage, pour les tests headless en CI. Livrée en 0.1.0.
- **x11** — repli pour les sessions X11. **Non livrée en 0.1.0** : le trait
  existe pour garantir que le moteur ne dépend d'aucune API GNOME, mais
  l'implémentation viendra plus tard si le besoin se présente.

### 3.5 `petd`

Binaire : chargement des pets, boucle de temps, audio via `rodio` (remplace
NAudio), service D-Bus, téléchargement de pets depuis `Pets/pets.json` du dépôt
amont, cache disque.

Service utilisateur systemd **activable par D-Bus**. L'extension l'active si
absent ; désactiver l'extension arrête les animaux.

### 3.6 Extension GNOME Shell

Cible : GNOME Shell 50 (`shell-version: ["50"]`), modules ESM, GJS 1.88.
Environ 300 lignes, sans logique métier — donc réparable rapidement quand GNOME
casse ses API.

- **Rendu** : un `St.Widget` par animal, dont le CSS pointe la feuille de
  sprites et décale `background-position` sur la bonne tuile. Technique de
  sprite CSS classique des extensions GNOME, accélérée par le GPU.
- **Placement** : `Main.layoutManager.uiGroup` — au-dessus des fenêtres, sous
  les popups et l'aperçu des activités. Les animaux se masquent donc d'eux-mêmes
  quand la vue Activités s'ouvre.
- **Interaction** : widgets `reactive`, capture du clic et du glisser.
- **Menu** : `PopupMenu` dans le panneau GNOME (changer de pet, en ajouter,
  télécharger, quitter, version affichée). Pas de dépendance à AppIndicator.

## 4. Transfert des images

Les spritesheets pèsent jusqu'à plusieurs mégaoctets ; les faire transiter par
D-Bus serait coûteux. Le démon décode le base64, applique la couleur-clé, et
écrit un PNG RGBA dans `~/.cache/rustypet/<pet>/sheet.png`. Il ne transmet
ensuite que **le chemin du fichier**. L'extension charge la texture depuis le
disque.

## 5. Protocole D-Bus

Nom de bus : `dev.yrbane.RustyPet`.

| Sens | Message | Fréquence |
|---|---|---|
| démon → shell | `Frame(pets: a(u s ii ii d b))` — id, feuille, tuile, position, opacité, miroir | ~10–20/s par animal |
| démon → shell | `PetAdded(id, sheet_path, tile_w, tile_h)` | rare |
| démon → shell | `PetRemoved(id)` | rare |
| shell → démon | `WindowsChanged(a(iiii))` | sur `restacked`, `position-changed`, `size-changed` |
| shell → démon | `PointerEvent(id, kind, x, y)` | à l'interaction |
| shell → démon | `SetPetSet(names)`, `SpawnPet(name)`, `Quit()` | menu |

D-Bus suffit à ces débits. Si des saccades apparaissent en multi-animaux, seul
le message `Frame` bascule sur une socket Unix — le reste du protocole est
inchangé. Cette bascule n'est pas implémentée d'avance.

La géométrie des fenêtres est poussée **sur signal**, jamais par frame.

## 6. Tests

Les 20+ `animations.xml` du dépôt amont constituent le corpus de référence.

1. **`pet-format`** — chaque XML du corpus parse sans perte ; tests ciblés sur
   le base64 non padé, `areaH`, la découpe en ligne d'abord, la couleur-clé.
2. **`pet-expr`** — table de cas issue de l'analyse : `screenW/2 - imageW/2`,
   `random`, `randS`, priorités des opérateurs.
3. **`pet-engine`** — RNG injecté et seedé, donc entièrement déterministe : on
   avance N ticks avec une géométrie fictive et on compare la trace de sprites
   à un instantané. Rend testables les branches probabilistes et la gravité
   sans écran.
4. **backend `null`** — le moteur tourne en CI headless, sans GNOME ni X.
5. **Extension et pont D-Bus** — vérification manuelle E2E. Ils ne contiennent
   aucune logique métier.

Développement en TDD : les tests précèdent l'implémentation, crate par crate.

## 7. Versionnage

Repo neuf, `0.1.0` au premier commit. Chaque commit porte une version SemVer,
bumpée dans `Cargo.toml` (workspace et crates) **et** dans
`gnome-extension/metadata.json`, avec une entrée en tête de `CHANGELOG.md`. La
version est affichée dans le menu du panneau.

## 8. Risques

| Risque | Parade |
|---|---|
| GNOME casse ses API à chaque version majeure | Logique entièrement en Rust ; extension mince et réparable vite |
| Saccades du flux `Frame` par D-Bus | Message isolé, bascule possible sur socket Unix |
| Pets dont les séquences `only="window"` supposent des barres de titre | L'extension fournit la vraie géométrie ; le backend `null` les ignore proprement |
| Licence des pets d'origine | À vérifier avant toute redistribution ; le dépôt amont reste la source |

## 9. Hors périmètre

Windows, macOS, KDE, les compositeurs wlroots, l'éditeur de pets, la version
Android.
