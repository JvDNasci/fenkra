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

# ---------------- features de forma, acabamento e padrões
I["ranhura"] = (
    f'<path d="M10 14 V34 A14 5 0 0 0 38 34 V14" fill="{MID}" {S}/>'
    f'<ellipse cx="24" cy="14" rx="14" ry="5" fill="{LIGHT}" {S}/>'
    f'<path d="M10 22 A14 5 0 0 0 38 22 L38 27 A14 5 0 0 1 10 27 Z" fill="{INK}" opacity="0.85"/>'
    f'<line x1="24" y1="3" x2="24" y2="46" stroke="{INK}" stroke-width="1.4" stroke-dasharray="4 2.5"/>')
I["nervura"] = (
    f'<path d="M6 40 C 14 40, 20 30, 26 22 S 38 8, 44 8" stroke="{SKETCH}" stroke-width="1.8" stroke-dasharray="3 2" fill="none"/>'
    f'<path d="M4 36 L10 36 C 16 34, 22 26, 28 18 S 38 6, 46 4 L46 10 C 40 12, 34 20, 30 26 S 18 40, 10 42 L4 42 Z" fill="{MID}" {S}/>')
I["rasgo_varrido"] = box((24, 8), 16) + \
    f'<path d="M10 22 C 18 26, 26 18, 38 22" stroke="{INK}" stroke-width="5" fill="none" stroke-linecap="round" opacity="0.85"/>'
I["multisecoes"] = (
    f'<polygon points="6,38 24,30 42,38 24,46" fill="{LIGHT}" {S}/>'
    f'<path d="M6 38 L14 14 M42 38 L34 14 M24 46 L24 22" stroke="{INK}" stroke-width="1.4"/>'
    f'<ellipse cx="24" cy="14" rx="10" ry="5" fill="{MID}" {S}/>')
I["multisecoes_remover"] = box((24, 8), 16) + \
    f'<polygon points="16,18 24,14 32,18 24,22" fill="{INK}" opacity="0.85"/><ellipse cx="24" cy="30" rx="4" ry="2" fill="{INK}" opacity="0.7"/>'
I["prim_caixa"] = box((24, 6), 18)
I["prim_cilindro"] = (f'<path d="M10 12 V36 A14 6 0 0 0 38 36 V12" fill="{MID}" {S}/>'
                      f'<ellipse cx="24" cy="12" rx="14" ry="6" fill="{LIGHT}" {S}/>')
I["prim_esfera"] = (f'<circle cx="24" cy="24" r="18" fill="{MID}" {S}/>'
                    f'<ellipse cx="24" cy="24" rx="18" ry="6" fill="none" stroke="{INK}" stroke-width="1.4"/>'
                    f'<ellipse cx="18" cy="16" rx="5" ry="3" fill="#ffffff" opacity="0.6"/>')
I["prim_cone"] = (f'<path d="M24 4 L8 38 A16 6 0 0 0 40 38 Z" fill="{MID}" {S}/>'
                  f'<ellipse cx="24" cy="38" rx="16" ry="6" fill="{DARK}" {S}/>')
I["prim_toro"] = (f'<ellipse cx="24" cy="26" rx="20" ry="12" fill="{MID}" {S}/>'
                  f'<ellipse cx="24" cy="24" rx="8" ry="4" fill="#ffffff" {S}/>')
I["prim_cunha"] = (f'<polygon points="6,38 24,46 42,38 24,30" fill="{LIGHT}" {S}/>'
                   f'<polygon points="6,38 24,46 24,30 6,22" fill="{MID}" {S}/>'
                   f'<polygon points="24,46 42,38 24,30" fill="{DARK}" {S}/>')
I["prim_prisma"] = (f'<polygon points="12,14 24,8 36,14 36,34 24,40 12,34" fill="{MID}" {S}/>'
                    f'<polygon points="12,14 24,8 36,14 24,20" fill="{LIGHT}" {S}/>'
                    f'<line x1="24" y1="20" x2="24" y2="40" stroke="{INK}" stroke-width="1.6"/>')
I["reforco"] = (f'<path d="M4 38 H44 M4 38 V10" stroke="{INK}" stroke-width="3" fill="none"/>'
                f'<polygon points="4,38 4,14 28,38" fill="{ACTION}" {S}/>')
I["filete_variavel"] = (f'<path d="M8 8 V22 C 8 34, 18 40, 40 40" stroke="{SKETCH}" stroke-width="3" fill="none" stroke-linecap="round"/>'
                        f'<path d="M8 22 V40 H40" stroke="{INK}" stroke-width="1.6" stroke-dasharray="4 2.5" fill="none"/>'
                        f'<circle cx="8" cy="22" r="3" fill="{ACTION}"/><circle cx="40" cy="40" r="5" fill="{ACTION}"/>')
I["casca"] = box((24, 8), 16) + f'<polygon points="12,16 24,10 36,16 24,22" fill="{INK}" opacity="0.85"/>' + \
    f'<polygon points="12,16 24,22 24,34 12,28" fill="{DARK}" opacity="0.45"/>'
I["saida"] = (f'<polygon points="4,40 44,40 36,10 12,10" fill="{MID}" {S}/>'
              f'<line x1="12" y1="10" x2="12" y2="40" stroke="{INK}" stroke-width="1.4" stroke-dasharray="3 2"/>'
              f'<path d="M12 30 A10 10 0 0 0 9 31" stroke="{ACTION}" stroke-width="2.2" fill="none"/>') + arrow(24, 30, 24, 4, ACTION, 2.4)
I["espessura"] = box((24, 14), 12) + f'<polygon points="6,23 24,14 42,23 24,32" fill="none" stroke="{ACTION}" stroke-width="2" stroke-dasharray="3 2"/>' + \
    arrow(24, 20, 24, 2, ACTION, 2.4)
I["rosca"] = (f'<rect x="14" y="6" width="20" height="36" rx="2" fill="{MID}" {S}/>' +
              "".join(f'<path d="M14 {y} L34 {y + 4}" stroke="{INK}" stroke-width="2"/>' for y in range(10, 38, 5)))
I["padrao_linear"] = "".join(box((x, y), 7, w=7, hh=3.5) for x, y in [(10, 26), (24, 19), (38, 12)])
I["padrao_circular"] = f'<circle cx="24" cy="24" r="15" fill="none" stroke="{INK}" stroke-width="1.4" stroke-dasharray="3 2"/>' + \
    "".join(f'<circle cx="{24 + 15 * c:.1f}" cy="{24 + 15 * s:.1f}" r="5" fill="{MID}" {S}/>' for c, s in [(1, 0), (0, 1), (-1, 0), (0, -1), (0.707, 0.707), (-0.707, -0.707)])
I["padrao_pontos"] = "".join(f'<rect x="{x - 5}" y="{y - 5}" width="10" height="10" rx="2" fill="{MID}" {S}/>' for x, y in [(10, 12), (34, 16), (18, 36), (40, 38)])
I["espelho"] = (f'<line x1="24" y1="4" x2="24" y2="44" stroke="{INK}" stroke-width="1.6" stroke-dasharray="4 2.5"/>'
                f'<polygon points="20,36 6,30 14,12 20,14" fill="{MID}" {S}/><polygon points="28,36 42,30 34,12 28,14" fill="{LIGHT}" {S}/>')
I["escala"] = (f'<rect x="6" y="22" width="18" height="18" rx="2" fill="{MID}" {S}/>'
               f'<rect x="6" y="8" width="34" height="32" rx="3" fill="none" stroke="{INK}" stroke-width="1.6" stroke-dasharray="4 2.5"/>') + arrow(22, 26, 38, 10, ACTION, 2.4)

# ---------------- alpha.5: modos de geometria, superfícies, booleanas, filetes, sketch, macro
SURF = lambda d: f'<path d="{d}" fill="{LIGHT}" {S}/>'
I["bool_unir"] = f'<circle cx="18" cy="24" r="12" fill="{MID}" {S}/><circle cx="30" cy="24" r="12" fill="{MID}" {S}/>' + \
    f'<path d="M24 13.6 A12 12 0 0 1 24 34.4 A12 12 0 0 1 24 13.6 Z" fill="{MID}"/>'
I["bool_subtrair"] = f'<circle cx="18" cy="24" r="12" fill="{MID}" {S}/><circle cx="30" cy="24" r="12" fill="#ffffff" stroke="{INK}" stroke-width="2" stroke-dasharray="3 2"/>'
I["bool_intersecao"] = f'<circle cx="18" cy="24" r="12" fill="none" {S}/><circle cx="30" cy="24" r="12" fill="none" {S}/>' + \
    f'<path d="M24 13.6 A12 12 0 0 1 24 34.4 A12 12 0 0 1 24 13.6 Z" fill="{ACTION}"/>'
I["filete_face_face"] = f'<path d="M6 8 V30 A12 12 0 0 0 18 42 H42" stroke="{SKETCH}" stroke-width="3" fill="none"/>' + \
    f'<rect x="2" y="6" width="6" height="22" fill="{ACTION}" opacity="0.6"/><rect x="20" y="40" width="22" height="6" fill="{ACTION}" opacity="0.6"/>'
I["filete_completo"] = f'<path d="M6 44 V20 A18 18 0 0 1 42 20 V44" fill="{MID}" {S}/>' + \
    f'<path d="M6 20 H42" stroke="{INK}" stroke-width="1.4" stroke-dasharray="3 2"/>'
I["sk_estender"] = f'<line x1="6" y1="24" x2="24" y2="24" {G}/><line x1="24" y1="24" x2="38" y2="24" {DASH}/>' + \
    f'<line x1="40" y1="8" x2="40" y2="40" {G}/>' + arrow(26, 32, 38, 32, ACTION, 2.2)
I["macro"] = f'<rect x="5" y="7" width="38" height="34" rx="5" fill="{LIGHT}" {S}/>' + \
    f'<path d="M12 18 L18 24 L12 30" stroke="{ACTION}" stroke-width="3" fill="none" stroke-linecap="round"/>' + \
    f'<line x1="22" y1="30" x2="34" y2="30" stroke="{ACTION}" stroke-width="3" stroke-linecap="round"/>' + \
    f'<path d="M34 6 L28 18 H34 L30 26" stroke="#ffd34d" stroke-width="2.4" fill="none"/>'
I["plano_ponto_normal"] = PL + LN(24, 32, 24, 4) + DOT(24, 32, 4)
I["plano_meio"] = f'<polygon points="4,40 24,32 44,40 24,48" fill="{REF_L}" {S}/>' + \
    f'<polygon points="4,24 24,16 44,24 24,32" fill="{ACTION_L}" {S}/>' + \
    f'<polygon points="4,8 24,0 44,8 24,16" fill="{REF_L}" {S}/>'
I["ponto_intersecao"] = PL + CV("M8 8 C 14 20, 30 22, 40 44", ACTION) + DOT(24, 30, 4.2)
I["ponto_intersecao_curvas"] = CV("M4 40 C 16 4, 32 4, 44 40") + CV("M4 10 L44 30", ACTION) + DOT(16, 16, 4) + DOT(37, 26.5, 4)
I["ponto_projetado"] = f'<path d="M4 36 C 14 28, 34 28, 44 36 L44 42 C 34 36, 14 36, 4 42 Z" fill="{LIGHT}" {S}/>' + \
    DOT(24, 8, 3.4, INK) + f'<line x1="24" y1="11" x2="24" y2="28" {DASH}/>' + DOT(24, 31, 4)
I["ponto_proj_curva"] = CV("M4 38 C 16 24, 32 24, 44 38") + DOT(24, 8, 3.4, INK) + f'<line x1="24" y1="11" x2="24" y2="26" {DASH}/>' + DOT(24, 28.5, 4)
I["ponto_deslocado"] = DOT(10, 36, 3.4, INK) + arrow(12, 34, 34, 14, INK, 1.8) + DOT(38, 10, 4.5)
I["linha_planos"] = f'<polygon points="4,30 24,22 44,30 24,38" fill="{LIGHT}" {S}/>' + \
    f'<polygon points="20,6 30,2 30,34 20,38" fill="{ACTION_L}" opacity="0.9" {S}/>' + LN(10, 30, 40, 29, ACTION, 3)
I["linha_bissetriz"] = f'<path d="M6 40 H42 M6 40 L36 8" {G}/>' + LN(6, 40, 42, 24, ACTION, 2.6)
I["linha_eixo"] = f'<path d="M12 12 V36 A12 5 0 0 0 36 36 V12" fill="{MID}" {S}/><ellipse cx="24" cy="12" rx="12" ry="5" fill="{LIGHT}" {S}/>' + \
    LN(24, 2, 24, 46, ACTION, 2.4)
I["curva_projetada"] = CV("M6 14 C 16 2, 30 26, 42 10", SKETCH, 2.4) + PL + CV("M8 34 C 16 28, 30 40, 40 32", ACTION, 2.6)
I["spline_controle"] = f'<polyline points="4,38 14,8 34,8 44,38" stroke="{INK}" stroke-width="1.4" stroke-dasharray="3 2" fill="none"/>' + \
    CV("M4 38 C 14 8, 34 8, 44 38", ACTION, 2.8) + "".join(DOT(x, y, 3, INK) for x, y in [(4, 38), (14, 8), (34, 8), (44, 38)])
I["curva_extraida"] = box((24, 14), 14) + LN(6, 23, 24, 32, ACTION, 4)
I["sup_extraida"] = box((24, 14), 14) + f'<polygon points="6,23 24,14 42,23 24,32" fill="{ACTION}" opacity="0.85"/>'
I["sup_deslocada"] = SURF("M4 34 C 14 24, 34 24, 44 34 L44 38 C 34 30, 14 30, 4 38 Z") + \
    f'<path d="M4 20 C 14 10, 34 10, 44 20" stroke="{ACTION}" stroke-width="2.6" fill="none" stroke-dasharray="4 2"/>' + arrow(24, 26, 24, 14, ACTION, 2)
I["sup_preenchida"] = f'<path d="M6 30 C 10 12, 30 6, 42 16 C 44 30, 30 42, 10 40 Z" fill="{LIGHT}" stroke="{ACTION}" stroke-width="2.8"/>'
I["sup_extrudada"] = f'<path d="M6 36 C 14 28, 22 40, 30 32 L30 12 C 22 20, 14 8, 6 16 Z" fill="{LIGHT}" {S}/>' + \
    CV("M6 36 C 14 28, 22 40, 30 32", ACTION) + arrow(34, 34, 42, 14, INK, 1.8)
I["sup_revolucao"] = f'<path d="M14 8 C 6 16, 6 32, 14 40 L34 40 C 42 32, 42 16, 34 8 Z" fill="{LIGHT}" {S}/>' + \
    f'<line x1="24" y1="2" x2="24" y2="46" stroke="{INK}" stroke-width="1.4" stroke-dasharray="4 2.5"/>' + CV("M34 8 C 42 16, 42 32, 34 40", ACTION)

# ---------------- alpha.6: bancada de superfícies, operações, transformações, volume e análise
PATCH = lambda d, fill=LIGHT: f'<path d="{d}" fill="{fill}" {S}/>'
HL = lambda d, w=3: f'<path d="{d}" stroke="{ACTION}" stroke-width="{w}" fill="none" stroke-linecap="round" stroke-linejoin="round"/>'
WAVE = "M4 30 C 12 22, 20 22, 26 28 C 32 34, 38 32, 44 26"
SHAPE = lambda dx=0, dy=0, fill=LIGHT: f'<path transform="translate({dx} {dy})" d="M6 30 C 6 22, 14 16, 22 18 C 26 19, 26 26, 20 30 Z" fill="{fill}" {S}/>'

# término até superfície: prisma que sobe e para numa superfície ondulada
I["ext_ate_superficie"] = PATCH("M2 14 C 12 6, 22 8, 28 12 C 34 16, 40 14, 46 8 L46 14 C 40 20, 34 22, 28 18 C 22 14, 12 12, 2 20 Z") + \
    f'<path d="M14 42 V15 M34 42 V17" stroke="{INK}" stroke-width="2"/>' + f'<rect x="14" y="30" width="20" height="12" fill="{MID}" {S}/>' + arrow(24, 30, 24, 17, ACTION, 2.6)

# superfícies primitivas
I["sup_esfera"] = f'<circle cx="24" cy="24" r="19" fill="{LIGHT}" {S}/>' + \
    f'<ellipse cx="24" cy="24" rx="19" ry="6" fill="none" stroke="{INK}" stroke-width="1.4" stroke-dasharray="3 2.5"/>' + \
    f'<path d="M15 12 C 18 9, 22 8, 25 8" stroke="#ffffff" stroke-width="2.4" fill="none" stroke-linecap="round" opacity="0.9"/>'
I["sup_cilindro"] = f'<path d="M10 10 V38 A14 5 0 0 0 38 38 V10" fill="{MID}" {S}/>' + \
    f'<ellipse cx="24" cy="10" rx="14" ry="5" fill="#ffffff" {S}/>' + f'<ellipse cx="24" cy="10" rx="9" ry="2.6" fill="none" stroke="{INK}" stroke-width="1" opacity="0.4"/>'
I["sup_varredura"] = f'<path d="M6 34 C 16 34, 22 14, 40 12 L40 22 C 26 24, 20 42, 6 42 Z" fill="{LIGHT}" {S}/>' + \
    f'<path d="M6 34 V42" stroke="{ACTION}" stroke-width="3.2" stroke-linecap="round"/>' + \
    f'<path d="M6 38 C 18 38, 22 18, 40 17" stroke="{INK}" stroke-width="1.4" stroke-dasharray="3 2" fill="none"/>'
I["sup_tubo"] = f'<path d="M6 30 C 16 30, 20 10, 36 10 L36 20 C 26 20, 22 40, 6 40 Z" fill="{MID}" {S}/>' + \
    f'<ellipse cx="36" cy="15" rx="3.2" ry="5" fill="{LIGHT}" {S}/>' + \
    f'<ellipse cx="6" cy="35" rx="3.2" ry="5" fill="#ffffff" stroke="{ACTION}" stroke-width="2.4"/>'
I["sup_multisecoes"] = f'<path d="M6 34 C 6 20, 18 8, 30 8 C 40 8, 44 14, 44 18 L34 40 C 30 40, 6 42, 6 34 Z" fill="{LIGHT}" stroke="none"/>' + \
    f'<ellipse cx="14" cy="34" rx="8" ry="6" fill="none" stroke="{ACTION}" stroke-width="2.6"/>' + \
    f'<ellipse cx="36" cy="14" rx="7" ry="5" fill="none" stroke="{ACTION}" stroke-width="2.6"/>' + \
    f'<path d="M6 34 C 10 22, 24 12, 29 14 M22 34 C 28 28, 38 24, 43 14" stroke="{INK}" stroke-width="1.6" fill="none"/>'
I["sup_concordancia"] = PATCH("M2 34 L16 34 L16 42 L2 42 Z", MID) + PATCH("M32 8 L46 8 L46 16 L32 16 Z", MID) + \
    f'<path d="M16 34 C 26 34, 22 16, 32 16 L32 8 C 18 8, 26 26, 16 26 Z" fill="{ACTION_L}" {S}/>' + HL("M16 30 C 25 30, 22 12, 32 12", 2.6)

# operações
I["op_juntar"] = PATCH("M4 16 L22 12 L24 36 L6 38 Z") + PATCH("M24 12 L44 14 L42 38 L24 36 Z", MID) + HL("M23 12 L24 36", 3.4) + \
    f'<circle cx="23.5" cy="24" r="4" fill="#ffffff" stroke="{ACTION}" stroke-width="2"/>'
I["op_curar"] = PATCH("M4 14 L21 12 L22 38 L6 38 Z") + PATCH("M27 12 L44 14 L42 38 L26 38 Z", MID) + \
    f'<path d="M18 24 H30" stroke="{ACTION}" stroke-width="5" stroke-linecap="round"/>' + \
    f'<path d="M20 21 V27 M24 21 V27 M28 21 V27" stroke="#ffffff" stroke-width="1.2"/>'
I["op_dividir"] = PATCH("M4 20 C 12 12, 20 12, 24 18 L24 40 C 18 34, 10 34, 4 42 Z", ACTION_L) + \
    PATCH("M24 18 C 30 26, 38 26, 44 18 L44 38 C 38 46, 30 44, 24 40 Z") + \
    f'<path d="M24 4 V46" stroke="{ACTION}" stroke-width="2.4" stroke-dasharray="4 3"/>'
I["op_aparar"] = PATCH("M4 26 L24 26 L24 44 L4 44 Z") + PATCH("M24 6 L42 6 L42 26 L24 26 Z", MID) + \
    f'<path d="M24 26 L44 26 M24 26 V46" stroke="{INK}" stroke-width="1.4" stroke-dasharray="3 2.5" opacity="0.7"/>' + HL("M4 26 H24 V6", 3)
I["op_contorno"] = f'<path d="M6 30 C 6 14, 20 6, 32 8 C 44 10, 44 28, 38 36 C 30 44, 10 44, 6 30 Z" fill="{LIGHT}" stroke="{ACTION}" stroke-width="3.4"/>'
I["op_multi_extrair"] = box((24, 14), 14) + f'<polygon points="6,23 24,14 42,23 24,32" fill="{ACTION}" opacity="0.85"/>' + \
    f'<polygon points="24,32 42,23 42,41 24,50" fill="{ACTION}" opacity="0.6" transform="translate(0 -6)"/>'
I["op_desmontar"] = PATCH("M4 8 L18 6 L18 20 L4 22 Z") + PATCH("M28 6 L44 8 L44 22 L28 20 Z", MID) + PATCH("M14 28 L34 28 L34 44 L14 44 Z") + \
    arrow(20, 22, 14, 18, ACTION, 1.8) + arrow(28, 22, 34, 18, ACTION, 1.8)
I["op_desaparar"] = f'<rect x="4" y="6" width="40" height="36" rx="2" fill="none" stroke="{ACTION}" stroke-width="2" stroke-dasharray="4 3"/>' + \
    PATCH("M8 12 H30 L38 22 V36 H8 Z") + f'<circle cx="20" cy="24" r="5" fill="#ffffff" {S}/>'
I["op_extrapolar"] = PATCH("M4 14 C 12 10, 20 10, 26 14 L26 38 C 20 34, 12 34, 4 38 Z") + \
    f'<path d="M26 14 C 32 18, 38 18, 44 14 L44 38 C 38 42, 32 42, 26 38" fill="{ACTION_L}" stroke="{ACTION}" stroke-width="2" stroke-dasharray="4 3"/>' + \
    arrow(26, 26, 42, 26, ACTION, 2.4)

# transformações (de pontos, curvas, planos e superfícies)
I["xf_mover"] = SHAPE(0, 4, MID) + SHAPE(18, -10) + arrow(16, 30, 34, 14, ACTION, 2.6)
I["xf_girar"] = SHAPE(2, 6, MID) + f'<path d="M30 40 A16 16 0 0 0 38 14" stroke="{ACTION}" stroke-width="2.6" fill="none"/>' + \
    AR("38,8 42,17 33,15") + DOT(24, 24, 3, INK)
I["xf_simetria"] = SHAPE(-2, 0) + f'<path transform="translate(48 0) scale(-1 1)" d="M6 30 C 6 22, 14 16, 22 18 C 26 19, 26 26, 20 30 Z" fill="{MID}" stroke="{ACTION}" stroke-width="2"/>' + \
    f'<line x1="24" y1="4" x2="24" y2="44" stroke="{INK}" stroke-width="1.6" stroke-dasharray="4 3"/>'
I["xf_escala"] = f'<rect x="4" y="28" width="14" height="14" fill="{MID}" {S}/>' + \
    f'<rect x="4" y="8" width="34" height="34" fill="none" stroke="{ACTION}" stroke-width="2" stroke-dasharray="4 3"/>' + arrow(16, 30, 36, 10, ACTION, 2.6)

# curvas de wireframe
I["curva_intersecao"] = PATCH("M2 30 L24 22 L46 30 L24 38 Z") + \
    f'<path d="M14 6 C 20 16, 28 16, 34 6 L34 30 C 28 40, 20 40, 14 30 Z" fill="{MID}" opacity="0.85" {S}/>' + HL("M14 30 C 20 36, 28 36, 34 30", 3.4)
I["curva_proj_sup"] = SURF("M4 34 C 14 26, 34 26, 44 34 L44 42 C 34 34, 14 34, 4 42 Z") + CV("M6 10 C 16 2, 30 18, 42 8", SKETCH, 2.4) + \
    "".join(f'<line x1="{x}" y1="{y}" x2="{x}" y2="{y2}" {DASH}/>' for x, y, y2 in [(10, 8, 31), (24, 10, 29), (38, 9, 31)]) + \
    CV("M6 34 C 16 29, 30 31, 42 34", ACTION, 3)

# volume (superfície ↔ sólido)
I["hib_dividir"] = f'<path d="M6 22 C 14 16, 22 16, 26 20 C 32 26, 38 24, 42 18 V40 H6 Z" fill="{MID}" {S}/>' + \
    f'<path d="M6 22 C 14 16, 22 16, 26 20 C 32 26, 38 24, 42 18 V8 H6 Z" fill="#ffffff" stroke="{INK}" stroke-width="1.4" stroke-dasharray="3 2.5" opacity="0.8"/>' + \
    HL("M2 24 C 12 14, 22 16, 26 20 C 32 26, 40 24, 46 16", 3)
I["hib_engrossar"] = f'<path d="M4 26 C 14 18, 32 18, 44 26 L44 36 C 32 28, 14 28, 4 36 Z" fill="{MID}" {S}/>' + \
    HL("M4 26 C 14 18, 32 18, 44 26", 3) + arrow(24, 12, 24, 22, ACTION, 2) + arrow(24, 40, 24, 30, ACTION, 2)
I["hib_fechar"] = f'<path d="M10 12 V36 A14 5 0 0 0 38 36 V12" fill="{MID}" {S}/>' + \
    f'<ellipse cx="24" cy="12" rx="14" ry="5" fill="{ACTION}" opacity="0.9" {S}/>' + \
    f'<path d="M10 36 A14 5 0 0 0 38 36" stroke="{ACTION}" stroke-width="3" fill="none"/>'
I["hib_costurar"] = box((24, 22), 14) + f'<path d="M6 31 C 10 14, 38 14, 42 31" fill="{ACTION_L}" stroke="{ACTION}" stroke-width="2.6"/>' + \
    "".join(f'<line x1="{x}" y1="{y - 3}" x2="{x}" y2="{y + 3}" stroke="{ACTION}" stroke-width="1.6"/>' for x, y in [(10, 28), (16, 23), (24, 20), (32, 23), (38, 28)])
I["hib_preencher"] = f'<path d="M4 14 H44 M4 36 H44 M12 4 V44 M36 4 V44" stroke="{INK}" stroke-width="1.6" stroke-dasharray="4 2.5"/>' + \
    f'<rect x="12" y="14" width="24" height="22" fill="{ACTION}" opacity="0.85"/>' + f'<rect x="12" y="14" width="24" height="22" fill="none" {S}/>'
I["hib_malha"] = f'<path d="M4 38 L12 14 L22 30 Z M12 14 L26 10 L22 30 Z M22 30 L26 10 L30 34 Z" fill="{LIGHT}" stroke="{INK}" stroke-width="1.4" stroke-linejoin="round"/>' + \
    arrow(30, 22, 36, 22, ACTION, 2) + f'<path d="M38 36 V18 C 38 12, 46 12, 46 18 V36 Z" fill="{MID}" {S}/>'

# análise
I["an_zebra"] = f'<clipPath id="cz"><circle cx="24" cy="24" r="19"/></clipPath>' + f'<circle cx="24" cy="24" r="19" fill="#ffffff"/>' + \
    '<g clip-path="url(#cz)">' + "".join(f'<path d="M0 {y} C 16 {y - 8}, 32 {y - 8}, 48 {y}" stroke="#1b2430" stroke-width="4.2" fill="none"/>' for y in (10, 20, 30, 40, 50)) + '</g>' + \
    f'<circle cx="24" cy="24" r="19" fill="none" {S}/>'
I["an_curvatura"] = '<defs><radialGradient id="ck" cx="0.38" cy="0.35" r="0.75"><stop offset="0" stop-color="#ef3b2c"/><stop offset="0.3" stop-color="#fdd835"/>' + \
    '<stop offset="0.55" stop-color="#43c25a"/><stop offset="0.8" stop-color="#22c5e8"/><stop offset="1" stop-color="#2a4ff0"/></radialGradient></defs>' + \
    f'<circle cx="24" cy="24" r="19" fill="url(#ck)" {S}/>'
I["an_sombreado"] = box((24, 8), 18)

# opções de painel
I["op_unir"] = f'<circle cx="18" cy="24" r="12" fill="{MID}" {S}/><circle cx="30" cy="24" r="12" fill="{MID}" {S}/>' + \
    f'<path d="M24 13.6 A12 12 0 0 1 24 34.4 A12 12 0 0 1 24 13.6 Z" fill="{MID}"/>'
_PROF = lambda x, y, a: f'<rect x="{x - 4}" y="{y - 4}" width="8" height="8" fill="{ACTION}" opacity="0.85" transform="rotate({a} {x} {y})"/>'
I["sweep_corrigido"] = CV("M4 40 C 14 40, 20 10, 44 10") + _PROF(6, 40, 0) + _PROF(20, 26, -55) + _PROF(40, 11, -10)
I["sweep_fixo"] = CV("M4 40 C 14 40, 20 10, 44 10") + _PROF(6, 40, 0) + _PROF(20, 26, 0) + _PROF(40, 11, 0)
I["sweep_frenet"] = CV("M4 40 C 14 40, 20 10, 44 10") + _PROF(6, 40, 20) + _PROF(20, 26, -80) + _PROF(40, 11, 45)
I["cont_g0"] = CV("M4 40 L24 14", INK, 3) + CV("M24 14 L44 36", ACTION, 3) + DOT(24, 14, 3.4, INK)
I["cont_g1"] = CV("M4 40 C 10 26, 16 14, 24 14", INK, 3) + CV("M24 14 C 34 14, 40 24, 44 40", ACTION, 3) + \
    f'<line x1="12" y1="14" x2="36" y2="14" stroke="{INK}" stroke-width="1.2" stroke-dasharray="3 2"/>' + DOT(24, 14, 3.4, INK)
I["cont_g2"] = CV("M4 40 C 10 26, 16 14, 24 14", INK, 3) + CV("M24 14 C 32 14, 38 26, 44 40", ACTION, 3) + \
    "".join(f'<line x1="{x}" y1="{y}" x2="{x}" y2="{y - h}" stroke="{ACTION}" stroke-width="1" opacity="0.7"/>' for x, y, h in [(14, 22, 5), (19, 17, 6), (24, 14, 6), (29, 15, 6), (34, 19, 5)]) + DOT(24, 14, 3.4, INK)

# ---------------- alpha.7: até o último, saída + filetes, afinidade, contrafurado, paralela, extrair
I["ext_ate_ultimo"] = box((24, 26), 16) + f'<path d="M24 6 V20" stroke="{ACTION}" stroke-width="3"/>' + AR("18,18 30,18 24,26") + \
    f'<line x1="6" y1="44" x2="42" y2="44" stroke="{ACTION}" stroke-width="3" stroke-dasharray="5 3"/>'
I["ext_saida_filete"] = f'<path d="M10 42 L15 14 Q16 10 20 10 L28 10 Q32 10 33 14 L38 42 Z" fill="{MID}" {S}/>' + \
    f'<path d="M15 14 Q16 10 20 10 M28 10 Q32 10 33 14" stroke="{ACTION}" stroke-width="3.2" fill="none"/>' + \
    f'<line x1="6" y1="42" x2="42" y2="42" stroke="{INK}" stroke-width="2"/>'
I["xf_afinidade"] = f'<rect x="6" y="22" width="14" height="14" fill="{MID}" {S}/>' + \
    f'<rect x="6" y="10" width="34" height="26" fill="none" stroke="{ACTION}" stroke-width="2" stroke-dasharray="4 3"/>' + \
    arrow(20, 30, 40, 30, ACTION, 2.2) + arrow(13, 22, 13, 8, ACTION, 2.2)
I["furo_contrafurado"] = f'<path d="M4 10 H44 V44 H4 Z" fill="{MID}" {S}/>' + \
    f'<path d="M14 10 V18 L20 24 V44 H28 V24 L34 18 V10 Z" fill="#ffffff" {S}/>'
I["curva_paralela"] = CV("M4 34 C 14 18, 30 18, 44 30", INK, 2.6) + CV("M4 24 C 14 8, 30 8, 44 20", ACTION, 3) + \
    "".join(f'<line x1="{x}" y1="{y}" x2="{x}" y2="{y - 9}" stroke="{INK}" stroke-width="1.2" stroke-dasharray="2 2"/>' for x, y in [(12, 27), (24, 22), (36, 25)])
I["extrair"] = box((22, 16), 14) + f'<polygon points="4,25 22,16 40,25 22,34" fill="{ACTION}" opacity="0.85"/>' + arrow(30, 20, 42, 8, ACTION, 2.4)

# ---------------- montagem: componentes e restrições
BLK = lambda x, y, s=12, fill=MID: f'<rect x="{x}" y="{y}" width="{s}" height="{s}" rx="1.5" fill="{fill}" {S}/>'
I["asm_nova"] = BLK(6, 24, 14) + BLK(22, 10, 14, LIGHT) + f'<path d="M34 30 V42 M28 36 H40" stroke="{ACTION}" stroke-width="3" stroke-linecap="round"/>'
I["asm_inserir"] = BLK(4, 26, 16) + BLK(26, 6, 16, LIGHT) + arrow(30, 28, 22, 36, ACTION, 2.4)
I["asm_subconjunto"] = f'<rect x="4" y="6" width="40" height="36" rx="4" fill="none" stroke="{INK}" stroke-width="1.6" stroke-dasharray="4 3"/>' + BLK(10, 22, 12) + BLK(24, 14, 12, LIGHT) + BLK(24, 28, 10)
I["asm_peca_nova"] = BLK(6, 20, 18) + f'<path d="M28 8 L42 8 L42 22 L28 22 Z" fill="#ffffff" stroke="{ACTION}" stroke-width="2" stroke-dasharray="3 2"/>' + \
    f'<path d="M35 11 V19 M31 15 H39" stroke="{ACTION}" stroke-width="2.4" stroke-linecap="round"/>'
I["asm_grupo"] = f'<path d="M4 14 H18 L22 18 H44 V40 H4 Z" fill="{LIGHT}" {S}/>' + BLK(12, 24, 9, MID) + BLK(26, 24, 9, MID)
I["asm_substituir"] = BLK(4, 24, 14) + BLK(30, 10, 14, LIGHT) + f'<path d="M20 22 C 24 12, 28 12, 30 14" stroke="{ACTION}" stroke-width="2.4" fill="none"/>' + AR("31,9 34,17 26,16") + \
    f'<path d="M28 34 C 24 42, 20 42, 18 40" stroke="{ACTION}" stroke-width="2.4" fill="none"/>' + AR("16,45 13,37 21,38")
I["asm_numerar"] = BLK(4, 6, 12) + BLK(4, 20, 12) + BLK(4, 34, 10) + "".join(f'<circle cx="30" cy="{y}" r="6" fill="{ACTION}"/>' for y in (12, 26, 39)) + \
    "".join(f'<line x1="17" y1="{y}" x2="24" y2="{y}" stroke="{INK}" stroke-width="1.4"/>' for y in (12, 26, 39))
I["rest_auto"] = BLK(4, 24, 16) + BLK(28, 8, 16, LIGHT) + f'<path d="M22 6 L18 18 H26 L20 30" stroke="#ffd34d" stroke-width="3" fill="none" stroke-linejoin="round"/>'
I["rest_coincidencia"] = f'<circle cx="24" cy="24" r="16" fill="{LIGHT}" {S}/><circle cx="24" cy="24" r="8" fill="{MID}" {S}/>' + \
    f'<line x1="24" y1="2" x2="24" y2="46" stroke="{ACTION}" stroke-width="2.4" stroke-dasharray="5 3"/>'
I["rest_contato"] = f'<rect x="6" y="26" width="36" height="14" fill="{MID}" {S}/><rect x="10" y="10" width="28" height="14" fill="{LIGHT}" {S}/>' + \
    f'<line x1="4" y1="25" x2="44" y2="25" stroke="{ACTION}" stroke-width="3"/>'
I["rest_distancia"] = f'<rect x="4" y="30" width="40" height="10" fill="{MID}" {S}/><rect x="4" y="8" width="40" height="10" fill="{LIGHT}" {S}/>' + \
    arrow(24, 22, 24, 19, ACTION, 2) + arrow(24, 26, 24, 29, ACTION, 2) + f'<line x1="24" y1="20" x2="24" y2="28" stroke="{ACTION}" stroke-width="2"/>'
I["rest_angulo"] = f'<path d="M6 40 H42" stroke="{INK}" stroke-width="3"/><path d="M6 40 L34 10" stroke="{INK}" stroke-width="3"/>' + \
    f'<path d="M22 40 A16 16 0 0 0 17 28" stroke="{ACTION}" stroke-width="2.6" fill="none"/>'
I["rest_paralelo"] = f'<path d="M8 36 L30 10 M18 40 L40 14" stroke="{ACTION}" stroke-width="3.4" stroke-linecap="round"/>'
I["rest_perpendicular"] = f'<path d="M8 40 H40 M24 40 V8" stroke="{ACTION}" stroke-width="3.4" stroke-linecap="round"/><rect x="24" y="32" width="8" height="8" fill="none" stroke="{INK}" stroke-width="1.6"/>'
I["rest_fixar"] = BLK(8, 24, 22) + f'<path d="M30 6 L40 16 L34 18 L26 26 L22 22 L30 14 Z" fill="{ACTION}"/><line x1="24" y1="24" x2="16" y2="32" stroke="{ACTION}" stroke-width="2.4"/>'
I["rest_conjunta"] = BLK(4, 22, 16) + BLK(28, 22, 16) + f'<rect x="16" y="8" width="16" height="12" rx="2" fill="{ACTION}"/>' + \
    f'<path d="M19 8 V5 A5 5 0 0 1 29 5 V8" stroke="{ACTION}" stroke-width="2.4" fill="none"/>' + f'<line x1="20" y1="30" x2="28" y2="30" stroke="{ACTION}" stroke-width="3"/>'
I["rest_alinhar"] = BLK(4, 28, 14) + f'<rect x="28" y="6" width="14" height="14" rx="1.5" fill="none" stroke="{INK}" stroke-width="1.6" stroke-dasharray="3 2"/>' + \
    BLK(18, 28, 14, LIGHT) + arrow(34, 22, 26, 28, ACTION, 2.4)
I["rest_atualizar"] = f'<path d="M38 18 A15 15 0 1 0 39 30" stroke="{ACTION}" stroke-width="3.4" fill="none" stroke-linecap="round"/>' + AR("44,10 42,22 32,16") + BLK(18, 18, 12)
I["rest_interferencia"] = BLK(6, 16, 20) + f'<rect x="20" y="10" width="20" height="20" rx="1.5" fill="{LIGHT}" opacity="0.85" {S}/>' + \
    f'<rect x="20" y="16" width="6" height="14" fill="#e5484d" opacity="0.9"/>'

I["salvar"] = f'<path d="M6 6 H36 L42 12 V42 H6 Z" fill="{MID}" {S}/><rect x="12" y="6" width="20" height="12" fill="#ffffff" {S}/><rect x="12" y="26" width="24" height="16" fill="{LIGHT}" {S}/><rect x="25" y="9" width="4" height="7" fill="{ACTION}"/>'

I["asm_repeticao"] = BLK(4, 30, 10) + "".join(f'<rect x="{x}" y="{y}" width="10" height="10" rx="1.5" fill="{LIGHT}" stroke="{ACTION}" stroke-width="1.6" stroke-dasharray="3 2"/>' for x, y in [(18, 22), (32, 14)]) + \
    f'<circle cx="9" cy="24" r="3" fill="{ACTION}"/><circle cx="23" cy="16" r="3" fill="{ACTION}" opacity="0.7"/><circle cx="37" cy="8" r="3" fill="{ACTION}" opacity="0.5"/>'

# ---------------- cinemática
ROD = lambda x1, y1, x2, y2, fill=MID: f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{INK}" stroke-width="9" stroke-linecap="round"/><line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#cfe6ff" stroke-width="5.5" stroke-linecap="round"/>'
PIN = lambda x, y: f'<circle cx="{x}" cy="{y}" r="4.5" fill="{ACTION}" stroke="#ffffff" stroke-width="1.4"/>'
I["junta_revolucao"] = ROD(10, 38, 38, 12) + PIN(10, 38) + f'<path d="M22 44 A14 14 0 0 0 4 26" stroke="{ACTION}" stroke-width="2.2" fill="none"/>' + AR("2,30 6,22 9,30")
I["junta_prismatica"] = f'<rect x="4" y="20" width="40" height="8" fill="{LIGHT}" {S}/><rect x="16" y="14" width="14" height="20" rx="2" fill="{MID}" {S}/>' + arrow(32, 40, 44, 40, ACTION, 2.2) + arrow(14, 40, 4, 40, ACTION, 2.2)
I["junta_cilindrica"] = f'<rect x="4" y="20" width="40" height="8" rx="4" fill="{LIGHT}" {S}/><rect x="16" y="13" width="14" height="22" rx="3" fill="{MID}" {S}/>' + \
    f'<path d="M34 10 A10 6 0 0 1 34 38" stroke="{ACTION}" stroke-width="2" fill="none"/>' + arrow(18, 42, 30, 42, ACTION, 2)
I["junta_parafuso"] = f'<rect x="20" y="4" width="8" height="40" fill="{LIGHT}" {S}/>' + "".join(f'<path d="M18 {y} L30 {y + 4}" stroke="{INK}" stroke-width="1.6"/>' for y in range(8, 40, 6)) + \
    f'<rect x="12" y="20" width="24" height="10" fill="{MID}" {S}/>' + arrow(40, 34, 40, 18, ACTION, 2)
I["junta_esferica"] = f'<circle cx="24" cy="26" r="10" fill="{MID}" {S}/><path d="M10 20 A16 16 0 0 1 38 20" stroke="{INK}" stroke-width="5" fill="none"/>' + ROD(24, 26, 36, 6) + PIN(24, 26)
I["junta_planar"] = f'<polygon points="4,32 24,22 44,32 24,42" fill="{LIGHT}" {S}/><rect x="17" y="20" width="14" height="10" fill="{MID}" {S}/>' + arrow(32, 25, 42, 20, ACTION, 2) + arrow(18, 25, 8, 20, ACTION, 2)
I["junta_rigida"] = BLK(4, 20, 18) + BLK(26, 20, 18, LIGHT) + f'<path d="M20 29 H28" stroke="{ACTION}" stroke-width="4"/><circle cx="24" cy="29" r="3" fill="#ffffff" stroke="{ACTION}" stroke-width="1.6"/>'
I["junta_ponto_curva"] = CV("M4 38 C 14 8, 30 8, 44 30", INK, 2.6) + f'<circle cx="28" cy="13" r="5" fill="{ACTION}" stroke="#ffffff" stroke-width="1.4"/>' + arrow(32, 15, 40, 22, ACTION, 1.8)
I["mec_auto"] = ROD(8, 36, 26, 18) + ROD(26, 18, 40, 34) + PIN(8, 36) + PIN(26, 18) + PIN(40, 34) + f'<path d="M30 4 L26 12 H32 L28 20" stroke="#ffd34d" stroke-width="2.4" fill="none"/>'
I["mec_simular"] = f'<rect x="4" y="8" width="40" height="28" rx="4" fill="{LIGHT}" {S}/>' + ROD(12, 28, 26, 16) + PIN(12, 28) + \
    f'<polygon points="32,16 32,30 42,23" fill="{ACTION}"/>' + f'<line x1="6" y1="42" x2="42" y2="42" stroke="{INK}" stroke-width="2"/><circle cx="18" cy="42" r="3" fill="{ACTION}"/>'

# ---------------- chapa metálica
SHEET = lambda pts: f'<path d="{pts}" fill="{LIGHT}" {S}/>'
I["sm_parametros"] = f'<path d="M4 34 L24 26 L44 34 L24 42 Z" fill="{LIGHT}" {S}/><path d="M4 34 V38 L24 46 V42 M24 46 L44 38 V34" fill="{MID}" {S}/>' + \
    f'<path d="M34 6 L36 10 L40 10.5 L37 13.5 L38 18 L34 15.8 L30 18 L31 13.5 L28 10.5 L32 10 Z" fill="{ACTION}"/>' + arrow(10, 28, 10, 20, ACTION, 1.6)
I["sm_parede"] = f'<path d="M4 30 L24 20 L44 30 L24 40 Z" fill="{LIGHT}" {S}/><path d="M4 30 V35 L24 45 V40 M24 45 L44 35 V30" fill="{MID}" {S}/>' + \
    f'<path d="M10 30 L24 23 L38 30 L24 37 Z" fill="none" stroke="{ACTION}" stroke-width="2" stroke-dasharray="3 2"/>'
I["sm_aba"] = f'<path d="M4 38 H28 Q36 38 36 30 V6 H42 V30 Q42 44 28 44 H4 Z" fill="{MID}" {S}/>' + f'<path d="M28 38 Q36 38 36 30" stroke="{ACTION}" stroke-width="3" fill="none"/>'
I["sm_bainha"] = f'<path d="M4 34 H32 A7 7 0 0 0 32 20 H16 V26 H32 A1 1 0 0 1 32 28 H4 Z" fill="{MID}" {S}/>' + f'<path d="M32 34 A7 7 0 0 0 32 20" stroke="{ACTION}" stroke-width="3" fill="none"/>'
I["sm_joggle"] = f'<path d="M4 18 H18 L26 30 H44 V36 H24 L16 24 H4 Z" fill="{MID}" {S}/>' + arrow(34, 12, 34, 26, ACTION, 2)
I["sm_dobra_linha"] = f'<path d="M4 40 H24 Q30 40 32 34 L40 8 L45 10 L37 36 Q34 46 24 46 H4 Z" fill="{MID}" {S}/>' + \
    f'<path d="M4 30 H44" stroke="{ACTION}" stroke-width="1.8" stroke-dasharray="6 2 2 2"/>' + arrow(18, 28, 30, 18, ACTION, 2)
I["sm_veneziana"] = f'<path d="M2 32 H14 V36 H2 Z M34 32 H46 V36 H34 Z" fill="{MID}" {S}/>' + \
    f'<path d="M14 36 L34 36 L34 32 L14 20 L14 24 L30 33 Z" fill="{LIGHT}" {S}/>' + f'<path d="M14 20 V36" stroke="{ACTION}" stroke-width="2.4"/>' + \
    f'<path d="M4 14 C 8 10, 10 18, 14 14" stroke="{ACTION}" stroke-width="1.6" fill="none"/>'
I["junta_came"] = f'<path d="M24 8 C 36 8, 40 20, 38 28 C 36 38, 28 42, 20 40 C 10 38, 8 28, 10 20 C 12 12, 16 8, 24 8 Z" fill="{LIGHT}" {S}/>' +     f'<circle cx="22" cy="28" r="3" fill="{INK}"/><rect x="20" y="0" width="8" height="4" fill="{MID}"/>' + f'<circle cx="24" cy="4" r="4" fill="{MID}" stroke="{ACTION}" stroke-width="2"/>' +     f'<path d="M40 30 A 18 18 0 0 1 30 44" stroke="{ACTION}" stroke-width="2" fill="none"/>' + AR("28,46 34,46 32,40")
I["des_esquema"] = f'<path d="M4 40 H22 M7 40 L4 45 M12 40 L9 45 M17 40 L14 45 M22 40 L19 45" stroke="{INK}" stroke-width="1.8"/>' +     f'<path d="M13 34 L30 14 L42 30" stroke="{INK}" stroke-width="3" fill="none" stroke-linejoin="round"/>' +     "".join(f'<circle cx="{x}" cy="{y}" r="4" fill="#ffffff" stroke="{ACTION}" stroke-width="2.2"/>' for x, y in [(13, 34), (30, 14)]) +     f'<rect x="36" y="26" width="10" height="8" fill="#ffffff" stroke="{ACTION}" stroke-width="2.2"/>'
I["sm_estampo"] = f'<path d="M4 18 H14 L18 30 H30 L34 18 H44 V24 H38 L34 36 H14 L10 24 H4 Z" fill="{MID}" {S}/>' + f'<path d="M18 30 H30" stroke="{ACTION}" stroke-width="3"/>'
I["sm_furo_flangeado"] = f'<path d="M4 16 H16 V36 H20 V22 H4 Z M44 16 H32 V36 H28 V22 H44 Z" fill="{MID}" {S}/>' + f'<line x1="24" y1="6" x2="24" y2="44" stroke="{ACTION}" stroke-width="1.6" stroke-dasharray="4 3"/>'
I["sm_desdobrar"] = f'<path d="M4 24 H20 Q26 24 26 18 V6 H30 V18 Q30 28 20 28 H4 Z" fill="{MID}" {S}/>' + \
    f'<path d="M4 40 H44 V44 H4 Z" fill="{LIGHT}" {S}/>' + arrow(34, 14, 40, 34, ACTION, 2.2)

# ---------------- estrutural
TUBE = lambda x, y, s=12: f'<rect x="{x}" y="{y}" width="{s}" height="{s}" fill="{MID}" {S}/><rect x="{x + 3}" y="{y + 3}" width="{s - 6}" height="{s - 6}" fill="#ffffff" stroke="{INK}" stroke-width="1"/>'
I["st_membro"] = f'<path d="M6 40 L30 16 L40 26 L16 50" fill="none"/>' + f'<polygon points="4,36 28,12 36,20 12,44" fill="{MID}" {S}/>' + \
    f'<polygon points="28,12 36,4 44,12 36,20" fill="{LIGHT}" {S}/><polygon points="31,12 36,7 41,12 36,17" fill="#ffffff" stroke="{INK}" stroke-width="1"/>' + \
    f'<line x1="2" y1="46" x2="46" y2="2" stroke="{ACTION}" stroke-width="1.6" stroke-dasharray="4 3"/>'
I["st_placa"] = f'<rect x="16" y="4" width="16" height="32" fill="{MID}" {S}/><rect x="6" y="36" width="36" height="8" fill="{ACTION}" {S}/>'
I["st_reforco"] = f'<rect x="4" y="4" width="10" height="40" fill="{MID}" {S}/><rect x="4" y="34" width="40" height="10" fill="{MID}" {S}/>' + \
    f'<polygon points="14,12 14,34 36,34" fill="{ACTION}" opacity="0.9"/><line x1="14" y1="30" x2="18" y2="34" stroke="#ffffff" stroke-width="2"/>'
I["st_biblioteca"] = TUBE(4, 6) + f'<path d="M26 6 H40 V10 H35 V24 H40 V28 H26 V24 H31 V10 H26 Z" fill="{MID}" {S}/>' + \
    f'<path d="M4 30 H16 V44 H12 V34 H4 Z" fill="{MID}" {S}/><circle cx="34" cy="38" r="7" fill="{MID}" {S}/><circle cx="34" cy="38" r="3.5" fill="#ffffff" stroke="{INK}" stroke-width="1"/>'
I["st_corte"] = f'<rect x="6" y="6" width="30" height="36" rx="2" fill="#ffffff" {S}/>' + "".join(f'<line x1="11" y1="{y}" x2="31" y2="{y}" stroke="{INK}" stroke-width="1.6"/>' for y in (14, 21, 28, 35)) + \
    f'<path d="M30 30 L44 44 M44 30 L30 44" stroke="{ACTION}" stroke-width="2.6"/>'
I["st_canto_meia"] = f'<path d="M4 44 V4 H16 V32 Z" fill="{MID}" {S}/><path d="M16 32 L44 32 V44 H4 Z" fill="{LIGHT}" {S}/>' + f'<line x1="4" y1="44" x2="16" y2="32" stroke="{ACTION}" stroke-width="2.6"/>'
I["st_canto_aparar"] = f'<path d="M4 4 H16 V44 H4 Z" fill="{MID}" {S}/><path d="M16 32 H44 V44 H16 Z" fill="{LIGHT}" {S}/>' + f'<line x1="16" y1="32" x2="16" y2="44" stroke="{ACTION}" stroke-width="2.6"/>'
I["st_canto_encaixe"] = f'<circle cx="12" cy="24" r="9" fill="{MID}" {S}/><path d="M21 18 H44 V30 H21 A9 9 0 0 0 21 18 Z" fill="{LIGHT}" {S}/>' + f'<path d="M21 18 A9 9 0 0 1 21 30" stroke="{ACTION}" stroke-width="2.6" fill="none"/>'
I["st_canto_nenhum"] = f'<path d="M4 4 H16 V44 H4 Z" fill="{MID}" {S}/><path d="M4 32 H44 V44 H4 Z" fill="{LIGHT}" opacity="0.8" {S}/>'

I["composer"] = f'<rect x="4" y="8" width="40" height="30" rx="3" fill="{LIGHT}" {S}/>' + BLK(10, 20, 10) + f'<rect x="26" y="12" width="10" height="10" rx="1.5" fill="{MID}" stroke="{ACTION}" stroke-width="1.6" stroke-dasharray="3 2"/>' + \
    arrow(21, 22, 26, 18, ACTION, 1.6) + f'<line x1="6" y1="43" x2="42" y2="43" stroke="{INK}" stroke-width="2"/>' + "".join(f'<polygon points="{x},40 {x + 3},43 {x},46 {x - 3},43" fill="#ffb020"/>' for x in (10, 24, 38))

# ---------------- PMI
PC = "#e8701a"
I["pmi_linear"] = box((24, 22), 14) + f'<path d="M6 10 H42" stroke="{PC}" stroke-width="2"/>' + AR("6,10 12,7 12,13").replace(ACTION, PC) + AR("42,10 36,7 36,13").replace(ACTION, PC) + \
    f'<path d="M6 6 V20 M42 6 V20" stroke="{PC}" stroke-width="1.4"/>'
I["pmi_raio"] = f'<circle cx="20" cy="28" r="14" fill="{LIGHT}" {S}/>' + f'<path d="M20 28 L34 18 L44 18" stroke="{PC}" stroke-width="2.2" fill="none"/><circle cx="20" cy="28" r="2.4" fill="{PC}"/>' + \
    f'<text x="36" y="14" font-size="10" font-family="sans-serif" fill="{PC}">R</text>'
I["pmi_diametro"] = f'<circle cx="20" cy="28" r="14" fill="{LIGHT}" {S}/>' + f'<path d="M10 38 L30 18 L44 18" stroke="{PC}" stroke-width="2.2" fill="none"/>' + \
    f'<circle cx="38" cy="10" r="5" fill="none" stroke="{PC}" stroke-width="1.6"/><line x1="33" y1="15" x2="43" y2="5" stroke="{PC}" stroke-width="1.6"/>'
I["pmi_angulo"] = f'<path d="M6 42 H44 M6 42 L36 10" stroke="{INK}" stroke-width="2.6"/>' + f'<path d="M30 42 A24 24 0 0 0 22 25" stroke="{PC}" stroke-width="2.4" fill="none"/>'
I["pmi_nota"] = box((16, 30), 12) + f'<path d="M16 26 L30 12" stroke="{PC}" stroke-width="2"/><rect x="28" y="4" width="16" height="12" rx="2" fill="#fff4e8" stroke="{PC}" stroke-width="1.6"/>' + \
    f'<path d="M31 8 H41 M31 12 H38" stroke="{PC}" stroke-width="1.4"/>'
I["pmi_rugosidade"] = f'<path d="M6 26 L14 40 L30 8 H44" stroke="{PC}" stroke-width="2.6" fill="none" stroke-linejoin="round"/>' + f'<line x1="4" y1="44" x2="44" y2="44" stroke="{INK}" stroke-width="2"/>'
I["pmi_solda"] = f'<path d="M4 30 H22 V44 H4 Z" fill="{MID}" {S}/><path d="M22 30 H44 V44 H22 Z" fill="{LIGHT}" {S}/>' + \
    f'<path d="M22 30 L12 6 H40" stroke="{PC}" stroke-width="2" fill="none"/><polygon points="26,6 32,6 32,0" fill="{PC}"/>'
I["pmi_referencia"] = f'<rect x="14" y="4" width="20" height="16" fill="#fff4e8" stroke="{PC}" stroke-width="2"/><text x="19" y="17" font-size="13" font-family="sans-serif" font-weight="bold" fill="{PC}">A</text>' + \
    f'<line x1="24" y1="20" x2="24" y2="32" stroke="{PC}" stroke-width="2"/><polygon points="16,40 32,40 24,31" fill="{PC}"/><line x1="4" y1="42" x2="44" y2="42" stroke="{INK}" stroke-width="2.4"/>'
I["pmi_tolerancia"] = f'<rect x="3" y="14" width="42" height="18" fill="#fff4e8" stroke="{PC}" stroke-width="2"/>' + \
    f'<line x1="17" y1="14" x2="17" y2="32" stroke="{PC}" stroke-width="1.6"/><line x1="35" y1="14" x2="35" y2="32" stroke="{PC}" stroke-width="1.6"/>' + \
    f'<path d="M6 27 L9 20 H15 L12 27 Z" fill="none" stroke="{PC}" stroke-width="1.4"/><path d="M20 23 H32" stroke="{PC}" stroke-width="1.6"/><text x="37" y="28" font-size="11" font-family="sans-serif" fill="{PC}">A</text>'

# ---------------- Desenho 2D
SHEET = f'<path d="M8 4 H32 L40 12 V44 H8 Z" fill="#ffffff" {S}/><path d="M32 4 V12 H40" fill="none" {S}/>'
TH = lambda x, y, t, c=ACTION, s=10: f'<text x="{x}" y="{y}" font-size="{s}" font-family="sans-serif" font-weight="bold" fill="{c}">{t}</text>'
PLUS = f'<circle cx="37" cy="37" r="9" fill="{ACTION}"/><path d="M37 32 V42 M32 37 H42" stroke="#ffffff" stroke-width="2.6" stroke-linecap="round"/>'
THIN = lambda d, c=INK, w=1.4: f'<path d="{d}" stroke="{c}" stroke-width="{w}" fill="none" stroke-linecap="round" stroke-linejoin="round"/>'
I["des_novo"] = SHEET + THIN("M13 36 H28 M13 31 H24") + PLUS
I["des_abrir"] = SHEET + f'<path d="M2 22 H14 L18 26 H34 V44 H2 Z" fill="{MID}" {S}/>' + f'<path d="M6 30 H40 L34 44 H2 Z" fill="{LIGHT}" {S}/>'
I["des_salvar"] = SHEET + f'<rect x="24" y="24" width="20" height="20" rx="2" fill="{ACTION}"/><rect x="28" y="24" width="12" height="7" fill="#ffffff"/><rect x="28" y="35" width="12" height="9" fill="{ACTION_L}"/>'
I["des_pdf"] = SHEET + f'<rect x="4" y="24" width="32" height="15" rx="2" fill="#d03a2f"/>' + TH(7, 35.5, "PDF", "#ffffff", 11)
I["des_svg"] = SHEET + f'<rect x="4" y="24" width="32" height="15" rx="2" fill="#2f9e44"/>' + TH(7, 35.5, "SVG", "#ffffff", 11)
I["des_dxf"] = SHEET + f'<rect x="4" y="24" width="32" height="15" rx="2" fill="{ACTION}"/>' + TH(7, 35.5, "DXF", "#ffffff", 11)
I["des_folha"] = f'<path d="M14 2 H36 L42 8 V36 H14 Z" fill="{ACTION_L}" {S}/>' + f'<path d="M6 10 H28 L34 16 V44 H6 Z" fill="#ffffff" {S}/>' + PLUS
I["des_tres_vistas"] = f'<rect x="4" y="4" width="18" height="12" fill="{LIGHT}" {S}/><rect x="4" y="22" width="18" height="18" fill="{LIGHT}" {S}/><rect x="28" y="4" width="10" height="12" fill="{LIGHT}" {S}/>' + \
    THIN("M13 16 V22 M22 10 H28", ACTION, 1.2) + box((38, 26), 10, w=8, hh=4)
I["des_vista"] = f'<rect x="12" y="10" width="30" height="24" fill="{LIGHT}" {S}/>' + arrow(2, 22, 11, 22) + THIN("M18 16 H36 M18 28 H30", INK, 1.2)
I["des_projetada"] = f'<rect x="6" y="4" width="22" height="14" fill="{LIGHT}" {S}/>' + f'<rect x="6" y="28" width="22" height="16" fill="{ACTION_L}" stroke="{ACTION}" stroke-width="2" stroke-dasharray="4 2"/>' + \
    f'<path d="M6 18 V28 M28 18 V28" stroke="{ACTION}" stroke-width="1.2" stroke-dasharray="2 2"/>' + arrow(38, 8, 38, 36)
I["des_iso"] = box((24, 6), 16)
I["des_corte"] = f'<rect x="6" y="10" width="36" height="22" fill="#ffffff" {S}/>' + \
    "".join(f'<line x1="{x}" y1="32" x2="{x + 10}" y2="10" stroke="{INK}" stroke-width="1.2"/>' for x in (2, 9, 16, 23, 30) ) + \
    f'<rect x="6" y="10" width="36" height="22" fill="none" {S}/>' + f'<path d="M2 40 H46" stroke="{ACTION}" stroke-width="2" stroke-dasharray="7 2 2 2"/>' + AR("4,36 8,44 0,44") + AR("44,36 48,44 40,44")
I["des_detalhe"] = f'<rect x="2" y="20" width="22" height="18" fill="{LIGHT}" {S}/><circle cx="20" cy="22" r="8" fill="none" stroke="{ACTION}" stroke-width="1.8"/>' + \
    f'<circle cx="34" cy="14" r="11" fill="#ffffff" stroke="{ACTION}" stroke-width="2.4"/><path d="M26 18 H38 V26" stroke="{INK}" stroke-width="2.4" fill="none"/>' + THIN("M27 17 L28 15", ACTION, 1)
I["des_planificada"] = f'<path d="M4 16 H14 V4 H34 V16 H44 V32 H34 V44 H14 V32 H4 Z" fill="{LIGHT}" {S}/>' + \
    f'<path d="M14 16 V32 M34 16 V32 M14 16 H34 M14 32 H34" stroke="{ACTION}" stroke-width="1.6" stroke-dasharray="3 2"/>'
# cotas
DIMC = ACTION
I["des_cota_h"] = f'<rect x="8" y="24" width="32" height="18" fill="{LIGHT}" {S}/>' + THIN("M8 6 V22 M40 6 V22", INK, 1.2) + LN(10, 12, 38, 12, DIMC, 1.8) + AR("8,12 14,9 14,15") + AR("40,12 34,9 34,15")
I["des_cota_v"] = f'<rect x="20" y="6" width="22" height="36" fill="{LIGHT}" {S}/>' + THIN("M4 6 H18 M4 42 H18", INK, 1.2) + LN(10, 8, 10, 40, DIMC, 1.8) + AR("10,6 7,12 13,12") + AR("10,42 7,36 13,36")
I["des_cota_al"] = f'<path d="M8 40 L36 12" stroke="{INK}" stroke-width="3" stroke-linecap="round"/>' + LN(4, 26, 22, 8, DIMC, 1.8) + AR("2,28 5,21 9,25") + AR("24,6 17,9 21,13") + THIN("M8 40 L3 35 M36 12 L23 3", INK, 1)
I["des_cota_r"] = f'<path d="M8 42 V22 A16 16 0 0 1 24 6 H42" fill="none" stroke="{INK}" stroke-width="3"/><circle cx="24" cy="22" r="2" fill="{INK}"/>' + LN(24, 22, 14, 12, DIMC, 1.8) + AR("12,10 19,12 14,17") + TH(26, 40, "R", DIMC, 14)
I["des_cota_d"] = f'<circle cx="22" cy="26" r="15" fill="{LIGHT}" {S}/>' + LN(12, 36, 32, 16, DIMC, 1.8) + AR("10,38 12,31 17,36") + AR("34,14 27,16 32,21") + \
    f'<circle cx="40" cy="9" r="5" fill="none" stroke="{DIMC}" stroke-width="1.8"/><line x1="35" y1="14" x2="45" y2="4" stroke="{DIMC}" stroke-width="1.8"/>'
I["des_cota_ang"] = f'<path d="M6 42 H44 M6 42 L34 8" stroke="{INK}" stroke-width="3" stroke-linecap="round"/>' + f'<path d="M34 42 A28 28 0 0 0 24 20" stroke="{DIMC}" stroke-width="1.8" fill="none"/>' + AR("34,44 31,37 37,37") + AR("23,18 30,20 26,25")
# anotações
I["des_nota"] = f'<rect x="4" y="6" width="30" height="24" rx="2" fill="#ffffff" {S}/>' + THIN("M9 13 H29 M9 19 H25 M9 25 H27", INK, 1.6) + LN(34, 26, 44, 40, ACTION, 1.8) + AR("45,42 39,38 43,35")
I["des_balao"] = f'<circle cx="18" cy="18" r="13" fill="#ffffff" stroke="{ACTION}" stroke-width="2.4"/>' + TH(13.5, 23.5, "1", ACTION, 15) + LN(27, 27, 40, 40, ACTION, 1.8) + f'<circle cx="41" cy="41" r="3" fill="{ACTION}"/>'
I["des_baloes_auto"] = "".join(f'<circle cx="{x}" cy="{y}" r="8" fill="#ffffff" stroke="{ACTION}" stroke-width="2"/>' + TH(x - 3, y + 4, n, ACTION, 11) for x, y, n in [(10, 10, "1"), (24, 24, "2"), (38, 38, "3")])
I["des_rugosidade"] = f'<path d="M4 22 L12 36 L26 6 H44" stroke="{ACTION}" stroke-width="2.8" fill="none" stroke-linejoin="round"/>' + THIN("M4 22 H18", ACTION, 2.4) + f'<line x1="2" y1="42" x2="46" y2="42" stroke="{INK}" stroke-width="2.4"/>' + THIN("M30 13 H42", INK, 1.4)
I["des_solda"] = LN(6, 12, 42, 12, INK, 2.2) + f'<polygon points="16,12 16,22 26,12" fill="{ACTION}"/>' + LN(6, 12, 18, 36, INK, 1.8) + AR("20,40 14,35 20,33") + THIN("M30 18 H40", INK, 1.4) + \
    f'<path d="M4 40 H26 V44 H4 Z" fill="{MID}"/>'
I["des_centro"] = f'<circle cx="24" cy="24" r="13" fill="{LIGHT}" {S}/>' + f'<path d="M2 24 H46 M24 2 V46" stroke="{ACTION}" stroke-width="1.8" stroke-dasharray="8 2 2 2"/>' + THIN("M20 24 H28 M24 20 V28", ACTION, 2.4)
# tabelas
GRID_T = f'<rect x="4" y="6" width="40" height="34" fill="#ffffff" {S}/><rect x="4" y="6" width="40" height="8" fill="{ACTION_L}" {S}/>' + THIN("M4 22 H44 M4 30 H44 M16 6 V40 M32 6 V40", INK, 1.2)
I["des_bom"] = GRID_T + TH(6, 13, "#", ACTION, 7) + f'<circle cx="10" cy="26" r="3" fill="none" stroke="{ACTION}" stroke-width="1.4"/>' + f'<circle cx="10" cy="35" r="3" fill="none" stroke="{ACTION}" stroke-width="1.4"/>'
I["des_tab_furos"] = GRID_T + f'<circle cx="38" cy="38" r="8" fill="{LIGHT}" {S}/><circle cx="38" cy="38" r="3.5" fill="#ffffff" stroke="{ACTION}" stroke-width="1.8"/>'
I["des_tab_dobras"] = GRID_T + f'<path d="M28 46 V36 A6 6 0 0 1 34 30 H46" fill="none" stroke="{ACTION}" stroke-width="3"/>'
I["des_tab_corte"] = GRID_T + f'<path d="M30 30 H46 V33 H40 V43 H46 V46 H30 V43 H36 V33 H30 Z" fill="{MID}" stroke="{ACTION}" stroke-width="1.4"/>'

for name, body in I.items():
    with open(os.path.join(OUT, f"{name}.svg"), "w", encoding="utf8", newline="\n") as f:
        f.write(svg(body))
print(len(I), "ícones gerados em", OUT)
