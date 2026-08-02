// Calculs purs de l'extension, sans dépendance à GNOME Shell — testables via
// gjs-console. Découpe des tuiles en ligne d'abord : index = row*columns + col.

/**
 * Décalage `background-position` (en pixels, négatif) affichant la tuile
 * d'index `tile` d'un spritesheet de `columns` tuiles par ligne.
 */
export function tileBackgroundPosition(tile, tileW, tileH, columns) {
    const cols = Math.max(1, columns | 0);
    const t = Math.max(0, tile | 0);
    const col = t % cols;
    const row = Math.floor(t / cols);
    return { x: -col * tileW, y: -row * tileH };
}

/** Borne une opacité 0–255 en entier valide pour Clutter. */
export function clutterOpacity(opacity255) {
    const v = Math.round(opacity255);
    if (v < 0) return 0;
    if (v > 255) return 255;
    return v;
}

/**
 * Vrai si deux listes de rectangles de fenêtres [x, y, largeur, hauteur]
 * diffèrent. `previous` peut être null (rien encore envoyé : toujours vrai).
 */
export function windowRectsChanged(previous, current) {
    if (previous === null || previous === undefined) return true;
    if (previous.length !== current.length) return true;
    return previous.some((rect, i) => rect.some((v, j) => v !== current[i][j]));
}

/**
 * Chemin du pet à charger : celui de la config s'il désigne un fichier
 * existant (`exists` est injecté pour rester testable), sinon le repli.
 */
export function chosenPetPath(configText, fallback, exists) {
    const path = (configText ?? '').trim();
    if (path !== '' && exists(path)) return path;
    return fallback;
}
