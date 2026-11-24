# Regenerates logo/StarPlay.ico from logo/StarPlay.svg: python scripts/make-icon.py logo/StarPlay.svg logo/StarPlay.ico <preview.png>
# Rasterizes the SVG (absolute and relative M/L/C/Z paths + vertical gradient) and writes a multi-size .ico (Pillow).
import re, sys
from PIL import Image, ImageDraw

svg_path, ico_path, png_path = sys.argv[1], sys.argv[2], sys.argv[3]
svg = open(svg_path, encoding="utf-8").read()
paths = re.findall(r'<path d="([^"]+)"', svg)
assert len(paths) == 2, paths

def parse(d, steps=48):
    """Outlines (point lists) of an SVG path; cubic Bezier curves are split into segments."""
    toks = re.findall(r'[MmLlCcZz]|-?\d*\.?\d+(?:e-?\d+)?', d)
    contours, cur, pos, start, cmd, i = [], [], (0.0, 0.0), (0.0, 0.0), None, 0
    def num():
        nonlocal i
        v = float(toks[i]); i += 1; return v
    while i < len(toks):
        if re.match(r'[A-Za-z]', toks[i]):
            cmd = toks[i]; i += 1
            if cmd in 'Zz':
                if cur: contours.append(cur)
                cur, pos = [], start
                continue
        rel = cmd.islower()
        ox, oy = pos if rel else (0.0, 0.0)
        c = cmd.upper()
        if c == 'M':
            if cur: contours.append(cur)
            pos = (ox + num(), oy + num()); start = pos; cur = [pos]
            cmd = 'l' if rel else 'L'
        elif c == 'L':
            pos = (ox + num(), oy + num()); cur.append(pos)
        elif c == 'C':
            p0 = pos
            p1 = (ox + num(), oy + num()); p2 = (ox + num(), oy + num()); p3 = (ox + num(), oy + num())
            for k in range(1, steps + 1):
                t = k / steps; u = 1 - t
                cur.append((u**3*p0[0] + 3*u*u*t*p1[0] + 3*u*t*t*p2[0] + t**3*p3[0],
                            u**3*p0[1] + 3*u*u*t*p1[1] + 3*u*t*t*p2[1] + t**3*p3[1]))
            pos = p3
        else:
            raise ValueError(c)
    if cur: contours.append(cur)
    return contours

S = 4096 / 480  # rendered at 4096 px then downscaled (anti-aliasing)
big = 4096
def scaled(contours):
    return [[(x * S, y * S) for x, y in c] for c in contours]

# Background: rounded square filled with a vertical gradient #acacac -> #6c6c6c
mask = Image.new("L", (big, big), 0)
for c in scaled(parse(paths[0])):
    ImageDraw.Draw(mask).polygon(c, fill=255)
grad = Image.new("RGB", (1, big))
for y in range(big):
    t = y / (big - 1)
    v = round(0xac + (0x6c - 0xac) * t)
    grad.putpixel((0, y), (v, v, v))
grad = grad.resize((big, big))
img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
img.paste(grad, (0, 0), mask)
# White star (evenodd fill rule: a single contour here)
star = Image.new("L", (big, big), 0)
for c in scaled(parse(paths[1])):
    ImageDraw.Draw(star).polygon(c, fill=255)
img.paste((255, 255, 255, 255), (0, 0), star)

img.resize((512, 512), Image.LANCZOS).save(png_path)
sizes = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]
frames = [img.resize((s, s), Image.LANCZOS) for s in sizes]
frames[-1].save(ico_path, format="ICO", sizes=[(s, s) for s in sizes], append_images=frames[:-1])
print("ok", ico_path)
