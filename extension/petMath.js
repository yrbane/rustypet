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
