import os, math
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.misc.transform import Transform
from PIL import Image, ImageDraw, ImageFont

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'out')
PNG = os.path.join(OUT, 'png')
FONT = '/usr/share/fonts/truetype/google-fonts/Poppins-Medium.ttf'

VINE   = '#1C6B55'
AMBER  = '#E39A3C'
INK    = '#14201C'
PAPER  = '#F7F5F0'
D_VINE = '#4FBF9B'
D_AMBER= '#F0B460'

# --- mark geometry, 64x64 viewBox, optically centred (bounds x 8..56, y 10..54)
SW = 6.0
STROKES = [
    ((11, 13), (53, 13), 'primary'),   # crossbar
    ((32, 13), (32, 50), 'primary'),   # stem
    ((19, 28), (45, 28), 'primary'),   # upper rung
    ((26, 41), (38, 41), 'accent'),    # lower rung
]

def mark_paths(primary, accent, scale=1.0, dx=0.0, dy=0.0):
    out = []
    for (x1, y1), (x2, y2), role in STROKES:
        c = accent if role == 'accent' else primary
        p1 = (x1 * scale + dx, y1 * scale + dy)
        p2 = (x2 * scale + dx, y2 * scale + dy)
        d = 'M%g %g L%g %g' % (p1[0], p1[1], p2[0], p2[1])
        out.append('  <path d="%s" stroke="%s" stroke-width="%g" stroke-linecap="round"/>'
                   % (d, c, SW * scale))
    return '\n'.join(out)

def svg_doc(w, h, body, title):
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 %g %g" width="%g" height="%g" '
            'fill="none" role="img" aria-label="%s">\n%s\n</svg>\n' % (w, h, w, h, title, body))

# --- wordmark outlines
KERN = {('T', 'r'): -0.030}   # manual pair (no HarfBuzz available)

def wordmark_path(text, size, x=0.0, baseline=0.0, tracking=-0.012):
    f = TTFont(FONT)
    upem = f['head'].unitsPerEm
    gs = f.getGlyphSet()
    cmap = f.getBestCmap()
    hmtx = f['hmtx']
    s = size / upem
    pen_x = x
    parts = []
    for i, ch in enumerate(text):
        gname = cmap[ord(ch)]
        spen = SVGPathPen(gs)
        tpen = TransformPen(spen, Transform(s, 0, 0, -s, pen_x, baseline))
        gs[gname].draw(tpen)
        d = spen.getCommands()
        if d:
            parts.append(d)
        adv = hmtx[gname][0] / upem
        pen_x += adv * size + tracking * size
        if i + 1 < len(text):
            pen_x += KERN.get((ch, text[i + 1]), 0.0) * size
    return ' '.join(parts), pen_x - tracking * size - x

def write(name, content):
    p = os.path.join(OUT, name)
    with open(p, 'w') as fh:
        fh.write(content)
    print('svg ', name)

# ---------------- marks ----------------
write('trellis-mark.svg',
      svg_doc(64, 64, mark_paths(VINE, AMBER), 'Trellis'))
write('trellis-mark-mono.svg',
      svg_doc(64, 64, mark_paths(INK, INK), 'Trellis'))
write('trellis-mark-dark.svg',
      svg_doc(64, 64, mark_paths(D_VINE, D_AMBER), 'Trellis'))

# ---------------- favicon (mark on a vine tile) ----------------
tile = ('  <rect width="64" height="64" rx="14" fill="%s"/>\n' % VINE) + \
       mark_paths(PAPER, D_AMBER, scale=0.82, dx=32 * (1 - 0.82), dy=32 * (1 - 0.82))
write('favicon.svg', svg_doc(64, 64, tile, 'Trellis'))

# ---------------- lockups ----------------
# mark visual bounds: x 8..56 (48 wide), y 10..54 (44 tall)
MARK_H = 43.0
CAP = 0.700          # Poppins cap height ratio
WORD_SIZE = 46.0     # em size -> cap height ~32
GAP = 22.0
PAD = 4.0            # breathing room so nothing clips

def lockup(primary, accent, textcolor, name):
    mark = mark_paths(primary, accent, scale=1.0, dx=PAD - 8, dy=PAD - 10)
    # mark now occupies x PAD..PAD+48, y PAD..PAD+44
    cap_h = WORD_SIZE * CAP
    baseline = PAD + MARK_H - (MARK_H - cap_h) / 2.0
    tx = PAD + 48 + GAP
    d, adv = wordmark_path('Trellis', WORD_SIZE, x=tx, baseline=baseline)
    w = tx + adv + PAD
    h = PAD * 2 + MARK_H
    body = mark + '\n  <path d="%s" fill="%s"/>' % (d, textcolor)
    write(name, svg_doc(round(w, 1), h, body, 'Trellis'))

lockup(VINE, AMBER, INK, 'trellis-lockup.svg')
lockup(D_VINE, D_AMBER, PAPER, 'trellis-lockup-dark.svg')

# ---------------- PNGs ----------------
def hx(c):
    c = c.lstrip('#')
    return tuple(int(c[i:i+2], 16) for i in (0, 2, 4))

SS = 4  # supersample

def draw_mark(dr, scale, dx, dy, primary, accent):
    for (x1, y1), (x2, y2), role in STROKES:
        c = hx(accent if role == 'accent' else primary)
        dr.line([(x1*scale+dx, y1*scale+dy), (x2*scale+dx, y2*scale+dy)],
                fill=c, width=int(round(SW*scale)))
        r = SW*scale/2.0
        for (px, py) in ((x1, y1), (x2, y2)):
            cx, cy = px*scale+dx, py*scale+dy
            dr.ellipse([cx-r, cy-r, cx+r, cy+r], fill=c)

def png_mark(size, primary, accent, name, bg=None, radius=None, inset=1.0):
    S = size * SS
    img = Image.new('RGBA', (S, S), (0, 0, 0, 0))
    dr = ImageDraw.Draw(img)
    if bg:
        r = radius * SS * size / 64.0 if radius else 0
        dr.rounded_rectangle([0, 0, S-1, S-1], radius=r, fill=hx(bg))
    sc = (S / 64.0) * inset
    off = (S - 64*sc) / 2.0
    draw_mark(dr, sc, off, off, primary, accent)
    img = img.resize((size, size), Image.LANCZOS)
    p = os.path.join(PNG, name)
    img.save(p)
    print('png ', name)
    return img

for s in (512, 256, 128, 64):
    png_mark(s, VINE, AMBER, 'trellis-mark-%d.png' % s)
png_mark(512, D_VINE, D_AMBER, 'trellis-mark-dark-512.png')

icons = []
for s in (16, 32, 48, 64, 128, 256):
    icons.append(png_mark(s, PAPER, D_AMBER, 'favicon-%d.png' % s, bg=VINE, radius=14, inset=0.82))
icons[-1].save(os.path.join(OUT, 'favicon.ico'),
               sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
print('ico  favicon.ico')

def png_lockup(height, primary, accent, textcolor, name):
    scale = height * SS / (PAD*2 + MARK_H)
    fsize = WORD_SIZE * scale
    font = ImageFont.truetype(FONT, int(round(fsize)))
    tmp = ImageDraw.Draw(Image.new('RGBA', (10, 10)))
    tw = tmp.textlength('Trellis', font=font)
    W = int(round((PAD + 48 + GAP) * scale + tw + PAD * scale))
    H = int(round((PAD*2 + MARK_H) * scale))
    img = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    dr = ImageDraw.Draw(img)
    draw_mark(dr, scale, (PAD-8)*scale, (PAD-10)*scale, primary, accent)
    cap_h = WORD_SIZE * CAP
    baseline = (PAD + MARK_H - (MARK_H - cap_h) / 2.0) * scale
    dr.text(((PAD + 48 + GAP) * scale, baseline), 'Trellis',
            font=font, fill=hx(textcolor), anchor='ls')
    img = img.resize((max(1, W // SS), max(1, H // SS)), Image.LANCZOS)
    img.save(os.path.join(PNG, name))
    print('png ', name, img.size)

png_lockup(160, VINE, AMBER, INK, 'trellis-lockup-160.png')
png_lockup(320, VINE, AMBER, INK, 'trellis-lockup-320.png')
png_lockup(320, D_VINE, D_AMBER, PAPER, 'trellis-lockup-dark-320.png')
print('done')
