"""Build the favicon and the Open Graph card from real Inter outlines.

Text is emitted as vector paths, not <text>, so nothing depends on the
viewer having Inter installed. Writes static/favicon.svg, static/og-image.png,
and docs/superpowers/og-card-lede.txt.

The OG card prints the lede from content/site.toml; its name, role line, and
domain are still literals here. og-card-lede.txt records the lede the PNG was
drawn with, and a test in crates/site fails when it drifts from site.toml —
rerun this script whenever that test says so.

Requires macOS (Quick Look's `qlmanage` and `sips` rasterise the card),
Python 3.11+, and fontTools with the brotli extra to decode the source
.woff2. These are not project dependencies — install them separately
(e.g. `pip install fontTools[woff] brotli`). The build (`cargo run -p site`)
never invokes this script; its outputs are committed.

Quick Look scales an SVG to fill a square thumbnail, so rendering the
630-tall card directly would get magnified and cropped. The card is instead
placed on a 1200x1200 canvas, shifted down 285px to centre it, and cropped
back to 1200x630 afterwards.

static/apple-touch-icon.png is NOT produced here and is still a manual step:

  apple-touch-icon.png (180x180):
    iOS ignores transparency and applies its own corner rounding, so the
    icon needs an opaque background rather than the transparent favicon.
    Put the favicon's paths over an opaque #FBFAF8 background and save
    that as touch.svg. Then:
      qlmanage -t -s 180 -o <dir> touch.svg
"""
import subprocess
import tempfile
import tomllib
from pathlib import Path

from fontTools.ttLib import TTFont
from fontTools.varLib import instancer
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.misc.transform import Transform

REPO_ROOT = Path(__file__).resolve().parents[2]
SRC = REPO_ROOT / "static" / "fonts" / "InterVariable.woff2"
OUT = REPO_ROOT / "static"
LEDE_STAMP = REPO_ROOT / "docs" / "superpowers" / "og-card-lede.txt"

INK, PAPER, MUTED, RUST = "#14161A", "#FBFAF8", "#6E7076", "#A8431E"
INK_D, PAPER_D = "#E9E7E2", "#0E0F11"

_cache = {}
def face(weight):
    if weight not in _cache:
        f = TTFont(str(SRC))
        if "fvar" in f:
            f = instancer.instantiateVariableFont(f, {"wght": weight}, inplace=False)
        _cache[weight] = f
    return _cache[weight]


def glyph_name(cmap, ch):
    name = cmap.get(ord(ch))
    if name is None:
        raise ValueError(f"Inter has no glyph for {ch!r}")
    return name


def text_path(s, weight, size_px, x, y, tracking_em=0.0, fill=INK):
    """Render `s` as one <path> at baseline (x, y), `size_px` tall per em."""
    f = face(weight)
    upem = f["head"].unitsPerEm
    gs, cmap = f.getGlyphSet(), f.getBestCmap()
    kern = tracking_em * upem
    scale = size_px / upem

    pen = SVGPathPen(gs)
    cursor = 0.0
    for ch in s:
        name = glyph_name(cmap, ch)
        t = Transform(scale, 0, 0, -scale, x + cursor * scale, y)
        gs[name].draw(TransformPen(pen, t))
        cursor += gs[name].width + kern
    return f'<path d="{pen.getCommands()}" fill="{fill}"/>', cursor * scale


def measure(s, weight, size_px, tracking_em=0.0):
    f = face(weight)
    upem = f["head"].unitsPerEm
    gs, cmap = f.getGlyphSet(), f.getBestCmap()
    total = 0.0
    for ch in s:
        total += gs[glyph_name(cmap, ch)].width + tracking_em * upem
    return total * (size_px / upem)


def greedy_wrap(s, weight, size_px, max_width):
    lines, current = [], ""
    for word in s.split():
        candidate = f"{current} {word}".strip()
        if current and measure(candidate, weight, size_px) > max_width:
            lines.append(current)
            current = word
        else:
            current = candidate
    return lines + [current]


def balanced_wrap(s, weight, size_px, max_width):
    """Greedy wrapping leaves a short last line; narrow the measure until one
    more step would add a line, so the lines come out near-equal instead."""
    lines = greedy_wrap(s, weight, size_px, max_width)
    for width in range(int(max_width), 0, -8):
        narrower = greedy_wrap(s, weight, size_px, width)
        if len(narrower) > len(lines):
            break
        lines = narrower
    return lines


def site_lede():
    with open(REPO_ROOT / "content" / "site.toml", "rb") as f:
        return tomllib.load(f)["lede"]


# ---------------------------------------------------------------- favicon
def favicon():
    f = face(600)
    upem = f["head"].unitsPerEm
    cap = f["OS/2"].sCapHeight
    BOX, TRACK = 64, -0.045
    w_units = measure("PM", 600, upem, TRACK)
    scale = (BOX * 0.86) / max(w_units, cap)
    w, h = w_units * scale, cap * scale
    body, _ = text_path("PM", 600, upem * scale, (BOX - w) / 2, (BOX + h) / 2, TRACK, "currentColor")
    body = body.replace(' fill="currentColor"', "")
    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {BOX} {BOX}">
<style>path{{fill:{INK}}}@media(prefers-color-scheme:dark){{path{{fill:{INK_D}}}}}</style>
{body}
</svg>
'''


# --------------------------------------------------------------- OG card
def og_card(lede):
    W, H = 1200, 630
    M = 96
    name_size, lede_size, meta_size = 108, 34, 22
    parts = []
    parts.append(f'<rect width="{W}" height="{H}" fill="{PAPER}"/>')
    # hairline rule + rust marker, echoing the site's section headers
    parts.append(f'<rect x="{M}" y="{H-150}" width="{W-2*M}" height="1" fill="#E5E2DC"/>')
    parts.append(f'<rect x="{M}" y="{M}" width="44" height="5" fill="{RUST}"/>')

    lede_lines = balanced_wrap(lede, 350, lede_size, W - 2 * M)
    widest = max(measure(line, 350, lede_size) for line in lede_lines)
    if len(lede_lines) > 3 or widest > W - 2 * M:
        raise ValueError(f"lede does not fit the OG card: {lede_lines}")
    lede_leading, last_lede_y = 45, 435
    first_lede_y = last_lede_y - lede_leading * (len(lede_lines) - 1)
    p, _ = text_path("Paul Maxwell", 600, name_size, M, first_lede_y - 75, -0.035, INK)
    parts.append(p)
    for i, line in enumerate(lede_lines):
        p, _ = text_path(line, 350, lede_size, M, first_lede_y + i * lede_leading, 0, MUTED)
        parts.append(p)
    p, _ = text_path("LEAD SOFTWARE ENGINEER / P-1 AI", 500, meta_size, M, H - 96, 0.14, INK)
    parts.append(p)
    w = measure("paul-maxwell.com", 400, meta_size, 0.02)
    p, _ = text_path("paul-maxwell.com", 400, meta_size, W - M - w, H - 96, 0.02, MUTED)
    parts.append(p)

    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" '
            f'width="{W}" height="{H}">' + "".join(parts) + "</svg>\n")


def rasterise_og_card(card_svg):
    inner = card_svg[card_svg.index(">") + 1 : card_svg.rindex("</svg>")]
    square = ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 1200" '
              f'width="1200" height="1200"><rect width="1200" height="1200" fill="{PAPER}"/>'
              f'<g transform="translate(0,285)">{inner}</g></svg>')
    with tempfile.TemporaryDirectory() as tmp:
        src = Path(tmp) / "og_square.svg"
        src.write_text(square)
        subprocess.run(["qlmanage", "-t", "-s", "1200", "-o", tmp, str(src)],
                       check=True, capture_output=True)
        subprocess.run(["sips", "-c", "630", "1200", f"{src}.png", "--out", str(OUT / "og-image.png")],
                       check=True, capture_output=True)


if __name__ == "__main__":
    lede = site_lede()
    (OUT / "favicon.svg").write_text(favicon())
    rasterise_og_card(og_card(lede))
    LEDE_STAMP.write_text(lede + "\n")
    print(f"wrote favicon.svg, og-image.png, and {LEDE_STAMP.name}")
