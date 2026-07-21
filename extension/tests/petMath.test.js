// Test autonome, lancé par gjs-console (pas besoin de GNOME Shell).
// Sortie : « OK » et code 0 si tout passe ; sinon lève et code non nul.

import { tileBackgroundPosition, clutterOpacity } from '../petMath.js';

function assertEq(actual, expected, label) {
    const a = JSON.stringify(actual);
    const e = JSON.stringify(expected);
    if (a !== e) throw new Error(`${label} : attendu ${e}, obtenu ${a}`);
}

// Tuile 0 → coin haut-gauche.
assertEq(tileBackgroundPosition(0, 64, 64, 4), { x: 0, y: 0 }, 'tuile 0');
// Tuile 1 → une colonne à droite.
assertEq(tileBackgroundPosition(1, 64, 64, 4), { x: -64, y: 0 }, 'tuile 1');
// Tuile 4 → ligne suivante (4 colonnes) : ligne d'abord.
assertEq(tileBackgroundPosition(4, 64, 64, 4), { x: 0, y: -64 }, 'tuile 4');
// Tuile 6 → ligne 1, colonne 2.
assertEq(tileBackgroundPosition(6, 32, 48, 4), { x: -64, y: -48 }, 'tuile 6');
// columns=0 protégé.
assertEq(tileBackgroundPosition(3, 10, 10, 0), { x: 0, y: -30 }, 'columns=0');

assertEq(clutterOpacity(300), 255, 'opacité haute');
assertEq(clutterOpacity(-5), 0, 'opacité basse');
assertEq(clutterOpacity(128), 128, 'opacité milieu');

print('OK');
