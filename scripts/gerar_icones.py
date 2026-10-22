"""Gera os ícones SVG do SolidCadCraft (estilo ilustrado, 48x48). Uso: icons.py <pasta_destino>"""
import os, sys

OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)

INK = "#1b2a41"      # contorno
LIGHT = "#cfe0f3"    # face clara
MID = "#93b4d9"      # face média
DARK = "#5b84b5"     # face escura
SKETCH = "#0ea5e9"   # sketch
ACTION = "#f59e0b"   # ação
ACTION_L = "#fde68a"
CUT = "#ef4444"
REF = "#34d399"
REF_L = "#d1fae5"

S = f'stroke="{INK}" stroke-width="1.6" stroke-linejoin="round" stroke-linecap="round"'


def svg(body):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" width="48" height="48">{body}</svg>\n'


def box(top, h, cols=(LIGHT, MID, DARK), w=18, hh=9):
    """Caixa isométrica: `top` = (x, y) do canto de cima do losango superior."""
    x, y = top
    t = [(x, y), (x + w, y + hh), (x, y + 2 * hh), (x - w, y + hh)]
    p = lambda pts: " ".join(f"{a},{b}" for a, b in pts)
    left = [t[3], t[2], (t[2][0], t[2][1] + h), (t[3][0], t[3][1] + h)]
    right = [t[2], t[1], (t[1][0], t[1][1] + h), (t[2][0], t[2][1] + h)]
    return (f'<polygon points="{p(left)}" fill="{cols[1]}" {S}/>'
            f'<polygon points="{p(right)}" fill="{cols[2]}" {S}/>'
            f'<polygon points="{p(t)}" fill="{cols[0]}" {S}/>')


def arrow(x1, y1, x2, y2, color=ACTION, w=3.2):
    import math
    a = math.atan2(y2 - y1, x2 - x1)
    hl, hw = 6.5, 4.5
    bx, by = x2 - hl * math.cos(a), y2 - hl * math.sin(a)
    px, py = -math.sin(a) * hw, math.cos(a) * hw
    return (f'<line x1="{x1}" y1="{y1}" x2="{bx:.1f}" y2="{by:.1f}" stroke="{color}" stroke-width="{w}" stroke-linecap="round"/>'
            f'<polygon points="{x2},{y2} {bx+px:.1f},{by+py:.1f} {bx-px:.1f},{by-py:.1f}" fill="{color}" stroke="{INK}" stroke-width="1.1" stroke-linejoin="round"/>')


I = {}

I["sketch"] = (
    f'<polygon points="4,31 24,21 44,31 24,41" fill="{REF_L}" {S}/>'
    f'<polygon points="13,31 24,25.5 35,31 24,36.5" fill="none" stroke="{SKETCH}" stroke-width="2.2" stroke-linejoin="round"/>'
    f'<circle cx="24" cy="31" r="1.8" fill="{SKETCH}"/>'
    f'<g transform="rotate(40 30 13)"><rect x="27" y="1" width="7" height="19" rx="1.5" fill="{ACTION}" {S}/>'
    f'<rect x="27" y="1" width="7" height="4" rx="1.5" fill="#f9a8d4" {S}/>'
    f'<polygon points="27,20 30.5,26 34,20" fill="{ACTION_L}" {S}/></g>')

I["extrusao"] = (
    f'<polygon points="6,36 24,27 42,36 24,45" fill="none" stroke="{SKETCH}" stroke-width="1.8" stroke-dasharray="3 2"/>'
    + box((24, 15), 12) + arrow(24, 24, 24, 3))

I["recorte"] = (
    box((24, 12), 14)
    + f'<polygon points="15,21 24,16.5 33,21 24,25.5" fill="{INK}" opacity="0.85"/>'
    + f'<polygon points="15,21 24,25.5 33,21 33,24 24,28.5 15,24" fill="{DARK}" opacity="0.6"/>'
    + arrow(24, 2, 24, 18, CUT))

I["revolucao"] = (
    f'<path d="M10 14 V34 A14 5 0 0 0 38 34 V14" fill="{MID}" {S}/>'
    f'<ellipse cx="24" cy="14" rx="14" ry="5" fill="{LIGHT}" {S}/>'
    f'<rect x="26" y="15" width="9" height="19" fill="none" stroke="{SKETCH}" stroke-width="2.2"/>'
    f'<line x1="24" y1="3" x2="24" y2="46" stroke="{INK}" stroke-width="1.4" stroke-dasharray="4 2.5"/>'
    f'<path d="M7 26 A17 6.5 0 0 0 41 26" fill="none" stroke="{ACTION}" stroke-width="3" stroke-linecap="round"/>'
    f'<polygon points="41,20 45,27 37,27" fill="{ACTION}" stroke="{INK}" stroke-width="1.1" stroke-linejoin="round"/>')

I["furo"] = (
    box((24, 14), 13)
    + f'<ellipse cx="24" cy="23" rx="7" ry="3.5" fill="{INK}"/>'
    + f'<ellipse cx="24" cy="23" rx="9.5" ry="4.8" fill="none" stroke="{ACTION}" stroke-width="1.6"/>'
    + arrow(24, 2, 24, 19))

I["filete"] = (
    f'<path d="M7 42 V24 A16 16 0 0 1 23 8 H42 V42 Z" fill="{MID}" {S}/>'
    f'<path d="M7 24 A16 16 0 0 1 23 8" fill="none" stroke="{ACTION}" stroke-width="3.6" stroke-linecap="round"/>'
    f'<line x1="23" y1="24" x2="11.7" y2="12.7" stroke="{INK}" stroke-width="1.3" stroke-dasharray="2.5 2"/>'
    f'<circle cx="23" cy="24" r="1.8" fill="{INK}"/>')

I["chanfro"] = (
    f'<path d="M7 42 V21 L20 8 H42 V42 Z" fill="{MID}" {S}/>'
    f'<line x1="7" y1="21" x2="20" y2="8" stroke="{ACTION}" stroke-width="3.6" stroke-linecap="round"/>'
    f'<path d="M7 8 H20 M7 8 V21" stroke="{INK}" stroke-width="1.2" stroke-dasharray="2.5 2" fill="none"/>')

I["plano"] = (
    f'<polygon points="4,36 24,27 44,36 24,45" fill="{REF_L}" {S}/>'
    f'<polygon points="4,17 24,8 44,17 24,26" fill="{ACTION_L}" {S}/>'
    + arrow(24, 35, 24, 19, ACTION, 2.6))

I["parametro"] = (
    f'<rect x="5" y="7" width="38" height="34" rx="7" fill="{LIGHT}" {S}/>'
    f'<line x1="12" y1="18" x2="36" y2="18" stroke="{DARK}" stroke-width="3" stroke-linecap="round"/>'
    f'<line x1="12" y1="30" x2="36" y2="30" stroke="{DARK}" stroke-width="3" stroke-linecap="round"/>'
    f'<circle cx="28" cy="18" r="4.5" fill="{ACTION}" {S}/>'
    f'<circle cx="18" cy="30" r="4.5" fill="{SKETCH}" {S}/>')

I["pasta"] = (
    f'<path d="M4 14 A3 3 0 0 1 7 11 H18 L22 15 H41 A3 3 0 0 1 44 18 V37 A3 3 0 0 1 41 40 H7 A3 3 0 0 1 4 37 Z" fill="{ACTION}" {S}/>'
    f'<path d="M4 20 H44" stroke="{INK}" stroke-width="1.3"/>'
    f'<rect x="10" y="24" width="12" height="3" rx="1.5" fill="{ACTION_L}"/>')

I["coletor"] = (
    f'<path d="M4 30 V38 A3 3 0 0 0 7 41 H41 A3 3 0 0 0 44 38 V30" fill="none" {S}/>'
    + box((14, 12), 8, w=9, hh=4.5)
    + box((33, 10), 10, (REF_L, REF, "#059669"), w=9, hh=4.5)
    + box((24, 22), 6, (ACTION_L, ACTION, "#d97706"), w=8, hh=4))

I["aparencia"] = (
    f'<path d="M24 5 C12 5 5 13 5 23 C5 33 13 41 22 41 C26 41 26 37 24 35 C22 33 23 29 27 29 H34 C40 29 43 25 43 20 C43 11 35 5 24 5 Z" fill="{LIGHT}" {S}/>'
    f'<circle cx="14" cy="20" r="3.5" fill="{CUT}" {S}/>'
    f'<circle cx="21" cy="12" r="3.5" fill="{ACTION}" {S}/>'
    f'<circle cx="31" cy="13" r="3.5" fill="{REF}" {S}/>'
    f'<circle cx="14" cy="31" r="3.5" fill="{SKETCH}" {S}/>')

I["ajustar"] = (
    box((24, 13), 10, w=12, hh=6)
    + "".join(f'<path d="{d}" fill="none" stroke="{ACTION}" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>'
              for d in ["M4 13 V4 H13", "M35 4 H44 V13", "M44 35 V44 H35", "M13 44 H4 V35"]))

def cube(hi):
    cols = {"top": LIGHT, "left": MID, "right": DARK}
    cols[hi] = ACTION
    return box((24, 6), 18, (cols["top"], cols["left"], cols["right"]))

I["vista_iso"] = box((24, 6), 18)
I["vista_frente"] = cube("left")
I["vista_topo"] = cube("top")
I["vista_direita"] = cube("right")

I["estilo"] = (
    box((24, 6), 18)
    + f'<polygon points="24,24 42,15 42,33 24,42" fill="#ffffff" {S}/>'
    + f'<path d="M24 24 L42 33 M42 15 L24 42" stroke="{SKETCH}" stroke-width="1.2" stroke-dasharray="2 1.5"/>')

EYE = (f'<path d="M3 24 C10 13 17 9 24 9 C31 9 38 13 45 24 C38 35 31 39 24 39 C17 39 10 35 3 24 Z" fill="{LIGHT}" {S}/>'
       f'<circle cx="24" cy="24" r="8" fill="{SKETCH}" {S}/><circle cx="24" cy="24" r="3.5" fill="{INK}"/>')
I["mostrar"] = EYE
I["ocultar"] = EYE + f'<line x1="7" y1="41" x2="41" y2="7" stroke="{CUT}" stroke-width="4" stroke-linecap="round"/>'
I["ocultos"] = (
    f'<rect x="4" y="6" width="40" height="36" rx="6" fill="{INK}" opacity="0.85"/>'
    f'<path d="M9 24 C14 17 19 14 24 14 C29 14 34 17 39 24 C34 31 29 34 24 34 C19 34 14 31 9 24 Z" fill="none" stroke="{ACTION}" stroke-width="2.2"/>'
    f'<circle cx="24" cy="24" r="4.5" fill="{ACTION}"/>'
    f'<line x1="10" y1="37" x2="38" y2="11" stroke="{ACTION_L}" stroke-width="2.2" stroke-dasharray="3 2.5"/>')

I["filtro"] = (
    f'<path d="M5 7 H43 L28 25 V40 L20 44 V25 Z" fill="{LIGHT}" {S}/>'
    f'<path d="M5 7 H43 L38 13 H10 Z" fill="{ACTION}" {S}/>')

I["config"] = (
    "".join(f'<rect x="21" y="2.5" width="6" height="9" rx="1.5" fill="{MID}" {S} transform="rotate({a} 24 24)"/>' for a in range(0, 360, 45))
    + f'<circle cx="24" cy="24" r="14" fill="{MID}" {S}/>'
    + f'<circle cx="24" cy="24" r="5.5" fill="{ACTION}" {S}/>')

I["ortho"] = (
    f'<rect x="6" y="10" width="20" height="20" fill="{LIGHT}" {S}/>'
    f'<path d="M24 20 L42 14 V34 L24 40 Z" fill="{MID}" {S}/>'
    f'<path d="M30 18 L36 16 M30 36 L36 34" stroke="{ACTION}" stroke-width="2" stroke-linecap="round"/>')

for name, body in I.items():
    with open(os.path.join(OUT, f"{name}.svg"), "w", encoding="utf8", newline="\n") as f:
        f.write(svg(body))
print(len(I), "ícones gerados em", OUT)
