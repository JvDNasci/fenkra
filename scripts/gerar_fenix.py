"""Gera o ícone do aplicativo: fênix colorida (amarelo → laranja → vermelho) sobre fundo azul.

    python scripts/gerar_fenix.py assets/app/fenix.svg

Desenho original, gerado por código; pode ser refinado no VectorCraft/Inkscape.
Depois rode scripts/gerar_ico.py para atualizar o .ico (ver docs/prototipo.md).
"""
import math
import sys

INK = "#4a1a0c"        # contorno marrom-escuro
Y, O, R, DR = "#ffd23f", "#ffa12b", "#f0532a", "#b81f2c"
BG1, BG2 = "#4a90f0", "#2a5fd0"

out = []


def P(x, y):
    return f"{x:.1f},{y:.1f}"


def feather(bx, by, ang, length, width, fill, curl=0.0, tip=None, tip_len=0.25, veins=True, sw=2.0):
    """Pena curva: base, ângulo (graus), comprimento, largura, cor, curvatura e cor da ponta."""
    a = math.radians(ang)
    c = math.radians(curl)

    def along(t, side):
        th = a + c * t * t
        L = length * t
        x = bx + L * (math.cos(a) + math.cos(th)) / 2
        y = by + L * (math.sin(a) + math.sin(th)) / 2
        w = width * math.sin(math.pi * min(t, 0.999) ** 0.75) * (1 - 0.2 * t)
        return x - side * w / 2 * math.sin(th), y + side * w / 2 * math.cos(th)

    ts = [i / 16 for i in range(17)]
    pts = [along(t, 1) for t in ts] + [along(t, -1) for t in reversed(ts)]
    d = "M" + " L".join(P(*p) for p in pts) + " Z"
    out.append(f'<path d="{d}" fill="{fill}" stroke="{INK}" stroke-width="{sw}" stroke-linejoin="round"/>')
    out.append(f'<path d="{d}" fill="url(#hatch)"/>')
    if tip:
        t0 = 1 - tip_len
        tl = [along(t0 + (1 - t0) * i / 8, 1) for i in range(9)]
        tr = [along(t0 + (1 - t0) * i / 8, -1) for i in reversed(range(9))]
        out.append(f'<path d="M{" L".join(P(*p) for p in tl + tr)} Z" fill="{tip}" stroke="{INK}" stroke-width="{sw * 0.6}"/>')
    if veins:
        spine = [along(i / 10, 0) for i in range(1, 10)]
        out.append(f'<path d="M{" L".join(P(*p) for p in spine)}" fill="none" stroke="{INK}" stroke-width="0.9" opacity="0.65"/>')


def spiral(cx, cy, r0, turns, direction=1, start=0.0, sw=1.6, color=INK):
    pts = []
    n = int(40 * turns)
    for i in range(n + 1):
        t = i / n
        ang = start + direction * t * turns * 2 * math.pi
        r = r0 * (1 - 0.85 * t)
        pts.append((cx + r * math.cos(ang), cy + r * math.sin(ang)))
    out.append(f'<path d="M{" L".join(P(*p) for p in pts)}" fill="none" stroke="{color}" stroke-width="{sw}" stroke-linecap="round"/>')


def ribbon(points, width, eye=True):
    """Pena longa da cauda: fita ondulada com listras e ponta em 'olho'."""
    d = "M" + P(*points[0]) + " C" + " ".join(P(*p) for p in points[1:])
    out.append(f'<path d="{d}" fill="none" stroke="{INK}" stroke-width="{width + 4}" stroke-linecap="round"/>')
    out.append(f'<path d="{d}" fill="none" stroke="{O}" stroke-width="{width}" stroke-linecap="round"/>')
    out.append(f'<path d="{d}" fill="none" stroke="{Y}" stroke-width="{width * 0.45}" stroke-linecap="round"/>')
    out.append(f'<path d="{d}" fill="none" stroke="{R}" stroke-width="{width}" stroke-dasharray="2.2 4.5" opacity="0.7"/>')
    if eye:
        ex, ey = points[-1]
        out.append(f'<ellipse cx="{ex:.1f}" cy="{ey:.1f}" rx="{width * 0.95:.1f}" ry="{width * 0.7:.1f}" fill="{R}" stroke="{INK}" stroke-width="1.8"/>')
        out.append(f'<ellipse cx="{ex:.1f}" cy="{ey:.1f}" rx="{width * 0.45:.1f}" ry="{width * 0.32:.1f}" fill="{Y}" stroke="{INK}" stroke-width="1.1"/>')


# ---- cauda (fica atrás do corpo)
# plumas longas que descem e se enrolam para os dois lados
for side in (-1, 1):
    feather(128 + side * 4, 150, 90 + side * 10, 112, 22, O, curl=side * 115, tip=R, tip_len=0.3)
    feather(128 + side * 4, 150, 90 + side * 26, 88, 18, Y, curl=side * 95, tip=O, tip_len=0.35)
    feather(128 + side * 2, 152, 90 - side * 6, 100, 16, R, curl=-side * 85, tip=DR, tip_len=0.3)
feather(128, 152, 90, 72, 16, Y, curl=20, tip=R, tip_len=0.3)

# ---- asas abertas em três camadas: penas longas (vermelho), médias (laranja), cobertura (amarelo)
for side in (-1, 1):
    sx = 128 + side * 12
    long_ = [(-168, 96), (-150, 106), (-132, 110), (-114, 104), (-97, 90)]
    mid = [(-162, 72), (-144, 78), (-126, 78), (-108, 70)]
    short = [(-154, 46), (-134, 50), (-114, 44)]
    for row, (feathers, fill, tip, w) in enumerate([(long_, O, R, 24), (mid, Y, O, 22), (short, Y, None, 20)]):
        for ang, ln in feathers:
            a = ang if side < 0 else -180 - ang
            feather(sx, 116 - row * 2, a, ln, w, fill, curl=side * 30, tip=tip, tip_len=0.35)
    # espirais decorativas nas pontas das asas
    spiral(128 + side * 104, 34, 9, 1.3, direction=-side, start=math.pi / 2, sw=1.8)

# ---- corpo
body = "M113 104 C106 122 113 144 128 154 C143 144 150 122 143 104 C137 93 119 93 113 104 Z"
out.append(f'<path d="{body}" fill="url(#corpo)" stroke="{INK}" stroke-width="2.4"/>')
for row in range(4):
    y = 112 + row * 9
    for col in range(3 - (row > 2)):
        x = 117 + col * 8 + (row % 2) * 4 + (row > 2) * 4
        out.append(f'<path d="M{x},{y} q4 6 8 0" fill="none" stroke="{INK}" stroke-width="1.1" opacity="0.8"/>')

# ---- pescoço e cabeça de perfil (olhando para a direita)
neck = "M116 104 C112 88 116 72 126 62 C134 54 146 54 152 62 C156 70 150 78 144 82 C140 88 140 96 142 104 Z"
out.append(f'<path d="{neck}" fill="url(#corpo)" stroke="{INK}" stroke-width="2.4"/>')
out.append(f'<path d="M150 60 C160 60 168 66 168 72 C162 70 156 70 150 72 Z" fill="{O}" stroke="{INK}" stroke-width="2" stroke-linejoin="round"/>')
out.append(f'<circle cx="142" cy="64" r="3.4" fill="{INK}"/><circle cx="143" cy="63" r="1.1" fill="#fff"/>')
out.append(f'<path d="M136 70 q6 4 12 2" fill="none" stroke="{R}" stroke-width="2"/>')
# crista: plumas para trás terminando em espirais
for k, (ang, ln) in enumerate([(-160, 40), (-140, 48), (-120, 40), (-100, 30)]):
    feather(132, 58, ang, ln, 9, R if k % 2 else O, curl=70, tip=DR if k % 2 else R, tip_len=0.3, veins=False, sw=1.5)
spiral(104, 26, 7, 1.2, direction=1, start=0, sw=1.6)
spiral(120, 18, 6, 1.2, direction=1, start=0.5, sw=1.6)

defs = f'''<defs>
<linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{BG1}"/><stop offset="1" stop-color="{BG2}"/></linearGradient>
<radialGradient id="glow"><stop offset="0" stop-color="#ffe08a" stop-opacity="0.6"/><stop offset="0.55" stop-color="#ff9a3c" stop-opacity="0.22"/><stop offset="1" stop-color="#ff7a1a" stop-opacity="0"/></radialGradient>
<linearGradient id="corpo" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{Y}"/><stop offset="1" stop-color="{O}"/></linearGradient>
<pattern id="hatch" patternUnits="userSpaceOnUse" width="4.5" height="4.5" patternTransform="rotate(35)">
  <line x1="0" y1="0" x2="0" y2="4.5" stroke="{INK}" stroke-width="0.8" opacity="0.28"/></pattern>
</defs>'''

svg = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256">{defs}'
       f'<rect x="4" y="4" width="248" height="248" rx="44" fill="url(#bg)"/>'
       f'<circle cx="128" cy="118" r="104" fill="url(#glow)"/>'
       f'<g>{"".join(out)}</g></svg>\n')

with open(sys.argv[1], "w", encoding="utf8", newline="\n") as f:
    f.write(svg)
print("ok", sys.argv[1])
