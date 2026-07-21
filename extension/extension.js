import St from 'gi://St';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import { tileBackgroundPosition, clutterOpacity } from './petMath.js';

const BUS_NAME = 'dev.yrbane.RustyPet';
const OBJECT_PATH = '/dev/yrbane/RustyPet';
const IFACE = 'dev.yrbane.RustyPet1';

export default class RustyPetExtension extends Extension {
    enable() {
        this._actor = null;
        this._proxy = null;
        this._signalId = 0;
        this._subprocess = null;
        this._tileW = 0;
        this._tileH = 0;
        this._columns = 1;

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
        this._proxy = await Gio.DBusProxy.new_for_bus(
            Gio.BusType.SESSION, Gio.DBusProxyFlags.NONE, null,
            BUS_NAME, OBJECT_PATH, IFACE, null);

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
        this._actor = new St.Widget({ reactive: false, width: tileW, height: tileH });
        this._actor.set_pivot_point(0.5, 0.5);
        this._baseStyle =
            `background-image: url("${uri}"); background-repeat: no-repeat;`;
        this._actor.set_style(this._baseStyle);
        Main.layoutManager.uiGroup.add_child(this._actor);

        this._signalId = this._proxy.connectSignal(
            'PetState', (_p, _s, args) => this._onState(args));
    }

    _onState(args) {
        if (!this._actor) return;
        const [x, y, tile, flipped, opacity] = args;
        const pos = tileBackgroundPosition(tile, this._tileW, this._tileH, this._columns);
        this._actor.set_style(
            `${this._baseStyle} background-position: ${pos.x}px ${pos.y}px;`);
        this._actor.set_position(x, y);
        this._actor.scale_x = flipped ? -1 : 1;
        this._actor.opacity = clutterOpacity(opacity);
    }

    disable() {
        if (this._connectId) { GLib.source_remove(this._connectId); this._connectId = 0; }
        if (this._proxy && this._signalId) {
            this._proxy.disconnectSignal(this._signalId);
            this._signalId = 0;
        }
        this._proxy = null;
        this._actor?.destroy();
        this._actor = null;
        // Arrête le démon lancé par l'extension.
        this._subprocess?.force_exit();
        this._subprocess = null;
    }
}
