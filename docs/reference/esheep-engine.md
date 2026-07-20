# eSheep / DesktopPet — Spécification technique du moteur d'animation

Référence pour un portage Rust. Basée sur `src/dotNet/Animations.cs`, `Xml.cs`, `FormPet.cs`, `Program.cs`, `src/LocalData/AnimationXML.cs`, `Resources/animations.xsd`.

Toutes les coordonnées sont en **pixels entiers**. Le repère est celui du bureau Windows (origine en haut-gauche de l'écran, Y croissant vers le bas).

---

## 1. Schéma XML (`Resources/animations.xsd`)

Racine : `<animations>` (attention : le nom `animations` désigne **à la fois** la racine et le conteneur de la liste d'animations — deux niveaux distincts).

```
animations (root)
├── header        [1..1]
├── image         [1..1]
├── spawns        [1..1]
├── animations    [1..1]   ← conteneur
├── childs        [1..1]
└── sounds        [0..1]
```

Note : à l'intérieur de `header`, `image`, `spawn`, `child`, `animation`, `sound`, le modèle est `xsd:all` → **les sous-éléments peuvent apparaître dans n'importe quel ordre**, chacun au plus une fois. Un parseur Rust doit accepter l'ordre libre (ne pas s'appuyer sur un ordre séquentiel).

### 1.1 `<header>` — `xsd:all`, tous obligatoires [1..1]

| Élément | Type | Sémantique |
|---|---|---|
| `author` | string | Nom de l'auteur |
| `title` | string | Titre de l'animation (site web) |
| `petname` | string | Nom affiché dans le menu contextuel |
| `version` | string | Version du pet (`0.1` = beta), immuable après publication |
| `info` | string | Description libre, crédits, email |
| `application` | integer | Version de format ; **doit valoir 1** (seule version) |
| `icon` | string | Icône **ICO en base64**, typiquement en CDATA |

### 1.2 `<image>` — `xsd:all`, tous obligatoires [1..1]

| Élément | Type | Sémantique |
|---|---|---|
| `tilesx` | integer | Nombre de tuiles sur l'axe X du spritesheet |
| `tilesy` | integer | Nombre de tuiles sur l'axe Y |
| `png` | string | Spritesheet **PNG en base64** (CDATA) |
| `transparency` | string | Couleur clé de transparence. Défaut documenté : `Magenta` |

### 1.3 `<spawns>` — `xsd:sequence` de `<spawn>` [1..*]

Attributs de `<spawn>` :
- `id` : `xsd:int` — identifiant du spawn
- `probability` : `xsd:integer` — poids de tirage (voir §5)

Contenu (`xsd:all`) :
- `x` : string [1..1] — **expression** (voir §3)
- `y` : string [1..1] — expression
- `next` : [0..1] — contenu = `xsd:integer` (ID de l'animation à jouer), attribut optionnel `probability` (`xsd:integer`)

### 1.4 `<animations>` (conteneur) — `xsd:sequence` de `<animation>` [1..*]

Attributs de `<animation>` :
- `id` : identifiant entier unique de l'animation (clé de dictionnaire)

Contenu (`xsd:all`) :

| Élément | Card. | Contenu |
|---|---|---|
| `name` | [1..1] | string — nom logique ; certains noms sont **réservés** (voir §2.6) |
| `start` | [1..1] | groupe `step` |
| `end` | [0..1] | groupe `step` |
| `sequence` | [1..1] | voir ci-dessous |
| `border` | [0..1] | `xsd:choice` de `<next>`, max 10 |
| `gravity` | [0..1] | `xsd:choice` de `<next>`, max 10 |

Groupe **`step`** (`xsd:all`) — utilisé par `start` et `end` :

| Élément | Type | Card. | Défaut |
|---|---|---|---|
| `x` | string (expression) | [1..1] | — |
| `y` | string (expression) | [1..1] | — |
| `offsety` | integer | [0..1] | `0` |
| `opacity` | double | [0..1] | `1.0` |
| `interval` | string (expression) | [1..1] | — |

`<sequence>` : `xsd:choice maxOccurs="unbounded"` mêlant
- `frame` : integer [1..*] — **index dans la liste de sprites** découpée du spritesheet
- `action` : string [0..*] — seule valeur reconnue par le moteur : `"flip"`
- `next` : élément `next` (voir §1.7) — animations suivantes en fin de séquence

Attributs de `<sequence>` :
- `repeat` : `xsd:string` — **expression** (donc peut être dynamique/aléatoire) — nombre de répétitions supplémentaires
- `repeatfrom` : `xsd:integer` — index 0-based de la frame à partir de laquelle on répète

### 1.5 `<childs>` — `xsd:sequence` de `<child>` [0..*]

Attribut : `animationid` (`xsd:int`) — ID de l'animation **parente** qui déclenche la création de cet enfant.

Contenu (`xsd:all`) : `x` (string, expr) [1..1], `y` (string, expr) [1..1], `next` [0..1] (integer + attr `probability`).

### 1.6 `<sounds>` [0..1] — `xsd:sequence` de `<sound>` [0..*]

Attribut : `animationid` (`xsd:int`). Contenu (`xsd:all`) : `probability` (integer) [1..1], `loop` (integer) [0..1], `base64` (string) [1..1] — le son (WAV) encodé.

### 1.7 Élément global `<next>`

```
<next probability="N" only="borderType">ID_ANIMATION</next>
```
- Contenu textuel : `xsd:integer` = ID de l'animation cible.
- `probability` : `xsd:integer` — **poids relatif**, pas un pourcentage absolu (voir §2.4).
- `only` : type énuméré `borderType` = `none` | `taskbar` | `window` | `horizontal` | `horizontal+` | `vertical` — filtre contextuel.

Il existe aussi un élément global `<only>` (choice de `next`), non utilisé par le moteur C# courant.

---

## 2. Sémantique d'une `<animation>`

### 2.1 Structures runtime

```
TValue    { is_dynamic: bool, is_screen: bool, compute: String, value: i32 }
TMovement { x: TValue, y: TValue, interval: TValue, offset_y: i32, opacity: f64 }
TSequence { repeat: TValue, repeat_from: i32, frames: Vec<i32>, total_steps: i32, action: String }
TAnimation{ id, name, start: TMovement, end: TMovement, sequence: TSequence,
            end_animation: Vec<TNextAnimation>,   // <sequence><next>
            end_border:    Vec<TNextAnimation>,   // <border><next>
            end_gravity:   Vec<TNextAnimation>,   // <gravity><next>
            gravity: bool, border: bool }
TNextAnimation { id: i32, probability: i32, only: TOnly }
```

`gravity` / `border` sont des **booléens dérivés** : vrais si la liste `end_gravity` / `end_border` correspondante est non vide.

### 2.2 Nombre total de pas

```
total_steps = frames.len() + (frames.len() - repeat_from) * repeat
```
`repeat` étant un `TValue`, il est réévalué (donc potentiellement aléatoire) **à chaque démarrage d'animation**, dans `UpdateValues()`.

### 2.3 Interpolation start → end sur la durée de l'animation

À chaque pas `step` (0-based), avec `T = total_steps` :

```
interval = start.interval + (end.interval - start.interval) * step / T      // en ms
opacity  = start.opacity  + (end.opacity  - start.opacity)  * step / T
offset_y = start.offset_y + (end.offset_y - start.offset_y) * step / T
```

Et pour le déplacement **par pas** (ce sont des vitesses, pas des positions absolues) — attention au dénominateur différent (`T - 1`) :

```
si T > 1 :
  x = start.x + (end.x - start.x) * step / (T - 1)
  y = start.y + (end.y - start.y) * step / (T - 1)
sinon :
  x = start.x ; y = start.y
```

Puis, si le pet est retourné (`is_moving_left == false`) : `x = -x`.

Enfin, après toutes les détections de collision : `position_x += x ; position_y += y`.

**Important** : `x`/`y` d'une animation sont des **deltas de déplacement appliqués à chaque frame**, pas des coordonnées. Seuls les `x`/`y` de `<spawn>` et `<child>` sont des positions absolues.

### 2.4 Choix de l'animation suivante (`SetNextGeneralAnimation`)

Entrées : une liste de `TNextAnimation` et un contexte `where: TOnly`.

```
TOnly (bitflags) :
  NONE       = 0x7F   (= masque « tous »)
  TASKBAR    = 0x01
  WINDOW     = 0x02
  HORIZONTAL = 0x04
  VERTICAL   = 0x08
  (HORIZONTAL+ ≈ variante de HORIZONTAL, bit supplémentaire du même masque)
```

Algorithme :
1. Filtrer : on **saute** toute entrée telle que `anim.only != NONE && (anim.only & where) == 0`.
2. Sommer les `probability` des entrées retenues → `rand_max`.
3. Tirer `val = rand(0, rand_max)`.
4. Parcourir en accumulant `sum += probability` ; la première entrée avec `sum >= val` est choisie.
5. Si un ID est choisi (> 0) : appeler `UpdateAnimationValues(id)` (réévalue les expressions dynamiques et `total_steps`) et, si un son est associé, le jouer avec probabilité `rand(0,100) < sound.probability`.
6. Si la liste est vide → retourne **-1**, ce qui signifie « **respawn** » (voir §5).

Trois points d'entrée :
- `SetNextSequenceAnimation(id, where)` → utilise `end_animation`
- `SetNextBorderAnimation(id, where)` → utilise `end_border`
- `SetNextGravityAnimation(id, where)` → utilise `end_gravity`

### 2.5 Choix de la frame affichée à un pas donné

```
si step < frames.len() :
    frame = frames[step]
sinon :
    idx = ((step - frames.len() + repeat_from) % (frames.len() - repeat_from)) + repeat_from
    frame = frames[idx]
```

### 2.6 Animations « clé » par nom

Le moteur associe automatiquement certaines animations par leur `name` (voir `Animations.AnimationDrag/AnimationFall/AnimationKill/AnimationSync`) :

| Nom XML | Champ | Défaut | Déclencheur |
|---|---|---|---|
| `drag` | `AnimationDrag` | 1 | Bouton gauche enfoncé sur le pet |
| `fall` | `AnimationFall` | 1 | Relâchement du bouton après drag |
| `kill` | `AnimationKill` | -1 | Fermeture de l'application |
| `sync` | `AnimationSync` | 1 | Annulation de la boîte « À propos » |

### 2.7 Action `flip`

Si `sequence.action == "flip"`, **à la fin** de la séquence :
- `is_moving_left = !is_moving_left`
- **toutes** les images de la liste sont retournées horizontalement en place (`RotateNoneFlipX`).

En Rust, préférer un flag d'orientation + miroir au rendu plutôt qu'une mutation destructive du cache d'images (le C# retourne réellement le buffer partagé, ce qui se propage aux enfants créés ensuite).

---

## 3. Évaluation des expressions x / y / interval / repeat

**Oui** : `x`, `y`, `interval` (dans `step`) et `repeat` (dans `sequence`) sont des **chaînes contenant une expression arithmétique**, pas des entiers.

### 3.1 Table de substitution (`Xml.ParseValue`)

Le C# fait une substitution **textuelle** puis évalue avec `DataTable.Compute` (moteur d'expressions ADO.NET).

| Jeton | Remplacé par |
|---|---|
| `screenW` | `screen.Bounds.Width` — largeur totale de l'écran |
| `screenH` | `screen.Bounds.Height` — hauteur totale |
| `areaW` | `screen.WorkingArea.Width` — largeur de la zone de travail (hors barre des tâches) |
| `areaH` | `screen.WorkingArea.Height + screen.WorkingArea.Y` — **bord bas** de la zone de travail |
| `imageW` | `spriteWidth` — largeur d'une frame |
| `imageH` | `spriteHeight` — hauteur d'une frame |
| `imageX` | `parentX` — X du parent (−1 si pas un enfant) |
| `imageY` | `parentY` — Y du parent (−1 si pas un enfant) |
| `random` | `rand(0, 100)` — **réévalué à chaque appel** |
| `randS` | `iRandomSpawn` — tiré une fois par chargement XML, dans `rand(10, 90)` |
| `scale` | `Program.MyData.GetScale()` — facteur d'échelle HiDPI |

Pièges de la substitution textuelle à reproduire (ou volontairement corriger) :
- L'ordre compte : `random` est remplacé **avant** `randS`. Or `randS` contient la sous-chaîne `rand`… mais pas `random`, donc pas de collision — en revanche toute expression contenant `random` verra ses occurrences remplacées par des valeurs **différentes** si l'implémentation Rust n'utilise pas un remplacement global unique. Le C# fait un `Replace` global : **toutes** les occurrences de `random` dans une même expression reçoivent la **même** valeur.
- `screenW` est remplacé avant `screenH` etc. — sans ambiguïté car les jetons ne sont pas préfixes les uns des autres, sauf `imageW`/`imageH`/`imageX`/`imageY` qui partagent le préfixe `image` (pas de problème car remplacement de jetons complets).

### 3.2 Cas particulier : enfant d'un parent retourné

Avant substitution, si `parentFlipped` :
- si l'expression contient `-imageW` → remplacer `-imageW` par `+imageW`
- sinon → remplacer `imageW` par `(-imageW)`

C'est un miroir horizontal du placement de l'enfant relativement au parent.

### 3.3 Grammaire à supporter en Rust

`DataTable.Compute` accepte une expression arithmétique classique : entiers, `+ - * / %`, parenthèses, priorité standard, et des fonctions ADO.NET (`IIF`, `LEN`, `ISNULL`, `SUBSTRING`, `CONVERT`, opérateurs de comparaison et `AND`/`OR`/`NOT`). En pratique les XML de pets n'utilisent que **arithmétique entière + parenthèses**. Une implémentation Rust raisonnable : parseur pratt d'expressions arithmétiques sur `f64`, résultat tronqué en `i32` (`(int)dv`, troncature vers zéro, pas d'arrondi).

En cas d'échec de parsing, le C# logge une erreur et retourne **0**.

### 3.4 Classification statique (`GetXMLCompute`)

```
is_dynamic = compute contient "random" || "randS" || "imageX" || "imageY"
is_screen  = compute contient "screen" || "area"
```

- `is_dynamic == true` → l'expression est **réévaluée à chaque `GetValue()`** (chaque nouveau départ d'animation).
- `is_dynamic == false` → la valeur pré-calculée est renvoyée telle quelle, **sauf** si un `screenIndex >= 0` est fourni, auquel cas on réévalue avec les dimensions de cet écran.
- `is_screen` sert au support multi-écran : ces valeurs doivent être recalculées quand le pet change de moniteur.

`UpdateValues(screen_index)` recalcule : `sequence.total_steps` (si `repeat.is_dynamic`), puis `start.{interval,x,y}` et `end.{interval,x,y}` (si l'un des trois est dynamique). Applique ensuite le facteur `scale` si `> 1`.

---

## 4. Physique et interactions

### 4.1 Boucle et état

Champs runtime du pet (`FormPet`) :
- `position_x`, `position_y` : f64 — position réelle (accumulateur), distincte de `Left`/`Top` entiers de la fenêtre
- `offset_y` : f64 — décalage vertical visuel appliqué au rendu (`Top = position_y + offset_y`)
- `animation_step` : i32 — pas courant (initialisé à **-1** par `SetNewAnimation`, incrémenté après chaque tick)
- `is_moving_left` : bool — sens ; l'axe X est nié quand faux
- `is_dragging` : bool
- `is_leaving` : bool — le pet sort de l'écran (découpe du rendu)
- `hwnd_window` : handle de la fenêtre sur laquelle le pet marche (`0` = aucune)
- `current_window_size` : rect de cette fenêtre
- `display_index` : index du moniteur courant
- `screen_bounds` / `screen_area` : bornes totales et zone de travail du moniteur courant

### 4.2 Ordre des opérations dans `NextStep()`

1. Sélection et affichage de la frame (§2.5).
2. Si `is_dragging` : `position_x = Left = curseur.x - width/2` ; `position_y = Top = curseur.y - 2` ; **return immédiat** (aucune physique pendant le drag).
3. Mise à jour de `interval`, `opacity`, `offset_y` par interpolation (§2.3).
4. Calcul de `x`, `y` (§2.3), négation de `x` si retourné.
5. Détections dans cet ordre : bord gauche (`x < 0`), bord droit (`x > 0`), bas (`y > 0`), haut (`y < 0`).
6. Gravité (si `animation.gravity`).
7. Fin de séquence (`animation_step >= total_steps`) : flip éventuel, choix de l'animation suivante.
8. `position_x += x ; position_y += y`.
9. Gestion `is_leaving` (découpe de la fenêtre aux bords de l'écran).

### 4.3 Détection des bords

Chaque détection appelle `SetNextBorderAnimation(current_id, where)`. Si le résultat est `>= 0`, la position est **clampée** au bord, la vitesse concernée est mise à `0`, et la nouvelle animation démarre. Sinon, le pet **continue** (et, pour les bords latéraux/haut, `is_leaving = true`).

| Situation | Condition | `where` | Clamp |
|---|---|---|---|
| Bord gauche écran | `x < 0 && hwnd == 0 && position_x + x < area.x` | `VERTICAL` | `position_x = area.x` |
| Bord gauche fenêtre | `x < 0 && hwnd != 0 && position_x + x < rect.left` | `WINDOW` | `position_x = rect.left` ; si aucune anim → `hwnd = 0` (le pet quitte la fenêtre) |
| Bord droit | symétrique (`x > 0`) | `VERTICAL` / `WINDOW` | idem côté droit |
| Bas / barre des tâches | `y > 0 && position_y + y > (area.y + area.h) - height` | `TASKBAR` | `position_y = area.y + area.h - height` ; `offset_y = 0` |
| Haut | `y < 0 && position_y + y < area.y` | `HORIZONTAL` | `position_y = area.y` |
| Atterrissage sur fenêtre | `y > 0` et `FallDetect(y) > 0` | `WINDOW` | `position_y = window_top - height` ; `offset_y = 0` ; si `start.y != 0` → `hwnd = 0` |

Note : `area` = zone de travail (`ScreenArea`), `bounds` = écran complet (`ScreenBounds`). La barre des tâches est donc implicitement gérée par la différence entre les deux.

### 4.4 Gravité

Si `animation.gravity` (liste `<gravity>` non vide) :

- **Hors fenêtre** (`hwnd == 0`) : si `position_y + y < (area.y + area.h) - height` (le pet n'est pas au sol) :
  - tolérance : si `position_y + y + 3 >= sol`, on **colle au sol** (`y = sol - position_y`) — marge de 3 px sans chute ;
  - sinon → `SetNextGravityAnimation(id, ...)` — le pet tombe.
- **Sur une fenêtre** (`hwnd != 0`) : on revérifie que la fenêtre est toujours là / toujours sous le pet ; si elle a disparu ou bougé → `hwnd = 0` et `SetNextGravityAnimation(id, WINDOW)` (le pet chute).

### 4.5 « Walk on window » — `FallDetect(y)`

Algorithme :
1. `CheckFullScreen()` : via `GetForegroundWindow()`, détermine si une fenêtre plein écran est active (dans ce cas la marche sur fenêtre est neutralisée).
2. `EnumWindows` : collecte toutes les fenêtres candidates, en excluant :
   - la fenêtre du pet lui-même (`hWnd == Handle`) ;
   - les fenêtres non visibles (`!IsWindowVisible`) ;
   - les fenêtres dont `GetTitleBarInfo` échoue ;
   - les fenêtres dont la barre de titre a l'état `STATE_SYSTEM_INVISIBLE` (`rgstate[0] & 0x8000`) ;
   - les fenêtres sans titre (`GetWindowText` vide).
   - Cas spécial : un titre exactement `"Sheep"` court-circuite les tests de barre de titre.
3. Pour chaque candidate, `GetWindowRect` puis test d'atterrissage :
```
position_y + height       <  rect.top                       // au-dessus juste avant
&& position_y + height + y >= rect.top                      // franchit le bord haut ce pas-ci
&& position_x             >= rect.left  - width/2           // tolérance latérale d'une demi-largeur
&& position_x + width     <= rect.right + width/2
&& position_y             >  20 + area.y                    // pas trop haut sur l'écran
```
4. Si un match : `hwnd_window = handle`, `current_window_size = rect`, on lit le titre (`GetWindowText`, 128 car.), et optionnellement, si `GetWindowForeground()` est activé dans les préférences, on remonte la fenêtre (`ShowWindow(hwnd, 5 /*SW_SHOW*/)`). Retourne `rect.top`.
5. Sinon, retourne 0.

Le pet suit ensuite les bords **de cette fenêtre** au lieu des bords d'écran (§4.3), tant que `hwnd_window != 0`.

Une vérification Z-ordre existe (`CheckTopWindow`, via `GetWindow(hwnd, 2 /*GW_HWNDNEXT*/)`) pour s'assurer que la fenêtre visée est bien au-dessus.

### 4.6 Drag souris

- **MouseDown** (bouton gauche, et **pas** un enfant — `name` ne commence pas par `"child"`) :
  `hwnd_window = 0` ; bascule `TopMost` false→true (force la remontée) ; `is_dragging = true` ; `SetNewAnimation(AnimationDrag)`.
- **Pendant le drag** : dans `NextStep`, position asservie au curseur (§4.2 étape 2), physique désactivée.
- **MouseUp** (bouton gauche, pas un enfant) : `SetNewAnimation(AnimationFall)` ; puis, si `is_dragging`, on **recalcule le moniteur courant** : on cherche le premier écran `k` tel que le centre horizontal (`Left + width/2`) et le bloc vertical (`Top + height/2 >= bounds.y` et `Top + height <= bounds.y + bounds.h`) tombent dans ses bornes → `display_index = k`. Puis `is_dragging = false`.
- **MouseDown droit** : menu de debug (uniquement si le mode debug est actif).
- Les **enfants** (`name` commençant par `"child"`) sont non interactifs : le handler MouseDown est remplacé par un no-op et le curseur est `Default`.

### 4.7 Sortie d'écran (`is_leaving`)

Quand aucune animation de bord n'existe, le pet continue et « sort » : la fenêtre est **rognée** pour ne pas déborder sur un autre moniteur.
- Sortie à gauche : `cut = area.x - position_x` ; si `width > 2` → `width = sprite_w - cut`, `Left = area.x`, l'image est décalée de `-cut` ; sinon → `Left -= width`, l'image repositionnée, et `animation_step += frames.len()/3` (accélère la fin).
- Sortie à droite : symétrique.

En Rust/Wayland, ce rognage manuel est remplaçable par un simple clipping / masquage de la surface.

---

## 5. Spawns et enfants

### 5.1 Spawn (apparition initiale) — `Play()`

1. `GetRandomSpawn()` : somme des `probability` de tous les spawns → `percent` ; tirage `rand(0, percent)` ; parcours cumulatif, le premier spawn dont le cumul `>= randValue` est retenu.
   - Si aucun spawn n'est défini, un **spawn de secours** est fabriqué : `x = "0"`, `y = "0"`, `opacity = 1.0`, `interval = "1000"`, `offset_y = 0`, `next` = première animation connue (ou `1`), `probability = 100`.
2. Positionnement absolu :
   ```
   Top  = screen_bounds.y + spawn.y.get(display_index)
   Left = screen_bounds.x + spawn.x.get(display_index)
   si !is_moving_left :
       Left = screen_bounds.x - (spawn.x.get(display_index) - screen_bounds.width) - sprite_width
   ```
   (miroir horizontal de la position de spawn quand le pet est retourné)
3. `position_x = Left`, `position_y = Top`, `offset_y = 0`, `is_leaving = false`.
4. `SetNewAnimation(spawn.next)`, fenêtre rendue visible, `opacity = 0.0` (la première frame est indéfinie et donc masquée), timer activé, `TopMost = true`.

Un `Play(force_spawn: i32)` permet de forcer un spawn par index (debug).

### 5.2 Enfants (`<child>`)

Après `SetNewAnimation(id)`, si `HasAnimationChild(id)`, pour **chaque** `TChild` associé à cet `animationid` :

1. Création d'un nouveau pet, avec :
   - `parent_pos = (screen_bounds.x + Left, screen_bounds.y + Top)`,
   - `parent_flipped = !is_moving_left`,
   - le même `display_index`,
   - **copie de la liste d'images du parent** (donc de l'état de flip courant).
2. Nommage : le nom vaut `"child"` puis, pour les descendants, `"child" + (n+1)`. **Limite : 5 niveaux de sous-enfants** (`int.Parse(name[5..]) < 5`).
3. Positionnement (identique au spawn, avec miroir si retourné) :
   ```
   Top  = screen_bounds.y + child.y.get(display_index)
   Left = screen_bounds.x + child.x.get(display_index)     // si is_moving_left
        = screen_bounds.x - (child.x.get(display_index) - screen_bounds.width) - sprite_width  // sinon
   ```
   Les expressions `imageX` / `imageY` référencent la position du parent (§3.1).
4. `animation_step = 0`, `hwnd_window = 0`, `opacity = 1.0`, `is_leaving = false`, drag désactivé.
5. `SetNewAnimation(child.next)` puis timer activé.

**Fin de vie d'un enfant** : quand aucune animation suivante n'est trouvée (`iNextAni < 0`), un enfant se **ferme** au lieu de respawner (le pet principal, lui, appelle `Play(false)` → nouveau spawn). Les enfants sont aussi fermés/disposés avec le parent (`Kill()`).

---

## 6. Décodage du spritesheet

1. Source : `image/png` en base64 (ou une image utilisateur stockée dans les préférences, essayée en premier).
2. **Padding base64** : si `len % 4 != 0`, ajouter `4 - (len % 4)` caractères `=`. Nécessaire car beaucoup de XML de pets ont un base64 non padé.
3. Décodage → PNG → bitmap.
4. Facteur d'échelle `iScale` (HiDPI), avec garde-fou : tant que `image.width * iScale / tilesx > 255`, décrémenter `iScale` (limite du composant `ImageList` Windows — **contrainte purement Win32, à supprimer en Rust**).
5. Dimensions d'une frame :
   ```
   sprite_width  = image.width  * iScale / tilesx
   sprite_height = image.height * iScale / tilesy
   ```
6. Découpe (`BuildSprites`) : parcours en **ligne d'abord** (`for yOffset … for xOffset …`), chaque tuile faisant `sprite_width/iScale × sprite_height/iScale` pixels source. **L'index de sprite est donc `row * tilesx + col`**, ce qui correspond aux valeurs de `<frame>`.
7. Mise à l'échelle : agrandissement **nearest-neighbor manuel**, pixel par pixel (chaque pixel source devient un carré `iScale × iScale`). Pas d'interpolation — indispensable pour du pixel-art.
8. **Transparence** : la valeur `<transparency>` est une couleur clé (défaut `Magenta`, `#FF00FF`). Tous les pixels égaux à cette couleur doivent devenir totalement transparents. Sous Windows c'est réalisé via la `TransparencyKey` de la Form (composition par le gestionnaire de fenêtres). En Rust il faut **convertir explicitement en RGBA** : parcourir le buffer et mettre `alpha = 0` sur les pixels correspondant à la couleur clé (et non pas s'appuyer sur une propriété de fenêtre).
   - Attention : un PNG déjà doté d'un canal alpha existe aussi ; l'implémentation Rust doit gérer les deux cas (alpha présent → ne rien faire ; sinon → color-key).
9. L'icône (`header/icon`) est un ICO base64 décodé séparément, utilisé pour la barre des tâches et l'icône de zone de notification.

---

## 7. Boucle temporelle

Il n'y a **pas de framerate fixe** : le moteur utilise un timer mono-coup ré-armé, dont l'intervalle est piloté par l'animation.

```
Timer1_Tick:
    timer.enabled = false
    si animation_step < 0 : animation_step = 0
    NextStep()                      // affiche la frame, calcule la physique, règle timer.interval
    si !disposed :
        animation_step += 1
        timer.enabled = true
```

Points clés :
- **L'intervalle est recalculé à chaque tick** dans `NextStep` par interpolation `start.interval → end.interval` (§2.3). Une animation peut donc accélérer ou ralentir progressivement.
- `SetNewAnimation(id)` positionne `animation_step = -1` puis, en fin de fonction, `timer.interval = animation.start.interval.get()`. Le premier tick porte donc `animation_step = 0`.
- Quand une nouvelle animation est déclenchée **au milieu** d'un pas (bord, gravité, fin de séquence), le C# force `timer.interval = 1` et affiche immédiatement `frames[0]` — la première frame de la nouvelle animation s'exécute quasi instantanément.
- L'animation `kill` a un traitement spécial : à chaque fin de séquence, l'opacité est décrémentée de `0.1` ; à `<= 0.1` la fenêtre se ferme (fondu de sortie).
- Précision : le timer WinForms est un timer message-loop, résolution ~15 ms. Un portage Rust devrait utiliser un timer monotone haute résolution et **compenser la dérive** (planifier sur des instants absolus plutôt que `sleep(interval)`), sous peine d'animations plus lentes qu'en C#.
- Un `Application.DoEvents()` est utilisé dans les boucles d'attente de la détection de fenêtres — antipattern à ne pas reproduire ; en Rust, opérations asynchrones ou non bloquantes.

---

## 8. Dépendances Windows à remplacer sous Linux / GNOME

### 8.1 API Win32 utilisées

| API (`user32.dll`) | Rôle | Équivalent Linux |
|---|---|---|
| `EnumWindows` | Lister toutes les fenêtres du bureau | X11 : `XQueryTree` + EWMH `_NET_CLIENT_LIST`. **Wayland : aucun équivalent** (voir 8.3) |
| `GetWindowRect` | Géométrie d'une fenêtre | X11 : `XGetGeometry` / `_NET_FRAME_EXTENTS`. Wayland : indisponible |
| `IsWindowVisible` | Fenêtre mappée | X11 : `XGetWindowAttributes().map_state` |
| `GetWindowText` | Titre de la fenêtre | X11 : `_NET_WM_NAME` |
| `GetTitleBarInfo` | État de la barre de titre (filtre les fenêtres fantômes) | Pas d'équivalent direct ; filtrer via `_NET_WM_WINDOW_TYPE == _NET_WM_WINDOW_TYPE_NORMAL` et `_NET_WM_STATE` |
| `GetForegroundWindow` | Fenêtre active (détection plein écran) | X11 : `_NET_ACTIVE_WINDOW` ; GNOME : D-Bus / extension |
| `GetWindow(hwnd, GW_HWNDNEXT)` | Parcours du Z-ordre | X11 : ordre de `_NET_CLIENT_LIST_STACKING` |
| `ShowWindow(hwnd, SW_SHOW)` | Remonter une fenêtre survolée | X11 : `_NET_ACTIVE_WINDOW` client message |

### 8.2 WinForms / .NET à remplacer

| Élément | Remplacement Rust |
|---|---|
| `Form` sans bordure, `TopMost` | Surface `layer-shell` (`wlr-layer-shell` / `gtk4-layer-shell`) sur la couche `overlay`, ou `GtkWindow` X11 avec `_NET_WM_WINDOW_TYPE_DOCK` |
| `WS_EX_TOOLWINDOW` (hors Alt-Tab) | `_NET_WM_STATE_SKIP_TASKBAR` + `_NET_WM_STATE_SKIP_PAGER`, ou layer-shell (naturellement hors du switcher) |
| `WS_EX_LAYERED` | Surface RGBA avec compositing alpha (natif sous Wayland) |
| `TransparencyKey` | Conversion explicite color-key → alpha 0 (§6.8) |
| `Form.Opacity` | Alpha global au rendu ou `wl_surface` opacity |
| `PictureBox` + `ImageList` | Texture / buffer de sprites (`softbuffer`, `wgpu`, ou `GtkPicture`) |
| `System.Windows.Forms.Timer` | `tokio::time` / boucle GLib avec instants absolus |
| `Screen.PrimaryScreen`, `Screen.AllScreens`, `Bounds`, `WorkingArea` | Wayland : `wl_output` / `xdg-output` ; GNOME : Mutter D-Bus. `WorkingArea` ⇒ `_NET_WORKAREA` (X11) ou zone hors panels (layer-shell exclusive zones) |
| `Cursor.Position` (drag) | Wayland : événements pointeur de la surface uniquement ; **la position globale du curseur n'est pas exposée** — le drag doit se faire en delta relatif à partir du `wl_pointer` sur la surface du pet |
| `DataTable.Compute` | Évaluateur d'expressions maison (§3.3) |
| `System.Drawing.Bitmap`, `RotateFlip`, `GetPixel` | crate `image` + retournement par transformation à l'affichage |
| `SoundPlayer` (WAV base64) | `rodio` / `libpulse` |
| Registre / `Properties.Settings` | `~/.config/…` (XDG), format TOML/JSON |
| Icône de zone de notification | `StatusNotifierItem` / D-Bus (`ksni`), GNOME nécessitant une extension AppIndicator |
| `MessageBox.Show` | Dialogue GTK ou log |

### 8.3 Le problème central sous Wayland

**La détection de fenêtres (`FallDetect`, « walk on window ») est irréalisable en Wayland pur** : par conception, un client ne peut ni énumérer ni géolocaliser les fenêtres des autres clients. Stratégies possibles, par ordre de préférence :

1. **Extension GNOME Shell** exposant en D-Bus la liste des fenêtres et leurs rects (`global.get_window_actors()`) → fidélité complète, mais requiert une installation côté utilisateur et casse à chaque version majeure de GNOME.
2. **Session X11 / XWayland** : `EnumWindows` se traduit directement en `_NET_CLIENT_LIST_STACKING` + `XGetGeometry`. Fonctionne, mais X11 est en fin de vie sous GNOME.
3. **Mode dégradé** : désactiver la marche sur fenêtre, ne conserver que les bords d'écran et la barre des tâches (`ScreenArea` via `_NET_WORKAREA` / exclusive zones). Le moteur reste parfaitement jouable — la plupart des animations utilisent `taskbar`/`horizontal`/`vertical`, `window` étant un bonus.

**Recommandation d'architecture** : isoler tout cela derrière un trait `DesktopBackend` (énumération de fenêtres, géométrie des moniteurs, zone de travail, position du pointeur, création de surface), avec des implémentations `X11Backend`, `GnomeShellExtensionBackend` et `NullWindowBackend`. Le cœur du moteur (parsing XML, expressions, machine à états d'animation, physique) doit être **totalement indépendant de la plateforme** et testable sans affichage.

### 8.4 Autres écarts à prévoir

- **Multi-écran** : le modèle C# suppose un espace de coordonnées virtuel unifié (`ScreenBounds.X/Y` décalés). Sous Wayland, chaque surface appartient à un output ; la migration d'un pet d'un écran à l'autre doit être réimplémentée.
- **HiDPI** : `iScale` est un entier appliqué au découpage des sprites. Sous Wayland, préférer `wl_surface.set_buffer_scale` / fractional-scaling et garder les sprites à leur taille native.
- **Limite 255 px** (§6.4) : contrainte `ImageList` Win32, à supprimer.
- **Clic-à-travers** : `WS_EX_TRANSPARENT` est commenté dans le C#. Sous Wayland, l'équivalent est une région d'entrée vide (`wl_surface.set_input_region`), utile pour les enfants non interactifs.
- **`Application.DoEvents()`** et les boucles d'attente synchrones sont à remplacer par de l'asynchrone.
- **Validation XSD** : le C# valide contre `animations.xsd` avec des handlers non bloquants (erreurs loguées, chargement poursuivi). En Rust, `quick-xml` + validation manuelle tolérante, avec repli sur le pet par défaut en cas d'échec.

---

## 9. Résumé de la machine à états (pseudo-Rust)

```
loop {
    frame = pick_frame(anim, step);
    render(frame, position + (0, offset_y), opacity);

    if dragging { position = cursor - (w/2, 2); continue; }

    interval = lerp(anim.start.interval, anim.end.interval, step, total);
    opacity  = lerp(anim.start.opacity,  anim.end.opacity,  step, total);
    offset_y = lerp(anim.start.offset_y, anim.end.offset_y, step, total);

    let (mut dx, mut dy) = lerp_xy(anim, step, total);   // dénominateur total-1
    if !moving_left { dx = -dx; }

    check_horizontal_borders(&mut dx);   // VERTICAL ou WINDOW
    check_vertical_borders(&mut dy);     // TASKBAR / WINDOW / HORIZONTAL
    if anim.gravity { check_gravity(&mut dy); }

    if step >= total {
        if anim.sequence.action == "flip" { moving_left = !moving_left; }
        let next = pick_next(anim.end_animation, context);
        match next { Some(id) => set_animation(id), None => respawn_or_close() }
    }

    position += (dx, dy);
    step += 1;
    sleep_until(now + interval);
}
```
