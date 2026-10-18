#!/usr/bin/env python3
"""Generate peel.ico — rounded blue square, open peel/zip motif."""
from PIL import Image, ImageDraw

def rounded(draw, box, radius, fill):
    draw.rounded_rectangle(box, radius=radius, fill=fill)

def draw_icon(size):
    s = size
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    # background rounded square, blue gradient-ish (solid is fine at icon sizes)
    m = s * 0.04
    rounded(d, [m, m, s - m, s - m], s * 0.22, (59, 130, 246, 255))  # #3B82F6

    # peel motif: a box being "peeled" — diagonal light flap + zipper line
    # flap (peeled corner), lighter blue triangle top-right
    pad = s * 0.18
    cx0, cy0 = pad, pad            # top-left of flap region
    cx1, cy1 = s - pad, s - pad    # bottom-right
    # white "peeled" flap: triangle from top-right corner
    flap = [(s * 0.62, s * 0.14), (s * 0.86, s * 0.14), (s * 0.86, s * 0.38)]
    d.polygon(flap, fill=(255, 255, 255, 235))
    # fold shadow line under flap
    d.line([flap[0], flap[2]], fill=(219, 234, 254, 255), width=max(1, int(s * 0.02)))

    # zipper: vertical dashed white line from top flap down to a pull
    zx = s * 0.52
    zy0, zy1 = s * 0.30, s * 0.72
    dash, gap = s * 0.07, s * 0.05
    y = zy0
    w = max(1, int(s * 0.045))
    while y < zy1:
        d.rectangle([zx - w / 2, y, zx + w / 2, min(y + dash, zy1)], fill=(255, 255, 255, 255))
        y += dash + gap
    # zipper pull: small rounded rect + ring below the line
    pr = s * 0.055
    d.ellipse([zx - pr, zy1 - s * 0.01, zx + pr, zy1 + pr * 2 - s * 0.01],
              outline=(255, 255, 255, 255), width=max(1, int(s * 0.03)))
    return img

sizes = [256, 128, 64, 48, 32, 16]
imgs = [draw_icon(sz) for sz in sizes]
imgs[0].save("/home/agentuser/project/peel/assets/peel.ico",
             format="ICO", sizes=[(sz, sz) for sz in sizes], append_images=imgs[1:])
print("saved peel.ico")
# also a png preview
imgs[0].save("/home/agentuser/project/peel/assets/peel.png")
print("saved peel.png")
