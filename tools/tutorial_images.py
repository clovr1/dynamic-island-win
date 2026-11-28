"""Draws the four pictures of the install tutorial on the website (docs/img/tut-*.png).
Needs Pillow and docs/img/raw-pill.png + settings-general.png (made by: cargo run --release --bin sitepics -- docs/img).
Run: python3 tools/tutorial_images.py docs/img assets"""
import sys
from PIL import Image, ImageDraw, ImageFont, ImageFilter

IMG = sys.argv[1] if len(sys.argv) > 1 else "docs/img"
ASSETS = sys.argv[2] if len(sys.argv) > 2 else "assets"
W, H = 1000, 620
ORANGE = (255, 92, 40)
INK = (18, 18, 20)
CREAM = (244, 238, 225)


def font(name, size):
    return ImageFont.truetype(f"{ASSETS}/Inter-{name}.ttf", size)


def wallpaper(w=W, h=H, seed=0):
    im = Image.new("RGB", (w, h), (14, 14, 22))
    px = im.load()
    for y in range(h):
        for x in range(w):
            fx, fy = x / w, y / h
            g1 = max(0.0, 1 - ((fx - 0.15) ** 2 + (fy - 0.05) ** 2) ** 0.5 * 1.6)
            g2 = max(0.0, 1 - ((fx - 0.9) ** 2 + (fy - 0.95) ** 2) ** 0.5 * 1.5)
            g3 = max(0.0, 1 - ((fx - 0.85) ** 2 + (fy - 0.1) ** 2) ** 0.5 * 1.8)
            r = 14 + 120 * g1 * 0.7 + 255 * g2 * 0.4 + 40 * g3 * 0.6
            g = 14 + 70 * g1 * 0.7 + 92 * g2 * 0.4 + 190 * g3 * 0.6
            b = 22 + 255 * g1 * 0.7 + 40 * g2 * 0.4 + 200 * g3 * 0.6
            px[x, y] = (min(255, int(r)), min(255, int(g)), min(255, int(b)))
    return im


def shadowed(base, box, radius, fill, blur=18, offset=(0, 14), alpha=110):
    sh = Image.new("RGBA", base.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(sh)
    x0, y0, x1, y1 = box
    d.rounded_rectangle((x0 + offset[0], y0 + offset[1], x1 + offset[0], y1 + offset[1]), radius, fill=(0, 0, 0, alpha))
    sh = sh.filter(ImageFilter.GaussianBlur(blur))
    base.alpha_composite(sh)
    d = ImageDraw.Draw(base)
    d.rounded_rectangle(box, radius, fill=fill)


def badge(d, cx, cy, n, r=22):
    d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=ORANGE, outline=(255, 255, 255), width=3)
    f = font("Bold", 26)
    w = d.textlength(str(n), font=f)
    d.text((cx - w / 2, cy - 15), str(n), font=f, fill=(255, 255, 255))


def ring(d, box, radius, width=5):
    d.rounded_rectangle(box, radius, outline=ORANGE, width=width)


def cursor(base, x, y):
    d = ImageDraw.Draw(base)
    pts = [(x, y), (x, y + 34), (x + 9, y + 26), (x + 16, y + 40), (x + 23, y + 37), (x + 16, y + 23), (x + 28, y + 23)]
    d.polygon([(px + 2, py + 3) for px, py in pts], fill=(0, 0, 0, 90))
    d.polygon(pts, fill=(255, 255, 255), outline=(20, 20, 20))


def text_center(d, cx, y, s, f, fill):
    w = d.textlength(s, font=f)
    d.text((cx - w / 2, y), s, font=f, fill=fill)


def wrap(d, s, f, maxw):
    words, lines, cur = s.split(), [], ""
    for w in words:
        t = (cur + " " + w).strip()
        if d.textlength(t, font=f) <= maxw:
            cur = t
        else:
            lines.append(cur)
            cur = w
    lines.append(cur)
    return lines


# ---------------------------------------------------------------- 1. download
def step1():
    im = wallpaper().convert("RGBA")
    box = (70, 46, 930, 574)
    shadowed(im, box, 22, (30, 30, 40, 255))
    d = ImageDraw.Draw(im)
    # browser bar
    for i, c in enumerate([(255, 95, 86), (255, 189, 46), (39, 201, 63)]):
        d.ellipse((96 + i * 26, 70, 112 + i * 26, 86), fill=c)
    d.rounded_rectangle((220, 62, 780, 94), 16, fill=(52, 52, 64))
    d.text((244, 67), "clovr1.github.io/bloop", font=font("Regular", 19), fill=(215, 215, 225))
    # the page
    d.rounded_rectangle((70, 110, 930, 574), 22, fill=CREAM)
    d.rectangle((70, 110, 930, 140), fill=CREAM)
    icon = Image.open(f"{ASSETS}/app.png").convert("RGBA").resize((34, 34), Image.LANCZOS)
    im.alpha_composite(icon, (100, 128))
    d.text((144, 130), "Bloop", font=font("Bold", 26), fill=INK)
    d.rounded_rectangle((780, 128, 900, 164), 18, fill=ORANGE)
    text_center(d, 840, 134, "Download", font("Bold", 18), (255, 255, 255))
    f = font("Bold", 54)
    d.text((100, 200), "A dynamic island for", font=f, fill=INK)
    d.text((100, 262), "your", font=f, fill=INK)
    d.text((100 + d.textlength("your ", font=f), 262), "Windows", font=f, fill=ORANGE)
    d.text((100 + d.textlength("your Windows ", font=f), 262), "desktop.", font=f, fill=INK)
    # the button
    b = (100, 360, 520, 430)
    d.rounded_rectangle(b, 35, fill=ORANGE)
    # arrow glyph
    ax, ay = 140, 395
    d.line((ax, ay - 14, ax, ay + 12), fill=(255, 255, 255), width=5)
    d.line((ax - 12, ay, ax, ay + 13, ax + 12, ay), fill=(255, 255, 255), width=5, joint="curve")
    d.line((ax - 16, ay + 20, ax + 16, ay + 20), fill=(255, 255, 255), width=5)
    d.text((176, 380), "Download for Windows", font=font("Bold", 26), fill=(255, 255, 255))
    ring(d, (88, 348, 532, 442), 47)
    cursor(im, 470, 412)
    badge(d, 92, 356, 1)
    # the download chip
    chip = (560, 470, 900, 552)
    shadowed(im, chip, 18, (38, 38, 50, 255), blur=10, offset=(0, 6), alpha=90)
    d = ImageDraw.Draw(im)
    d.rounded_rectangle((580, 488, 630, 534), 10, fill=(255, 255, 255))
    d.text((590, 498), "exe", font=font("Bold", 20), fill=INK)
    d.text((646, 484), "Bloop.exe", font=font("Bold", 24), fill=(255, 255, 255))
    d.text((646, 516), "1.7 MB  ·  done", font=font("Regular", 18), fill=(150, 220, 160))
    badge(d, 884, 474, 2)
    d.text((100, 480), "Press the orange button.", font=font("Semibold", 26), fill=INK)
    d.text((100, 516), "The file lands in your Downloads folder.", font=font("Regular", 22), fill=(93, 90, 82))
    return im.convert("RGB")


# ---------------------------------------------------------- 2. SmartScreen
def dialog(im, x, y, w, h, stage):
    shadowed(im, (x, y, x + w, y + h), 14, (12, 52, 130, 255), blur=16, offset=(0, 10), alpha=120)
    d = ImageDraw.Draw(im)
    d.text((x + 28, y + 26), "Windows protected your PC", font=font("Bold", 26), fill=(255, 255, 255))
    f = font("Regular", 19)
    body = "Microsoft Defender SmartScreen prevented an unrecognized app from starting. Running this app might put your PC at risk."
    yy = y + 78
    for line in wrap(d, body, f, w - 56):
        d.text((x + 28, yy), line, font=f, fill=(225, 235, 255))
        yy += 28
    if stage == 1:
        lx, ly = x + 36, yy + 30
        d.text((lx, ly), "More info", font=font("Semibold", 20), fill=(255, 255, 255))
        d.line((lx, ly + 28, lx + d.textlength("More info", font=font("Semibold", 20)), ly + 28), fill=(255, 255, 255), width=2)
        ring(d, (lx - 12, ly - 8, lx + 112, ly + 40), 12)
        badge(d, lx - 12, ly - 8, 1)
        tx, ty = lx + 100, ly + 22
    else:
        d.text((x + 28, yy + 10), "App: Bloop.exe", font=font("Semibold", 19), fill=(255, 255, 255))
        d.text((x + 28, yy + 38), "Publisher: Unknown publisher", font=font("Regular", 19), fill=(225, 235, 255))
    by = y + h - 70
    bw = 170
    if stage == 2:
        d.rounded_rectangle((x + w - 2 * bw - 40, by, x + w - bw - 40, by + 46), 8, fill=(255, 255, 255))
        text_center(d, x + w - 1.5 * bw - 40, by + 10, "Run anyway", font("Bold", 20), (12, 52, 130))
        ring(d, (x + w - 2 * bw - 52, by - 12, x + w - bw - 28, by + 58), 14)
        badge(d, x + w - 2 * bw - 52, by - 12, 2)
    d.rounded_rectangle((x + w - bw - 20, by, x + w - 20, by + 46), 8, fill=(255, 255, 255) if stage == 1 else (40, 84, 160))
    text_center(d, x + w - bw / 2 - 20, by + 10, "Don't run", font("Bold", 20), (12, 52, 130) if stage == 1 else (255, 255, 255))
    if stage == 1:
        return tx, ty
    return None


def step2():
    im = wallpaper().convert("RGBA")
    dialog(im, 40, 120, 430, 400, 1)
    dialog(im, 530, 120, 430, 400, 2)
    d = ImageDraw.Draw(im)
    # arrow between
    d.polygon([(478, 300), (508, 320), (478, 340)], fill=ORANGE)
    d.text((44, 60), "If Windows shows this warning (the app is new and not signed):", font=font("Semibold", 24), fill=(255, 255, 255))
    d.rounded_rectangle((40, 548, 392, 584), 18, fill=(0, 0, 0, 120))
    d.text((56, 553), "Illustration, not a screenshot", font=font("Regular", 19), fill=(220, 220, 230))
    return im.convert("RGB")


# ------------------------------------------------------ 3. the island + tray
def step3():
    im = wallpaper().convert("RGBA")
    raw = Image.open(f"{IMG}/raw-pill.png").convert("RGBA")
    bbox = raw.getchannel("A").point(lambda v: 255 if v > 8 else 0).getbbox()
    pill = raw.crop(bbox)
    sc = 0.9
    pill = pill.resize((int(pill.width * sc), int(pill.height * sc)), Image.LANCZOS)
    px, py = (W - pill.width) // 2, 40
    im.alpha_composite(pill, (px, py))
    d = ImageDraw.Draw(im)
    # a taskbar
    tb = Image.new("RGBA", (W, 64), (16, 16, 24, 230))
    im.alpha_composite(tb, (0, H - 64))
    d = ImageDraw.Draw(im)
    for i, c in enumerate([(74, 168, 255), (60, 60, 72), (255, 178, 58), (60, 60, 72), (255, 106, 168)]):
        d.rounded_rectangle((390 + i * 46, H - 46, 420 + i * 46, H - 16), 9, fill=c)
    # tray: bloop icon, wifi, speaker, clock
    icon = Image.open(f"{ASSETS}/app.png").convert("RGBA").resize((34, 34), Image.LANCZOS)
    tx = 700
    im.alpha_composite(icon, (tx, H - 47))
    d = ImageDraw.Draw(im)
    for k in range(2):
        d.ellipse((tx + 56 + k * 38, H - 40, tx + 74 + k * 38, H - 22), fill=(170, 170, 185))
    d.text((tx + 140, H - 52), "14:05", font=font("Semibold", 20), fill=(235, 235, 245))
    d.text((tx + 140, H - 30), "28/11", font=font("Regular", 15), fill=(190, 190, 205))
    ring(d, (tx - 9, H - 55, tx + 43, H - 7), 12)
    badge(d, tx - 9, H - 55, 2)
    # labels
    f = font("Semibold", 26)
    lx0 = px + pill.width + 36
    d.rounded_rectangle((lx0, py + 10, lx0 + 190, py + 58), 24, fill=(0, 0, 0, 255))
    d.text((lx0 + 40, py + 18), "The island", font=f, fill=(255, 255, 255))
    badge(d, lx0, py + 34, 1)
    d.line((px + pill.width + 4, py + 34, lx0, py + 34), fill=ORANGE, width=4)
    d.rounded_rectangle((520, 270, 960, 340), 22, fill=(0, 0, 0, 150))
    d.text((545, 280), "Click the tray icon to open Settings", font=font("Semibold", 22), fill=(255, 255, 255))
    d.text((545, 308), "(next to the clock)", font=font("Regular", 18), fill=(200, 200, 215))
    d.line((860, 340, tx + 18, H - 62), fill=ORANGE, width=4)
    d.rounded_rectangle((40, 160, 470, 250), 22, fill=(0, 0, 0, 150))
    d.text((64, 172), "Windows asks once: may Bloop read", font=font("Regular", 21), fill=(235, 235, 245))
    d.text((64, 204), "your notifications? Say yes.", font=font("Semibold", 21), fill=(255, 255, 255))
    return im.convert("RGB")


# ------------------------------------------------------------ 4. settings
def step4():
    src = Image.open(f"{IMG}/settings-general.png").convert("RGBA")
    crop = src.crop((70, 340, 910, 640))
    sc = 1.19
    crop = crop.resize((int(crop.width * sc), int(crop.height * sc)), Image.LANCZOS)
    canvas = wallpaper(W, H).convert("RGBA")
    ox, oy = (W - crop.width) // 2, 140
    shadowed(canvas, (ox - 6, oy - 6, ox + crop.width + 6, oy + crop.height + 6), 20, (0, 0, 0, 255), blur=16, offset=(0, 12), alpha=140)
    canvas.alpha_composite(crop, (ox, oy))
    d = ImageDraw.Draw(canvas)
    # highlight the "Start with Windows" card: it sits at (109,480)-(485,547) in the original picture
    def tr(x, y):
        return ((x - 70) * sc + ox, (y - 340) * sc + oy)
    x0, y0 = tr(105, 476)
    x1, y1 = tr(490, 551)
    ring(d, (x0, y0, x1, y1), 22)
    badge(d, int(x0), int(y0), 1)
    d.text((44, 50), "Open Settings (click the tray icon), then:", font=font("Semibold", 26), fill=(255, 255, 255))
    d.rounded_rectangle((int(x1) + 40, int(y0) + 10, int(x1) + 40 + 330, int(y0) + 66), 24, fill=(0, 0, 0, 150))
    d.text((int(x1) + 62, int(y0) + 22), "Starts Bloop with Windows", font=font("Semibold", 21), fill=(255, 255, 255))
    d.line((x1 + 4, (y0 + y1) / 2, x1 + 40, (y0 + y1) / 2), fill=ORANGE, width=4)
    d.text((44, 560), "To quit or remove Bloop: Quit Bloop (same page), then delete the file.", font=font("Regular", 21), fill=(230, 230, 240))
    return canvas.convert("RGB")


for name, fn in [("tut-1-download", step1), ("tut-2-smartscreen", step2), ("tut-3-island", step3), ("tut-4-settings", step4)]:
    fn().save(f"{IMG}/{name}.png", optimize=True)
    print("wrote", name)
