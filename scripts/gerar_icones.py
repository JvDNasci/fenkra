"""Gera os ícones SVG do SolidCadCraft (estilo ilustrado, 48x48). Uso: icons.py <pasta_destino>"""
import os, sys

OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)

# Estilo (referência do usuário): degradê ciano → azul, contorno azul forte, setas azul-escuro.
INK = "#1d6fd1"        # contorno
LIGHT = "url(#gl)"     # face clara (degradê)
MID = "url(#gm)"       # face média
DARK = "url(#gd)"      # face escura
SKETCH = "#1d6fd1"     # geometria de sketch
ACTION = "#0d4aa6"     # setas / destaque
ACTION_L = "#e3f6ff"
CUT = "#0d4aa6"
REF = "url(#gl)"
REF_L = "#e3f6ff"
DASH = f'stroke="{INK}" stroke-width="2" stroke-dasharray="4 3" fill="none" stroke-linejoin="round"'

S = f'stroke="{INK}" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"'

DEFS = ('<defs>'
        '<linearGradient id="gl" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#bff1ff"/><stop offset="1" stop-color="#5cc8fb"/></linearGradient>'
        '<linearGradient id="gm" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#86dcff"/><stop offset="1" stop-color="#2fa5f2"/></linearGradient>'
        '<linearGradient id="gd" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#4fb9f8"/><stop offset="1" stop-color="#1f86de"/></linearGradient>'
        '</defs>')


def svg(body):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" width="48" height="48">{DEFS}{body}</svg>\n'


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

# ---------------------------------------------------------------- opções e variações dos comandos
def sq(x, y, w, h, fill=MID, dash=False):
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="1.5" ' + (DASH + '/>' if dash else f'fill="{fill}" {S}/>')


# término da extrusão (vista lateral: perfil embaixo, material para cima)
BASE = f'<line x1="6" y1="40" x2="42" y2="40" stroke="{INK}" stroke-width="2.6"/>'
I["ext_cego"] = BASE + sq(14, 18, 20, 22) + arrow(24, 16, 24, 5)
I["ext_simetrico"] = f'<line x1="4" y1="26" x2="44" y2="26" {DASH}/>' + sq(14, 12, 20, 28) + arrow(38, 26, 38, 8) + arrow(38, 26, 38, 44)
I["ext_dois_sentidos"] = f'<line x1="4" y1="30" x2="44" y2="30" {DASH}/>' + sq(14, 10, 20, 20) + sq(14, 30, 20, 10, LIGHT)
I["ext_passante"] = sq(6, 10, 36, 30, LIGHT) + sq(18, 6, 12, 38, MID) + arrow(24, 4, 24, 46, ACTION, 2.4)
I["ext_ate_plano"] = BASE + f'<path d="M14 40 V16 L34 8 V40 Z" fill="{MID}" {S}/>' + \
    f'<line x1="6" y1="19" x2="42" y2="5" stroke="{ACTION}" stroke-width="2.6" stroke-dasharray="5 3"/>'
I["ext_ate_face"] = BASE + sq(14, 20, 20, 20) + f'<path d="M4 8 H44 V16 H4 Z" fill="{LIGHT}" {S}/>' + arrow(24, 30, 24, 17)
# operação
I["op_adicionar"] = sq(6, 16, 22, 22, LIGHT) + sq(18, 8, 22, 22) + f'<path d="M36 34 V46 M30 40 H42" stroke="{ACTION}" stroke-width="3" stroke-linecap="round"/>'
I["op_remover"] = sq(6, 10, 30, 30) + sq(22, 4, 20, 20, dash=True) + f'<path d="M34 40 H46" stroke="{ACTION}" stroke-width="3" stroke-linecap="round"/>'
I["op_intersecao"] = sq(6, 6, 24, 24, dash=True) + sq(18, 18, 24, 24, dash=True) + sq(18, 18, 12, 12)
# furos (corte lateral)
PLATE = f'<path d="M4 12 H18 V42 H4 Z M30 12 H44 V42 H30 Z" fill="{MID}" {S}/>'
I["furo_simples"] = PLATE
I["furo_rebaixado"] = f'<path d="M4 12 H12 V22 H18 V42 H4 Z M36 12 H44 V42 H30 V22 H36 Z" fill="{MID}" {S}/>'
I["furo_escareado"] = f'<path d="M4 12 H10 L18 22 V42 H4 Z M38 12 H44 V42 H30 V22 Z" fill="{MID}" {S}/>'
I["furo_roscado"] = PLATE + "".join(f'<path d="M18 {y} l4 3 M30 {y} l-4 3" stroke="{ACTION}" stroke-width="1.6"/>' for y in range(15, 40, 5))
I["furo_passante"] = PLATE + arrow(24, 4, 24, 46, ACTION, 2.4)
I["furo_cego"] = f'<path d="M4 12 H18 V32 L24 37 L30 32 V12 H44 V42 H4 Z" fill="{MID}" {S}/>'
# arestas para filete/chanfro
CUBE = box((24, 6), 18)
I["arestas_selecionar"] = CUBE + f'<line x1="24" y1="24" x2="42" y2="15" stroke="{ACTION}" stroke-width="4" stroke-linecap="round"/>' + \
    f'<path d="M34 30 L34 45 L38 41 L42 47 L44 45 L40 39 L45 38 Z" fill="#ffffff" {S}/>'
I["arestas_todas"] = CUBE.replace(f'stroke="{INK}" stroke-width="2"', f'stroke="{ACTION}" stroke-width="3.2"')
I["arestas_comando"] = CUBE + f'<ellipse cx="24" cy="15" rx="7" ry="3.5" fill="none" stroke="{ACTION}" stroke-width="3"/>'
I["chanfro_dist"] = f'<path d="M7 42 V21 L20 8 H42 V42 Z" fill="{MID}" {S}/><path d="M3 21 H7 M20 3 V8" stroke="{ACTION}" stroke-width="2.2"/>'
I["chanfro_dist_dist"] = f'<path d="M7 42 V26 L28 8 H42 V42 Z" fill="{MID}" {S}/><path d="M3 26 V42 M28 3 H42" stroke="{ACTION}" stroke-width="2.2"/>'
I["chanfro_dist_angulo"] = f'<path d="M7 42 V21 L20 8 H42 V42 Z" fill="{MID}" {S}/><path d="M20 8 A10 10 0 0 0 12 16" stroke="{ACTION}" stroke-width="2.2" fill="none"/>'
# planos
I["plano_deslocado"] = I["plano"]
I["plano_angulo"] = f'<polygon points="4,38 24,30 44,38 24,46" fill="{LIGHT}" {S}/>' + \
    f'<polygon points="8,16 30,4 40,28 18,40" fill="{MID}" {S}/>' + f'<path d="M30 42 A12 12 0 0 0 36 30" stroke="{ACTION}" stroke-width="2.4" fill="none"/>'
I["plano_3pontos"] = f'<polygon points="4,30 24,20 44,30 24,40" fill="{LIGHT}" {S}/>' + "".join(
    f'<circle cx="{x}" cy="{y}" r="3.4" fill="{ACTION}"/>' for x, y in [(12, 30), (34, 26), (24, 36)])
# transformações do sketch
GEO = f'<path d="M10 34 L18 18 L28 30 Z" {G}/>'
I["sk_mover"] = GEO + arrow(26, 22, 42, 10)
I["sk_copiar"] = f'<path d="M6 40 L14 24 L24 36 Z" {DASH}/>' + f'<path d="M22 26 L30 10 L40 22 Z" {G}/>'
I["sk_girar"] = GEO + f'<path d="M36 34 A14 14 0 1 0 26 40" stroke="{ACTION}" stroke-width="2.6" fill="none"/>' + \
    f'<polygon points="22,40 30,36 29,44" fill="{ACTION}"/>'
I["sk_espelhar"] = f'<line x1="24" y1="4" x2="24" y2="44" stroke="{INK}" stroke-width="1.6" stroke-dasharray="4 2.5"/>' + \
    f'<path d="M20 36 L8 30 L16 14 Z" {G}/><path d="M28 36 L40 30 L32 14 Z" {DASH}/>'
I["sk_offset"] = f'<rect x="12" y="12" width="24" height="24" rx="2" {G}/>' + f'<rect x="5" y="5" width="38" height="38" rx="6" {DASH}/>'
I["sk_filete"] = f'<path d="M8 8 V26 A14 14 0 0 0 22 40 H40" {G}/>' + f'<path d="M8 26 V40 H22" {DASH}/>'
I["sk_aparar"] = f'<line x1="6" y1="24" x2="30" y2="24" {G}/><line x1="24" y1="6" x2="24" y2="42" {G}/>' + \
    f'<line x1="30" y1="24" x2="42" y2="24" {DASH}/>' + \
    f'<path d="M34 12 L40 18 M40 12 L34 18" stroke="{ACTION}" stroke-width="2.4" stroke-linecap="round"/>'
I["sk_converter"] = f'<line x1="6" y1="40" x2="22" y2="24" {G}/><line x1="26" y1="20" x2="42" y2="6" {DASH}/>' + arrow(14, 16, 32, 34, ACTION, 2.2)
I["sk_checar"] = f'<path d="M8 34 L8 12 L30 12" {G}/><path d="M30 12 L40 22" {DASH}/>' + \
    f'<circle cx="38" cy="36" r="8" fill="{LIGHT}" {S}/><path d="M34 36 L37 39 L42 33" stroke="{ACTION}" stroke-width="2.4" fill="none"/>' + \
    f'<circle cx="8" cy="34" r="4" fill="none" stroke="{ACTION}" stroke-width="2.4"/>'
# variações de cota
AR = lambda pts: f'<polygon points="{pts}" fill="{ACTION}"/>'
I["cota_horizontal"] = f'<path d="M8 40 V18 M40 40 V18" stroke="{INK}" stroke-width="1.6"/>' + \
    f'<line x1="10" y1="22" x2="38" y2="22" stroke="{ACTION}" stroke-width="2.2"/>' + AR("9,22 15,19 15,25") + AR("39,22 33,19 33,25") + \
    f'<line x1="8" y1="40" x2="40" y2="40" {G}/>'
I["cota_vertical"] = f'<path d="M40 8 H18 M40 40 H18" stroke="{INK}" stroke-width="1.6"/>' + \
    f'<line x1="22" y1="10" x2="22" y2="38" stroke="{ACTION}" stroke-width="2.2"/>' + AR("22,9 19,15 25,15") + AR("22,39 19,33 25,33") + \
    f'<line x1="40" y1="8" x2="40" y2="40" {G}/>'
I["cota_angulo"] = f'<path d="M6 40 H42 M6 40 L36 12" {G}/><path d="M26 40 A20 20 0 0 0 21 26" stroke="{ACTION}" stroke-width="2.2" fill="none"/>'
I["cota_raio"] = f'<path d="M8 40 A32 32 0 0 1 40 8" {G}/>' + arrow(8, 40, 28, 20, ACTION, 2.2) + f'<circle cx="8" cy="40" r="2.5" fill="{INK}"/>'
I["cota_diametro"] = f'<circle cx="24" cy="24" r="16" {G}/>' + arrow(24, 24, 35, 13, ACTION, 2.2) + arrow(24, 24, 13, 35, ACTION, 2.2)
# seleção e filtro de seleção
I["sel_face"] = box((24, 6), 18, (ACTION, MID, DARK))
I["sel_aresta"] = CUBE + f'<line x1="24" y1="24" x2="24" y2="42" stroke="{ACTION}" stroke-width="4.5" stroke-linecap="round"/>'
I["sel_vertice"] = CUBE + f'<circle cx="24" cy="24" r="5" fill="{ACTION}" stroke="#ffffff" stroke-width="1.5"/>'
I["sel_corpo"] = box((24, 6), 18, (ACTION, ACTION, ACTION))
I["sel_filtro"] = I["filtro"] + f'<circle cx="38" cy="36" r="7" fill="#ffffff" {S}/><path d="M35 36 H41 M38 33 V39" stroke="{ACTION}" stroke-width="2"/>'
I["navegacao"] = f'<circle cx="24" cy="24" r="18" fill="{LIGHT}" {S}/>' + \
    f'<path d="M24 10 L28 24 L24 38 L20 24 Z" fill="{ACTION}"/><circle cx="24" cy="24" r="3" fill="#ffffff"/>'
# tela inicial
I["novo_projeto"] = f'<path d="M6 12 A3 3 0 0 1 9 9 H18 L22 13 H39 A3 3 0 0 1 42 16 V37 A3 3 0 0 1 39 40 H9 A3 3 0 0 1 6 37 Z" fill="{MID}" {S}/>' + \
    f'<path d="M24 20 V34 M17 27 H31" stroke="#ffffff" stroke-width="3.4" stroke-linecap="round"/>'
I["abrir_projeto"] = f'<path d="M6 12 A3 3 0 0 1 9 9 H18 L22 13 H39 A3 3 0 0 1 42 16 V20 H14 L6 38 Z" fill="{DARK}" {S}/>' + \
    f'<path d="M14 20 H46 L38 40 H6 Z" fill="{LIGHT}" {S}/>'
I["nova_peca"] = box((24, 8), 16) + f'<circle cx="38" cy="36" r="8" fill="{ACTION}"/><path d="M38 32 V40 M34 36 H42" stroke="#ffffff" stroke-width="2.6"/>'
I["exemplo"] = box((24, 8), 16) + f'<path d="M36 28 L38.5 33 L44 33.5 L40 37 L41 42.5 L36 39.8 L31 42.5 L32 37 L28 33.5 L33.5 33 Z" fill="#ffd34d" stroke="{INK}" stroke-width="1.2"/>'

# ---------------- geometria de referência (planos, pontos, linhas e curvas)
PL = f'<polygon points="4,32 24,22 44,32 24,42" fill="{LIGHT}" {S}/>'
DOT = lambda x, y, r=3.4, c=ACTION: f'<circle cx="{x}" cy="{y}" r="{r}" fill="{c}" stroke="#ffffff" stroke-width="1"/>'
LN = lambda x1, y1, x2, y2, c=ACTION, w=2.6: f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{c}" stroke-width="{w}" stroke-linecap="round"/>'
CV = lambda d, c=SKETCH, w=2.8: f'<path d="{d}" stroke="{c}" stroke-width="{w}" fill="none" stroke-linecap="round" stroke-linejoin="round"/>'
I["plano_paralelo_ponto"] = f'<polygon points="4,36 24,27 44,36 24,45" fill="{REF_L}" {S}/>' + \
    f'<polygon points="4,17 24,8 44,17 24,26" fill="{ACTION_L}" {S}/>' + DOT(24, 17, 4)
I["plano_2linhas"] = PL + LN(10, 33, 30, 24) + LN(18, 37, 38, 28)
I["plano_ponto_linha"] = PL + LN(10, 33, 32, 23) + DOT(28, 35)
I["plano_curva"] = PL + CV("M9 33 C 16 24, 24 40, 38 30", ACTION)
I["plano_normal_curva"] = CV("M4 38 C 16 34, 28 18, 44 12") + \
    f'<polygon points="18,10 32,4 32,34 18,40" fill="{ACTION_L}" opacity="0.9" {S}/>' + DOT(25, 24, 3)
I["plano_tangente"] = f'<path d="M8 44 A16 16 0 0 1 40 44 Z" fill="{MID}" {S}/>' + \
    f'<polygon points="6,26 24,18 42,26 24,34" fill="{ACTION_L}" opacity="0.92" {S}/>' + DOT(24, 28, 3)
I["plano_equacao"] = PL + f'<text x="24" y="17" font-family="Segoe UI, Arial" font-size="13" font-weight="bold" text-anchor="middle" fill="{ACTION}">Ax=D</text>'
I["plano_medio"] = PL + DOT(12, 26, 3) + DOT(34, 24, 3) + DOT(20, 40, 3) + DOT(38, 36, 3)
I["ponto_coord"] = LN(10, 38, 40, 38, "#d94040", 2) + LN(10, 38, 10, 6, "#2f9e44", 2) + LN(10, 38, 30, 24, INK, 2) + \
    f'<path d="M10 20 H30 V38" stroke="{INK}" stroke-width="1.2" stroke-dasharray="3 2" fill="none"/>' + DOT(30, 20, 4.5)
I["ponto_curva"] = CV("M4 36 C 14 10, 30 44, 44 14") + DOT(24, 27, 4.5)
I["ponto_plano"] = PL + DOT(24, 32, 4.5)
I["ponto_superficie"] = f'<path d="M4 34 C 14 22, 30 22, 44 30 L44 40 C 30 34, 14 34, 4 44 Z" fill="{LIGHT}" {S}/>' + DOT(24, 30, 4.5)
I["ponto_centro"] = f'<circle cx="24" cy="24" r="16" {G}/>' + LN(18, 24, 30, 24, INK, 1.4) + LN(24, 18, 24, 30, INK, 1.4) + DOT(24, 24, 4)
I["ponto_tangente"] = CV("M6 40 A22 22 0 0 1 42 40") + LN(6, 18, 42, 18, INK, 1.8) + DOT(24, 18.5, 4)
I["ponto_entre"] = f'<line x1="6" y1="38" x2="42" y2="10" {DASH}/>' + DOT(6, 38, 3.4, INK) + DOT(42, 10, 3.4, INK) + DOT(24, 24, 5)
I["linha_pontos"] = LN(8, 38, 40, 10) + DOT(8, 38, 4, INK) + DOT(40, 10, 4, INK)
I["linha_direcao"] = DOT(10, 36, 4, INK) + arrow(10, 36, 42, 10, ACTION, 2.6)
I["linha_angulo"] = CV("M4 40 C 16 36, 30 30, 44 20") + LN(22, 34, 34, 6) + \
    f'<path d="M32 30 A10 10 0 0 0 26 25" stroke="{INK}" stroke-width="1.8" fill="none"/>'
I["linha_tangente"] = f'<circle cx="22" cy="28" r="13" {G}/>' + LN(4, 15, 44, 15) + DOT(22, 15, 3)
I["linha_normal"] = f'<path d="M4 36 C 14 26, 34 26, 44 36 L44 42 C 34 34, 14 34, 4 42 Z" fill="{LIGHT}" {S}/>' + arrow(24, 30, 24, 4, ACTION, 2.6)
I["polilinha3d"] = f'<polyline points="6,40 16,14 28,32 42,8" stroke="{ACTION}" stroke-width="2.6" fill="none" stroke-linejoin="round"/>' + \
    "".join(DOT(x, y, 3, INK) for x, y in [(6, 40), (16, 14), (28, 32), (42, 8)])
I["circulo3d"] = f'<ellipse cx="24" cy="26" rx="19" ry="10" stroke="{ACTION}" stroke-width="2.8" fill="{REF_L}"/>' + DOT(24, 26, 3, INK)
I["circulo3p"] = f'<circle cx="24" cy="24" r="16" stroke="{ACTION}" stroke-width="2.6" fill="none"/>' + \
    "".join(DOT(x, y, 3.4, INK) for x, y in [(8, 24), (35.3, 12.7), (32, 37.9)])
I["spline"] = CV("M4 34 C 10 6, 22 6, 24 24 S 38 42, 44 14", ACTION) + "".join(DOT(x, y, 2.8, INK) for x, y in [(4, 34), (24, 24), (44, 14)])
I["helice"] = CV("M10 42 C 10 36, 38 38, 38 33 C 38 28, 10 30, 10 25 C 10 20, 38 22, 38 17 C 38 12, 10 14, 10 9 C 10 6, 30 5, 36 6", ACTION, 2.6) + \
    f'<line x1="24" y1="2" x2="24" y2="46" stroke="{INK}" stroke-width="1.2" stroke-dasharray="3 2"/>'
I["espiral"] = CV("M24 24 C 24 20, 30 20, 30 25 C 30 31, 20 32, 18 25 C 15 16, 31 12, 35 22 C 40 34, 22 42, 13 34 C 4 26, 10 8, 26 7", ACTION, 2.4)
I["conica"] = f'<ellipse cx="18" cy="24" rx="13" ry="8" stroke="{ACTION}" stroke-width="2.4" fill="none"/>' + CV("M30 6 Q 52 24, 30 42", SKETCH, 2.4)
I["conexao"] = LN(4, 36, 16, 36, SKETCH, 3) + LN(32, 12, 44, 12, SKETCH, 3) + CV("M16 36 C 26 36, 22 12, 32 12", ACTION, 2.6) + \
    DOT(16, 36, 3, INK) + DOT(32, 12, 3, INK)
I["canto"] = f'<path d="M8 6 V24" {G}/><path d="M24 40 H42" {G}/>' + CV("M8 24 A16 16 0 0 0 24 40", ACTION, 3) + \
    f'<path d="M8 24 V40 H24" {DASH}/>'
I["spine"] = "".join(f'<polygon points="{x-5},{y-9} {x+5},{y-13} {x+5},{y+7} {x-5},{y+11}" fill="{ACTION_L}" {S}/>' for x, y in [(8, 34), (24, 22), (40, 14)]) + \
    CV("M4 36 C 14 34, 18 24, 24 22 S 36 14, 46 13", ACTION, 2.6)

for name, body in I.items():
    with open(os.path.join(OUT, f"{name}.svg"), "w", encoding="utf8", newline="\n") as f:
        f.write(svg(body))
print(len(I), "ícones gerados em", OUT)
