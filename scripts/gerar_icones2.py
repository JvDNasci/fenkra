"""Gera a família nova de ícones do Fenkra em assets/icons2 e a prancha de conferência.

Linguagem (B258, B315): traço fino sem preenchimento, duas cores. O traço BASE mostra o que já
existia; o traço RES mostra o que o comando faz. Os SVGs guardam os marcadores BASE e RES no lugar
da cor: `icons.rs` troca os marcadores pelas cores do tema em tempo de execução, então nenhum
arquivo tem cor fixa. Grade de 48, com margem de 6; legível em 24 px.

Uso: python -I scripts/gerar_icones2.py
Nomes iguais aos de assets/icons. Não escreve em assets/icons. Nome sem desenho aqui continua caindo
no ícone antigo, e o script lista quais são.
"""
import math
import os
import re
import sys
import xml.etree.ElementTree as ET

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ANTIGOS = os.path.join(RAIZ, "assets", "icons")
SAIDA = os.path.join(RAIZ, "assets", "icons2")
PRANCHA = os.path.join(RAIZ, "docs", "benchmark", "detalhe", "icones.html")

# Traço em unidades da grade de 48: em 24 px, 2 vira 1 px e 2,5 vira 1,25 px. O resultado é mais
# grosso que a base para o ícone continuar legível mesmo num tema de pouco contraste entre as cores.
TRACO_BASE = 2
TRACO_RES = 2.5
PONTO = 5
TRACEJADO = "3.5 4"
CIMA = -math.pi / 2

I = {}


def n(v):
    """Número curto para o SVG."""
    s = f"{v:.1f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def ic(nome, B="", R="", b="", r="", pb=(), pr=()):
    """Registra um ícone. B e R: traço cheio. b e r: tracejado. pb e pr: pontos."""
    if nome in I:
        raise SystemExit(f"ícone repetido: {nome}")
    I[nome] = (B, R, b, r, tuple(pb), tuple(pr))


# ---------- primitivas de traço ----------

def L(*p):
    return "M" + "L".join(f"{n(x)} {n(y)}" for x, y in p)


def Z(*p):
    return L(*p) + "z"


def E(x, y, rx, ry):
    return (f"M{n(x - rx)} {n(y)}a{n(rx)} {n(ry)} 0 1 0 {n(2 * rx)} 0"
            f"a{n(rx)} {n(ry)} 0 1 0 {n(-2 * rx)} 0")


def C(x, y, r):
    return E(x, y, r, r)


def Q(x, y, w, h):
    return f"M{n(x)} {n(y)}h{n(w)}v{n(h)}h{n(-w)}z"


def pol(cx, cy, r, a):
    t = math.radians(a)
    return cx + r * math.cos(t), cy + r * math.sin(t)


def arc(cx, cy, r, a0, a1):
    """Arco de a0 a a1 em graus (0 à direita, 90 para baixo)."""
    x0, y0 = pol(cx, cy, r, a0)
    x1, y1 = pol(cx, cy, r, a1)
    grande = 1 if abs(a1 - a0) > 180 else 0
    sentido = 1 if a1 > a0 else 0
    return f"M{n(x0)} {n(y0)}A{n(r)} {n(r)} 0 {grande} {sentido} {n(x1)} {n(y1)}"


def ponta(x, y, ang, h=5.5):
    """Ponta de seta aberta em (x, y), apontando para `ang` em radianos."""
    return L(pol(x, y, h, math.degrees(ang) + 150), (x, y), pol(x, y, h, math.degrees(ang) - 150))


def S(x1, y1, x2, y2, h=5.5):
    """Seta."""
    return L((x1, y1), (x2, y2)) + ponta(x2, y2, math.atan2(y2 - y1, x2 - x1), h)


def S2(x1, y1, x2, y2, h=5.5):
    """Seta dupla."""
    return S(x1, y1, x2, y2, h) + ponta(x1, y1, math.atan2(y1 - y2, x1 - x2), h)


def SA(cx, cy, r, a0, a1, h=5.5):
    """Arco com ponta no fim."""
    x, y = pol(cx, cy, r, a1)
    return arc(cx, cy, r, a0, a1) + ponta(x, y, math.radians(a1 + (90 if a1 > a0 else -90)), h)


def SA2(cx, cy, r, a0, a1, h=5):
    """Arco com ponta nos dois lados."""
    x, y = pol(cx, cy, r, a0)
    return SA(cx, cy, r, a0, a1, h) + ponta(x, y, math.radians(a0 - (90 if a1 > a0 else -90)), h)


def amostra(fn, t0, t1, passos):
    return L(*[fn(t0 + (t1 - t0) * i / passos) for i in range(passos + 1)])


def DENTES(x, y0, y1, amp, passos):
    """Parede de rosca vertical: serrilhado de x para x + amp."""
    p = [(x, y0)]
    for i in range(1, passos + 1):
        p.append((x + (amp if i % 2 else 0), y0 + (y1 - y0) * i / passos))
    return L(*p)


# ---------- vocabulário ----------

def CUBO(x=24, y=6, w=16, d=8, h=18):
    """Caixa isométrica: contorno e as três arestas da frente."""
    return (Z((x, y), (x + w, y + d), (x + w, y + d + h), (x, y + 2 * d + h), (x - w, y + d + h), (x - w, y + d))
            + L((x - w, y + d), (x, y + 2 * d), (x + w, y + d)) + L((x, y + 2 * d), (x, y + 2 * d + h)))


def CUBO_OCULTO(x=24, y=6, w=16, d=8, h=18):
    return L((x - w, y + d + h), (x, y + h), (x + w, y + d + h)) + L((x, y), (x, y + h))


def F_TOPO(x=24, y=6, w=16, d=8):
    return Z((x, y), (x + w, y + d), (x, y + 2 * d), (x - w, y + d))


def F_ESQ(x=24, y=6, w=16, d=8, h=18):
    return Z((x - w, y + d), (x, y + 2 * d), (x, y + 2 * d + h), (x - w, y + d + h))


def F_DIR(x=24, y=6, w=16, d=8, h=18):
    return Z((x + w, y + d), (x, y + 2 * d), (x, y + 2 * d + h), (x + w, y + d + h))


def PL(y=34, x=6, w=26, h=16, s=10):
    """Plano em perspectiva: paralelogramo com a borda de baixo em y."""
    return Z((x, y), (x + s, y - h), (x + s + w, y - h), (x + w, y))


def ONDA(x=6, y=20):
    return f"M{n(x)} {n(y)}c6-6 12-6 18-2s12 4 18-2"


def PATCH(x=6, y=20):
    """Retalho de superfície: bordas onduladas, sem volume."""
    return ONDA(x, y) + "v16c-6 6-12 6-18 2s-12-4-18 2z"


# As duas metades de PATCH(6, 20), para aparar, juntar e estender.
MEIO_ESQ = "M6 20c6-6 12-6 18-2v16c-6-4-12-4-18 2z"
MEIO_DIR = "M24 18c6 4 12 4 18-2v16c-6 6-12 6-18 2"

CURVA = "M6 36C14 8 30 40 42 12"
PASTA = "M6 38V10h13l4 5h19v23z"
OLHO = "M4 24c8-13 32-13 40 0c-8 13-32 13-40 0z"
GUIA = "M6 38c10 2 18-6 20-14s8-14 16-14"


def MAIS(x=37, y=37, s=6):
    return f"M{n(x - s)} {n(y)}h{n(2 * s)}M{n(x)} {n(y - s)}v{n(2 * s)}"


def XIS(x=37, y=37, s=5):
    return L((x - s, y - s), (x + s, y + s)) + L((x + s, y - s), (x - s, y + s))


def CHECK(x=37, y=37, s=6):
    return L((x - s, y), (x - s / 3, y + s * .7), (x + s, y - s * .8))


def FAISCA(x=37, y=37, s=7):
    """Selo de automático: estrela de quatro pontas."""
    k = s * .28
    return Z((x, y - s), (x + k, y - k), (x + s, y), (x + k, y + k), (x, y + s), (x - k, y + k), (x - s, y), (x - k, y - k))


def CADEADO(x=37, y=37):
    return Q(x - 7, y - 2, 14, 10) + f"M{n(x - 4)} {n(y - 2)}v-3a4 4 0 0 1 8 0v3"


def ENGRENAGEM(x, y, ro, ri, dentes):
    p = []
    q = 360 / dentes
    for i in range(dentes):
        for da, r in ((-.3, ri), (-.17, ro), (.17, ro), (.3, ri)):
            p.append(pol(x, y, r, q * (i + da)))
    return Z(*p)


def CURSOR(x, y, k=1.0):
    p = [(0, 0), (0, 22), (6, 16.5), (10, 25), (14, 23), (10, 15), (17, 15)]
    return Z(*[(x + a * k, y + b * k) for a, b in p])


def PAGINA(x=10, y=6, w=28, h=36):
    return f"M{n(x)} {n(y)}h{n(w - 8)}l8 8v{n(h - 8)}h{n(-w)}zM{n(x + w - 8)} {n(y)}v8h8"


def FOLHA(x=6, y=8, w=36, h=32):
    """Folha de desenho com carimbo."""
    return Q(x, y, w, h) + f"M{n(x + w - 15)} {n(y + h)}v-8h15"


def TABELA(x=6, y=8, w=36, h=32, lin=3, col=2):
    s = Q(x, y, w, h)
    for i in range(1, lin):
        s += f"M{n(x)} {n(y + h * i / lin)}h{n(w)}"
    for i in range(1, col):
        s += f"M{n(x + w * i / col)} {n(y)}v{n(h)}"
    return s


def desenhos():
    # ---------- arquivo e aplicativo ----------
    ic("pasta", B=PASTA, R=MAIS(24, 27, 7))
    ic("abrir_projeto", B="M8 38V10h11l4 5h15v7", R="M8 38l7-16h29l-7 16z")
    ic("novo_projeto", B=PASTA, R=CUBO(24, 17, 9, 4.5, 8))
    ic("nova_peca", B=CUBO(20, 7, 14, 7, 14), R=MAIS(37, 37, 6))
    ic("salvar", B="M8 8h25l7 7v25H8z", R="M16 8v10h14V8M14 40V26h20v14")
    ic("importar_arquivo", B=CUBO(24, 20, 12, 6, 10), R="M24 5v12M18 11l6 6 6-6")
    ic("exportar_arquivo", B=CUBO(24, 20, 12, 6, 10), R="M24 17V5M18 11l6-6 6 6")
    ic("conversao_lote", B=Q(6, 6, 18, 24) + "M12 30v8h18V34", R="M30 18h12M37 13l5 5-5 5")
    ic("formatos_suportados", B=Q(6, 8, 36, 32) + "M6 19h36M6 30h36M19 8v32", R=CHECK(30, 24, 4))
    ic("liberar_revisao", B=Q(8, 6, 24, 34), R=CADEADO(34, 34))
    ic("onde_usado", B=CUBO(20, 6, 12, 6, 12), R=C(32, 32, 7) + "M37 37l6 6")
    ic("curso", B=Z((6, 12), (24, 18), (42, 12), (42, 34), (24, 40), (6, 34)) + "M24 18V40", R=CHECK(34, 8, 4))
    ic("mapa_macro", B=Q(6, 6, 14, 10) + Q(28, 6, 14, 10) + Q(17, 32, 14, 10), R="M20 11h8M13 16v8h11v8M35 16v8H24")
    ic("perfil_fluxo", B=C(20, 16, 7) + "M6 42c0-9 6-14 14-14s14 5 14 14", R="M30 12h12M38 8l4 4-4 4")
    ic("comparar", B=Q(6, 10, 15, 28) + Q(27, 10, 15, 28), R=S2(15, 24, 33, 24, 4))
    ic("estado_sem_revisao", B=Q(8, 6, 24, 34), R="M28 36h14")
    ic("estado_liberada", B=Q(8, 6, 24, 34), R=CHECK(34, 34, 8))
    ic("estado_editado", B=Q(8, 6, 24, 34), R="M26 42l2-7 11-11 5 5-11 11z")
    ic("estado_leitura", B=Q(8, 6, 24, 34), R=E(33, 34, 10, 6) + C(33, 34, 2))
    ic("estado_conflito", B=Q(8, 6, 24, 34), R="M33 22l11 20H22zM33 30v6M33 39v1")
    ic("config", B=ENGRENAGEM(24, 24, 18, 13.5, 8), R=C(24, 24, 6))
    ic("personalizar", B="M8 13h32M8 24h32M8 35h32", R=C(30, 13, 4) + C(16, 24, 4) + C(26, 35, 4))
    ic("parametro", B="M6 36h36M6 32v8M15 34v4M24 34v4M33 34v4M42 32v8", R="M30 28l-5-8h10zM30 20V9")
    ic("macro", B=PAGINA() + "M16 22h8M16 29h6", R=Z((24, 25), (24, 39), (36, 32)))
    ic("idioma", B=C(24, 24, 18), R=E(24, 24, 8, 18) + "M6 24h36")
    ic("navegacao", B=C(24, 24, 8), R=SA(24, 24, 17, 200, 340) + SA(24, 24, 17, 20, 160))
    ic("aparencia", B=CUBO(20, 13, 13, 6.5, 13), R="M36 6c-4 6-6 8-6 11a6 6 0 0 0 12 0c0-3-2-5-6-11z")
    ic("filtro", B="M6 9h36L30 24H18z", R="M18 24v13l12 5V24")
    ic("estilo", B=CUBO(), R=L((24, 26), (32, 22)) + L((24, 33), (40, 25)) + L((24, 40), (40, 32)))
    ic("exemplo", B="M24 13c-5-4-11-5-18-4v28c7-1 13 0 18 4 5-4 11-5 18-4V9c-7-1-13 0-18 4z", R="M24 13v28")
    ic("exibicao", B=F_TOPO(24, 24, 16, 8) + L((8, 32), (8, 37), (24, 45), (40, 37), (40, 32)),
       R=F_TOPO(24, 3, 16, 8) + L((8, 11), (8, 16), (24, 24), (40, 16), (40, 11)))
    ic("coletor", B="M6 22v18h36V22", R=Q(10, 8, 11, 11) + Q(26, 12, 11, 11) + S(24, 26, 24, 36, 5))
    ic("conexao", B="M6 12h12M30 36h12", R="M18 12c14 0-2 24 12 24", pr=[(18, 12), (30, 36)])
    ic("ortho", B=Q(26, 12, 16, 24), R=S(6, 14, 22, 14) + S(6, 24, 22, 24) + S(6, 34, 22, 34))
    ic("ajustar", B="M6 16V6h10M32 6h10v10M42 32v10H32M16 42H6V32", R=CUBO(24, 12, 10, 5, 10))
    ic("mostrar", B=OLHO, R=C(24, 24, 6))
    ic("ocultar", B=OLHO, R=L((8, 40), (40, 8)))
    ic("ocultos", b=CUBO(), R="M12 24c6-8 18-8 24 0c-6 8-18 8-24 0z" + C(24, 24, 2.5))
    ic("an_zebra", B=C(24, 24, 18), R="M10 13c8 4 20 4 28 0M6.5 23c10 6 25 6 35 0M9 34c9 4 21 4 30 0")
    ic("an_curvatura", B="M6 38c10-16 26-22 36-22",
       R=L((12, 30), (7, 20)) + L((20, 24), (15, 11)) + L((28, 19), (24, 7)) + L((36, 17), (35, 9))
       + "M7 20c2-4 5-7 8-9s6-3 9-4 7-1 11 2")
    ic("an_sombreado", B=C(24, 24, 18), R="M30 7a18 18 0 0 1 0 34a24 24 0 0 0 0-34z")
    ic("an_inclinacao", B="M6 40h36", R=L((13, 40), (18, 10)) + L((18, 10), (30, 10)) + L((30, 10), (35, 40)))
    ic("an_isofotas", B=C(24, 24, 18), R="M12 28a12 12 0 0 1 24 0M17 28a7 7 0 0 1 14 0M22 28a2 2 0 0 1 4 0M10 18c4-5 24-5 28 0")
    ic("an_dfm", B=Q(8, 12, 32, 24), R=L((15, 25), (21, 31), (33, 17)) + L((8, 40), (40, 40)))
    ic("an_conexao", B="M6 30c6-10 12-10 16-10M42 30c-6-10-12-10-16-10", R="M24 10v28M18 14l6-4 6 4M18 34l6 4 6-4")
    ic("an_desvio", B="M6 34c10-12 26-12 36 0", R="M6 22c10-12 26-12 36 0M14 17v9M24 14v9M34 17v9")
    ic("an_pente", B="M6 38c8-14 26-20 36-20", R="M10 32l-5-7M17 26l-4-9M25 22l-2-12M33 19l1-9M41 18l3-5")
    ic("canto", B="M24 26v18M24 26L8 16M24 26l16-10", R="M16 21c4-5 12-5 16 0c0 6-3 10-8 14c-5-4-8-8-8-14z")

    # ---------- sketch ----------
    ic("sketch", B=PL(40, 4, 28, 18, 12), R="M26 28l14-16 5 4-14 16-7 2zM36 16l5 4")
    ic("sk_concluir", B=PL(40, 4, 28, 18, 12), R=CHECK(24, 16, 12))
    ic("sk_selecionar", B="M30 40c4-3 8-6 12-12", R=CURSOR(12, 8, 1.2), pb=[(42, 28)])
    ic("sk_linha", R=L((10, 38), (38, 10)), pb=[(10, 38), (38, 10)])
    ic("sk_polilinha", R=L((8, 36), (18, 14), (30, 30), (40, 10)), pb=[(8, 36), (18, 14), (30, 30), (40, 10)])
    ic("sk_ponto", B="M24 8v9M24 31v9M8 24h9M31 24h9", pr=[(24, 24)])
    ic("sk_retangulo", R=Q(8, 12, 32, 24), b=L((8, 36), (40, 12)), pb=[(8, 36), (40, 12)])
    ic("sk_retangulo_centro", R=Q(8, 12, 32, 24), b=L((24, 24), (40, 12)), pb=[(24, 24), (40, 12)])
    ic("sk_circulo", R=C(24, 24, 17), B=L((24, 24), pol(24, 24, 17, -40)), pb=[(24, 24)])
    ic("sk_circulo3", R=C(24, 24, 17), pb=[pol(24, 24, 17, -120), pol(24, 24, 17, 0), pol(24, 24, 17, 120)])
    ic("sk_arco3", R=arc(24, 32, 18, 180, 360), pb=[(6, 32), (24, 14), (42, 32)])
    ic("sk_arco_centro", R=arc(14, 34, 24, -80, -5),
       B=L((14, 34), pol(14, 34, 24, -80)) + L((14, 34), pol(14, 34, 24, -5)), pb=[(14, 34)])
    ic("sk_poligono", R=Z(*[pol(24, 24, 18, 60 * i) for i in range(6)]), B=L((24, 24), pol(24, 24, 18, 0)), pb=[(24, 24)])
    ic("sk_spline", R="M6 34C12 12 22 12 26 24S38 36 42 14", pb=[(6, 34), (17, 16), (26, 24), (35, 31), (42, 14)])
    ic("sk_rasgo", R="M16 14h16a10 10 0 0 1 0 20H16a10 10 0 0 1 0-20z", B=L((16, 24), (32, 24)), pb=[(16, 24), (32, 24)])
    ic("sk_aparar", B="M16 7v34M32 7v34", R="M6 24h10M32 24h10" + XIS(24, 24, 4.5))
    ic("sk_estender", B="M38 7v34", R=L((8, 24), (20, 24)), r=L((20, 24), (37, 24)), pr=[(20, 24)])
    ic("sk_espelhar", b="M24 6v36", B=L((20, 12), (8, 18), (14, 36)), R=L((28, 12), (40, 18), (34, 36)), pb=[(8, 18)], pr=[(40, 18)])
    ic("sk_offset", B="M8 40c0-18 14-32 32-32", R="M19 40c0-12 9-21 21-21")
    ic("sk_mover", b=Q(6, 26, 16, 16), R=Q(26, 6, 16, 16) + S(16, 24, 24, 16, 5))
    ic("sk_copiar", B=Q(6, 6, 22, 22), R=Q(18, 18, 24, 24))
    ic("sk_girar", B=Q(12, 22, 18, 18), R=Z(*[pol(21, 31, 13, a) for a in (-100, -10, 80, 170)]) + SA(21, 31, 20, -80, -5, 5))
    ic("sk_grade", B="M6 16h36M6 32h36M16 6v36M32 6v36", R=Q(16, 16, 16, 16))
    ic("sk_snap", B="M8 42h32", R="M12 6v14a12 12 0 0 0 24 0V6h-7v14a5 5 0 0 1-10 0V6zM12 13h7M29 13h7", pb=[(24, 42)])
    ic("sk_cota", B="M10 36V18M38 36V18", R=S2(10, 24, 38, 24, 4.5))
    ic("sk_auto_cota", B=Q(6, 20, 24, 20), R=S2(6, 11, 30, 11, 4.5) + FAISCA(38, 34, 8))
    ic("sk_auto_restricao", B=Q(6, 12, 24, 26), R="M12 32V20M12 32h12" + FAISCA(38, 34, 8))
    ic("sk_checar", B="M26 8H8v24h10", R=C(30, 26, 10) + L((37, 33), (43, 40)))
    ic("sk_construcao", B="M8 8v32h32", r=C(26, 22, 13) + L((8, 40), (40, 8)))
    ic("sk_converter", B="M8 12h32", r="M8 36h32", R=S2(24, 18, 24, 30, 4.5))
    ic("sk_projetar", B=F_TOPO(24, 4, 14, 7), R=PL(44, 6, 26, 12, 10) + S(24, 21, 24, 34, 4.5))
    ic("sk_intersecao", B=CUBO(24, 5, 13, 6.5, 26), R=Z((4, 34), (14, 24), (44, 24), (34, 34)))

    # restrições do sketch
    ic("r_coincidente", B=L((6, 40), (24, 24)) + L((24, 24), (42, 34)), R=C(24, 24, 7))
    ic("r_concentrica", B=C(24, 24, 18), R=C(24, 24, 9), pr=[(24, 24)])
    ic("r_fixa", B=L((6, 40), (18, 26)), pb=[(18, 26)], R=Q(22, 20, 20, 16) + "M26 20v-5a6 6 0 0 1 12 0v5")
    ic("r_horizontal", b=L((8, 38), (40, 12)), R="M6 25h36", pr=[(6, 25), (42, 25)])
    ic("r_vertical", b=L((10, 40), (38, 8)), R="M24 6v36", pr=[(24, 6), (24, 42)])
    ic("r_igual", B="M6 10h36M6 38h36", R="M14 20h20M14 28h20")
    ic("r_paralela", B=L((6, 32), (30, 8)), R=L((18, 40), (42, 16)))
    ic("r_perpendicular", B="M6 40h36", R="M22 40V8M22 30h10v10")
    ic("r_ponto_medio", B="M6 24h36M6 18v12M42 18v12", pr=[(24, 24)], R="M24 12v5M24 31v5")
    ic("r_simetrica", b="M24 6v36", B="M6 24h36", R="M12 18v12M36 18v12", pr=[(12, 24), (36, 24)])
    ic("r_tangente", B=C(20, 28, 14), R="M4 14h40", pr=[(20, 14)])

    # cotas do sketch
    ic("cota_horizontal", B="M8 38V16M40 38V16", R=S2(8, 24, 40, 24, 4.5))
    ic("cota_vertical", B="M36 8H14M36 40H14", R=S2(24, 8, 24, 40, 4.5))
    ic("cota_angulo", B=L((42, 40), (8, 40), (34, 14)), R=SA2(8, 40, 24, -44, -2, 4.5))
    ic("cota_raio", B=C(24, 24, 18), R=S(24, 24, *pol(24, 24, 18, -40)), pr=[(24, 24)])
    ic("cota_diametro", B=C(24, 24, 18), R=S2(*pol(24, 24, 18, 140), *pol(24, 24, 18, -40)))

    # ---------- primitivas: o tracejado é o lado oculto ----------
    ic("prim_caixa", R=CUBO(), b=CUBO_OCULTO())
    ic("prim_cilindro", R=E(24, 12, 15, 6) + "M9 12v24a15 6 0 0 0 30 0V12", b="M9 36a15 6 0 0 1 30 0")
    ic("prim_cone", R="M24 6L8 36a16 6 0 0 0 32 0z", b="M8 36a16 6 0 0 1 32 0")
    ic("prim_cunha", R=Z((6, 32), (24, 41), (42, 32), (42, 12), (24, 3)) + L((24, 41), (42, 12)),
       b=L((6, 32), (24, 23), (42, 32)) + L((24, 23), (24, 3)))
    ic("prim_esfera", R=C(24, 24, 18), b="M6 24a18 7 0 0 0 36 0")
    ic("prim_prisma", R=Z((14, 8), (34, 8), (42, 14), (42, 34), (34, 40), (14, 40), (6, 34), (6, 14))
       + L((6, 14), (14, 20), (34, 20), (42, 14)) + L((14, 20), (14, 40)) + L((34, 20), (34, 40)),
       b=L((6, 34), (14, 28), (34, 28), (42, 34)))
    ic("prim_toro", R=E(24, 24, 20, 12), b=E(24, 23, 8, 4))

    # ---------- sólidos ----------
    ic("extrusao", B="M10 34l14-7 14 7-14 7z", R="M10 34V20l14-7 14 7v14M10 20l14 7 14-7M24 27v14")
    ic("filete", B="M8 40V24M24 8h16", R="M8 24A16 16 0 0 1 24 8")
    ic("chanfro", B="M8 40V24M24 8h16", R="M8 24L24 8")
    ic("furo", B="M8 20l16-8 16 8-16 8zM8 20v12l16 8 16-8V20", R="M16 20a8 4 0 1 0 16 0a8 4 0 1 0-16 0M16 20v10M32 20v10")
    ic("casca", B="M8 12h32v28H8z", R="M14 12v22h20V12")
    ic("espelho", B="M20 12L8 36h12zM24 6v36", R="M28 12l12 24H28z")

    ic("recorte", B=CUBO(24, 10, 18, 9, 16), R=F_TOPO(24, 14, 9, 4.5) + L((15, 18.5), (15, 25), (24, 29.5), (33, 25), (33, 18.5)))
    ic("revolucao", b="M24 6v36", B=L((24, 12), (34, 12), (34, 20), (29, 24), (29, 36), (24, 36)), R=E(24, 24, 18, 7) + ponta(6, 24, CIMA, 5))
    ic("nervura", B="M10 36c6-16 18-20 32-21", R=Q(4, 30, 12, 12) + "M16 30c6-14 14-20 26-22M16 42c8-8 18-16 26-20M42 8v14")
    ic("rasgo_varrido", B=CUBO(24, 10, 18, 9, 16), R="M13 19c6-5 10 3 22-1v5c-12 4-16-4-22 1z")
    ic("ranhura", B=E(24, 11, 12, 5) + "M12 11v26a12 5 0 0 0 24 0V11", R="M12 20a12 5 0 0 0 24 0M12 31a12 5 0 0 0 24 0M12 20l4 3v5l-4 3M36 20l-4 3v5l4 3")
    ic("reforco", B="M8 8v32h32", R=L((8, 16), (32, 40)) + L((8, 22), (26, 40)))
    ic("multisecoes", B=Q(6, 26, 14, 14), R=C(34, 15, 8) + L((6, 26), (28, 10)) + L((20, 40), (40, 21)))
    ic("multisecoes_remover", B=Q(6, 6, 36, 36), R=Q(12, 26, 10, 10) + C(32, 16, 6) + L((12, 26), (28, 12)) + L((22, 36), (37, 20)))
    ic("saida", b="M24 40V8", B="M6 40h36", R=L((24, 40), (34, 8)) + SA(24, 40, 28, -90, -75, 3.5))
    ic("espessura", B=Q(8, 24, 32, 16), R="M8 24V10h32v14" + S2(24, 12, 24, 22, 3.5))
    # edição direta de faces (B33)
    ic("edit_mover_face", B=CUBO(18, 14, 12, 6, 12), R=F_DIR(30, 8, 8, 4, 8) + S(32, 30, 44, 22, 5))
    ic("edit_apagar_face", B=CUBO(24, 8, 16, 8, 18), R=F_DIR(24, 8, 16, 8, 18) + XIS(33, 28, 4.5))
    ic("edit_deslocar_face", B=Q(6, 14, 22, 24), R="M38 14v24" + S2(28, 26, 38, 26, 4.5))
    ic("edit_raio_filete", b="M8 40V8h32", B="M8 40V24M24 8h16", R="M8 24A16 16 0 0 1 24 8" + L((24, 24), pol(24, 24, 16, -135)), pr=[(24, 24)])
    ic("escala", B=CUBO(16, 22, 10, 5, 10), R=L((8, 24), (8, 14), (24, 6), (40, 14), (40, 32), (28, 38)) + S(22, 26, 36, 18, 5))
    ic("extrair", B=CUBO(18, 16, 13, 6.5, 14), R=S(24, 18, 37, 10, 5) + Z((30, 22), (42, 16), (42, 28), (30, 34)))
    ic("rosca", B="M16 6v36M32 6v36", R="M16 12l16 4M16 20l16 4M16 28l16 4M16 36l16 4")
    ic("padrao_linear", B=C(11, 37, 5), R=C(11, 11, 5) + C(37, 11, 5) + C(37, 37, 5) + C(24, 11, 5) + C(24, 37, 5))
    ic("padrao_circular", B=C(24, 8, 4.5), R="".join(C(*pol(24, 24, 16, a), 4.5) for a in (-30, 30, 90, 150, 210)), pb=[(24, 24)])
    ic("padrao_pontos", pb=[(10, 14), (34, 10), (22, 26), (12, 38), (38, 34)],
       R=C(10, 14, 6) + C(34, 10, 6) + C(22, 26, 6) + C(12, 38, 6) + C(38, 34, 6))

    # filetes
    ic("filete_completo", B="M12 42V20M36 42V20M4 42h40", R="M12 20a12 12 0 0 1 24 0")
    ic("filete_face_face", B="M6 8l14 22M42 8L28 30", R="M20 30a7 7 0 0 0 8 0")
    ic("filete_variavel", B="M6 14h28", R="M6 14l28 20M34 14a10 10 0 0 1 0 20")
    ic("filete_assimetrico", B="M6 40V8M6 40h38", R="M6 14a16 26 0 0 0 16 26")
    ic("arestas_selecionar", B=CUBO(), R=L((8, 14), (24, 22)))
    ic("arestas_todas", b=CUBO_OCULTO(), R=CUBO())
    ic("arestas_comando", B=CUBO(21, 8, 15, 7.5, 16), R=L((6, 15.5), (21, 23)) + CURSOR(30, 22, .85))
    ic("prop_minima", B="M26 34c10 0 10-20 18-20", R="M6 34h20M26 28v12")
    ic("prop_tangente", B="M6 42h36", R="M6 34h20c10 0 10-20 18-20", pr=[(26, 34)])
    ic("sk_filete", B="M10 42V26M26 10h16", R="M10 26A16 16 0 0 1 26 10", pr=[(10, 26), (26, 10)])
    ic("sk_chanfro", B="M10 42V26M26 10h16", R="M10 26L26 10", pr=[(10, 26), (26, 10)])

    # chanfros
    ic("chanfro_dist", B="M8 18v12M18 8h12M40 18v12M18 40h12", R=L((8, 18), (18, 8)) + L((30, 8), (40, 18)) + L((40, 30), (30, 40)) + L((18, 40), (8, 30)))
    ic("chanfro_dist_dist", B="M14 42V30M28 14h14", b=L((14, 30), (14, 14), (28, 14)),
       R=L((14, 30), (28, 14)) + "M6 14v16M4 14h4M4 30h4M14 6h14M14 4v4M28 4v4")
    ic("chanfro_dist_angulo", B="M14 42V30M30 14h12", b=L((14, 30), (14, 14), (30, 14)),
       R=L((14, 30), (30, 14)) + "M14 6h16M14 4v4M30 4v4" + SA(30, 14, 11, 135, 180, 3.5))

    # furos: corte da peça, porque só o perfil separa os tipos em 24 px
    ic("furo_simples", B="M6 12h12M30 12h12M6 38h12M30 38h12", b="M24 6v36", R="M18 12v26M30 12v26")
    ic("furo_passante", B="M6 16h12M30 16h12M6 34h12M30 34h12", R="M18 16v18M30 16v18" + S(24, 5, 24, 44, 6))
    ic("furo_cego", B="M6 12h12M30 12h12M6 40h36", R="M18 12v16l6 6 6-6V12")
    ic("furo_rebaixado", B="M6 12h7M35 12h7M6 40h13M29 40h13", R="M13 12v10h6v18M35 12v10h-6v18")
    ic("furo_escareado", B="M6 12h6M36 12h6M6 40h13M29 40h13", R="M12 12l7 10v18M36 12l-7 10v18")
    ic("furo_contrafurado", B="M6 12h7M35 12h7M6 40h13M29 40h13", R="M13 12v8l6 6v14M35 12v8l-6 6v14")
    ic("furo_roscado", B="M6 12h12M30 12h12M6 38h12M30 38h12", R=DENTES(18, 12, 38, 4, 8) + DENTES(30, 12, 38, -4, 8))
    ic("asm_furo_pecas", B="M6 12h12M30 12h12M6 25h12M30 25h12M6 38h12M30 38h12", R="M18 12v26M30 12v26" + S(24, 5, 24, 44, 5))
    ic("sm_furo_flangeado", B="M6 16h8M34 16h8", R="M14 16a4 4 0 0 1 4 4v20M34 16a4 4 0 0 0-4 4v20")

    # extensão: vista de lado, com o perfil embaixo e o alvo em cima
    ic("ext_cego", B="M10 40h22", R="M10 40V14h22v26" + S2(40, 14, 40, 40, 4.5))
    ic("ext_passante", B=Q(6, 16, 36, 14), R="M16 42V6M32 42V6" + ponta(16, 6, CIMA, 5) + ponta(32, 6, CIMA, 5))
    ic("ext_ate_plano", B="M14 42h20" + PL(16, 4, 30, 10, 10), R="M14 42V16M34 42V16")
    ic("ext_ate_face", B="M14 42h20M6 24h36M6 10h36", R="M14 42V24M34 42V24")
    ic("ext_ate_ultimo", B="M14 42h20M6 24h36M6 10h36", R="M14 42V10M34 42V10")
    ic("ext_ate_superficie", B="M14 42h20M5 16c10-12 24 8 38-6", R="M14 42V14M34 42V13")
    ic("ext_dois_sentidos", B="M6 30h36", R=Q(14, 8, 20, 30) + S(24, 26, 24, 13, 4.5))
    ic("ext_simetrico", B="M6 24h36", R=Q(14, 8, 20, 32) + S(24, 20, 24, 12, 4) + S(24, 28, 24, 36, 4))
    ic("ext_saida_filete", B="M8 42h32", R="M12 42l4-24a8 8 0 0 1 16 0l4 24")

    # booleanas: dois círculos de raio 12 que se cortam em (24; 13,6) e (24; 34,4)
    ic("bool_unir", b="M24 13.6a12 12 0 0 1 0 20.8a12 12 0 0 1 0-20.8", R="M24 13.6A12 12 0 1 0 24 34.4A12 12 0 1 0 24 13.6z")
    ic("bool_subtrair", b="M24 13.6A12 12 0 1 1 24 34.4", R="M24 13.6A12 12 0 1 0 24 34.4A12 12 0 0 0 24 13.6z")
    ic("bool_intersecao", B="M24 13.6A12 12 0 1 0 24 34.4M24 13.6A12 12 0 1 1 24 34.4", R="M24 13.6A12 12 0 0 1 24 34.4A12 12 0 0 1 24 13.6z")
    ic("op_adicionar", B=Q(6, 22, 26, 20), R=L((12, 22), (12, 8), (26, 8), (26, 22)) + MAIS(38, 14, 5))
    ic("op_remover", B=L((14, 12), (6, 12), (6, 42), (42, 42), (42, 12), (30, 12)), R=L((14, 12), (14, 28), (30, 28), (30, 12)))
    ic("op_intersecao", B=Q(6, 6, 24, 24) + Q(18, 18, 24, 24), R=Q(18, 18, 12, 12))
    ic("op_unir", B=Q(6, 28, 36, 14), R=ONDA(6, 16) + "M6 16v12M42 16v12")

    # ---------- planos: o que define é base, o plano é resultado ----------
    ic("plano", B="M6 42V30M6 42h14", R=PL(32, 6, 26, 18, 10))
    ic("plano_3pontos", R=PL(38, 6, 26, 26, 10), pb=[(16, 30), (28, 20), (34, 30)])
    ic("plano_2linhas", R=PL(38, 6, 26, 26, 10), B=L((14, 32), (24, 16)) + L((26, 32), (36, 16)))
    ic("plano_ponto_linha", R=PL(38, 6, 26, 26, 10), B=L((14, 32), (24, 16)), pb=[(32, 26)])
    ic("plano_ponto_normal", R=PL(40, 6, 26, 16, 10), B=S(24, 32, 24, 8, 5), pb=[(24, 32)])
    ic("plano_deslocado", B=PL(42, 6, 26, 12, 10), R=PL(20, 6, 26, 12, 10) + S2(24, 23, 24, 29, 3))
    ic("plano_paralelo_ponto", B=PL(42, 6, 26, 12, 10), R=PL(20, 6, 26, 12, 10), pb=[(24, 14)])
    ic("plano_angulo", B=PL(40, 4, 28, 14, 10), R=Z((10, 34), (20, 8), (32, 8), (22, 34)) + SA(22, 34, 14, -70, -8, 4))
    ic("plano_curva", R=PL(40, 4, 30, 32, 10), B="M12 32c4-12 10-14 14-8s8 2 8-8")
    ic("plano_normal_curva", B="M4 36C14 34 30 30 44 12", R=Z((20, 40), (20, 16), (32, 8), (32, 32)), pr=[(26, 25)])
    ic("plano_tangente", B=arc(24, 42, 16, 180, 360), R=PL(26, 4, 30, 12, 10), pr=[(24, 26)])
    ic("plano_medio", pb=[(11, 18), (22, 12), (35, 22), (14, 36), (33, 37), (40, 12)], R=PL(34, 6, 26, 16, 10))
    ic("plano_meio", B=PL(44, 6, 26, 9, 10) + PL(13, 6, 26, 9, 10), R=PL(29, 6, 26, 9, 10))
    ic("plano_equacao", B=S(14, 36, 14, 6, 4.5) + S(14, 36, 42, 36, 4.5) + S(14, 36, 4, 44, 4.5), R=Z((14, 14), (36, 36), (8, 40.8)))

    # linhas
    ic("linha_pontos", R=L((6, 40), (42, 8)), pb=[(14, 33), (34, 15)])
    ic("linha_direcao", B=S(8, 14, 24, 14, 5), R=L((14, 36), (42, 36)), pr=[(14, 36)])
    ic("linha_angulo", B="M6 36c10-3 26-3 36 0", R=L((20, 34), (36, 8)) + arc(20, 34, 12, -58, -5), pr=[(20, 34)])
    ic("linha_bissetriz", B=L((42, 8), (8, 24), (42, 40)), R=L((8, 24), (44, 24)))
    ic("linha_eixo", B=E(24, 13, 11, 4.5) + "M13 13v22a11 4.5 0 0 0 22 0V13", R="M24 4v40")
    ic("linha_normal", B=ONDA(6, 38), R=S(24, 36, 24, 8, 5.5), pr=[(24, 36)])
    ic("linha_planos", B=Z((4, 38), (14, 14), (26, 14), (26, 38)) + Z((26, 14), (44, 8), (44, 32), (26, 38)), R="M26 6v38")
    ic("linha_tangente", B="M8 40C8 20 22 8 42 8", R=L((4, 28), (28, 4)), pr=[(16, 16)])

    # pontos
    ic("ponto_centro", B=C(24, 24, 18), R=MAIS(24, 24, 7))
    ic("ponto_coord", B=S(10, 40, 10, 6, 4.5) + S(10, 40, 44, 40, 4.5), r="M10 18h20M30 18v22", pr=[(30, 18)])
    ic("ponto_curva", B=CURVA, pr=[(24, 24)])
    ic("ponto_deslocado", pb=[(10, 38)], R=S(13, 35, 31, 17, 5), pr=[(37, 11)])
    ic("ponto_entre", B="M8 24h32", pb=[(8, 24), (40, 24)], pr=[(24, 24)], R="M24 14v4M24 30v4")
    ic("ponto_intersecao", B=PL(40, 4, 30, 18, 10), R="M12 8c4 10 8 16 14 22", pr=[(26, 30)], r="M26 30l8 8")
    ic("ponto_intersecao_curvas", B="M6 12c14 2 22 16 36 24M6 36c14-2 22-16 36-24", pr=[(24, 24)], R=C(24, 24, 7))
    ic("ponto_plano", B=PL(40, 4, 30, 28, 10), r=L((10, 28), (24, 28)) + L((24, 28), (28, 16)), pr=[(24, 28)])
    ic("ponto_proj_curva", B="M6 38c12-12 24-12 36 0", R=S(24, 10, 24, 26, 5), pb=[(24, 10)], pr=[(24, 29)])
    ic("ponto_projetado", B=PATCH(6, 24), R=S(24, 6, 24, 25, 5), pb=[(24, 6)], pr=[(24, 29)])
    ic("ponto_superficie", B=ONDA(6, 36), R=S(13, 31, 10, 18, 4.5) + S(24, 34, 24, 21, 4.5) + S(35, 36, 38, 23, 4.5), pr=[(13, 31), (24, 34), (35, 36)])
    ic("ponto_tangente", B="M6 40c8-30 28-30 36 0", R="M12 17h24", pr=[(24, 17)])

    # ---------- curvas ----------
    ic("circulo3d", B=PL(44, 4, 30, 34, 10), R=E(24, 27, 12, 9), pr=[(24, 27)])
    ic("circulo3p", B=PL(44, 4, 30, 34, 10), R=E(24, 27, 12, 9), pb=[(12, 27), (31, 19.8), (30, 35)])
    ic("conica", B="M6 40h36M24 44V6", R="M8 8c4 30 28 30 32 0")
    ic("espiral", R=amostra(lambda t: pol(24, 24, 3 + t * 15 / 900, t), 0, 900, 60), pb=[(24, 24)])
    ic("helice", B="M24 4v40", R=amostra(lambda t: (24 + 14 * math.cos(t), 40 - t * 32 / (6 * math.pi) + 3 * math.sin(t)), 0, 6 * math.pi, 60))
    ic("polilinha3d", b=F_TOPO(24, 22, 20, 10), R=L((8, 32), (22, 12), (30, 26), (40, 8)), pr=[(8, 32), (22, 12), (30, 26), (40, 8)])
    ic("spline", R="M6 30C10 8 20 12 24 24S38 42 42 18", pb=[(6, 30), (24, 24), (42, 18)])
    ic("spline_controle", B=L((6, 38), (14, 8), (34, 40), (42, 10)), R="M6 38C14 8 34 40 42 10", pb=[(14, 8), (34, 40)])
    ic("spine", B=L((8, 14), (8, 34)) + L((22, 8), (26, 30)) + L((34, 18), (42, 36)), R="M4 26c8 0 14-4 20-6s12 4 20 8")
    ic("curva_extraida", B=CUBO(20, 14, 14, 7, 16), R=L((26, 8), (42, 16)) + L((6, 21), (20, 28)))
    ic("curva_intersecao", B=PATCH(6, 18) + "M16 8v32M32 8v32M16 8h16M16 40h16", R="M16 25c6-1 10 2 16 3")
    ic("curva_juntar", B="M6 36c4-12 8-14 14-14", R="M20 22c8 0 8 14 14 14s8-10 8-14", pr=[(20, 22)])
    ic("curva_paralela", B=PL(44, 4, 30, 34, 10), R="M10 34c4-12 12-16 24-18M18 38c4-10 8-14 20-16")
    ic("curva_proj_sup", B=PATCH(6, 26), b="M8 8c8-6 24 6 32-2", R="M8 31c8-6 24 6 32-2" + S(24, 9, 24, 24, 4.5))
    ic("curva_projetada", B=PL(44, 4, 30, 16, 10), b="M10 10c8-8 24 6 30-2", R="M14 36c6-6 16 2 22-4" + S(24, 10, 24, 28, 4.5))
    ic("wf_repetir", B=CURVA, pr=[(9, 28), (17, 21), (24, 24), (31, 27), (39, 18)])
    # continuidade: o que muda é a junção, por isso o pente de curvatura só aparece em G2
    ic("cont_g0", B="M6 36h18", R="M24 36L40 8", pr=[(24, 36)])
    ic("cont_g1", B="M6 36h18", R="M24 36c8 0 14-10 16-28", pr=[(24, 36)])
    ic("cont_g2", B="M6 36h18", R="M24 36c8 0 14-10 16-28M30 34l4 7M36 28l6 5M39 20l6 2", pr=[(24, 36)])

    # ---------- superfícies ----------
    ic("sup_extrudada", B=ONDA(6, 38), R="M6 38V18c6-6 12-6 18-2s12 4 18-2v20")
    ic("sup_revolucao", b="M24 4v40", B="M34 10c4 10-6 18 4 28", R=E(24, 24, 18, 6))
    ic("sup_varredura", B="M6 40c14-2 24-10 34-26", R="M6 40V22c14-2 24-10 30-18l4 10")
    ic("sup_varredura_av", B="M6 40c14-2 24-10 36-24M6 22c14-2 22-8 28-16", R=L((6, 22), (6, 40)) + L((20, 18), (20, 36)) + L((34, 6), (42, 16)))
    ic("sup_tubo", B="M6 34c10 0 16-16 34-20", R=E(9, 34, 4, 9) + "M9 25c10 0 12-16 31-20M9 43c12 0 20-16 31-20" + E(40, 14, 3.5, 9))
    ic("sup_multisecoes", B="M6 40c5-5 10-5 14 0M16 24c5-5 10-5 14 0M28 10c5-5 10-5 14 0", R=L((6, 40), (28, 10)) + L((20, 40), (42, 10)))
    ic("sup_concordancia", B="M8 10v16M40 22v16", R="M8 10c14 0 18 12 32 12M8 26c14 0 18 12 32 12")
    ic("sup_conectar", B=Q(4, 8, 14, 18) + Q(30, 22, 14, 18), R="M18 8c8 0 4 14 12 14M18 26c8 0 4 14 12 14")
    ic("sup_regrada", B="M6 34c10-8 26-8 36 0", R="M6 34V14M15 29V9M24 27V7M33 29V9M42 34V14M6 14c10-8 26-8 36 0")
    ic("sup_deslocada", B=PATCH(6, 28), R=ONDA(6, 12) + "M6 12v8M42 12v8" + S2(24, 12, 24, 26, 3.5))
    ic("sup_preenchida", B=Z((8, 14), (38, 8), (42, 34), (10, 40)), R="M9 27l31-6M23 11l3 26M16 13l2 25M31 9l3 26")
    ic("sup_extraida", B=CUBO(20, 18, 14, 7, 14), R=F_TOPO(26, 2, 14, 7))
    ic("sup_cilindro", R=E(24, 11, 13, 5) + "M11 11v26M37 11v26", b="M11 37a13 5 0 0 0 26 0")
    ic("sup_esfera", R=C(24, 24, 18), B=E(24, 24, 18, 6))
    ic("sup_filete", B="M6 8c2 10 4 16 8 22M42 38c-6-2-10-4-14-8", R="M14 30c3 3 10 3 14 0")
    ic("sup_filete_aresta", B=L((6, 18), (6, 40), (42, 40), (42, 18)), R="M6 18a10 10 0 0 1 10-10h16a10 10 0 0 1 10 10")
    ic("sweep_fixo", B=GUIA, R=Q(4, 28, 8, 8) + Q(20, 20, 8, 8) + Q(34, 6, 8, 8))
    ic("sweep_frenet", B=GUIA, R=Z(*[pol(8, 38, 6, a) for a in (10, 100, 190, 280)]) + Z(*[pol(26, 24, 6, a) for a in (40, 130, 220, 310)]) + Z(*[pol(40, 10, 6, a) for a in (70, 160, 250, 340)]))
    ic("sweep_corrigido", B=GUIA, R=L((8, 30), (8, 44)) + L((20, 18), (32, 30)) + L((40, 4), (40, 16)))

    # operações de superfície
    ic("op_aparar", B=MEIO_ESQ, r=MEIO_DIR, R="M24 10v32")
    ic("op_contorno", B="M6 28c6-6 12-6 18-2s12 4 18-2", R=PATCH(6, 20))
    ic("op_curar", B=PATCH(6, 20), R=MAIS(24, 26, 7))
    ic("op_desaparar", B=MEIO_ESQ, R=MEIO_DIR)
    ic("op_desmontar", B=PL(38, 6, 16, 18, 8), R=Z((24, 22), (32, 8), (44, 8), (36, 22)))
    ic("op_dividir", B=PATCH(6, 20), R="M24 6v36")
    ic("op_extrapolar", B=MEIO_ESQ, R=S(26, 26, 42, 26, 5) + "M30 20c3 1 6 1 9 0M30 36c3 1 6 1 9 0")
    ic("op_juntar", B="M6 20c4-5 10-5 14-2v16c-4-3-10-3-14 2zM28 18c4 3 10 3 14-2v16c-4 5-10 5-14 2z", R="M20 22h8M20 30h8")
    ic("op_multi_extrair", B=CUBO(18, 20, 12, 6, 12), R=F_TOPO(30, 2, 10, 5) + Z((34, 16), (44, 12), (44, 22), (34, 26)))

    # híbrido
    ic("hib_costurar", B=Q(6, 26, 36, 16), R="M6 16h36M12 12v8M24 12v8M36 12v8M12 22v6M24 22v6M36 22v6")
    ic("hib_dividir", B=CUBO(), R="M4 26c8-6 12 6 20 0s12-8 20-2")
    ic("hib_engrossar", B=ONDA(6, 34), R=ONDA(6, 18) + "M6 18v16M42 18v16" + S2(24, 19, 24, 31, 3.5))
    ic("hib_fechar", B="M8 16v24h32V16", R="M8 16c10-8 22-8 32 0M8 16h32")
    ic("hib_malha", B=L((14, 12), (22, 28), (30, 8)) + L((6, 38), (22, 28), (36, 40)) + L((22, 28), (42, 22)),
       R=Z((6, 38), (14, 12), (30, 8), (42, 22), (36, 40)))
    ic("hib_preencher", B=Z((8, 12), (40, 8), (42, 38), (10, 42)), R="M12 22l10-10M12 34L32 13M18 38l21-22M30 37l10-10")

    # ---------- corpo de malha ----------
    MALHA = Z((6, 38), (14, 12), (30, 8), (42, 22), (36, 40)) + L((14, 12), (22, 28), (30, 8)) + L((6, 38), (22, 28), (36, 40))
    ic("malha_reparar", B=MALHA, R="M30 26l10 10M26 30l10 10M26 30l4-4")
    ic("malha_decimar", B=Z((6, 38), (14, 12), (30, 8), (42, 22), (36, 40)), R=L((14, 12), (36, 40)) + L((30, 8), (6, 38)))
    ic("malha_booleana", B=Z((6, 30), (14, 10), (28, 14), (24, 36)), R=Z((20, 18), (34, 12), (42, 30), (28, 40)))
    ic("malha_solido", B=Z((6, 36), (10, 16), (22, 12), (22, 38)) + L((10, 16), (22, 38)), R=CUBO(34, 18, 8, 4, 10) + "M23 24h4")
    ic("solido_malha", B=CUBO(13, 18, 8, 4, 10), R=Z((26, 36), (30, 16), (42, 12), (42, 38)) + L((30, 16), (42, 38)) + "M21 24h4")
    ic("impressao_mesa", B="M6 38h36M12 42h24", R="M16 36l4-22h8l4 22M24 6v8M19 14h10")

    # ---------- montagem ----------
    ic("asm_nova", B=CUBO(14, 8, 9, 4.5, 9) + CUBO(28, 16, 9, 4.5, 9), R=MAIS(38, 13, 6))
    ic("asm_inserir", B="M6 28v14h36V28", R=CUBO(24, 16, 10, 5, 10) + S(24, 3, 24, 13, 4.5))
    ic("asm_peca_nova", b=CUBO(16, 16, 11, 5.5, 11), R=CUBO(30, 6, 11, 5.5, 11) + MAIS(38, 38, 6))
    ic("asm_grupo", B="M12 6H6v36h6M36 6h6v36h-6", R=CUBO(24, 8, 10, 5, 8) + CUBO(24, 26, 10, 5, 6))
    ic("asm_link", B=CUBO(14, 6, 9, 4.5, 9), R="M26 34h-4a6 6 0 0 1 0-12h10a6 6 0 0 1 6 6M30 30h4a6 6 0 0 1 0 12H24a6 6 0 0 1-6-6")
    ic("asm_fixadores", B="M6 22h10M32 22h10M6 34h10M32 34h10M16 22v12M32 22v12", R="M14 12h20v6H14zM20 18v24M28 18v24")
    ic("asm_catalogo", B="M10 8h24a4 4 0 0 1 4 4v28H14a4 4 0 0 1-4-4zM10 36a4 4 0 0 1 4-4h24", R="M19 13h10v5H19zM22 18v10M26 18v10")
    ic("asm_engrenagens", B=ENGRENAGEM(16, 18, 11, 8, 8), R=ENGRENAGEM(33, 33, 11, 8, 8))
    ic("asm_chaveta", B="M6 30v12h36V30H34M14 30H6", R="M14 20h20a4 4 0 0 1 0 8H14a4 4 0 0 1 0-8z")
    ic("asm_calculo", B=Q(8, 6, 32, 36), R=Q(13, 11, 22, 8) + "M14 26h4M22 26h4M30 26h4M14 34h4M22 34h4M30 34h4")
    ic("asm_lista", B="M20 12h22M20 24h22M20 36h22", R=C(10, 12, 4) + C(10, 24, 4) + C(10, 36, 4))
    ic("asm_numerar", B=CUBO(16, 22, 10, 5, 10), R=C(36, 14, 7) + L((31, 19), (20, 28)))
    ic("asm_recorte_peca", B=CUBO(24, 16, 18, 9, 12), R="M14 21V6h20v15M14 21l10 5 10-5")
    ic("asm_repeticao", B=CUBO(11, 6, 7, 3.5, 7), R=CUBO(24, 14, 7, 3.5, 7) + CUBO(37, 22, 7, 3.5, 7))
    ic("asm_sub_selecao", b=Q(6, 6, 36, 36), R="M24 15v7M14 30v-8h20v8" + C(24, 12, 3) + C(14, 33, 3) + C(34, 33, 3))
    ic("asm_subconjunto", B=C(24, 10, 4) + "M24 14v10", R="M10 34V24h28v10" + Q(5, 34, 10, 8) + Q(33, 34, 10, 8))
    ic("asm_substituir", B=CUBO(14, 6, 9, 4.5, 8), R=CUBO(34, 22, 9, 4.5, 8) + SA(24, 24, 18, -55, -20, 4) + SA(24, 24, 18, 125, 160, 4))
    ic("mec_auto", B=L((8, 38), (22, 16), (40, 24)), R=C(8, 38, 3.5) + C(22, 16, 3.5) + C(40, 24, 3.5) + FAISCA(36, 38, 7))
    ic("mec_simular", B=L((8, 40), (18, 26), (38, 36)) + C(18, 26, 3.5), R=Z((28, 6), (28, 22), (41, 14)))

    # restrições de montagem: duas peças e o vínculo em destaque
    ic("rest_alinhar", B=Q(6, 26, 14, 14) + Q(28, 12, 14, 14), R="M4 40h40" + S(35, 28, 35, 37, 4.5))
    ic("rest_angulo", B="M6 40h36", R=L((6, 40), (32, 12)) + SA(6, 40, 26, -40, -3, 4.5))
    ic("rest_atualizar", B=Q(16, 16, 16, 16), R=SA(24, 24, 18, -160, -20, 5) + SA(24, 24, 18, 20, 160, 5))
    ic("rest_auto", B=Q(6, 8, 14, 14) + Q(20, 22, 14, 14), R=FAISCA(36, 13, 8))
    ic("rest_coincidencia", B=PL(26, 6, 26, 12, 10), r=PL(40, 6, 26, 12, 10), R=S(24, 38, 24, 27, 4.5))
    ic("rest_conjunta", B=Q(6, 6, 16, 14) + Q(26, 6, 16, 14), R=CADEADO(24, 34))
    ic("rest_contato", B=Q(6, 28, 36, 14), R=C(24, 16, 12), pr=[(24, 28)])
    ic("rest_cremalheira", B="M4 42h40M4 36h4l2-4h4l2 4h4l2-4h4l2 4h4l2-4h4l2 4h4", R=ENGRENAGEM(24, 17, 12, 9, 8) + C(24, 17, 3))
    ic("rest_distancia", B="M6 8v32M42 8v32", R=S2(8, 24, 40, 24, 5))
    ic("rest_engrenagem", B=ENGRENAGEM(15, 20, 10, 7.5, 8), R=ENGRENAGEM(32, 30, 13, 10, 10))
    ic("rest_fixar", B=CUBO(24, 4, 15, 7.5, 12), R="M6 36h36M12 36l-5 7M22 36l-5 7M32 36l-5 7M42 36l-5 7")
    ic("rest_interferencia", B=Q(6, 8, 22, 22) + Q(20, 20, 22, 22), R=Q(20, 20, 8, 10) + XIS(38, 10, 4.5))
    ic("rest_paralelo", B=PL(16, 6, 26, 10, 10), R=PL(40, 6, 26, 10, 10) + "M16 22v8M24 22v8M32 22v8")
    ic("rest_perpendicular", B=PL(42, 4, 30, 12, 10), R=Z((20, 36), (20, 8), (36, 8), (36, 36)))
    ic("rest_polia", B=C(13, 30, 8), R=C(35, 16, 6) + L((9.3, 22.9), (32, 10.7)) + L((16.7, 37.1), (38, 21.3)))

    # juntas: o eixo é base, o movimento permitido é resultado
    ic("junta_rigida", B=Q(6, 14, 18, 20) + Q(24, 14, 18, 20), R=XIS(24, 24, 6) + "M24 4v8M24 36v8")
    ic("junta_revolucao", B="M24 4v40", R=E(24, 26, 16, 7) + ponta(40, 26, math.pi / 2, 5))
    ic("junta_prismatica", B="M6 34h36M6 42h36", R=Q(16, 20, 16, 14) + S2(8, 12, 40, 12, 5))
    ic("junta_cilindrica", B="M6 24h36", R=E(24, 24, 7, 13) + S2(12, 42, 36, 42, 4.5))
    ic("junta_parafuso", B="M6 24h36", R="M12 15l5 18M20 15l5 18M28 15l5 18" + S(14, 6, 36, 6, 4.5))
    ic("junta_planar", B=PL(42, 4, 30, 18, 10), R=S2(10, 32, 34, 32, 4) + SA(24, 18, 12, 180, 360, 4.5))
    ic("junta_esferica", B=L((24, 42), (24, 32)), R=C(24, 20, 12) + "M12 20a12 4.5 0 0 0 24 0")
    ic("junta_came", B="M24 4v9M18 4h12", R="M10 28a14 14 0 0 1 28-6c0 10-8 18-16 18s-12-6-12-12z", pb=[(24, 24)])
    ic("junta_ponto_curva", B="M6 36c10-24 26 8 36-20", R=S(19, 14, 31, 14, 4.5), pr=[(24, 26)])

    # ---------- desenho ----------
    ic("des_novo", B=FOLHA(), R=MAIS(19, 22, 7))
    ic("des_abrir", B=FOLHA(6, 16, 30, 26), R=S(20, 29, 36, 9, 6))
    ic("des_salvar", B=FOLHA(6, 8, 36, 24), R=S(24, 13, 24, 28, 5) + "M8 42h32")
    ic("des_folha", B="M14 6h28v28", R=FOLHA(6, 14, 28, 28))
    ic("des_vista", B=FOLHA(), R="M12 28V14h16v6h-6v8z")
    ic("des_tres_vistas", B=FOLHA(), R=Q(10, 12, 9, 8) + Q(10, 23, 9, 7) + Q(22, 12, 8, 8))
    ic("des_iso", B=FOLHA(), R=CUBO(21, 11, 9, 4.5, 8))
    ic("des_projetada", B=Q(6, 8, 14, 14), R=Q(28, 26, 14, 14) + L((20, 22), (28, 26)), r="M6 22v18h22")
    ic("des_corte", b="M6 22h36", B=Q(10, 8, 28, 32), R="M10 34l12-12M16 40l18-18M28 40l10-10")
    ic("des_detalhe", B=L((4, 40), (4, 12), (22, 12)), R=C(13, 12, 8) + C(33, 32, 11) + L((19, 18), (25, 24)))
    ic("des_planificada", B=FOLHA(), R=Z((16, 12), (22, 12), (22, 17), (28, 17), (28, 23), (22, 23), (22, 28), (16, 28), (16, 23), (10, 23), (10, 17), (16, 17)))
    ic("des_vista_camera", B=FOLHA(), R=Q(11, 15, 13, 11) + L((24, 19), (30, 15), (30, 26), (24, 22)))
    ic("des_centro", B=C(24, 24, 15), R="M24 4v12M24 20v8M24 32v12M4 24h12M20 24h8M32 24h12")
    ic("des_balao", B="M6 40h20", R=C(32, 15, 10) + L((25, 22), (14, 40)))
    ic("des_baloes_auto", B="M6 42h22", R=C(14, 13, 7) + L((14, 20), (14, 42)) + C(30, 21, 6) + L((26, 26), (20, 42)) + FAISCA(38, 37, 7))
    ic("des_bom", B=TABELA(6, 8, 36, 32, 4, 2), R="M6 16V8h36v8z")
    ic("des_esquema", B=L((10, 36), (22, 14), (40, 30)), R=C(10, 36, 4) + C(22, 14, 4) + C(40, 30, 4) + "M4 42h12")
    ic("des_nota", B="M26 16h16M26 24h16M26 32h10", R=S(20, 18, 7, 38, 5))
    ic("des_datum", B="M6 40h36", R=Z((16, 40), (32, 40), (24, 30)) + "M24 30v-6" + Q(15, 6, 18, 18))
    ic("des_gdt", B="M6 40h36", R=TABELA(6, 8, 36, 14, 1, 3) + "M24 22v18")
    ic("des_rugosidade", B="M6 38h36", R=L((14, 28), (20, 38), (32, 12), (42, 12)))
    ic("des_solda", B="M6 42h36M24 42V24", R=L((42, 10), (24, 10), (12, 28)) + Z((30, 10), (38, 10), (30, 2)))
    ic("des_dxf", B=FOLHA(6, 6, 28, 30), R=S(24, 42, 42, 42, 5) + L((12, 26), (18, 14), (26, 24)))
    ic("des_pdf", B=FOLHA(6, 6, 28, 30), R=S(24, 42, 42, 42, 5) + Q(12, 12, 14, 12))
    ic("des_svg", B=FOLHA(6, 6, 28, 30), R=S(24, 42, 42, 42, 5) + "M11 26c4-14 12-14 16 0")
    ic("des_tab_corte", B=TABELA(6, 8, 36, 16, 2, 2), R=Q(6, 30, 36, 10) + L((18, 30), (14, 40)) + L((34, 30), (30, 40)))
    ic("des_tab_dobras", B=TABELA(6, 8, 36, 16, 2, 2), R="M6 40h20a8 8 0 0 0 8-8v-2")
    ic("des_tab_furos", B=TABELA(6, 8, 36, 16, 2, 2), R=C(24, 35, 7) + "M24 25v3M24 42v3M14 35h3M31 35h3")

    # cotas de desenho: a mesma cota, agora sobre uma peça
    ic("des_cota_h", B=Q(10, 22, 28, 20), R="M10 18V8M38 18V8" + S2(11, 12, 37, 12, 4.5))
    ic("des_cota_v", B=Q(6, 10, 20, 28), R="M30 10h12M30 38h12" + S2(36, 11, 36, 37, 4.5))
    ic("des_cota_al", B=Z((12, 36), (34, 14), (42, 22), (20, 44)), R=S2(6, 30, 28, 8, 4.5))
    ic("des_cota_ang", B=L((8, 42), (42, 42)) + L((8, 42), (30, 14)), R=SA2(8, 42, 26, -50, -3, 4.5))
    ic("des_cota_d", B=Q(6, 12, 36, 30) + C(24, 27, 10), R=S2(14, 27, 34, 27, 4))
    ic("des_cota_r", B="M6 42V22a16 16 0 0 1 16-16h20", R=S(24, 24, 11, 11, 5), pr=[(24, 24)])
    # cota inteligente (B107): a peça com furo, a cota linear em cima e a seta do diâmetro
    ic("des_cota_inteligente", B=Q(6, 18, 36, 24) + C(17, 30, 5), R="M6 18V7M42 18V7" + S2(7, 11, 41, 11, 4.5) + S(38, 40, 24, 33, 4.5))

    # PMI: as mesmas ideias sobre a peça em 3D
    ic("pmi_linear", B=CUBO(24, 4, 15, 7.5, 16), R=S2(6, 33, 21, 41, 4))
    ic("pmi_angulo", B=CUBO(24, 4, 15, 7.5, 16), R=SA(24, 19, 26, 45, 130, 4))
    ic("pmi_diametro", B=E(20, 12, 12, 5) + "M8 12v22a12 5 0 0 0 24 0V12", R=S2(8, 12, 32, 12, 4) + L((32, 12), (42, 4)))
    ic("pmi_raio", B="M8 42V22M24 6h18", R="M8 22A16 16 0 0 1 24 6" + S(28, 26, 13, 11, 4.5))
    ic("pmi_nota", B=CUBO(16, 18, 11, 5.5, 11), R=L((22, 22), (30, 12), (42, 12)) + "M30 6h12")
    ic("pmi_baloes", B=CUBO(16, 18, 11, 5.5, 11), R=C(35, 12, 7) + L((30, 17), (20, 26)))
    ic("pmi_legenda", B=CUBO(16, 6, 11, 5.5, 11), R=TABELA(6, 32, 36, 10, 1, 3))
    ic("pmi_referencia", B=CUBO(18, 4, 13, 6.5, 14), R=Z((10, 42), (24, 42), (17, 33)) + Q(30, 30, 12, 12))
    ic("pmi_rugosidade", B=CUBO(18, 16, 13, 6.5, 12), R=L((24, 14), (28, 22), (36, 6), (44, 6)))
    ic("pmi_solda", B=CUBO(16, 18, 11, 5.5, 11), R=L((22, 24), (30, 10), (44, 10)) + Z((34, 10), (42, 10), (34, 3)))
    ic("pmi_tolerancia", B=CUBO(16, 22, 10, 5, 10), R=TABELA(14, 6, 28, 12, 1, 3) + L((20, 18), (20, 26)))

    # vistas
    ic("vista_frente", B=CUBO(), R=F_ESQ())
    ic("vista_topo", B=CUBO(), R=F_TOPO())
    ic("vista_direita", B=CUBO(), R=F_DIR())
    ic("vista_iso", B=Q(6, 6, 36, 36), R=CUBO(24, 12, 12, 6, 12))

    # seleção
    ic("sel_clique", B=C(14, 14, 8), R=CURSOR(18, 18, 1.0))
    ic("sel_caixa_janela", B=C(24, 24, 7), R=Q(6, 8, 36, 32))
    ic("sel_caixa_cruzada", B=C(36, 24, 9), r=Q(6, 8, 28, 32))
    ic("sel_caixa_auto", B=C(24, 24, 6), R="M26 8H6v32h20", r="M26 8h16v32H26")
    ic("sel_face", B=CUBO(), R=F_TOPO() + "M15 14h18")
    ic("sel_aresta", B=CUBO(), R=L((24, 22), (24, 40)))
    ic("sel_vertice", B=CUBO(), R=C(24, 22, 5))
    ic("sel_corpo", R=CUBO(), B="M4 12V4h8M36 4h8v8M44 36v8h-8M12 44H4v-8")
    ic("sel_filtro", b=Q(6, 8, 36, 32), R=XIS(24, 24, 9))

    # transformar
    ic("xf_mover", b=CUBO(14, 20, 9, 4.5, 9), R=CUBO(34, 4, 9, 4.5, 9) + S(14, 18, 25, 8, 4.5))
    ic("xf_girar", b=CUBO(24, 16, 12, 6, 12), R=SA(24, 26, 19, -160, -20, 5))
    ic("xf_escala", B=Q(8, 26, 14, 14), R=L((8, 16), (8, 8), (40, 8), (40, 40), (32, 40)) + S(22, 26, 34, 14, 5), pb=[(8, 40)])
    ic("xf_simetria", b="M24 4v40", B=CUBO(13, 12, 9, 4.5, 12), R=CUBO(35, 12, 9, 4.5, 12))
    ic("xf_afinidade", B=Q(8, 16, 20, 24), R=Z((8, 40), (22, 8), (42, 8), (28, 40)))

    # ---------- chapa: perfil de lado, a dobra é o resultado ----------
    ic("sm_parede", b=F_TOPO(24, 8, 19, 9.5), R=F_TOPO(24, 12, 19, 9.5) + L((5, 21.5), (5, 25), (24, 34.5), (43, 25), (43, 21.5)) + "M24 31v3.5")
    ic("sm_aba", B="M6 36h22", R="M28 36a8 8 0 0 0 8-8V10")
    ic("sm_aba_laco", B=Q(14, 16, 20, 16), R="M24 16V6M24 32v10M14 24H4M34 24h10")
    ic("sm_aba_curva", B="M6 38c4-14 18-18 36-18", R="M6 38V26c4-14 18-18 36-18M42 8v12")
    ic("sm_aba_perfil", B="M6 38h16", R="M22 38a6 6 0 0 0 6-6V22l8-6h6V8")
    ic("sm_aba_superficie", B="M6 38h14M44 8c-2 14-10 24-20 30", R="M20 38c8 0 12-14 12-24")
    ic("sm_alivio_canto", B="M6 24h11M24 31v11", b="M6 14h28v28", R=C(24, 24, 7))
    # fechamento de canto (B293): vista de cima, duas paredes e a peça que fecha o vão
    ic("sm_fecha_canto", B="M8 42V26M24 8h18", R="M8 26V8h16")
    ic("sm_canto_topo", B="M8 42V28M26 8h16", R="M8 28V14l6-6h12", b="M14 14l8-8")
    ic("sm_canto_sobreposto", B="M8 42V28M30 8h12", R="M8 28V8h22", b="M14 14h10")
    ic("sm_canto_sob", B="M8 42V28M30 8h12", R="M30 8H14V28", b="M8 14h12")
    # B35: aba entre perfis, desdobrar/redobrar parcial e converter em chapa
    ic("sm_aba_perfis", B="M6 40h36M16 10h16", R="M6 40L16 10M42 40L32 10")
    ic("sm_desdobrar_parcial", B="M6 12v20h14", R="M20 32h22", b="M42 32v-18")
    ic("sm_redobrar_parcial", B="M6 12v20h14", R="M20 32h14a6 6 0 0 0 6-6V12")
    ic("sm_converter_chapa", B=Q(6, 14, 18, 18), R="M28 12h14v30H12v-6")
    ic("sm_bainha", B="M6 28h26", R="M32 28a6 6 0 0 0 0-12H16")
    ic("sm_desdobrar", B="M6 32h16a6 6 0 0 0 6-6V8", r="M28 32h16", R=SA(28, 32, 16, -80, -10, 5))
    ic("sm_dobra_linha", B=Z((4, 34), (14, 20), (26, 20), (16, 34)), R=L((26, 20), (16, 34)) + L((26, 20), (40, 6), (44, 20), (16, 34)))
    ic("sm_estampo", B="M6 34h10M32 34h10", R="M16 34l4-14h8l4 14")
    ic("sm_joggle", B="M6 34h12", R="M18 34c6 0 6-16 12-16h12")
    ic("sm_parametros", B="M6 28h36M6 36h36", R=S(24, 16, 24, 28, 4) + S(24, 46, 24, 36, 4))
    ic("sm_rasgo_circular", B=PL(40, 4, 30, 30, 10), R=arc(24, 28, 12, -170, -10) + arc(24, 28, 7, -170, -10))
    ic("sm_recorte", B=PL(40, 4, 30, 30, 10), R=Z((16, 32), (20, 18), (32, 18), (28, 32)))
    ic("sm_veneziana", B=Q(6, 10, 36, 28), R="M12 22c6-8 18-8 24 0M12 22h24M12 33c6-8 18-8 24 0M12 33h24")

    # ---------- estrutural ----------
    ic("st_membro", b="M4 40L44 8", R=L((6, 32), (34, 8)) + L((14, 40), (42, 16)) + L((6, 32), (14, 40)) + L((34, 8), (42, 16)))
    ic("st_biblioteca", B="M6 6v36h36", R="M14 10h24M14 34h24M26 10v24")
    ic("st_canto_nenhum", B=Q(8, 8, 12, 34), R=Q(8, 30, 34, 12))
    ic("st_canto_aparar", B=Q(8, 8, 12, 22), R=Q(8, 30, 34, 12))
    ic("st_canto_meia", B=L((8, 42), (8, 8), (20, 8), (20, 30)), R=L((8, 42), (42, 42), (42, 30), (20, 30)) + L((8, 42), (20, 30)))
    ic("st_canto_encaixe", B=L((8, 30), (8, 8), (20, 8), (20, 30)), R=L((8, 42), (42, 42), (42, 30), (20, 30), (14, 36), (8, 30), (8, 42)))
    ic("st_corte", B=TABELA(6, 8, 36, 14, 2, 1), R=Q(6, 28, 36, 12) + L((22, 28), (18, 40)))
    ic("st_montagem", B=L((8, 42), (8, 10), (40, 10), (40, 42)), R=L((8, 26), (24, 10)) + L((40, 26), (24, 10)) + C(8, 10, 3.5) + C(40, 10, 3.5))
    ic("st_placa", B=L((6, 18), (30, 18)) + L((6, 30), (30, 30)), R=Q(30, 8, 8, 32))
    ic("st_reforco", B="M8 6v36h34M14 6v30h28", R=Z((14, 24), (14, 36), (32, 36)))
    ic("st_solda", B=L((6, 40), (42, 40)) + L((16, 6), (16, 40)), R=Z((16, 26), (16, 40), (30, 40)) + L((34, 34), (40, 34)) + L((34, 27), (40, 27)))

    # ---------- controles da interface: não têm par em assets/icons ----------
    LUPA = C(21, 21, 13) + "M31 31l11 11"
    ic("arvore_zoom_mais", B=LUPA, R=MAIS(21, 21, 6))
    ic("arvore_zoom_menos", B=LUPA, R="M15 21h12")
    # alvo: os quatro riscos apontam o item
    ic("arvore_localizar", B=C(24, 24, 13), R=C(24, 24, 4) + "M24 4v8M24 36v8M4 24h8M36 24h8")
    # cantos do enquadramento em volta de uma árvore
    ic("arvore_enquadrar", B="M6 16V6h10M32 6h10v10M42 32v10H32M16 42H6V32", R="M18 16h12M24 16v16M24 24h7M24 32h7")
    ic("arvore_recolher", B=Q(8, 8, 32, 32), R="M16 24h16")
    # painel volta ao lugar: seta curva sobre o quadro
    ic("painel_restaurar", B=Q(6, 16, 26, 26), R="M42 22V12a6 6 0 0 0-6-6H22" + L((27, 1), (22, 6), (27, 11)))
    ic("barra_mais", B=Q(6, 12, 36, 24), R=C(15, 24, 1.5) + C(24, 24, 1.5) + C(33, 24, 1.5))


desenhos()

# forma livre: nós em ponto sobre a gaiola fina, e a superfície no traço do resultado
ic("forma", b=Q(8, 8, 32, 32), R=C(24, 24, 14), pb=[(8, 8), (40, 8), (40, 40), (8, 40)])
ic("forma_caixa", R=CUBO(), pb=[(24, 6), (40, 14), (40, 32), (24, 40), (8, 32), (8, 14)])
ic("forma_esfera", R=C(24, 24, 18), pb=[(24, 6), (42, 24), (24, 42), (6, 24)])
ic("forma_cilindro", R=E(24, 12, 15, 6) + "M9 12v24a15 6 0 0 0 30 0V12", pb=[(9, 12), (39, 12), (9, 36), (39, 36)])
ic("forma_plano", R=Z((6, 34), (18, 14), (42, 14), (30, 34)), pb=[(6, 34), (18, 14), (42, 14), (30, 34)])
ic("forma_extrudar", B=Q(10, 28, 28, 14), R="M10 28V14h28v14" + S(24, 26, 24, 8))
ic("forma_laco", B=Q(8, 10, 32, 28), R="M24 10v28", pr=[(24, 10), (24, 38)])
ic("forma_aresta", B=Q(8, 10, 32, 28), R=L((8, 10), (40, 38)), pr=[(8, 10), (40, 38)])
ic("forma_borda", b="M8 8h20l12 12v20", R="M8 8h14a18 18 0 0 1 18 18v14")
ic("forma_tangencia", B="M6 36c10-14 26-14 36 0", R=L((8, 25), (40, 25)) + S(24, 25, 24, 7), pr=[(24, 25)])
ic("forma_simetria", b="M24 4v40", R="M20 10c-16 2-16 26 0 28M28 10c16 2 16 26 0 28")
ic("forma_converter", b=Q(6, 12, 18, 24), R=CUBO(32, 12, 10, 5, 12) + "M18 24h4", pb=[(6, 12), (24, 12), (24, 36), (6, 36)])

SO_INTERFACE = {"sm_aba_perfis", "sm_desdobrar_parcial", "sm_redobrar_parcial", "sm_converter_chapa", "sm_aba_laco", "sm_fecha_canto", "sm_canto_topo", "sm_canto_sobreposto", "sm_canto_sob", "importar_arquivo", "asm_catalogo", "arvore_zoom_mais", "arvore_zoom_menos", "arvore_localizar", "arvore_enquadrar", "arvore_recolher", "painel_restaurar", "barra_mais", "exportar_arquivo", "liberar_revisao", "onde_usado", "comparar", "estado_sem_revisao", "estado_liberada", "estado_editado", "estado_leitura", "estado_conflito", "forma", "forma_caixa", "forma_esfera", "forma_cilindro", "forma_plano", "forma_extrudar", "forma_laco", "forma_aresta", "forma_borda", "forma_tangencia", "forma_simetria", "forma_converter", "des_cota_inteligente", "curso", "mapa_macro", "perfil_fluxo", "an_conexao", "an_desvio", "an_pente", "sup_regrada", "st_solda", "conversao_lote", "formatos_suportados", "edit_mover_face", "edit_apagar_face", "edit_deslocar_face", "edit_raio_filete"}

<<<<<<< HEAD
SO_INTERFACE |= {"an_inclinacao", "an_isofotas", "an_dfm"}
=======
SO_INTERFACE |= {"asm_chaveta", "asm_calculo"}
>>>>>>> worktree-agent-a522f3f9c8e11b4ff


# ---------- saída ----------

def svg(nome):
    B, R, b, r, pb, pr = I[nome]
    partes = []

    def traco(cor, largura, d, extra=""):
        if d:
            partes.append(f'<path stroke="{cor}" stroke-width="{largura}"{extra} d="{d}"/>')

    tra = f' stroke-dasharray="{TRACEJADO}"'
    traco("BASE", TRACO_BASE, B)
    traco("BASE", TRACO_BASE, b, tra)
    traco("BASE", PONTO, "".join(f"M{n(x)} {n(y)}h.01" for x, y in pb))
    traco("RES", TRACO_RES, R)
    traco("RES", TRACO_RES, r, tra)
    traco("RES", PONTO, "".join(f"M{n(x)} {n(y)}h.01" for x, y in pr))
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" width="48" height="48" fill="none" '
            f'stroke-linecap="round" stroke-linejoin="round">{"".join(partes)}</svg>\n')


def valida(nome, texto):
    ET.fromstring(texto)
    falta = [m for m in ("BASE", "RES") if f'stroke="{m}"' not in texto]
    if falta:
        raise SystemExit(f"{nome}: falta o marcador {falta}")
    if "#" in texto:
        raise SystemExit(f"{nome}: cor fixa")


GRUPOS = [
    ("Furos", r"furo|asm_furo|sm_furo"),
    ("Chanfros e filetes", r"chanfro|filete|arestas|prop_|sk_filete|sk_chanfro|canto"),
    ("Extensão", r"ext_"),
    ("Sólidos", r"extrusao|recorte|revolucao|ranhura|nervura|rasgo|reforco|multisecoes|casca|saida|espessura|escala|espelho|extrair|rosca|padrao|bool_|op_(adicionar|remover|intersecao|unir)|prim_|xf_"),
    ("Sketch", r"sketch|sk_|r_|cota_"),
    ("Planos, linhas e pontos", r"plano|linha_|ponto_"),
    ("Curvas", r"circulo|conica|espiral|helice|polilinha|spline|spine|curva_|wf_|cont_|conexao"),
    ("Superfícies", r"sup_|sweep_|op_|hib_"),
    ("Montagem", r"asm_|mec_|rest_|junta_|coletor|exibicao"),
    ("Chapa", r"sm_"),
    ("Estrutural", r"st_"),
    ("Desenho e PMI", r"des_|pmi_"),
    ("Exibição e arquivo", r""),
]

# Fundo, base e resultado de dois temas de theme.rs: Prancheta Fria (o padrão) e Prancheta Noturna.
# Só a prancha usa estas cores; no programa quem manda é theme::icon_colors.
CORES = {"claro": ("#e5ebee", "#16242e", "#0b6f8e"), "escuro": ("#23272b", "#e4e3dc", "#f08a5d")}


def prancha(nomes, faltam):
    simbolos, secoes, usados = [], [], set()
    for nome in nomes:
        if nome in I:
            corpo = re.sub(r"^<svg[^>]*>|</svg>\s*$", "", svg(nome))
            corpo = corpo.replace('stroke="BASE"', 'class="b"').replace('stroke="RES"', 'class="r"')
            simbolos.append(f'<symbol id="{nome}" viewBox="0 0 48 48">{corpo}</symbol>')
    for titulo, padrao in GRUPOS:
        lista = [x for x in nomes if x in I and x not in usados and re.match(padrao, x)]
        usados.update(lista)
        celulas = []
        for nome in lista:
            uso = f'<use href="#{nome}"/>'
            lados = "".join(
                f'<div class="{t}"><svg width="48" height="48">{uso}</svg><svg width="24" height="24">{uso}</svg></div>'
                for t in CORES)
            celulas.append(f'<figure>{lados}<figcaption>{nome}</figcaption></figure>')
        secoes.append(f'<h2>{titulo} <small>{len(lista)}</small></h2><section>{"".join(celulas)}</section>')
    temas = "".join(f".{t}{{background:{c[0]};--b:{c[1]};--r:{c[2]}}}" for t, c in CORES.items())
    sem = ", ".join(faltam) if faltam else "nenhum"
    html = f"""<!doctype html>
<html lang="pt-BR"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Prancha de ícones</title>
<style>
:root{{color-scheme:light}}
body{{margin:0;padding:24px 16px 48px;background:#eceef1;color:#1f2329;font:14px/1.45 system-ui,sans-serif}}
main{{max-width:1320px;margin:auto}}
h1{{font-size:22px;margin:0 0 4px}} h2{{font-size:16px;margin:28px 0 10px}} small{{color:#6b7380;font-weight:400}}
p{{margin:0 0 8px;max-width:72ch}}
section{{display:grid;grid-template-columns:repeat(auto-fill,minmax(196px,1fr));gap:8px}}
figure{{margin:0;display:grid;grid-template-columns:1fr 1fr;border-radius:6px;overflow:hidden;background:#fff}}
figure>div{{display:flex;align-items:center;justify-content:center;gap:8px;padding:10px 4px}}
figcaption{{grid-column:1/3;padding:3px 8px;font-size:11.5px;color:#4a5260;overflow-wrap:anywhere}}
.b,.r{{fill:none;stroke-linecap:round;stroke-linejoin:round}} .b{{stroke:var(--b)}} .r{{stroke:var(--r)}}
{temas}
</style></head><body><main>
<h1>Prancha de ícones</h1>
<p>{len(I)} ícones novos (B258, B315). Cada quadro mostra um tema claro à esquerda (Prancheta Fria) e um escuro à direita (Prancheta Noturna), em 48 e 24 px.
O traço fino na cor do texto é o que já existia; o traço mais grosso na cor de destaque é o que o comando faz.</p>
<p>Sem desenho novo, caindo no ícone antigo: {sem}.</p>
<svg width="0" height="0" style="position:absolute" fill="none" stroke-linecap="round" stroke-linejoin="round"><defs>{"".join(simbolos)}</defs></svg>
{"".join(secoes)}
</main></body></html>
"""
    with open(PRANCHA, "w", encoding="utf-8", newline="\n") as f:
        f.write(html)


def main():
    nomes = sorted(a[:-4] for a in os.listdir(ANTIGOS) if a.endswith(".svg"))
    os.makedirs(SAIDA, exist_ok=True)
    sobras = sorted(set(I) - set(nomes) - SO_INTERFACE)
    if sobras:
        raise SystemExit(f"desenho sem nome em assets/icons: {sobras}")
    for nome in sorted(I):
        texto = svg(nome)
        valida(nome, texto)
        with open(os.path.join(SAIDA, nome + ".svg"), "w", encoding="utf-8", newline="\n") as f:
            f.write(texto)
    faltam = [x for x in nomes if x not in I]
    prancha(nomes, faltam)
    print(f"{len(I)} ícones em assets/icons2; prancha em docs/benchmark/detalhe/icones.html")
    print(f"sem desenho ({len(faltam)}): {', '.join(faltam) or 'nenhum'}")


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    main()
