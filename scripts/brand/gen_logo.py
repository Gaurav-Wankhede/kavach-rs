#!/usr/bin/env python3
"""Render the Kavach brand logo (candidate ① — evolved shield + check) to the
full asset set: app icon, README mark, social preview, and favicon sizes.

Master geometry is defined once at high resolution with 4x supersampling for
clean anti-aliased edges, then downscaled per target. The matching vector
master lives at scripts/brand/kavach-logo.svg (hand-authored, same geometry).

Steel/armor palette, harmonized with the kavach-app GUI dark theme.
Usage: python3 gen_logo.py            # writes all assets under scripts/brand/out/
"""
from __future__ import annotations

import os
from PIL import Image, ImageDraw

SS = 4
MASTER = 1024 * SS

# palette
BG_DEEP = (17, 20, 27)
BG_PANEL = (28, 32, 43)
STEEL_HI = (205, 211, 223)
STEEL_MID = (138, 147, 166)
STEEL_LO = (52, 57, 69)
PANEL_INK = (24, 27, 35)
ACCENT = (79, 140, 255)
ACCENT_HI = (124, 170, 255)

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "out")


def lerp(a, b, t):
    return tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3))


def vgrad(size, top, bottom):
    img = Image.new("RGB", size, top)
    px = img.load()
    h = size[1]
    for y in range(h):
        c = lerp(top, bottom, y / max(1, h - 1))
        row = [c] * size[0]
        for x in range(size[0]):
            px[x, y] = row[x]
    return img


def shield(cx, top, half_w, shoulder_y, point_y):
    return [
        (cx - half_w, top),
        (cx + half_w, top),
        (cx + half_w, shoulder_y),
        (cx, point_y),
        (cx - half_w, shoulder_y),
    ]


def paste_grad(img, poly, top_col, bot_col):
    grad = vgrad(img.size, top_col, bot_col).convert("RGBA")
    mask = Image.new("L", img.size, 0)
    ImageDraw.Draw(mask).polygon(poly, fill=255)
    img.paste(grad, (0, 0), mask)


def render_master(rounded_bg: bool):
    """Render the logo at MASTER resolution.

    rounded_bg=True  -> app-icon form (rounded-square gunmetal backdrop)
    rounded_bg=False -> transparent backdrop (README / favicon on any surface)
    """
    W = MASTER
    img = Image.new("RGBA", (W, W), (0, 0, 0, 0))
    if rounded_bg:
        bg = vgrad((W, W), BG_PANEL, BG_DEEP).convert("RGBA")
        mask = Image.new("L", (W, W), 0)
        ImageDraw.Draw(mask).rounded_rectangle([0, 0, W - 1, W - 1], radius=int(W * 0.22), fill=255)
        img.paste(bg, (0, 0), mask)

    cx = W // 2
    top = int(W * 0.225)
    half_w = int(W * 0.305)
    shoulder_y = int(W * 0.565)
    point_y = int(W * 0.805)

    d = ImageDraw.Draw(img)
    # 1) silver beveled rim (slightly larger shield behind)
    rim = shield(cx, top - int(W * 0.016), half_w + int(W * 0.016),
                 shoulder_y + int(W * 0.011), point_y + int(W * 0.020))
    paste_grad(img, rim, STEEL_HI, lerp(STEEL_HI, STEEL_MID, 0.6))
    # 2) gunmetal body plate (top-down metallic gradient)
    body = shield(cx, top, half_w, shoulder_y, point_y)
    paste_grad(img, body, lerp(STEEL_MID, STEEL_HI, 0.15), STEEL_LO)
    # 3) recessed inner panel for depth
    d = ImageDraw.Draw(img)
    inset = shield(cx, top + int(W * 0.055), half_w - int(W * 0.058),
                   shoulder_y - int(W * 0.035), point_y - int(W * 0.060))
    paste_grad(img, inset, lerp(PANEL_INK, STEEL_LO, 0.25), PANEL_INK)
    # 4) accent check with rounded caps + subtle top highlight stroke
    d = ImageDraw.Draw(img)
    lw = int(W * 0.052)
    p1 = (cx - int(W * 0.135), int(W * 0.495))
    p2 = (cx - int(W * 0.020), int(W * 0.600))
    p3 = (cx + int(W * 0.165), int(W * 0.395))

    def stroke(col, width, dy=0):
        pts = [(x, y + dy) for (x, y) in (p1, p2, p3)]
        d.line(pts, fill=col, width=width, joint="curve")
        for p in pts:
            d.ellipse([p[0] - width // 2, p[1] - width // 2,
                       p[0] + width // 2, p[1] + width // 2], fill=col)

    stroke(ACCENT, lw)
    stroke(ACCENT_HI, max(2, int(lw * 0.22)), dy=-int(lw * 0.28))  # top sheen
    return img


def save_png(img, size, path):
    img.resize((size, size), Image.LANCZOS).save(path)
    print(f"wrote {path} ({size}x{size})")


def main():
    os.makedirs(OUT, exist_ok=True)
    icon = render_master(rounded_bg=True)       # app/dmg/social use the badged form
    flat = render_master(rounded_bg=False)       # transparent mark for README/favicon

    # App icon master (Dioxus bundle expects assets/icon.png at 1024)
    save_png(icon, 1024, os.path.join(OUT, "icon-1024.png"))
    save_png(icon, 512, os.path.join(OUT, "icon-512.png"))
    save_png(icon, 256, os.path.join(OUT, "icon-256.png"))
    save_png(icon, 128, os.path.join(OUT, "icon-128.png"))

    # README mark (transparent, generous)
    save_png(flat, 512, os.path.join(OUT, "logo-512.png"))

    # Favicons
    for s in (16, 32, 48, 180, 192, 512):
        save_png(flat, s, os.path.join(OUT, f"favicon-{s}.png"))
    # .ico bundle (16/32/48)
    ico_path = os.path.join(OUT, "favicon.ico")
    flat.resize((256, 256), Image.LANCZOS).save(
        ico_path, sizes=[(16, 16), (32, 32), (48, 48)])
    print(f"wrote {ico_path} (16/32/48)")

    # Social preview 1280x640 — centered icon on deep gunmetal with wordmark space
    sp = Image.new("RGB", (1280, 640), BG_DEEP)
    badge = icon.resize((460, 460), Image.LANCZOS)
    sp.paste(badge, (150, 90), badge)
    dd = ImageDraw.Draw(sp)
    # wordmark — resolve a TTF across macOS / Linux / Windows. The lists are
    # ordered best-first; the renderer also probes a font-name lookup (so PIL's
    # own search path is honored) and finally degrades to the bundled bitmap
    # font, so the social card is always produced even with no system TTFs
    # (e.g. a minimal CI runner). On bitmap fallback the type sizing differs;
    # we log a warning rather than silently ship a degraded card.
    from PIL import ImageFont
    title_fonts = [
        "/System/Library/Fonts/SFCompactRounded.ttf",       # macOS
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",   # Debian/Ubuntu
        "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf",            # Fedora/Arch
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        "C:\\Windows\\Fonts\\arialbd.ttf",                        # Windows
        "DejaVuSans-Bold.ttf", "Arial Bold.ttf", "arialbd.ttf",   # name lookup
    ]
    sub_fonts = [
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
        "DejaVuSans.ttf", "Arial.ttf", "arial.ttf",
    ]

    def first_font(paths, size):
        for p in paths:
            try:
                # ImageFont.truetype resolves absolute paths AND bare font
                # names via its own search path, so both forms work here.
                return ImageFont.truetype(p, size)
            except OSError:
                continue
        print(f"WARN: no system TTF found among {len(paths)} candidates; "
              f"falling back to PIL bitmap font (degraded wordmark sizing)")
        return ImageFont.load_default()

    font = first_font(title_fonts, 150)
    sub = first_font(sub_fonts, 36)
    dd.text((680, 230), "Kavach", font=font, fill=STEEL_HI)
    subtitle = "Autonomous engineering guardrails"
    # Shrink the subtitle until it fits within the right margin (1280 - 686 - 40).
    # Drive the loop by an explicit `size` counter — NOT font.size — because the
    # bitmap fallback (load_default) has no resizable size and would otherwise
    # spin or raise. Only retry the lookup while it keeps yielding a truetype
    # font that actually honors the requested size.
    max_w = 1280 - 686 - 40
    size = 36
    while dd.textlength(subtitle, font=sub) > max_w and size > 20:
        size -= 2
        candidate = first_font(sub_fonts, size)
        if getattr(candidate, "size", None) != size:
            break  # bitmap fallback — can't shrink further; keep what we have
        sub = candidate
    dd.text((686, 400), subtitle, font=sub, fill=STEEL_MID)
    sp_path = os.path.join(OUT, "social-preview.png")
    sp.save(sp_path)
    print(f"wrote {sp_path} (1280x640)")


if __name__ == "__main__":
    main()
