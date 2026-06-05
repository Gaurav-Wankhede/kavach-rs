#!/usr/bin/env python3
"""Generate Kavach brand-logo candidates (steel/armor metallic).

Three distinct shield concepts rendered at 1024px with 4x supersampling for
crisp anti-aliased edges, then downscaled. Output: candidate_{1,2,3}.png plus a
side-by-side contact sheet for quick comparison.

Palette (steel/armor, harmonized with the kavach-app GUI dark theme):
  bg deep    #11141B   (GUI --bg)
  bg panel   #1A1D27
  steel hi   #C7CDD9   (silver highlight)
  steel mid  #8A93A6   (GUI steel gray)
  steel lo   #3A3F4B   (gunmetal)
  accent     #4F8CFF   (GUI blue accent)
"""
from __future__ import annotations

import math
from PIL import Image, ImageDraw

S = 1024
SS = 4  # supersample factor
W = S * SS

BG_DEEP = (17, 20, 27)
BG_PANEL = (26, 29, 39)
STEEL_HI = (199, 205, 217)
STEEL_MID = (138, 147, 166)
STEEL_LO = (58, 63, 75)
ACCENT = (79, 140, 255)
INK = (13, 15, 20)


def lerp(a, b, t):
    return tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3))


def vgrad(size, top, bottom):
    img = Image.new("RGB", size, top)
    px = img.load()
    h = size[1]
    for y in range(h):
        c = lerp(top, bottom, y / max(1, h - 1))
        for x in range(size[0]):
            px[x, y] = c
    return img


def rounded_panel(draw, box, radius, fill):
    draw.rounded_rectangle(box, radius=radius, fill=fill)


def shield_path(cx, top, half_w, shoulder_y, point_y):
    """Heater-shield polygon: flat top, straight shoulders, tapering to a point."""
    return [
        (cx - half_w, top),
        (cx + half_w, top),
        (cx + half_w, shoulder_y),
        (cx, point_y),
        (cx - half_w, shoulder_y),
    ]


def base_canvas():
    """Rounded-square app-icon background with a subtle vertical gunmetal gradient."""
    bg = vgrad((W, W), BG_PANEL, BG_DEEP).convert("RGBA")
    mask = Image.new("L", (W, W), 0)
    md = ImageDraw.Draw(mask)
    md.rounded_rectangle([0, 0, W - 1, W - 1], radius=int(W * 0.22), fill=255)
    out = Image.new("RGBA", (W, W), (0, 0, 0, 0))
    out.paste(bg, (0, 0), mask)
    return out


def metallic_shield(draw_img, cx, top, half_w, shoulder_y, point_y, light=STEEL_HI, dark=STEEL_LO):
    """Draw a shield filled with a top-down metallic gradient via clipped grad paste."""
    poly = shield_path(cx, top, half_w, shoulder_y, point_y)
    grad = vgrad((W, W), light, dark).convert("RGBA")
    smask = Image.new("L", (W, W), 0)
    sd = ImageDraw.Draw(smask)
    sd.polygon(poly, fill=255)
    draw_img.paste(grad, (0, 0), smask)
    return poly


def candidate_1():
    """Evolved shield + checkmark — metallic plate, accent check, beveled rim."""
    img = base_canvas()
    d = ImageDraw.Draw(img)
    cx = W // 2
    top = int(W * 0.24)
    half_w = int(W * 0.30)
    shoulder_y = int(W * 0.56)
    point_y = int(W * 0.80)
    # outer rim (silver)
    rim = shield_path(cx, top - int(W * 0.012), half_w + int(W * 0.012), shoulder_y + int(W * 0.008), point_y + int(W * 0.016))
    d.polygon(rim, fill=STEEL_HI)
    # metallic body
    metallic_shield(img, cx, top, half_w, shoulder_y, point_y, STEEL_MID, STEEL_LO)
    d = ImageDraw.Draw(img)
    # inset darker plate for depth
    inset = shield_path(cx, top + int(W * 0.05), half_w - int(W * 0.05), shoulder_y - int(W * 0.03), point_y - int(W * 0.05))
    d.polygon(inset, fill=lerp(STEEL_LO, INK, 0.35))
    # accent checkmark
    lw = int(W * 0.05)
    p1 = (cx - int(W * 0.13), int(W * 0.50))
    p2 = (cx - int(W * 0.02), int(W * 0.60))
    p3 = (cx + int(W * 0.17), int(W * 0.40))
    d.line([p1, p2, p3], fill=ACCENT, width=lw, joint="curve")
    for p in (p1, p2, p3):
        d.ellipse([p[0] - lw // 2, p[1] - lw // 2, p[0] + lw // 2, p[1] + lw // 2], fill=ACCENT)
    return img


def candidate_2():
    """Shield + monogram K — bold silver K carved into a gunmetal shield."""
    img = base_canvas()
    d = ImageDraw.Draw(img)
    cx = W // 2
    top = int(W * 0.23)
    half_w = int(W * 0.31)
    shoulder_y = int(W * 0.57)
    point_y = int(W * 0.81)
    # accent rim
    rim = shield_path(cx, top - int(W * 0.014), half_w + int(W * 0.014), shoulder_y + int(W * 0.01), point_y + int(W * 0.018))
    d.polygon(rim, fill=ACCENT)
    metallic_shield(img, cx, top, half_w, shoulder_y, point_y, lerp(STEEL_MID, STEEL_HI, 0.3), STEEL_LO)
    d = ImageDraw.Draw(img)
    # Monogram K (silver), geometric strokes
    kx = cx - int(W * 0.10)
    k_top = int(W * 0.32)
    k_bot = int(W * 0.66)
    lw = int(W * 0.055)
    # vertical stem
    d.line([(kx, k_top), (kx, k_bot)], fill=STEEL_HI, width=lw)
    mid = (kx, int((k_top + k_bot) / 2))
    # upper diagonal
    d.line([mid, (kx + int(W * 0.18), k_top)], fill=STEEL_HI, width=lw, joint="curve")
    # lower diagonal (accent leg for brand pop)
    d.line([mid, (kx + int(W * 0.20), k_bot)], fill=ACCENT, width=lw, joint="curve")
    for p in [(kx, k_top), (kx, k_bot), mid, (kx + int(W * 0.18), k_top), (kx + int(W * 0.20), k_bot)]:
        d.ellipse([p[0] - lw // 2, p[1] - lw // 2, p[0] + lw // 2, p[1] + lw // 2], fill=STEEL_HI if p != (kx + int(W*0.20), k_bot) else ACCENT)
    return img


def candidate_3():
    """Layered guard-mark — stacked shield plates (armor lamellae) + accent core."""
    img = base_canvas()
    d = ImageDraw.Draw(img)
    cx = W // 2
    # three nested shields offset downward = layered armor plates
    layers = [
        (0.21, 0.32, STEEL_LO),
        (0.255, 0.305, STEEL_MID),
        (0.30, 0.29, lerp(STEEL_MID, STEEL_HI, 0.5)),
    ]
    for top_f, hw_f, col in layers:
        top = int(W * top_f)
        half_w = int(W * hw_f)
        shoulder_y = top + int(W * 0.30)
        point_y = top + int(W * 0.50)
        d.polygon(shield_path(cx, top, half_w, shoulder_y, point_y), fill=col)
    # accent core diamond (the protected center)
    core = int(W * 0.085)
    ccy = int(W * 0.50)
    d.polygon([(cx, ccy - core), (cx + core, ccy), (cx, ccy + core), (cx - core, ccy)], fill=ACCENT)
    # thin silver outline on core
    d.line([(cx, ccy - core), (cx + core, ccy), (cx, ccy + core), (cx - core, ccy), (cx, ccy - core)],
           fill=STEEL_HI, width=int(W * 0.008), joint="curve")
    return img


def finalize(img):
    return img.resize((S, S), Image.LANCZOS)


def main():
    import os
    out_dir = os.path.dirname(os.path.abspath(__file__))
    cands = [candidate_1(), candidate_2(), candidate_3()]
    finals = []
    for i, c in enumerate(cands, 1):
        f = finalize(c)
        path = os.path.join(out_dir, f"candidate_{i}.png")
        f.save(path)
        finals.append(f)
        print(f"wrote {path}")
    # contact sheet
    pad = 40
    sheet_w = S * 3 + pad * 4
    sheet_h = S + pad * 2
    sheet = Image.new("RGB", (sheet_w, sheet_h), (8, 9, 12))
    for i, f in enumerate(finals):
        sheet.paste(f, (pad + i * (S + pad), pad))
    sheet_path = os.path.join(out_dir, "candidates_contact_sheet.png")
    sheet.resize((sheet_w // 2, sheet_h // 2), Image.LANCZOS).save(sheet_path)
    print(f"wrote {sheet_path}")


if __name__ == "__main__":
    main()
