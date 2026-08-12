"""Build the favicon and the Open Graph card from real Inter outlines.

Text is emitted as vector paths, not <text>, so nothing depends on the
viewer having Inter installed. Writes favicon.svg and og.svg into static/.

This script produces SVGs only. It does NOT produce the PNGs that are
actually shipped — static/og-image.png and static/apple-touch-icon.png.
Those were rasterised by hand on macOS with `qlmanage` and `sips`, a step
that lives only in this docstring, not in any script. If you change the
favicon or OG card, regenerate the SVGs here first, then repeat the manual
rasterisation below.

Requires fontTools (with the brotli extra, needed to decode the source
.woff2). These are not project dependencies — install them separately
(e.g. `pip install fontTools[woff] brotli`) before running. Run manually
when the wordmark or brand colours change; this script is never invoked
by the build (`cargo run -p site`), and the generated/rasterised assets
are committed to the repo as-is.

Manual PNG rasterisation (macOS, requires Quick Look + `sips`):

  og-image.png (1200x630):
    Quick Look scales an SVG to fill a square thumbnail, so rendering the
    630-tall card directly would get magnified and cropped. Instead, put
    the card's content on a square 1200x1200 canvas, with everything
    translated down by 285px to vertically center the original 630-tall
    composition, and save that as og_square.svg. Then:
      qlmanage -t -s 1200 -o <dir> og_square.svg
      sips -c 630 1200 <dir>/og_square.png --out og-image.png

  apple-touch-icon.png (180x180):
    iOS ignores transparency and applies its own corner rounding, so the
    icon needs an opaque background rather than the transparent favicon.
    Put the favicon's paths over an opaque #FBFAF8 background and save
    that as touch.svg. Then:
      qlmanage -t -s 180 -o <dir> touch.svg
"""
from pathlib import Path

from fontTools.ttLib import TTFont
from fontTools.varLib import instancer
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.misc.transform import Transform

REPO_ROOT = Path(__file__).resolve().parents[2]
SRC = REPO_ROOT / "static" / "fonts" / "InterVariable.woff2"
OUT = REPO_ROOT / "static"

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
        name = cmap.get(ord(ch))
        if name is None:
            cursor += upem * 0.3
            continue
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
        name = cmap.get(ord(ch))
        total += (gs[name].width if name else upem * 0.3) + tracking_em * upem
    return total * (size_px / upem)


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
def og_card():
    W, H = 1200, 630
    M = 96
    name_size, lede_size, meta_size = 108, 34, 22
    parts = []
    parts.append(f'<rect width="{W}" height="{H}" fill="{PAPER}"/>')
    # hairline rule + rust marker, echoing the site's section headers
    parts.append(f'<rect x="{M}" y="{H-150}" width="{W-2*M}" height="1" fill="#E5E2DC"/>')
    parts.append(f'<rect x="{M}" y="{M}" width="44" height="5" fill="{RUST}"/>')

    p, _ = text_path("Paul Maxwell", 600, name_size, M, 300, -0.035, INK)
    parts.append(p)
    p, _ = text_path("I build the infrastructure that gets AI systems", 350, lede_size, M, 375, 0, MUTED)
    parts.append(p)
    p, _ = text_path("safely into production.", 350, lede_size, M, 420, 0, MUTED)
    parts.append(p)
    p, _ = text_path("LEAD SOFTWARE ENGINEER / P-1 AI", 500, meta_size, M, H - 96, 0.14, INK)
    parts.append(p)
    w = measure("paul-maxwell.com", 400, meta_size, 0.02)
    p, _ = text_path("paul-maxwell.com", 400, meta_size, W - M - w, H - 96, 0.02, MUTED)
    parts.append(p)

    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" '
            f'width="{W}" height="{H}">' + "".join(parts) + "</svg>\n")


open(f"{OUT}/favicon.svg", "w").write(favicon())
open(f"{OUT}/og.svg", "w").write(og_card())
print("favicon.svg", len(favicon()), "bytes")
print("og.svg", len(og_card()), "bytes")
