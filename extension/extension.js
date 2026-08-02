import St from 'gi://St';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GdkPixbuf from 'gi://GdkPixbuf';

import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import { tileBackgroundPosition, clutterOpacity } from './petMath.js';

// new_for_bus est une fonction async C (callback en dernier argument) :
// la promisification est indispensable pour pouvoir l'await-er.
Gio._promisify(Gio.DBusProxy, 'new_for_bus', 'new_for_bus_finish');

const BUS_NAME = 'dev.yrbane.RustyPet';
const OBJECT_PATH = '/dev/yrbane/RustyPet';
const IFACE = 'dev.yrbane.RustyPet1';

export default class RustyPetExtension extends Extension {
    enable() {
        this._actor = null;
        this._sheet = null;
        this._proxy = null;
        this._signalId = 0;
        this._subprocess = null;
        this._tileW = 0;
        this._tileH = 0;
        this._columns = 1;
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

    _startDaemon() {
        const petXml = GLib.build_filenamev(
            [GLib.get_home_dir(), 'Dev', 'desktopPet', 'Pets', 'neko', 'animations.xml']);
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
        this._actor = new St.Widget({
            reactive: false, width: tileW, height: tileH,
            clip_to_allocation: true,
        });
        this._actor.set_pivot_point(0.5, 0.5);
        this._sheet = new St.Widget({
            width: sheetW, height: sheetH,
            style: `background-image: url("${uri}");`,
        });
        this._actor.add_child(this._sheet);
        Main.layoutManager.uiGroup.add_child(this._actor);

        // connectSignal ne relaie les signaux que sur les proxys issus de
        // makeProxyWrapper ; sur un proxy nu, on écoute g-signal directement.
        this._signalId = this._proxy.connect('g-signal', (_p, _sender, name, params) => {
            if (name === 'PetState') this._onState(params.deepUnpack());
        });
    }

    _onState(args) {
        if (!this._actor) return;
        const [x, y, tile, flipped, opacity] = args;
        const pos = tileBackgroundPosition(tile, this._tileW, this._tileH, this._columns);
        this._sheet.set_position(pos.x, pos.y);
        this._actor.set_position(x, y);
        this._actor.scale_x = flipped ? -1 : 1;
        this._actor.opacity = clutterOpacity(opacity);
    }

    disable() {
        // Marque l'extension comme démontée avant tout, afin qu'une
        // _connect() en cours (suspendue sur son await) se sache obsolète
        // dès qu'elle reprendra, et n'assigne ni proxy ni acteur.
        this._destroyed = true;
        if (this._connectId) { GLib.source_remove(this._connectId); this._connectId = 0; }
        if (this._proxy && this._signalId) {
            this._proxy.disconnect(this._signalId);
            this._signalId = 0;
        }
        this._proxy = null;
        // La planche est un enfant de l'acteur : détruite avec lui.
        this._actor?.destroy();
        this._actor = null;
        this._sheet = null;
        // Arrête le démon lancé par l'extension.
        this._subprocess?.force_exit();
        this._subprocess = null;
    }
}
