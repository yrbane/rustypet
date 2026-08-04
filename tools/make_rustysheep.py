#!/usr/bin/env python3
"""Génère le pet « RustySheep » : le mouton eSheep 64 enrichi de gags inédits.

Compose de nouvelles tuiles 40x40 (danse, joint, superman, crotte, lunettes,
fleur qui pousse, parachute, fusée, acide) par-dessus les sprites d'origine,
étend la spritesheet de 11 à 13 lignes, et produit un animations.xml complet
(base64) dans assets/rustysheep/.

Usage : python3 tools/make_rustysheep.py [chemin/vers/esheep64/animations.xml]
"""

import base64
import colorsys
import io
import math
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

from PIL import Image, ImageDraw

NS = "https://esheep.petrucci.ch/"
TILE = 40
COLS = 16
OLD_ROWS = 11
NEW_ROWS = 14
REPO = Path(__file__).resolve().parent.parent
DEFAULT_SRC = Path.home() / "Dev/desktopPet/Pets/esheep64/animations.xml"


def q(tag: str) -> str:
    """Nom qualifié dans l'espace de noms eSheep."""
    return f"{{{NS}}}{tag}"


def load_sheet(root: ET.Element) -> Image.Image:
    """Décode la spritesheet d'origine en RGBA (magenta -> transparent)."""
    png_b64 = root.find(f"{q('image')}/{q('png')}").text
    png_b64 = re.sub(r"\s+", "", png_b64)
    png_b64 += "=" * (-len(png_b64) % 4)
    img = Image.open(io.BytesIO(base64.b64decode(png_b64))).convert("RGBA")
    data = [
        (0, 0, 0, 0) if (p[0], p[1], p[2]) == (255, 0, 255) else p
        for p in img.getdata()
    ]
    img.putdata(data)
    return img


def tile_of(sheet: Image.Image, index: int) -> Image.Image:
    x, y = (index % COLS) * TILE, (index // COLS) * TILE
    return sheet.crop((x, y, x + TILE, y + TILE))


def on_canvas(sprite: Image.Image, dx: int = 0, dy: int = 0) -> Image.Image:
    """Pose un sprite (éventuellement transformé) sur une tuile vierge."""
    canvas = Image.new("RGBA", (TILE, TILE), (0, 0, 0, 0))
    canvas.alpha_composite(sprite, (dx, dy))
    return canvas


def rotated(sprite: Image.Image, angle: float) -> Image.Image:
    return sprite.rotate(angle, resample=Image.NEAREST, expand=False)


def note(draw: ImageDraw.ImageDraw, x: int, y: int, color=(40, 40, 220, 255)):
    """Petite note de musique pixel-art."""
    draw.ellipse((x, y + 5, x + 4, y + 8), fill=color)
    draw.line((x + 4, y + 6, x + 4, y - 2), fill=color, width=1)
    draw.line((x + 4, y - 2, x + 7, y), fill=color, width=1)


def hue_shift(sprite: Image.Image, shift: float) -> Image.Image:
    """Décale la teinte des pixels opaques (trip psychédélique)."""
    out = sprite.copy()
    px = out.load()
    for yy in range(out.height):
        for xx in range(out.width):
            r, g, b, a = px[xx, yy]
            if a == 0:
                continue
            h, l, s = colorsys.rgb_to_hls(r / 255, g / 255, b / 255)
            r2, g2, b2 = colorsys.hls_to_rgb((h + shift) % 1.0, l, max(s, 0.6))
            px[xx, yy] = (int(r2 * 255), int(g2 * 255), int(b2 * 255), a)
    return out


def build_tiles(sheet: Image.Image) -> list[Image.Image]:
    """Compose les 29 tuiles inédites, dans l'ordre des index 176+."""
    side = tile_of(sheet, 0)      # mouton de profil, face à gauche
    walk2 = tile_of(sheet, 2)     # pas de marche
    front = tile_of(sheet, 34)    # mouton de face, yeux ouverts
    rear = tile_of(sheet, 12)     # mouton de dos
    graze = tile_of(sheet, 33)    # tête baissée qui broute
    tiles: list[Image.Image] = []

    # --- dance (176-179) : déhanché + notes de musique ---
    for i, angle in enumerate((-14, 0, 14, 0)):
        t = on_canvas(rotated(front, angle))
        d = ImageDraw.Draw(t)
        if i % 2 == 0:
            note(d, 2, 6)
            note(d, 32, 12, (200, 30, 160, 255))
        else:
            note(d, 30, 4)
        tiles.append(t)

    # --- smoke (180-183) : joint et volutes qui montent ---
    for i in range(4):
        t = on_canvas(front)
        d = ImageDraw.Draw(t)
        # joint blanc au coin de la bouche, braise orange
        d.line((22, 25, 29, 21), fill=(245, 245, 235, 255), width=2)
        d.point((29, 21), fill=(255, 120, 30, 255))
        d.point((30, 20), fill=(255, 60, 20, 255))
        # volutes grises de plus en plus hautes
        for k in range(i + 1):
            r = 2 + k
            cx, cy = 31 + (k % 2) * 2, 16 - k * 5
            d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=(190, 190, 200, 180))
        tiles.append(t)

    # --- superman (184-185) : cape rouge, envol en diagonale ---
    flying = walk2.resize((30, 30), Image.NEAREST)
    for flap in (0, 4):
        t = Image.new("RGBA", (TILE, TILE), (0, 0, 0, 0))
        d = ImageDraw.Draw(t)
        # cape qui flotte derrière le corps (il vole vers la gauche/haut)
        d.polygon(
            [(20, 16), (39, 24 + flap), (37, 32 + flap), (18, 24)],
            fill=(210, 30, 30, 255),
        )
        d.polygon(
            [(21, 18), (36, 25 + flap), (35, 28 + flap), (20, 22)],
            fill=(240, 60, 50, 255),
        )
        t.alpha_composite(on_canvas(rotated(flying, 32), -4, 2))
        tiles.append(t)

    # --- poop (186-188) : accroupi, petite crotte, fierté ---
    squat = rear.resize((TILE, 34), Image.NEAREST)
    for i in range(3):
        t = Image.new("RGBA", (TILE, TILE), (0, 0, 0, 0))
        if i < 2:
            t.alpha_composite(squat, (0, TILE - 34))
        else:
            t.alpha_composite(side, (0, 0))
        d = ImageDraw.Draw(t)
        if i >= 1:
            # petite crotte marron à trois boules ; frame 3 : derrière
            # l'arrière-train du mouton de profil (il regarde à gauche)
            px = 18 if i == 1 else 32
            d.ellipse((px - 4, 35, px + 4, 39), fill=(101, 67, 33, 255))
            d.ellipse((px - 2, 32, px + 3, 36), fill=(120, 80, 40, 255))
            d.ellipse((px - 1, 30, px + 2, 33), fill=(101, 67, 33, 255))
        if i == 2:
            d.point((6, 8), fill=(255, 255, 120, 255))
            d.point((4, 12), fill=(255, 255, 120, 255))
        tiles.append(t)

    # --- sunglasses (189-191) : « deal with it » ---
    for i, dy in enumerate((-10, -4, 0)):
        t = on_canvas(front)
        d = ImageDraw.Draw(t)
        # les yeux du mouton de face sont vers y=17
        y = 17 + dy
        d.rectangle((10, y, 30, y + 4), fill=(10, 10, 10, 255))
        d.rectangle((12, y + 1, 18, y + 4), fill=(35, 35, 55, 255))
        d.rectangle((22, y + 1, 28, y + 4), fill=(35, 35, 55, 255))
        tiles.append(t)

    # --- flower_grow (192-195) : la fleur pousse, il la mange ---
    def flower_stage(stage: int, base: Image.Image, dx: int) -> Image.Image:
        t = on_canvas(base, dx, 0)
        d = ImageDraw.Draw(t)
        h = (6, 12, 16)[min(stage, 2)]
        d.line((36, 39, 36, 39 - h), fill=(30, 140, 40, 255), width=2)
        if stage >= 1:
            d.line((36, 33, 39, 30), fill=(30, 140, 40, 255), width=1)
        if stage >= 2:
            cy = 39 - h
            for ang in range(0, 360, 45):
                ax = 36 + round(4 * math.cos(math.radians(ang)))
                ay = cy + round(4 * math.sin(math.radians(ang)))
                d.ellipse((ax - 2, ay - 2, ax + 2, ay + 2), fill=(250, 250, 250, 255))
            d.ellipse((34, cy - 2, 39, cy + 3), fill=(250, 200, 40, 255))
        return t

    tiles.append(flower_stage(0, side, -4))
    tiles.append(flower_stage(1, side, -4))
    tiles.append(flower_stage(2, side, -4))
    # il broute la fleur : tête baissée, il ne reste que la tige
    last = on_canvas(graze, -4, 0)
    d = ImageDraw.Draw(last)
    d.line((36, 39, 36, 35), fill=(30, 140, 40, 255), width=2)
    tiles.append(last)

    # --- parachute (196-197) : suspendu, balancement ---
    small = front.resize((22, 22), Image.NEAREST)
    for sway in (-2, 2):
        t = Image.new("RGBA", (TILE, TILE), (0, 0, 0, 0))
        d = ImageDraw.Draw(t)
        cx = 20 + sway
        # voilure rayée rouge/blanc
        d.pieslice((cx - 16, -6, cx + 16, 16), 180, 360, fill=(220, 50, 50, 255))
        for k in (-10, 0, 10):
            d.pieslice((cx + k - 4, -6, cx + k + 4, 16), 180, 360, fill=(245, 245, 245, 255))
        # suspentes
        d.line((cx - 14, 6, 20 - sway, 20), fill=(80, 80, 80, 255), width=1)
        d.line((cx + 14, 6, 20 - sway + 8, 20), fill=(80, 80, 80, 255), width=1)
        t.alpha_composite(small, (9 - sway, 18))
        tiles.append(t)

    # --- rocket (198-200) : allumage puis décollage ---
    tiny = front.resize((18, 18), Image.NEAREST)
    for stage in range(3):
        t = Image.new("RGBA", (TILE, TILE), (0, 0, 0, 0))
        d = ImageDraw.Draw(t)
        # corps de fusée
        d.rectangle((14, 16, 26, 34), fill=(200, 200, 210, 255))
        d.polygon([(14, 16), (26, 16), (20, 8)], fill=(210, 30, 30, 255))
        d.polygon([(14, 34), (10, 39), (14, 28)], fill=(210, 30, 30, 255))
        d.polygon([(26, 34), (30, 39), (26, 28)], fill=(210, 30, 30, 255))
        d.ellipse((17, 20, 23, 26), fill=(90, 160, 220, 255))
        # le mouton dépasse du sommet
        t.alpha_composite(tiny, (11, 0))
        if stage >= 1:
            f = 4 if stage == 1 else 8
            d.polygon(
                [(16, 34), (24, 34), (20, 34 + f)],
                fill=(255, 160, 30, 255),
            )
            d.polygon(
                [(18, 34), (22, 34), (20, 34 + f - 2)],
                fill=(255, 240, 80, 255),
            )
        tiles.append(t)

    # --- outils pluie : nuage, gouttes, mouton mouillé ---
    def cloud(d: ImageDraw.ImageDraw, sway: int = 0):
        for cx, cy, r in ((12, 6, 6), (20, 4, 7), (28, 6, 6), (20, 8, 8)):
            d.ellipse(
                (cx + sway - r, cy - r // 2, cx + sway + r, cy + r // 2 + 3),
                fill=(150, 150, 165, 255),
            )

    def drops(d: ImageDraw.ImageDraw, phase: int):
        for k in range(5):
            dx = 8 + k * 6
            dy = 14 + ((k * 7 + phase * 5) % 14)
            d.line((dx, dy, dx, dy + 3), fill=(90, 150, 240, 255), width=1)

    def wet(sprite: Image.Image) -> Image.Image:
        # laine assombrie et bleutée : le mouton est trempé
        out = sprite.copy()
        px = out.load()
        for yy in range(out.height):
            for xx in range(out.width):
                r, g, b, a = px[xx, yy]
                if a and r > 180 and g > 160 and b < 200:
                    px[xx, yy] = (int(r * 0.62), int(g * 0.66), int(b * 0.85) + 30, a)
        return out

    small_front = front.resize((32, 32), Image.NEAREST)
    wet_front = wet(small_front)

    # --- acid (201-204) : teintes cyclées, yeux en spirale ---
    for i in range(4):
        t = on_canvas(hue_shift(front, i * 0.25))
        d = ImageDraw.Draw(t)
        for ex in (14, 24):
            d.ellipse((ex - 3, 14, ex + 3, 20), fill=(255, 255, 255, 255))
            d.arc((ex - 3, 14, ex + 3, 20), i * 90, i * 90 + 270, fill=(0, 0, 0, 255))
            d.point((ex, 17), fill=(0, 0, 0, 255))
        # petites étoiles colorées qui flottent
        rainbow = [(255, 0, 0), (255, 160, 0), (0, 200, 80), (80, 80, 255)]
        for k, c in enumerate(rainbow):
            d.point(((5 + k * 9 + i * 3) % 38, 3 + (k * 5 + i * 2) % 9), fill=c + (255,))
        tiles.append(t)

    # --- rain (205-206) : le nuage s'installe, premières gouttes ---
    for phase in range(2):
        t = Image.new("RGBA", (TILE, TILE), (0, 0, 0, 0))
        t.alpha_composite(small_front, (4, 8))
        d = ImageDraw.Draw(t)
        cloud(d, sway=phase)
        drops(d, phase)
        tiles.append(t)

    # --- soaked (207-208) : trempé jusqu'aux os, flaque ---
    for phase in range(2):
        t = Image.new("RGBA", (TILE, TILE), (0, 0, 0, 0))
        squashed = wet_front.resize((32, 28), Image.NEAREST)
        t.alpha_composite(squashed, (4, 12))
        d = ImageDraw.Draw(t)
        cloud(d, sway=phase)
        drops(d, phase)
        # flaque bleutée sous le mouton
        d.ellipse((6, 36, 34, 40), fill=(100, 150, 230, 160))
        # gouttes qui perlent de la laine
        d.point((8, 34 - phase), fill=(90, 150, 240, 255))
        d.point((30, 33 + phase), fill=(90, 150, 240, 255))
        tiles.append(t)

    # --- umbrella (209-210) : parapluie rouge, pluie qui rebondit ---
    for phase in range(2):
        t = Image.new("RGBA", (TILE, TILE), (0, 0, 0, 0))
        t.alpha_composite(small_front, (4, 8))
        d = ImageDraw.Draw(t)
        cloud(d, sway=phase)
        drops(d, phase)
        # parapluie : calotte rouge à liseré, mât et poignée
        d.pieslice((4, 8, 36, 30), 180, 360, fill=(210, 40, 40, 255))
        d.pieslice((10, 12, 30, 30), 180, 360, fill=(240, 80, 70, 255))
        d.line((20, 19, 20, 34), fill=(80, 60, 40, 255), width=2)
        d.arc((16, 32, 24, 38), 0, 180, fill=(80, 60, 40, 255), width=2)
        tiles.append(t)

    # --- outils amour : teinte rosée, nœud, petits cœurs ---
    def rosy(sprite: Image.Image) -> Image.Image:
        # laine rosée : c'est la moutonne
        out = sprite.copy()
        px = out.load()
        for yy in range(out.height):
            for xx in range(out.width):
                r, g, b, a = px[xx, yy]
                if a and r > 180 and g > 160 and b < 200:
                    px[xx, yy] = (
                        min(255, int(r * 0.9) + 40),
                        int(g * 0.78),
                        min(255, int(b * 0.9) + 55),
                        a,
                    )
        return out

    def bow(d: ImageDraw.ImageDraw, cx: int, cy: int):
        # nœud rose sur la tête
        d.polygon([(cx, cy), (cx - 5, cy - 4), (cx - 5, cy + 4)], fill=(240, 60, 130, 255))
        d.polygon([(cx, cy), (cx + 5, cy - 4), (cx + 5, cy + 4)], fill=(240, 60, 130, 255))
        d.ellipse((cx - 2, cy - 2, cx + 2, cy + 2), fill=(255, 120, 170, 255))

    def heart(d: ImageDraw.ImageDraw, x: int, y: int, s: int = 3):
        d.ellipse((x - s, y - s, x, y), fill=(235, 40, 80, 255))
        d.ellipse((x, y - s, x + s, y), fill=(235, 40, 80, 255))
        d.polygon([(x - s, y - 1), (x + s, y - 1), (x, y + s + 1)], fill=(235, 40, 80, 255))

    # --- moutonne (211-213) : marche rosée à nœud, puis cœur ---
    for base in (walk2, tile_of(sheet, 3)):
        t = on_canvas(rosy(base))
        d = ImageDraw.Draw(t)
        bow(d, 9, 6)
        tiles.append(t)
    t = on_canvas(rosy(side))
    d = ImageDraw.Draw(t)
    bow(d, 9, 6)
    heart(d, 25, 6, 4)
    tiles.append(t)

    # --- agneaux (214-215) : les mêmes pas, en tout petit ---
    for base in (walk2, tile_of(sheet, 3)):
        t = Image.new("RGBA", (TILE, TILE), (0, 0, 0, 0))
        lamb = base.resize((24, 24), Image.NEAREST)
        t.alpha_composite(lamb, (8, TILE - 24))
        tiles.append(t)

    # --- love (216-217) : le mouton de face, des cœurs plein la tête ---
    for phase in range(2):
        t = on_canvas(front)
        d = ImageDraw.Draw(t)
        heart(d, 6 + phase * 3, 8 - phase * 3, 3)
        heart(d, 33 - phase * 2, 6 + phase * 2, 4)
        heart(d, 20, 3 - phase, 2)
        tiles.append(t)

    return tiles


def extend_sheet(sheet: Image.Image, tiles: list[Image.Image]) -> Image.Image:
    out = Image.new("RGBA", (COLS * TILE, NEW_ROWS * TILE), (0, 0, 0, 0))
    out.alpha_composite(sheet, (0, 0))
    for i, t in enumerate(tiles):
        idx = OLD_ROWS * COLS + i
        out.alpha_composite(t, ((idx % COLS) * TILE, (idx // COLS) * TILE))
    return out


def anim_xml(
    anim_id: int,
    name: str,
    frames: list[int],
    *,
    start=("0", "0", "200"),
    end=None,
    repeat="0",
    repeat_from=0,
    nexts=(("100", None, "1"),),
    border=None,
    gravity=(("100", None, "5"),),
) -> ET.Element:
    """Construit un élément <animation> ; nexts = (probabilité, only, cible)."""

    def movement(tag: str, values) -> ET.Element:
        m = ET.Element(q(tag))
        for sub, val in zip(("x", "y", "interval"), values):
            ET.SubElement(m, q(sub)).text = str(val)
        ET.SubElement(m, q("offsety")).text = "0"
        ET.SubElement(m, q("opacity")).text = "1.0"
        return m

    def next_el(parent: ET.Element, entries):
        for prob, only, target in entries:
            n = ET.SubElement(parent, q("next"), {"probability": str(prob)})
            if only:
                n.set("only", only)
            n.text = str(target)

    a = ET.Element(q("animation"), {"id": str(anim_id)})
    ET.SubElement(a, q("name")).text = name
    a.append(movement("start", start))
    a.append(movement("end", end or start))
    seq = ET.SubElement(a, q("sequence"), {"repeat": str(repeat), "repeatfrom": str(repeat_from)})
    for f in frames:
        ET.SubElement(seq, q("frame")).text = str(f)
    next_el(seq, nexts)
    if border:
        b = ET.SubElement(a, q("border"))
        next_el(b, border)
    if gravity:
        g = ET.SubElement(a, q("gravity"))
        next_el(g, gravity)
    return a


def build_animations() -> list[ET.Element]:
    """Les neuf gags, frames 176+ (voir build_tiles pour l'ordre)."""
    return [
        anim_xml(100, "dance", [176, 177, 178, 177, 176, 177, 178, 177],
                 start=("0", "0", "160"), repeat="2"),
        anim_xml(101, "smoke", [180, 181, 182, 183, 183, 182, 183, 183],
                 start=("0", "0", "380"), repeat="1"),
        anim_xml(102, "superman", [184, 185],
                 start=("-6", "-3", "120"), end=("-12", "-8", "90"),
                 repeat="14", gravity=None),
        anim_xml(103, "poop", [186, 186, 186, 187, 187, 188, 188, 188],
                 start=("0", "0", "340")),
        anim_xml(104, "sunglasses", [189, 190, 191, 191, 191, 191, 191],
                 start=("0", "0", "300")),
        anim_xml(105, "flower_grow", [192, 192, 193, 193, 194, 194, 194, 195, 195, 195],
                 start=("0", "0", "330")),
        anim_xml(106, "parachute", [196, 197],
                 start=("0", "2", "220"), repeat="40",
                 nexts=(("100", None, "1"),),
                 border=(("100", None, "1"),), gravity=None),
        anim_xml(107, "rocket", [198, 199, 200, 200],
                 start=("0", "0", "260"), end=("0", "-14", "70"),
                 repeat="7", repeat_from=2, gravity=None),
        anim_xml(108, "acid", [201, 202, 203, 204],
                 start=("-1", "0", "170"), end=("1", "0", "170"), repeat="7"),
        anim_xml(109, "rain", [205, 206],
                 start=("0", "0", "250"), repeat="3",
                 nexts=(("50", None, "110"), ("50", None, "111"))),
        anim_xml(110, "soaked", [207, 208, 207, 208, 208, 208],
                 start=("0", "0", "340")),
        anim_xml(111, "umbrella", [209, 210],
                 start=("0", "0", "250"), repeat="4"),
        # Le coup de foudre : il s'arrête, des cœurs plein la tête, pendant
        # que la moutonne (enfant) traverse l'écran vers lui.
        anim_xml(112, "love", [216, 217],
                 start=("0", "0", "320"), repeat="10",
                 nexts=(("100", None, "113"),)),
        # La parade familiale : il repart, deux agneaux (enfants) trottinent
        # derrière lui.
        anim_xml(113, "family_walk", [2, 3],
                 start=("-2", "0", "200"), repeat="30",
                 nexts=(("100", None, "1"),),
                 border=(("100", None, "2"),)),
        # Chaîne de la moutonne (enfant) : traversée, cœur, sortie de scène.
        anim_xml(120, "ewe_walk", [211, 212],
                 start=("-3", "0", "180"), repeat="25",
                 nexts=(("100", None, "121"),), gravity=None),
        anim_xml(121, "ewe_heart", [213, 213],
                 start=("0", "0", "320"), repeat="3",
                 nexts=(("100", None, "122"),), gravity=None),
        anim_xml(122, "ewe_leave", [211, 212],
                 start=("-4", "0", "150"), repeat="60",
                 nexts=(), gravity=None),
        # Chaîne des agneaux (enfants) : ils suivent puis s'éclipsent.
        anim_xml(130, "lamb_walk", [214, 215],
                 start=("-2", "0", "190"), repeat="60",
                 nexts=(), gravity=None),
    ]


# Probabilité d'apparition de chaque gag depuis la marche (contexte libre).
WALK_HOOKS = [
    (100, 4), (101, 3), (102, 2), (103, 3),
    (104, 3), (105, 4), (107, 2), (108, 2),
    (109, 3), (112, 2),
]

# Enfants : (animation déclencheuse, x, y, première animation de l'enfant).
CHILD_HOOKS = [
    (112, "screenW-45", "areaH-imageH", 120),
    (113, "screenW+5", "areaH-imageH", 130),
    (113, "screenW+45", "areaH-imageH", 130),
]


def main() -> None:
    src = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SRC
    ET.register_namespace("", NS)
    tree = ET.parse(src)
    root = tree.getroot()

    sheet = load_sheet(root)
    tiles = build_tiles(sheet)
    new_sheet = extend_sheet(sheet, tiles)

    # Image : nouvelle sheet (alpha conservé par decode_sheet) et 13 lignes.
    buf = io.BytesIO()
    new_sheet.save(buf, format="PNG", optimize=True)
    image = root.find(q("image"))
    image.find(q("tilesy")).text = str(NEW_ROWS)
    image.find(q("png")).text = base64.b64encode(buf.getvalue()).decode()

    # En-tête : identité RustySheep, crédits d'origine conservés.
    header = root.find(q("header"))
    header.find(q("petname")).text = "RustySheep"
    header.find(q("title")).text = "RustySheep"
    info = header.find(q("info"))
    info.text = (
        "eSheep 64bit (Adriano Petrucci, images LiL_Stenly) enrichi par "
        "RustyPet : danse, joint, superman, crotte, lunettes, fleur, "
        "parachute, fusee, acide. [br]" + (info.text or "")
    )

    # Nouvelles animations en fin de liste.
    animations = root.find(q("animations"))
    for anim in build_animations():
        animations.append(anim)

    # Branchement depuis la marche (id 1) : petites probabilités.
    for anim in animations.findall(q("animation")):
        if anim.get("id") == "1":
            seq = anim.find(q("sequence"))
            for target, prob in WALK_HOOKS:
                n = ET.SubElement(seq, q("next"), {"probability": str(prob)})
                n.text = str(target)

    # Spawn : arrivée en parachute depuis le haut de l'écran.
    spawns = root.find(q("spawns"))
    spawn = ET.SubElement(spawns, q("spawn"), {"id": "9", "probability": "25"})
    ET.SubElement(spawn, q("x")).text = "random*(screenW-imageW-50)/100+25"
    ET.SubElement(spawn, q("y")).text = "-imageH"
    ET.SubElement(spawn, q("next")).text = "106"

    # Enfants : la moutonne du coup de foudre, les agneaux de la parade.
    childs = root.find(q("childs"))
    for trigger, x, y, next_id in CHILD_HOOKS:
        child = ET.SubElement(childs, q("child"), {"animationid": str(trigger)})
        ET.SubElement(child, q("x")).text = x
        ET.SubElement(child, q("y")).text = y
        ET.SubElement(child, q("next")).text = str(next_id)

    out_dir = REPO / "assets/rustysheep"
    out_dir.mkdir(parents=True, exist_ok=True)
    out = out_dir / "animations.xml"
    tree.write(out, encoding="unicode", xml_declaration=True)
    print(f"OK {out} ({out.stat().st_size // 1024} KiB, {len(tiles)} tuiles inédites)")

    # Planche contact des nouvelles tuiles, pour contrôle visuel.
    contact = Image.new("RGBA", (len(tiles) * (TILE + 4), TILE + 4), (40, 40, 40, 255))
    for i, t in enumerate(tiles):
        contact.alpha_composite(t, (i * (TILE + 4) + 2, 2))
    contact_path = out_dir / "contact-tuiles.png"
    contact.save(contact_path)
    print(f"OK {contact_path}")


if __name__ == "__main__":
    main()
