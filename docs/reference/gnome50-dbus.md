# GNOME Shell 50 + D-Bus — note de référence pour `petd`

Contexte : GNOME Shell 50.3 (Wayland), GJS 1.88, Rust 1.97, workspace Cargo existant (`resolver = "3"`, `edition = "2024"`). Objectif : extension GNOME Shell (JS/ESM) qui affiche un sprite animé, pilotée par un démon Rust (`petd`) via D-Bus session.

Convention de cette note : **[CERTAIN]** = vérifié directement sur une source primaire citée (doc officielle, code source GNOME/mutter, docs.rs). **[QUASI-CERTAIN]** = fortement corroboré mais pas re-testé sur cette machine précise. **[À VÉRIFIER]** = déduction logique ou connaissance générale non re-confirmée pour GNOME 50 exactement — à tester avant de bâtir dessus.

Sources principales consultées (juillet 2026) :
- https://gjs.guide/extensions/overview/anatomy.html
- https://gjs.guide/extensions/development/creating.html
- https://gjs.guide/extensions/development/debugging.html
- https://gjs.guide/extensions/upgrading/gnome-shell-50.html
- https://gjs.guide/guides/gio/dbus.html
- https://gjs.guide/extensions/topics/dialogs.html
- https://docs.rs/zbus/latest/zbus/ (zbus 5.18.0, 17 juillet 2026)
- https://docs.rs/zbus/latest/zbus/attr.interface.html
- https://dbus.freedesktop.org/doc/dbus-specification.html
- Code source GNOME Shell : `js/ui/layout.js` (gitlab.gnome.org/GNOME/gnome-shell, branche `main`)
- Code source Mutter : `src/core/window.c`, `src/core/display.c`, `src/st/st-theme-node.c` (gitlab.gnome.org/GNOME/mutter, branche `main`)

---

## 1. Extension GNOME Shell 50 (JS moderne, ESM)

### 1.1 Structure minimale [CERTAIN]

Deux fichiers requis, aucun autre. (Source : gjs.guide/extensions/overview/anatomy.html)

```
~/.local/share/gnome-shell/extensions/rustypet@yrbane.dev/
    extension.js
    metadata.json
```

`metadata.json` minimal :

```json
{
    "uuid": "rustypet@yrbane.dev",
    "name": "RustyPet",
    "description": "Affiche un pet animé piloté par petd",
    "shell-version": [ "50" ],
    "url": "https://github.com/yrbane/rustypet"
}
```

**Champ `shell-version`** [CERTAIN] : tableau de chaînes. Depuis GNOME 40, une seule valeur majeure suffit (`"50"`, pas `"50.3"` ni `"3.38"`-style). Point important et parfois mal compris : depuis GNOME 40, `disable-extension-version-validation` vaut **`false` par défaut** (avant GNOME 40 c'était `true`). Donc si `"50"` n'apparaît pas dans le tableau, **l'extension ne se chargera pas** sur cette machine, sauf si l'utilisateur active manuellement la validation permissive (`gsettings set org.gnome.shell disable-extension-version-validation true`). Il est possible de lister plusieurs versions (`["49", "50"]`) pour supporter plusieurs cibles. Le guide de portage officiel confirme qu'il n'y a **aucun changement pertinent** pour `metadata.json` ni `extension.js` entre GNOME 49 et 50 (gjs.guide/extensions/upgrading/gnome-shell-50.html) — donc pas de piège spécifique à la version 50 sur ce point.

### `extension.js` — forme ESM canonique [CERTAIN]

Exemple officiel (adapté) :

```js
import St from 'gi://St';
import Clutter from 'gi://Clutter';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

export default class RustyPetExtension extends Extension {
    enable() {
        // this.metadata, this.uuid disponibles ici
        this._petActor = new St.Widget({ reactive: false });
        Main.layoutManager.uiGroup.add_child(this._petActor);
    }

    disable() {
        this._petActor?.destroy();
        this._petActor = null;
    }
}
```

`export default class ... extends Extension` est obligatoire (plus de `init()`/`enable()`/`disable()` en module classique pré-45, plus de `imports.misc.extensionUtils` — tout ceci est mort depuis GNOME 45). `this.metadata` et `this.uuid` sont confirmés utilisables dans `enable()` par l'exemple officiel (`Main.panel.addToStatusArea(this.uuid, this._indicator)`). Les propriétés `this.path` / `this.dir` (répertoire d'installation de l'extension, utile pour charger des ressources embarquées) sont **[QUASI-CERTAIN]** disponibles sur la classe `Extension` — non re-testées cette session, mais pas critiques ici puisque le PNG du sprite sera chargé depuis un chemin absolu dans `~/.cache/`, pas depuis le répertoire de l'extension.

### 1.2 Installation et rechargement sous Wayland

Installation : copier le dossier dans `~/.local/share/gnome-shell/extensions/<uuid>/`, puis :

```sh
gnome-extensions enable rustypet@yrbane.dev
```

**Rechargement sous Wayland** [CERTAIN, avec correction importante par rapport à l'énoncé] : la session imbriquée existe bien, mais **la commande a changé en GNOME 49**. D'après gjs.guide/extensions/development/debugging.html :

- **GNOME 49 et ultérieur (donc GNOME 50.3, votre cas)** :
  ```sh
  dbus-run-session gnome-shell --devkit --wayland
  ```
- GNOME 48 et antérieur (obsolète pour vous, à ne pas utiliser) :
  ```sh
  dbus-run-session gnome-shell --nested --wayland
  ```

`--nested` a été renommé/remplacé par `--devkit` (« development kit ») à partir de GNOME 49. La session imbriquée tourne dans une fenêtre, avec son propre bus D-Bus de session — **elle n'est pas isolée** (avertissement officiel : « will not protect your system from data loss »). Une fois dans le terminal qui a lancé la session imbriquée (ou un autre terminal avec le même `DBUS_SESSION_BUS_ADDRESS`), on peut faire `gnome-extensions enable rustypet@yrbane.dev` normalement.

Sous X11 uniquement, `Alt+F2` puis `restart` recharge le shell in-place ; **impossible sous Wayland** (confirmé : « Wayland sessions can not restart GNOME Shell while the user is logged in, so you must log out and log back in »). Donc pour vous : développement itératif = session imbriquée `--devkit`, test final sur la vraie session = déconnexion/reconnexion.

Le JS étant mis en cache par le moteur, il faut relancer une nouvelle instance (imbriquée ou après logout) à chaque modification du code — pas de hot-reload.

### 1.3 Afficher un acteur graphique / technique de spritesheet

**Ajout à l'écran** [CERTAIN, vérifié dans `js/ui/layout.js`] :

```js
const actor = new St.Widget({ width: 64, height: 64 });
Main.layoutManager.uiGroup.add_child(actor);   // ou Main.layoutManager.addChrome(actor, {...})
actor.set_position(x, y);  // coordonnées absolues écran (repère du stage global)
```

Deux façons d'insérer dans `uiGroup`, avec un effet de profondeur (z-order) différent — confirmé par un commentaire du code source de `layout.js` :
- `Main.layoutManager.addChrome(actor, params)` : place l'acteur **sous** `top_window_group` (sous les popups/menus), et **enregistre** l'acteur comme « chrome » (fait varier les struts WM si `affectsStruts: true`, et sa visibilité peut suivre le plein écran si `trackFullscreen: true`).
- `Main.layoutManager.uiGroup.add_child(actor)` directement (équivalent à `addTopChrome`) : place l'acteur **au-dessus de tout**, y compris les popups. C'est probablement le bon choix pour un pet de bureau qui doit rester visible en permanence.

Pour un desktop-pet, **ne pas passer par `addChrome`/`addTopChrome`** si vous voulez rester non-interactif (voir §1.5) : ces fonctions enregistrent l'acteur dans `_trackedActors`, ce qui n'est utile que si vous voulez influer sur les struts de fenêtre — inutile pour un sprite flottant.

**Technique de spritesheet — recommandation** [CERTAIN pour le support CSS, recommandation raisonnée pour le choix] :
`St.Widget` avec **CSS inline** `background-image` + `background-position` est la technique la plus simple et directement supportée par le moteur de style de St. Vérifié en lisant le parseur CSS natif (`src/st/st-theme-node.c`, mutter) : les propriétés `background-image`, `background-position` (paire x/y, y compris négative), `background-repeat` et `background-size` (`auto`, `contain`, `cover`, ou taille fixe) sont bien implémentées nativement dans St — ce n'est pas une astuce fragile, c'est un sous-ensemble CSS officiellement pris en charge par le moteur.

Exemple concret : spritesheet de tuiles 64×64, on veut afficher la tuile en colonne 2, ligne 1 (index 0-based) :

```js
const TILE = 64;
const col = 2, row = 1;

const actor = new St.Widget({ width: TILE, height: TILE });
actor.set_style(`
    background-image: url("file:///home/seb/.cache/rustypet/sprite.png");
    background-position: -${col * TILE}px -${row * TILE}px;
    background-repeat: no-repeat;
`);
Main.layoutManager.uiGroup.add_child(actor);
actor.set_position(400, 300);
```

Changer d'animation/frame = ré-appeler `actor.set_style()` avec un nouveau décalage `background-position` (ou ne changer que cette propriété via un style_class + `-st-background-position` custom si vous voulez éviter de réécrire toute la chaîne — mais le remplacement complet de `set_style()` reste la voie la plus simple et documentée).

**Alternative `Clutter.Image`/`St.ImageContent`** [À VÉRIFIER, non recommandé] : possible en théorie (`actor.set_content(image)`), mais Clutter ne fournit pas de mécanisme simple de « viewport »/recadrage d'une sous-région d'une texture unique exposé au niveau JS — `content-box`/`content-gravity` contrôlent l'étirement du contenu dans les limites de l'acteur, pas un recadrage à une région arbitraire du fichier source. Il faudrait soit découper le spritesheet en textures séparées par tuile au chargement (coûteux, complexité inutile), soit passer par un `Clutter.Canvas` avec dessin Cairo manuel. Pour du sprite-tuile simple, **la voie CSS `background-position` est la technique effectivement utilisée dans l'écosystème des extensions GNOME Shell** et doit être préférée ici (KISS).

### 1.4 Charger un PNG depuis le disque [CERTAIN]

Avec la technique CSS ci-dessus, le chargement se fait simplement via une URI `file://` dans `background-image` — St/Cogl gère le décodage et la mise en texture en interne, aucune étape manuelle de chargement GdkPixbuf nécessaire :

```js
actor.set_style(`background-image: url("file://${GLib.get_home_dir()}/.cache/rustypet/sprite.png");`);
```

Attention : le chemin doit être une URI valide (espaces et caractères spéciaux à encoder si besoin — `GLib.filename_to_uri()` est l'outil correct pour construire cette chaîne proprement plutôt que de la concaténer à la main).

### 1.5 Interactivité souris / transparence aux clics [CERTAIN pour le mécanisme de base, nuance importante sur le mécanisme précis]

**Rendre réactif** : standard Clutter, aucune spécificité St.

```js
actor.reactive = true;
actor.connect('button-press-event', (a, event) => {
    const [x, y] = event.get_coords();
    // ...
    return Clutter.EVENT_STOP;
});
```
Pour le glisser (drag), Clutter fournit `Clutter.DragAction` (via `actor.add_action(new Clutter.DragAction())`, signaux `drag-begin`/`drag-motion`/`drag-end`), ou implémentation manuelle avec `button-press-event` + `motion-event` + `button-release-event` et `Clutter.grab()`.

**Rendre transparent aux clics** : mettre `actor.reactive = false`. C'est le comportement Clutter standard : un acteur non-réactif n'est jamais sélectionné par le « picking » du moteur de rendu, donc les événements pointeur traversent virtuellement l'acteur et atteignent ce qui est en dessous (fenêtre du bureau, etc.).

**Nuance à vérifier empiriquement** [À VÉRIFIER] : l'énoncé de la question mentionne une « input region vide » façon ancienne API GNOME Shell (`global.set_stage_input_region`). En lisant le `layout.js` actuel de la branche `main` (donc représentatif de GNOME 50), la fonction `_updateRegions()` de `LayoutManager` ne construit plus qu'une liste de **struts** (réservation d'espace pour le gestionnaire de fenêtres) — je n'ai trouvé aucun appel à une construction de région d'entrée globale dans ce fichier. Le mécanisme de « input region » historique semble avoir été remplacé/absorbé par le picking natif de Clutter au niveau de la scène (acteur réactif ou non, ordre de pile). **Conclusion pratique pour votre plan** : pour un acteur de pet non-interactif, il suffit très probablement de (a) ne pas passer par `addChrome`/`addTopChrome` (juste `uiGroup.add_child()`) et (b) laisser `reactive: false` (valeur par défaut de `St.Widget` de toute façon). Mais testez concrètement les clics à travers le sprite en session imbriquée avant de considérer ce point acquis — le comportement exact de picking multi-acteurs superposés (uiGroup au-dessus de window_group) mérite une vérification empirique sur votre GNOME 50.3 précis.

### 1.6 Énumération des fenêtres — confirmé faisable [CERTAIN]

```js
global.get_window_actors().forEach(windowActor => {
    const metaWindow = windowActor.meta_window;
    const rect = metaWindow.get_frame_rect(); // Mtk.Rectangle {x, y, width, height}
});
```

Signaux confirmés en lisant directement le code source C (`g_signal_new`) :
- `global.display.connect('restacked', ...)` — confirmé dans `src/core/display.c` (mutter), signal `RESTACKED` déclaré sans argument.
- `metaWindow.connect('position-changed', ...)` — confirmé dans `src/core/window.c` (mutter).
- `metaWindow.connect('size-changed', ...)` — confirmé dans `src/core/window.c` (mutter), émis « quand la taille du top-level ou de la fenêtre cliente a changé ».

C'est donc bien exploitable pour une future fonctionnalité (pet qui évite les fenêtres, etc.), mais hors scope immédiat.

---

## 2. D-Bus entre Rust et l'extension

Architecture retenue : **`petd` (Rust) possède le nom D-Bus et émet les signaux ; l'extension JS est cliente** (elle crée un proxy et s'abonne au signal). C'est le sens naturel et le plus simple : pas besoin d'exporter d'interface côté JS pour le flux de position.

### 2.1 Côté extension JS [CERTAIN — gjs.guide/guides/gio/dbus.html]

**Consommer un service (cas de `petd`)** — proxy bas niveau asynchrone, recommandé pour du code d'extension (pas de blocage du thread principal du shell) :

```js
import Gio from 'gi://Gio';

const proxy = await Gio.DBusProxy.new_for_bus(
    Gio.BusType.SESSION,
    Gio.DBusProxyFlags.NONE,
    null,
    'dev.yrbane.RustyPet',
    '/dev/yrbane/RustyPet',
    'dev.yrbane.RustyPet1',
    null);

proxy.connectSignal('PetState', (_proxy, _senderName, args) => {
    const [x, y, tileIndex] = args; // déjà dépaqueté (deepUnpack)
    // déplacer l'acteur ici
});
```

`connectSignal()`/`disconnectSignal()` sont les méthodes dédiées sur le wrapper de proxy (ne pas confondre avec `connect()` GObject, qui sert pour `g-properties-changed` etc.). Alternative bas niveau sans proxy : `Gio.DBus.session.signal_subscribe(busName, iface, signalName, objectPath, null, Gio.DBusSignalFlags.NONE, callback)`.

**Exporter une interface côté JS** (si un jour l'extension doit, elle aussi, exposer un service — pas nécessaire pour ce plan initial mais utile à savoir) :

```js
const interfaceXml = `
<node>
  <interface name="dev.yrbane.RustyPet1">
    <method name="Ping"/>
    <signal name="PetState">
      <arg name="x" type="i"/>
      <arg name="y" type="i"/>
      <arg name="tile" type="u"/>
    </signal>
  </interface>
</node>`;

class Service {
    Ping() { /* ... */ }
    emitState(x, y, tile) {
        this._impl.emit_signal('PetState',
            new GLib.Variant('(iiu)', [x, y, tile]));
    }
}

const service = new Service();
service._impl = Gio.DBusExportedObject.wrapJSObject(interfaceXml, service);
service._impl.export(Gio.DBus.session, '/dev/yrbane/RustyPet');
```

`Gio.DBusExportedObject.wrapJSObject(xml, instance)` est **la** fonction de convenance GJS documentée pour ce cas ; l'objet retourné (`_impl` par convention) et l'instance JS sont deux objets distincts — c'est `_impl.emit_signal(name, variant)` qui émet réellement le signal sur le bus.

### 2.2 Côté Rust — zbus [CERTAIN, version vérifiée sur docs.rs le jour même]

**Crate recommandée : `zbus`, version actuelle `5.18.0`** (docs.rs, publiée 17 juillet 2026). Pas de `dbus-rs`/`dbus-tokio` : `zbus` est l'implémentation D-Bus Rust moderne, pure-Rust, async-first, et c'est celle que recommande l'écosystème freedesktop actuellement.

**Attribut de macro correct pour cette version : `#[zbus::interface(...)]`** (pas `#[dbus_interface]`, qui est l'ancien nom pré-4.0, absent de la 5.x). Émission de signal : méthode déclarée **sans corps**, annotée `#[zbus(signal)]`, prenant un `&SignalEmitter<'_>` en premier paramètre — la macro génère le code d'émission.

Exemple minimal (service qui expose une méthode ET émet un signal), runtime **tokio** (recommandé — zbus a un support tokio de première classe ; l'alternative par défaut est son propre reactor basé sur `async-io`, qui marche aussi mais lance son propre thread interne, redondant si votre démon utilise déjà tokio) :

```rust
use std::error::Error;
use tokio::time::{interval, Duration};
use zbus::{connection, interface, object_server::SignalEmitter};

struct Pet {
    x: i32,
    y: i32,
    tile: u32,
}

#[interface(name = "dev.yrbane.RustyPet1")]
impl Pet {
    async fn ping(&self) -> &str {
        "pong"
    }

    #[zbus(signal)]
    async fn pet_state(
        signal_emitter: &SignalEmitter<'_>,
        x: i32,
        y: i32,
        tile: u32,
    ) -> zbus::Result<()>;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let pet = Pet { x: 0, y: 0, tile: 0 };

    let conn = connection::Builder::session()?
        .name("dev.yrbane.RustyPet")?
        .serve_at("/dev/yrbane/RustyPet", pet)?
        .build()
        .await?;

    let iface_ref = conn
        .object_server()
        .interface::<_, Pet>("/dev/yrbane/RustyPet")
        .await?;

    let mut tick = interval(Duration::from_millis(66)); // ~15 Hz
    loop {
        tick.tick().await;
        let emitter = iface_ref.signal_emitter();
        Pet::pet_state(emitter, 100, 200, 3).await?;
    }
}
```

`Cargo.toml` (à ajouter dans `[workspace.dependencies]`, cohérent avec le style déjà présent dans `rustypet/Cargo.toml`) :

```toml
zbus = { version = "5", default-features = false, features = ["tokio"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
```

Désactiver les features par défaut de `zbus` (qui incluent `async-io`) et activer `tokio` évite de faire tourner deux exécuteurs/reactors concurrents dans le même process — recommandation explicite de la doc zbus (« No threads launched behind your back by zbus now »).

### 2.3 Débit du flux position/tuile (10–20 Hz) [CERTAIN sur le mécanisme, raisonnement pour le dimensionnement]

Le bus de session D-Bus **supporte sans difficulté** 10–20 signaux/s : c'est un ordre de grandeur trivial comparé à des usages réels du bus (notifications desktop, indicateurs système, IPC Flatpak portals) qui tournent déjà en continu sans latence perceptible. Précautions pratiques :
- **Utiliser un signal, pas un appel de méthode** : un signal est du fire-and-forget (pas d'aller-retour, pas de blocage si personne n'écoute) — c'est le bon choix ici, conforme au flux « émission continue » demandé.
- **Garder le payload minimal** : `(i i u)` (x, y, tile-index) = quelques octets utiles ; évitez d'y mettre des chaînes ou structures larges. Le poids fixe (« corps ») d'un message D-Bus (en-tête + padding d'alignement) reste de l'ordre de quelques dizaines d'octets, négligeable à 15–20 Hz.
- **Ne pas dépasser largement 20–30 Hz sans raison** : au-delà, le gain visuel (interpolation d'un sprite discret par tuiles) est nul, et vous ajoutez du trafic sur un bus partagé par toute la session (d'autres apps y écoutent/y émettent aussi).
- Si vous deviez un jour transporter une image (pas nécessaire ici, le PNG est déjà en cache local partagé par chemin de fichier) : D-Bus supporte le passage de file descriptors (`UNIX_FD`), à privilégier plutôt que d'embarquer des octets bruts dans le message.

### 2.4 Activation du service [QUASI-CERTAIN — mécanisme confirmé sur la spec officielle, syntaxe exacte des clés = convention stable non re-citée verbatim cette session]

**Enregistrement du nom** : côté Rust, c'est `connection::Builder::session()?.name("dev.yrbane.RustyPet")?...` (voir §2.2) — rien de plus, `zbus` gère la requête `RequestName` en interne.

**Démarrage automatique par l'extension si le démon n'est pas lancé** — deux options :
1. **Lancement simple du process** depuis l'extension (`Gio.Subprocess` ou `GLib.spawn_async`) — le plus simple à mettre en œuvre pour un premier plan, contrôle total, pas de fichier à installer.
2. **Activation D-Bus déclarative** via un fichier `.service` — plus « propre » au sens desktop Linux standard, et gère nativement le cas « deux composants tentent de démarrer le démon en même temps » (le bus daemon dé-duplique). Confirmé par la spécification D-Bus officielle (dbus-specification.html, section « Message Bus Starting Services (Activation) ») : le bus cherche des *fichiers de description de service* dans des répertoires standards, et le lance automatiquement dès qu'un client tente de joindre le nom bien connu sans le flag `NO_AUTO_START`. Dans ce cas, **`Gio.DBusProxy.new_for_bus()`** suffit côté extension : la simple construction du proxy déclenche l'activation si le nom n'a pas encore de propriétaire (comportement par défaut, sauf si on passe explicitement `Gio.DBusProxyFlags.DO_NOT_AUTO_START`).

Emplacement (bus **session**, utilisateur) : `~/.local/share/dbus-1/services/dev.yrbane.RustyPet.service`

Forme du fichier (format clé=valeur de type desktop-entry — convention stable du écosystème D-Bus/freedesktop, cf. mêmes principes que `SystemdService` documenté dans la spec) :

```ini
[D-BUS Service]
Name=dev.yrbane.RustyPet
Exec=/home/seb/.local/bin/petd
```

`Exec` doit être un **chemin absolu** vers le binaire (pas de recherche dans `$PATH` garantie par tous les bus daemons). Le process lancé reçoit `DBUS_STARTER_ADDRESS` (et `DBUS_STARTER_BUS_TYPE=session`) en variables d'environnement — confirmé par la spec — utile si `petd` doit détecter qu'il a été activé par le bus plutôt que lancé manuellement.

Pour un premier plan (MVP), l'option 1 (lancement simple par l'extension) est suffisante et plus rapide à itérer ; le fichier `.service` peut être ajouté ensuite sans rien changer côté Rust.

---

## 3. Test et vérification

### 3.1 Session imbriquée / devkit [CERTAIN]

```sh
dbus-run-session gnome-shell --devkit --wayland
```
(rappel : `--devkit` remplace `--nested` depuis GNOME 49 ; vous êtes en 50.3, donc `--devkit` est la forme correcte — ne pas suivre d'anciens tutoriels qui utilisent encore `--nested`.)

Variables utiles pour plus de verbosité :
```sh
export G_MESSAGES_DEBUG=all
export SHELL_DEBUG=all
```

**Limites confirmées** : ce n'est **pas une isolation complète** (avertissement officiel explicite) — le process partage le système de fichiers hôte (donc `~/.local/share/gnome-shell/extensions/...` est identique dans les deux sessions, pratique), mais le bus D-Bus de session, lui, **est nouveau/isolé** pour cette instance imbriquée — ce qui est en fait un avantage pour vous : `petd` lancé depuis un terminal de la session imbriquée (ou avec le bon `DBUS_SESSION_BUS_ADDRESS` exporté) ne rentre pas en collision avec un `petd` déjà actif sur votre session hôte réelle.

**Lecture des logs** :
```sh
journalctl -f -o cat /usr/bin/gnome-shell
```
(confirmé dans gjs.guide/extensions/development/creating.html, section test X11 — s'applique identiquement en observant les logs de l'instance imbriquée si elle tourne sous systemd/journald ; sinon la sortie standard du terminal qui a lancé `dbus-run-session gnome-shell --devkit --wayland` affiche déjà tous les messages).

Alternative interactive : **Looking Glass** (`Alt+F2` puis `lg` dans la session imbriquée) — inspecteur intégré, pas un debugger pas-à-pas, mais montre la liste des extensions et leurs erreurs.

Pour un crash dur (segfault), `SHELL_DEBUG=backtrace-segfaults` imprime la pile JS avant de quitter.

### 3.2 Tester la logique JS hors GNOME Shell [CERTAIN]

`gjs-console` (ou `gjs` en ligne de commande) lance un moteur GJS **autonome**, sans accès au process `gnome-shell` ni aux modules `resource:///org/gnome/shell/...` — donc utilisable uniquement pour la logique pure ne dépendant pas de `St`/`Clutter`/`Main` (ex. : calcul de trajectoire, parsing, logique de sélection de tuile). Confirmé explicitement (« The GJS console is a separate process, without access to the gnome-shell process or the ability to import JavaScript modules used by extensions »). Pour tester une logique qui dépend de `Main`/`St`/`Clutter`, il faut passer par une vraie instance shell (imbriquée ou non) — pas de mock possible en dehors.

```sh
$ gjs-console
gjs> import('./pet-logic.js').then(m => console.log(m.pickNextTile(...)));
```

---

## Décisions à figer pour le plan d'implémentation

1. **Rendu du sprite** : `St.Widget` ajouté directement à `Main.layoutManager.uiGroup` (pas `addChrome`), texture via CSS inline `background-image` + `background-position` (technique confirmée nativement supportée par St, `src/st/st-theme-node.c`).
2. **Flux D-Bus** : `petd` possède `dev.yrbane.RustyPet` et **émet un signal** `PetState(x: i32, y: i32, tile: u32)` à ~15 Hz ; l'extension consomme via `Gio.DBusProxy.new_for_bus()` + `connectSignal()`. Pas d'appel de méthode pour le flux continu.
3. **zbus 5.18.0**, macro `#[interface(name = "...")]` + méthode signal `#[zbus(signal)]` avec `&SignalEmitter<'_>`, runtime **tokio** (`zbus` avec `default-features = false, features = ["tokio"]`).
4. **Rechargement dev sous Wayland/GNOME 50** : `dbus-run-session gnome-shell --devkit --wayland` (pas `--nested`, obsolète depuis GNOME 49) ; logs via `journalctl -f -o cat /usr/bin/gnome-shell` ou sortie du terminal de la session imbriquée.
5. **Point ouvert à vérifier avant de coder l'interactivité** : le mécanisme exact de « click-through » en GNOME 50 (le code de `layout.js` ne montre plus de construction explicite de région d'entrée globale, seulement des struts) — tester empiriquement `reactive: false` + acteur hors chrome en session imbriquée avant de considérer ce point acquis pour une future itération interactive.
