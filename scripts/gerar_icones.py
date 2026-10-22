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

# ---------------------------------------------------------------- ferramentas do sketch (2D)
G = f'stroke="{SKETCH}" stroke-width="3" fill="none" stroke-linecap="round" stroke-linejoin="round"'
PT = lambda x, y, c=INK: f'<rect x="{x - 3}" y="{y - 3}" width="6" height="6" rx="1" fill="{c}"/>'
GRID = "".join(f'<path d="M{v} 6 V42 M6 {v} H42" stroke="#94a3b8" stroke-width="0.8" opacity="0.6"/>' for v in (12, 24, 36))

I["sk_selecionar"] = f'<path d="M12 6 L12 38 L20 30 L26 43 L31 41 L25 28 L36 28 Z" fill="{LIGHT}" {S}/>'
I["sk_linha"] = f'<line x1="8" y1="38" x2="40" y2="10" {G}/>' + PT(8, 38) + PT(40, 10)
I["sk_polilinha"] = f'<polyline points="6,38 18,14 30,32 42,10" {G}/>' + PT(6, 38) + PT(18, 14) + PT(30, 32) + PT(42, 10)
I["sk_retangulo"] = f'<rect x="8" y="12" width="32" height="24" {G}/>' + PT(8, 36) + PT(40, 12)
I["sk_retangulo_centro"] = f'<rect x="8" y="12" width="32" height="24" {G}/>' + PT(24, 24, ACTION) + PT(40, 12) + \
    f'<line x1="24" y1="24" x2="40" y2="12" stroke="{INK}" stroke-width="1.2" stroke-dasharray="2.5 2"/>'
I["sk_circulo"] = f'<circle cx="24" cy="24" r="16" {G}/>' + PT(24, 24, ACTION) + \
    f'<line x1="24" y1="24" x2="40" y2="24" stroke="{INK}" stroke-width="1.2" stroke-dasharray="2.5 2"/>'
I["sk_circulo3"] = f'<circle cx="24" cy="24" r="16" {G}/>' + PT(8, 24) + PT(24, 8) + PT(36, 35)
I["sk_arco3"] = f'<path d="M8 36 A18 18 0 0 1 40 36" {G}/>' + PT(8, 36) + PT(24, 18) + PT(40, 36)
I["sk_arco_centro"] = f'<path d="M40 30 A16 16 0 0 0 12 14" {G}/>' + PT(24, 30, ACTION) + PT(40, 30) + PT(12, 14) + \
    f'<path d="M24 30 L40 30 M24 30 L12 14" stroke="{INK}" stroke-width="1.2" stroke-dasharray="2.5 2"/>'
I["sk_rasgo"] = f'<path d="M14 16 H34 A8 8 0 0 1 34 32 H14 A8 8 0 0 1 14 16 Z" {G}/>' + PT(14, 24, ACTION) + PT(34, 24, ACTION)
I["sk_poligono"] = f'<polygon points="24,6 40,15 40,33 24,42 8,33 8,15" {G}/>' + \
    f'<circle cx="24" cy="24" r="18" stroke="{ACTION}" stroke-width="1.2" stroke-dasharray="3 2" fill="none"/>'
I["sk_ponto"] = f'<path d="M24 8 V40 M8 24 H40" stroke="{SKETCH}" stroke-width="1.5" opacity="0.6"/>' + PT(24, 24, ACTION) + \
    f'<rect x="19" y="19" width="10" height="10" rx="2" fill="none" stroke="{INK}" stroke-width="1.5"/>'
I["sk_cota"] = (f'<path d="M8 34 V16 M40 34 V16" stroke="{INK}" stroke-width="1.5"/>'
                f'<line x1="10" y1="22" x2="38" y2="22" stroke="{ACTION}" stroke-width="2.4"/>'
                f'<polygon points="9,22 15,19 15,25" fill="{ACTION}"/><polygon points="39,22 33,19 33,25" fill="{ACTION}"/>'
                f'<rect x="14" y="27" width="20" height="10" rx="3" fill="{LIGHT}" {S}/>')
I["sk_construcao"] = f'<line x1="6" y1="40" x2="42" y2="8" stroke="{ACTION}" stroke-width="3" stroke-dasharray="5 3.5" stroke-linecap="round"/>' + PT(6, 40) + PT(42, 8)
I["sk_grade"] = GRID + f'<rect x="6" y="6" width="36" height="36" rx="3" fill="none" stroke="{INK}" stroke-width="1.6"/>'
I["sk_snap"] = GRID + f'<circle cx="24" cy="24" r="9" fill="none" stroke="{ACTION}" stroke-width="2.4"/>' + PT(24, 24, CUT)
I["sk_auto_restricao"] = (f'<line x1="6" y1="32" x2="42" y2="32" {G}/>'
                          f'<rect x="16" y="8" width="16" height="16" rx="4" fill="{ACTION}" {S}/>'
                          f'<path d="M20 12 V20 M28 12 V20 M20 16 H28" stroke="{INK}" stroke-width="2"/>')
I["sk_auto_cota"] = I["sk_cota"] + f'<circle cx="38" cy="10" r="6" fill="{REF}" {S}/>'
I["sk_concluir"] = f'<circle cx="24" cy="24" r="18" fill="{REF}" {S}/><path d="M15 24 L22 31 L34 17" stroke="#ffffff" stroke-width="4" fill="none" stroke-linecap="round" stroke-linejoin="round"/>'


def badge(sym):
    return f'<rect x="4" y="4" width="18" height="18" rx="4" fill="{ACTION}" {S}/>' + sym


I["r_coincidente"] = f'<line x1="8" y1="40" x2="26" y2="22" {G}/><line x1="26" y1="22" x2="42" y2="38" {G}/>' + \
    f'<circle cx="26" cy="22" r="5" fill="{ACTION}" {S}/>'
I["r_horizontal"] = f'<line x1="6" y1="32" x2="42" y2="32" {G}/>' + badge(f'<path d="M9 9 V17 M17 9 V17 M9 13 H17" stroke="{INK}" stroke-width="1.8"/>')
I["r_vertical"] = f'<line x1="32" y1="6" x2="32" y2="42" {G}/>' + badge(f'<path d="M8 9 L13 17 L18 9" stroke="{INK}" stroke-width="1.8" fill="none"/>')
I["r_paralela"] = f'<line x1="10" y1="40" x2="30" y2="10" {G}/><line x1="22" y1="42" x2="42" y2="12" {G}/>'
I["r_perpendicular"] = f'<path d="M8 40 H40 M14 40 V8" {G}/><path d="M14 32 H22 V40" stroke="{ACTION}" stroke-width="2" fill="none"/>'
I["r_tangente"] = f'<circle cx="22" cy="26" r="13" {G}/><line x1="4" y1="13" x2="44" y2="13" stroke="{ACTION}" stroke-width="3" stroke-linecap="round"/>' + PT(22, 13)
I["r_igual"] = f'<line x1="6" y1="14" x2="30" y2="14" {G}/><line x1="18" y1="36" x2="42" y2="36" {G}/>' + \
    f'<path d="M18 22 H30 M18 27 H30" stroke="{ACTION}" stroke-width="2.6"/>'
I["r_concentrica"] = f'<circle cx="24" cy="24" r="17" {G}/><circle cx="24" cy="24" r="9" {G}/>' + PT(24, 24, ACTION)
I["r_ponto_medio"] = f'<line x1="6" y1="36" x2="42" y2="12" {G}/>' + f'<circle cx="24" cy="24" r="5" fill="{ACTION}" {S}/>' + PT(6, 36) + PT(42, 12)
I["r_fixa"] = (f'<path d="M24 6 V30 M14 22 A10 10 0 0 0 34 22" stroke="{INK}" stroke-width="2.6" fill="none" stroke-linecap="round"/>'
               f'<path d="M14 22 L10 26 M34 22 L38 26" stroke="{INK}" stroke-width="2.6" stroke-linecap="round"/>'
               f'<circle cx="24" cy="8" r="3" fill="{ACTION}" {S}/><path d="M6 40 H42" stroke="{SKETCH}" stroke-width="3"/>')
I["r_simetrica"] = f'<line x1="24" y1="4" x2="24" y2="44" stroke="{INK}" stroke-width="1.5" stroke-dasharray="4 2.5"/>' + PT(10, 24, SKETCH) + PT(38, 24, SKETCH) + \
    f'<path d="M14 24 H20 M28 24 H34" stroke="{ACTION}" stroke-width="2"/>'
I["idioma"] = (f'<circle cx="24" cy="24" r="18" fill="{LIGHT}" {S}/>'
               f'<ellipse cx="24" cy="24" rx="8" ry="18" fill="none" stroke="{INK}" stroke-width="1.4"/>'
               f'<path d="M6 24 H42 M9 15 H39 M9 33 H39" stroke="{INK}" stroke-width="1.2" fill="none"/>')
I["personalizar"] = (f'<rect x="5" y="8" width="38" height="12" rx="3" fill="{LIGHT}" {S}/>'
                     f'<rect x="5" y="28" width="24" height="12" rx="3" fill="{MID}" {S}/>'
                     f'<path d="M36 26 V42 M28 34 H44" stroke="{ACTION}" stroke-width="3.2" stroke-linecap="round"/>')

for name, body in I.items():
    with open(os.path.join(OUT, f"{name}.svg"), "w", encoding="utf8", newline="\n") as f:
        f.write(svg(body))
print(len(I), "ícones gerados em", OUT)
