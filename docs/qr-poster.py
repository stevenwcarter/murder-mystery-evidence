# /// script
# requires-python = ">=3.11"
# dependencies = ["reportlab>=4", "qrcode>=8"]
# ///
"""Letter-size, black-and-white Art Deco QR poster for the evidence site.

Run:  uv run docs/qr-poster.py [OUTPUT.pdf]
"""

import math
import sys

import qrcode
from qrcode.constants import ERROR_CORRECT_H
from reportlab.lib.pagesizes import letter
from reportlab.pdfgen import canvas

URL = "https://mm.carterfam.us/"
TITLE_1 = "JAZZ, GIN,"
TITLE_2 = "MONEY & MURDER"
SUBTITLE = "EVIDENCE LOCKER"
LEAD = "Scan to see the evidence as the detective uncovers it."
FOOT = "EVERYONE IS A SUSPECT"

W, H = letter
OUT = sys.argv[1] if len(sys.argv) > 1 else "evidence-qr-poster.pdf"


def tracked(c, x, y, text, font, size, track, align="center"):
    """Draw text with letter-spacing (tracking); x is the anchor per align."""
    width = c.stringWidth(text, font, size) + track * (len(text) - 1)
    if align == "center":
        x -= width / 2
    elif align == "right":
        x -= width
    # Tc (character spacing) is part of the PDF graphics state and would
    # leak into later text, so bracket it with q/Q.
    c.saveState()
    t = c.beginText(x, y)
    t.setFont(font, size)
    t.setCharSpace(track)
    t.textOut(text)
    c.drawText(t)
    c.restoreState()
    return width


def stepped_corner(c, x, y, sx, sy, size, steps=3, lw=1.5):
    """Nested right-angle steps at a corner. (x, y) is the outer corner;
    sx, sy are +1/-1 pointing inward."""
    c.setLineWidth(lw)
    gap = size / steps
    for i in range(steps):
        d = gap * (i + 1)
        length = size * (1.6 - 0.35 * i)
        # horizontal arm
        c.line(x + sx * d, y + sy * d, x + sx * (d + length), y + sy * d)
        # vertical arm
        c.line(x + sx * d, y + sy * d, x + sx * d, y + sy * (d + length))


def double_frame(c, x0, y0, x1, y1, outer=2.4, inner=0.7, gap=7):
    c.setLineWidth(outer)
    c.rect(x0, y0, x1 - x0, y1 - y0)
    c.setLineWidth(inner)
    c.rect(x0 + gap, y0 + gap, x1 - x0 - 2 * gap, y1 - y0 - 2 * gap)


def sunburst(c, cx, cy, r, up=True, rays=21, lw=0.9):
    """Half-circle fan of rays, clipped to a semicircle, with a small keystone."""
    c.saveState()
    p = c.beginPath()
    if up:
        p.moveTo(cx - r, cy)
        p.arcTo(cx - r, cy - r, cx + r, cy + r, 0, 180)
    else:
        p.moveTo(cx + r, cy)
        p.arcTo(cx - r, cy - r, cx + r, cy + r, 180, 180)
    p.close()
    c.clipPath(p, stroke=0, fill=0)
    c.setLineWidth(lw)
    for i in range(rays):
        a = math.pi * i / (rays - 1)
        if not up:
            a = -a
        c.line(cx, cy, cx + r * math.cos(a), cy + r * math.sin(a))
    c.restoreState()
    # concentric arcs
    for k, w in ((1.0, 1.6), (0.72, 0.7), (0.45, 0.7)):
        c.setLineWidth(w)
        rr = r * k
        start = 0 if up else 180
        c.arc(cx - rr, cy - rr, cx + rr, cy + rr, start, 180)
    # keystone diamond at the hub
    c.setLineWidth(1)
    d = 7
    p = c.beginPath()
    p.moveTo(cx, cy + d)
    p.lineTo(cx + d, cy)
    p.lineTo(cx, cy - d)
    p.lineTo(cx - d, cy)
    p.close()
    c.drawPath(p, stroke=1, fill=1)


def rule_with_diamonds(c, cx, y, half, lw=1.2, d=4):
    c.setLineWidth(lw)
    c.line(cx - half, y, cx - d * 3, y)
    c.line(cx + d * 3, y, cx + half, y)
    for px in (cx - d * 1.6, cx + d * 1.6):
        p = c.beginPath()
        p.moveTo(px, y + d)
        p.lineTo(px + d, y)
        p.lineTo(px, y - d)
        p.lineTo(px - d, y)
        p.close()
        c.drawPath(p, stroke=0, fill=1)


def draw_qr(c, cx, cy, size):
    qr = qrcode.QRCode(error_correction=ERROR_CORRECT_H, border=0, box_size=1)
    qr.add_data(URL)
    qr.make(fit=True)
    m = qr.get_matrix()
    n = len(m)
    cell = size / n
    x0 = cx - size / 2
    y0 = cy - size / 2
    c.setFillColorRGB(0, 0, 0)
    for row in range(n):
        for col in range(n):
            if m[row][col]:
                # +0.25 overlap hides hairline seams between adjacent cells
                c.rect(
                    x0 + col * cell,
                    y0 + (n - 1 - row) * cell,
                    cell + 0.25,
                    cell + 0.25,
                    stroke=0,
                    fill=1,
                )
    return n


def main():
    c = canvas.Canvas(OUT, pagesize=letter)
    c.setTitle("Jazz, Gin, Money & Murder — Evidence Locker")
    c.setStrokeColorRGB(0, 0, 0)
    c.setFillColorRGB(0, 0, 0)

    # Page frame (0.5 in margin) with stepped corners inside it.
    m = 36
    double_frame(c, m, m, W - m, H - m)
    corner = 26
    for x, sx in ((m + 7, 1), (W - m - 7, -1)):
        for y, sy in ((m + 7, 1), (H - m - 7, -1)):
            stepped_corner(c, x, y, sx, sy, corner)

    cx = W / 2

    # Top sunburst, rising from behind the title rule.
    sunburst(c, cx, H - 168, 96, up=True)

    # Title block
    rule_with_diamonds(c, cx, H - 176, 190)
    tracked(c, cx, H - 216, TITLE_1, "Helvetica-Bold", 30, 7)
    tracked(c, cx, H - 254, TITLE_2, "Helvetica-Bold", 30, 7)
    rule_with_diamonds(c, cx, H - 272, 190)
    tracked(c, cx, H - 300, SUBTITLE, "Helvetica", 15, 6)

    # QR with its own double frame and stepped corners
    qr_size = 262
    pad = 20
    frame_top = H - 300 - 28
    qcx = cx
    qcy = frame_top - pad - qr_size / 2
    fx0, fy0 = qcx - qr_size / 2 - pad, qcy - qr_size / 2 - pad
    fx1, fy1 = qcx + qr_size / 2 + pad, frame_top
    double_frame(c, fx0, fy0, fx1, fy1, outer=1.8, inner=0.6, gap=5)
    for x, sx in ((fx0 + 5, 1), (fx1 - 5, -1)):
        for y, sy in ((fy0 + 5, 1), (fy1 - 5, -1)):
            stepped_corner(c, x, y, sx, sy, 12, steps=2, lw=1.0)
    draw_qr(c, qcx, qcy, qr_size)

    # Lead line, URL, footer
    y = fy0 - 32
    c.setFont("Times-Italic", 15)
    c.drawCentredString(cx, y, LEAD)
    rule_with_diamonds(c, cx, y - 20, 150, lw=0.9, d=3)
    tracked(c, cx, y - 50, "mm.carterfam.us", "Helvetica-Bold", 24, 4)
    tracked(c, cx, 58, FOOT, "Helvetica", 10, 5)

    c.showPage()
    c.save()
    print(OUT)


if __name__ == "__main__":
    main()
