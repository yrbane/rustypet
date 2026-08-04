import St from 'gi://St';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GdkPixbuf from 'gi://GdkPixbuf';
import Meta from 'gi://Meta';
import Clutter from 'gi://Clutter';

import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import {
    tileBackgroundPosition, clutterOpacity, windowRectsChanged, chosenPetPath,
    dragSendDue,
} from './petMath.js';

// Au plus un envoi de position D-Bus tous les 33 ms pendant le glisser.
const DRAG_SEND_GAP_MS = 33;

// new_for_bus est une fonction async C (callback en dernier argument) :
// la promisification est indispensable pour pouvoir l'await-er.
Gio._promisify(Gio.DBusProxy, 'new_for_bus', 'new_for_bus_finish');

const BUS_NAME = 'dev.yrbane.RustyPet';
const OBJECT_PATH = '/dev/yrbane/RustyPet';
const IFACE = 'dev.yrbane.RustyPet1';

export default class RustyPetExtension extends Extension {
    enable() {
        // Un widget par acteur du troupeau (le principal, puis les enfants) :
        // chaque entrée est { actor, sheet }.
        this._actors = [];
        this._sheetInfo = null;
        this._proxy = null;
        this._signalId = 0;
        this._subprocess = null;
        this._tileW = 0;
        this._tileH = 0;
        this._columns = 1;
        this._windowsId = 0;
        this._lastWindows = null;
        // État du glisser-déposer du pet principal.
        this._grab = null;
        this._stageHandlerId = 0;
        this._lastDragSent = null;
        // Drapeau de démontage : protège _connect() contre une reprise
        // après un disable() survenu pendant son await.
        this._destroyed = false;

        this._startDaemon();
        // Laisse au démon le temps de prendre le nom de bus, puis se connecte.
        this._connectId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 600, () => {
            this._connect().catch(e => logError(e, 'RustyPet: connexion'));
            this._connectId = 0;
            return GLib.SOURCE_REMOVE;
        });
    }

    _daemonPath() {
        // Binaire installé, sinon build de développement du dépôt.
        const installed = GLib.build_filenamev([GLib.get_home_dir(), '.local', 'bin', 'petd']);
        if (GLib.file_test(installed, GLib.FileTest.IS_EXECUTABLE)) return installed;
        return GLib.build_filenamev(
            [GLib.get_home_dir(), 'Dev', 'rustypet', 'target', 'debug', 'petd']);
    }

    // Pet à charger : le chemin écrit dans ~/.config/rustypet/pet s'il
    // désigne un animations.xml existant, sinon le neko par défaut.
    _petXmlPath() {
        const fallback = GLib.build_filenamev(
            [GLib.get_home_dir(), 'Dev', 'desktopPet', 'Pets', 'neko', 'animations.xml']);
        const config = GLib.build_filenamev(
            [GLib.get_user_config_dir(), 'rustypet', 'pet']);
        let text = null;
        try {
            const [ok, bytes] = GLib.file_get_contents(config);
            if (ok) text = new TextDecoder().decode(bytes);
        } catch {
            // Pas de config : pet par défaut.
        }
        return chosenPetPath(text, fallback,
            p => GLib.file_test(p, GLib.FileTest.EXISTS));
    }

    _startDaemon() {
        const petXml = this._petXmlPath();
        try {
            this._subprocess = Gio.Subprocess.new(
                [this._daemonPath(), petXml],
                Gio.SubprocessFlags.NONE);
        } catch (e) {
            logError(e, 'RustyPet: lancement de petd');
        }
    }

    async _connect() {
        // Ne pas assigner directement this._proxy ici : tant que le drapeau
        // _destroyed n'a pas été vérifié après l'await, aucune ressource ne
        // doit être conservée sur l'instance (sinon disable() ne peut pas la
        // voir si elle s'exécute pendant la suspension).
        const proxy = await Gio.DBusProxy.new_for_bus(
            Gio.BusType.SESSION, Gio.DBusProxyFlags.NONE, null,
            BUS_NAME, OBJECT_PATH, IFACE, null);

        // L'extension a été désactivée pendant l'attente du proxy : on
        // abandonne proprement sans rien créer (pas d'acteur zombie).
        if (this._destroyed) return;

        this._proxy = proxy;

        // Géométrie du moniteur primaire + zone de travail.
        const monitor = Main.layoutManager.primaryMonitor;
        const wa = Main.layoutManager.getWorkAreaForMonitor(Main.layoutManager.primaryIndex);
        this._proxy.call_sync(
            'Configure',
            new GLib.Variant('(iiiiii)',
                [monitor.width, monitor.height, wa.x, wa.y, wa.width, wa.height]),
            Gio.DBusCallFlags.NONE, -1, null);

        // Dimensions du sprite.
        const res = this._proxy.call_sync(
            'GetSprite', null, Gio.DBusCallFlags.NONE, -1, null);
        const [sheetPath, tileW, tileH, columns] = res.deepUnpack();
        this._tileW = tileW;
        this._tileH = tileH;
        this._columns = columns;

        const uri = GLib.filename_to_uri(sheetPath, null);
        // St ne supporte pas background-position : on clippe un conteneur à la
        // taille d'une tuile et on déplace la planche entière à l'intérieur.
        // Dimensions naturelles de la planche lues dans l'en-tête du PNG.
        const [, sheetW, sheetH] = GdkPixbuf.Pixbuf.get_file_info(sheetPath);
        this._sheetInfo = { uri, sheetW, sheetH };

        // connectSignal ne relaie les signaux que sur les proxys issus de
        // makeProxyWrapper ; sur un proxy nu, on écoute g-signal directement.
        this._signalId = this._proxy.connect('g-signal', (_p, _sender, name, params) => {
            if (name === 'PetState') this._onState(params.deepUnpack());
        });

        // Remonte la géométrie des fenêtres : le pet marche sur leurs toits.
        this._windowsId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 500, () => {
            this._pushWindows();
            return GLib.SOURCE_CONTINUE;
        });
    }

    // Fenêtres visibles [x, y, largeur, hauteur] du bureau courant. Une
    // fenêtre plein écran neutralise la marche (liste vide), comme l'original.
    _collectWindows() {
        const rects = [];
        for (const actor of global.get_window_actors()) {
            const win = actor.meta_window;
            if (!win || win.minimized) continue;
            if (win.get_window_type() !== Meta.WindowType.NORMAL) continue;
            if (win.is_fullscreen()) return [];
            const r = win.get_frame_rect();
            rects.push([r.x, r.y, r.width, r.height]);
        }
        return rects;
    }

    // Envoie les fenêtres au démon, seulement quand elles ont changé.
    _pushWindows() {
        if (!this._proxy) return;
        const rects = this._collectWindows();
        if (!windowRectsChanged(this._lastWindows, rects)) return;
        this._lastWindows = rects;
        this._proxy.call(
            'UpdateWindows',
            new GLib.Variant('(a(iiii))', [rects]),
            Gio.DBusCallFlags.NONE, -1, null,
            (proxy, res) => {
                try {
                    proxy.call_finish(res);
                } catch (e) {
                    logError(e, 'RustyPet: UpdateWindows');
                }
            });
    }

    // Crée le widget clippé d'un acteur (conteneur à la taille d'une tuile,
    // planche entière déplacée à l'intérieur). Seul le pet principal est
    // réactif : on peut l'attraper à la souris.
    _makeActor(isMain) {
        const { uri, sheetW, sheetH } = this._sheetInfo;
        const actor = new St.Widget({
            reactive: isMain, width: this._tileW, height: this._tileH,
            clip_to_allocation: true,
        });
        actor.set_pivot_point(0.5, 0.5);
        const sheet = new St.Widget({
            width: sheetW, height: sheetH,
            style: `background-image: url("${uri}");`,
        });
        actor.add_child(sheet);
        Main.layoutManager.uiGroup.add_child(actor);
        if (isMain)
            actor.connect('button-press-event', () => this._beginDrag(actor));
        return { actor, sheet };
    }

    // Attrape le pet : physique suspendue côté démon, suivi du pointeur en
    // local (aucune latence) + envois D-Bus throttlés.
    _beginDrag(actor) {
        if (this._grab || !this._proxy) return Clutter.EVENT_PROPAGATE;
        this._call('BeginDrag', null);
        this._grab = global.stage.grab(actor);
        this._stageHandlerId = global.stage.connect('captured-event', (_s, event) => {
            const type = event.type();
            if (type === Clutter.EventType.MOTION) {
                const [x, y] = event.get_coords();
                actor.set_position(Math.round(x) - this._tileW / 2, Math.round(y) - 2);
                const now = GLib.get_monotonic_time() / 1000;
                if (dragSendDue(this._lastDragSent, now, DRAG_SEND_GAP_MS)) {
                    this._lastDragSent = now;
                    this._call('DragTo',
                        new GLib.Variant('(ii)', [Math.round(x), Math.round(y)]));
                }
                return Clutter.EVENT_STOP;
            }
            if (type === Clutter.EventType.BUTTON_RELEASE) {
                this._endDrag();
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });
        return Clutter.EVENT_STOP;
    }

    _endDrag() {
        if (this._stageHandlerId) {
            global.stage.disconnect(this._stageHandlerId);
            this._stageHandlerId = 0;
        }
        this._grab?.dismiss();
        this._grab = null;
        this._lastDragSent = null;
        this._call('EndDrag', null);
    }

    // Appel D-Bus « fire and forget », erreurs journalisées.
    _call(method, args) {
        this._proxy?.call(method, args, Gio.DBusCallFlags.NONE, -1, null,
            (proxy, res) => {
                try {
                    proxy.call_finish(res);
                } catch (e) {
                    logError(e, `RustyPet: ${method}`);
                }
            });
    }

    _onState(args) {
        if (!this._sheetInfo || this._destroyed) return;
        const [actors] = args;
        // Ajuste le nombre de widgets au nombre d'acteurs reçus.
        while (this._actors.length < actors.length)
            this._actors.push(this._makeActor(this._actors.length === 0));
        while (this._actors.length > actors.length)
            this._actors.pop().actor.destroy();

        actors.forEach(([x, y, tile, flipped, opacity], i) => {
            const { actor, sheet } = this._actors[i];
            const pos = tileBackgroundPosition(tile, this._tileW, this._tileH, this._columns);
            sheet.set_position(pos.x, pos.y);
            // Pendant le glisser, la position du principal est pilotée en
            // local par le pointeur ; le démon ne fait que confirmer.
            if (i !== 0 || !this._grab)
                actor.set_position(x, y);
            actor.scale_x = flipped ? -1 : 1;
            actor.opacity = clutterOpacity(opacity);
        });
    }

    disable() {
        // Marque l'extension comme démontée avant tout, afin qu'une
        // _connect() en cours (suspendue sur son await) se sache obsolète
        // dès qu'elle reprendra, et n'assigne ni proxy ni acteur.
        this._destroyed = true;
        // Un glisser en cours est abandonné : le démon va être arrêté, seul
        // le nettoyage local (grab et handler de scène) importe.
        if (this._stageHandlerId) {
            global.stage.disconnect(this._stageHandlerId);
            this._stageHandlerId = 0;
        }
        this._grab?.dismiss();
        this._grab = null;
        if (this._connectId) { GLib.source_remove(this._connectId); this._connectId = 0; }
        if (this._windowsId) { GLib.source_remove(this._windowsId); this._windowsId = 0; }
        this._lastWindows = null;
        if (this._proxy && this._signalId) {
            this._proxy.disconnect(this._signalId);
            this._signalId = 0;
        }
        this._proxy = null;
        // Les planches sont des enfants des acteurs : détruites avec eux.
        for (const { actor } of this._actors) actor.destroy();
        this._actors = [];
        this._sheetInfo = null;
        // Arrête le démon lancé par l'extension.
        this._subprocess?.force_exit();
        this._subprocess = null;
    }
}
