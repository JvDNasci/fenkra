#!/usr/bin/env python3
"""Render do vídeo "CAD paramétrico em 10 s" (vertical 1080x1920, 30 fps).

Engrenagem desenhada por código: esboço 2D que se traça → extrusão 3D → número de dentes muda a cada
batida → furo e alívios se atualizam → chamada final. Tempos alinhados à trilha (118 bpm).
Uso: python graficos/render_cad.py render/cad_video.mp4   (rodar de dentro da pasta do projeto)
"""
import json
import math
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

AQUI = Path(__file__).resolve().parent
PROJ = AQUI.parent
RAIZ = PROJ.parents[2]
FONTES = RAIZ / "biblioteca" / "fontes"
PAL = json.loads((AQUI / "marca" / "paleta.json").read_text(encoding="utf-8"))

W, H, FPS, DUR = 1080, 1920, 30, 10.0
XS = 500  # centro horizontal da área segura (a direita tem 140 px de botões)
C = 4 * 60 / 118  # compasso de 4 batidas a 118 bpm
B = 60 / 118


def rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


FUNDO, PRIM, DEST, TEXTO = rgb(PAL["fundo"]), rgb(PAL["primaria"]), rgb(PAL["destaque"]), rgb(PAL["texto"])
ACO = (176, 192, 214)  # cor da peça: aço azulado, sempre distinta do fundo escuro


def fonte(rel, tam):
    return ImageFont.truetype(str(FONTES / rel), tam)


F_TITULO = "tech/rubikmonoone/RubikMonoOne-Regular.ttf"
F_MONO = "tech/spacemono/SpaceMono-Bold.ttf"


# ---------- curvas de animação ----------
def clamp(x, a=0.0, b=1.0):
    return max(a, min(b, x))


def ease_out(x):
    x = clamp(x)
    return 1 - (1 - x) ** 3


def ease_io(x):
    x = clamp(x)
    return 3 * x * x - 2 * x ** 3


def mola(x):
    """Pop com leve sobressalto (0 → 1)."""
    if x <= 0:
        return 0.0
    return 1 - math.exp(-x * 9) * math.cos(x * 16)


# ---------- geometria ----------
def contorno_engrenagem(n, r_raiz=250, r_ponta=300):
    pts = []
    p = 2 * math.pi / n
    for i in range(n):
        a = i * p
        for f, r in ((0.0, r_raiz), (0.18, r_raiz), (0.34, r_ponta), (0.5, r_ponta + 4), (0.66, r_ponta), (0.82, r_raiz)):
            pts.append((r * math.cos(a + f * p), r * math.sin(a + f * p)))
    return pts


def circulo(cx, cy, r, n=40):
    return [(cx + r * math.cos(2 * math.pi * k / n), cy + r * math.sin(2 * math.pi * k / n)) for k in range(n)]


def furos(r_furo, n_alivio, r_alivio, prog_alivio):
    hs = [circulo(0, 0, r_furo, 48)]
    for k in range(n_alivio):
        rr = r_alivio * ease_out(prog_alivio * 1.4 - k * 0.08)
        if rr > 2:
            a = 2 * math.pi * k / n_alivio + math.pi / n_alivio
            hs.append(circulo(165 * math.cos(a), 165 * math.sin(a), rr, 28))
    return hs


def girar(pts3, yaw, pitch):
    cy, sy, cp, sp = math.cos(yaw), math.sin(yaw), math.cos(pitch), math.sin(pitch)
    x, y, z = pts3[:, 0], pts3[:, 1], pts3[:, 2]
    x1, y1 = cy * x - sy * y, sy * x + cy * y
    y2, z2 = cp * y1 - sp * z, sp * y1 + cp * z
    return np.stack([x1, y2, z2], axis=1)


def projetar(p, escala, cx, cy, dist=1900.0):
    k = escala * dist / (dist - p[:, 2])
    return np.stack([cx + p[:, 0] * k, cy - p[:, 1] * k], axis=1)


LUZ = np.array([-0.35, 0.45, 0.82])  # de cima e da frente: tampa clara, borda em meia-luz
LUZ = LUZ / np.linalg.norm(LUZ)


def sombrear(cor, n):
    i = 0.32 + 0.68 * max(0.0, float(np.dot(n, LUZ)))
    return tuple(int(c * i) for c in cor)


def desenhar_peca(dr, contorno, buracos, h, yaw, pitch, escala, cx, cy, cor_aresta):
    """Extrusão com pintor + descarte de faces de trás. Coordenadas em pixels do quadro grande."""
    rot = lambda pts: girar(np.array(pts), yaw, pitch)  # noqa: E731
    cam = np.array([0, 0, 1900.0])
    faces = []

    def paredes(poly, para_dentro):
        n = len(poly)
        for i in range(n):
            (x0, y0), (x1, y1) = poly[i], poly[(i + 1) % n]
            nx, ny = (y1 - y0, -(x1 - x0)) if not para_dentro else (-(y1 - y0), x1 - x0)
            ln = math.hypot(nx, ny) or 1
            quad = [(x0, y0, h / 2), (x1, y1, h / 2), (x1, y1, -h / 2), (x0, y0, -h / 2)]
            q = rot(quad)
            nr = girar(np.array([[nx / ln, ny / ln, 0.0]]), yaw, pitch)[0]
            centro = q.mean(0)
            if np.dot(nr, cam - centro) > 0:
                faces.append((centro[2], q, sombrear(ACO, nr)))

    if h > 0.5:
        paredes(contorno, False)
        for b in buracos:
            paredes(b, True)
        faces.sort(key=lambda f: f[0])
        for _, q, cor in faces:
            pts = [tuple(v) for v in projetar(q, escala, cx, cy)]
            dr.polygon(pts, fill=cor, outline=tuple(int(c * 0.7) for c in cor))
    # tampa visível (topo ou base) com furos
    for zc, nz in ((h / 2, 1.0), (-h / 2, -1.0)):
        nr = girar(np.array([[0, 0, nz]]), yaw, pitch)[0]
        centro = girar(np.array([[0, 0, zc]]), yaw, pitch)[0]
        if np.dot(nr, cam - centro) <= 0 and h > 0.5:
            continue
        ext = [tuple(v) for v in projetar(rot([(x, y, zc) for x, y in contorno]), escala, cx, cy)]
        ints = [[tuple(v) for v in projetar(rot([(x, y, zc) for x, y in b]), escala, cx, cy)] for b in buracos]
        return ext, ints, sombrear(ACO, nr)
    return None, None, None


# ---------- textos ----------
def texto_img(txt, rel, tam, cor, contorno=8):
    f = fonte(rel, tam)
    x0, y0, x1, y1 = f.getbbox(txt, stroke_width=contorno)
    im = Image.new("RGBA", (x1 - x0 + 20, y1 - y0 + 20), (0, 0, 0, 0))
    ImageDraw.Draw(im).text((10 - x0, 10 - y0), txt, font=f, fill=cor + (255,), stroke_width=contorno,
                            stroke_fill=(0, 0, 0, 255))
    # cabe na área segura do vertical (x de 60 a 940), com folga para o pop da mola
    larg_max = 800
    if im.width > larg_max:
        im = im.resize((larg_max, int(im.height * larg_max / im.width)), Image.LANCZOS)
    return im


def colar_pop(quadro, im, cx, cy, prog, sair=None):
    """Cola a imagem centrada em (cx, cy) com escala de mola; `sair` (0-1) faz sumir."""
    s = mola(prog)
    if s <= 0.01:
        return
    alfa = clamp(prog * 4)
    if sair is not None:
        alfa *= 1 - clamp(sair)
        s *= 1 - 0.3 * clamp(sair)
    w, h = max(1, int(im.width * s)), max(1, int(im.height * s))
    im2 = im.resize((w, h), Image.LANCZOS)
    if alfa < 1:
        a = im2.split()[3].point(lambda v: int(v * alfa))
        im2.putalpha(a)
    quadro.alpha_composite(im2, (int(cx - w / 2), int(cy - h / 2)))


def fundo_grade():
    im = Image.new("RGBA", (W, H), FUNDO + (255,))
    d = ImageDraw.Draw(im)
    fina = tuple(int(a * 0.75 + b * 0.25) for a, b in zip(FUNDO, PRIM))
    for x in range(0, W, 40):
        d.line([(x, 0), (x, H)], fill=fina + (60,), width=1)
    for y in range(0, H, 40):
        d.line([(0, y), (W, y)], fill=fina + (60,), width=1)
    for x in range(0, W, 200):
        d.line([(x, 0), (x, H)], fill=fina + (140,), width=2)
    for y in range(0, H, 200):
        d.line([(0, y), (W, y)], fill=fina + (140,), width=2)
    vin = Image.new("L", (W, H), 0)
    ImageDraw.Draw(vin).ellipse([-300, -200, W + 300, H + 200], fill=255)
    vin = vin.filter(ImageFilter.GaussianBlur(220))
    escuro = Image.new("RGBA", (W, H), (0, 0, 0, 255))
    return Image.composite(im, escuro, vin)


# ---------- quadro ----------
TXT = {
    "h1": texto_img("DE UM ESBOÇO 2D...", F_TITULO, 58, TEXTO),
    "h2": texto_img("...A UMA PEÇA 3D", F_TITULO, 62, DEST),
    "h3": texto_img("MUDE 1 NÚMERO", F_TITULO, 70, TEXTO),
    "h4": texto_img("E TUDO SE ATUALIZA", F_TITULO, 56, DEST),
    "cta1": texto_img("CAD PARAMÉTRICO", F_TITULO, 74, TEXTO),
    "cta2": texto_img("Siga para a parte 2  →", F_MONO, 50, DEST, 6),
    "marca": texto_img("fenkra · CAD 3D livre", F_MONO, 34, (150, 170, 200), 4),
}


def dentes(t):
    passos = [(4 * C / 2 + 0.0, 12)]
    n = 12
    for k, tb in enumerate([2 * C, 2 * C + B, 2 * C + 2 * B, 2 * C + 3 * B]):
        if t >= tb:
            n = 14 + 2 * k
    return n


def quadro(t, bg):
    im = bg.copy()
    d = ImageDraw.Draw(im)
    S = 2  # super-amostragem da peça
    L = 1000
    camada = Image.new("RGBA", (L * S, L * S), (0, 0, 0, 0))
    dc = ImageDraw.Draw(camada)
    cx = cy = L * S / 2

    n = dentes(t)
    r_furo = 70 + 35 * ease_io((t - 3 * C) / 0.8)
    prog_alivio = clamp((t - 3 * C - 0.45) / 0.9)
    cont = contorno_engrenagem(n)
    bur = furos(r_furo, 6, 38, prog_alivio)

    # câmera
    pitch = -math.radians(56) * ease_io((t - C) / 1.0)  # câmera acima e à frente
    yaw = 0.0 if t < C else (t - C) * 0.9
    h = 110 * ease_out((t - C) / 0.7)
    escala = S * 1.25
    # zoom de impacto nas trocas de dente
    for tb in (2 * C, 2 * C + B, 2 * C + 2 * B, 2 * C + 3 * B, C, 4 * C):
        dt = t - tb
        if 0 <= dt < 0.35:
            escala *= 1 + 0.12 * (1 - ease_out(dt / 0.35))
    # chamada final: a peça sobe e diminui
    k_cta = ease_io((t - 4 * C) / 0.6)
    escala *= 1 - 0.38 * k_cta

    if t < C:
        # ESBOÇO: traço progressivo + cotas
        prog = ease_io(t / 1.55)
        pts = [(cx + x * escala, cy - y * escala) for x, y in cont]
        total = len(pts)
        m = int(total * prog)
        if m > 1:
            dc.line(pts[:m] + ([pts[0]] if m >= total else []), fill=PRIM + (255,), width=4 * S, joint="curve")
        if m > 0:
            px, py = pts[m - 1]
            dc.ellipse([px - 9 * S, py - 9 * S, px + 9 * S, py + 9 * S], fill=DEST + (255,))
        if t > 0.9:
            furo = [(cx + x * escala, cy - y * escala) for x, y in bur[0]]
            dc.line(furo + [furo[0]], fill=PRIM + (255,), width=4 * S)
        # linhas de centro tracejadas
        al = int(200 * clamp((t - 0.3) / 0.5))
        if al:
            for a in range(-int(340 * escala), int(340 * escala), 36 * S):
                dc.line([(cx + a, cy), (cx + a + 18 * S, cy)], fill=DEST + (al,), width=2 * S)
                dc.line([(cx, cy + a), (cx, cy + a + 18 * S)], fill=DEST + (al,), width=2 * S)
    else:
        ext, ints, cor = desenhar_peca(dc, cont, bur, h, yaw, pitch, escala, cx, cy, PRIM)
        if ext:
            masc = Image.new("L", camada.size, 0)
            dm = ImageDraw.Draw(masc)
            dm.polygon(ext, fill=255)
            for b in ints:
                dm.polygon(b, fill=0)
            tampa = Image.new("RGBA", camada.size, cor + (255,))
            camada.paste(tampa, (0, 0), masc)
            dc.line(ext + [ext[0]], fill=PRIM + (255,), width=3 * S)
            for b in ints:
                dc.line(b + [b[0]], fill=PRIM + (255,), width=3 * S)
    peca = camada.resize((L, L), Image.LANCZOS)
    # glitch RGB nas trocas de dente
    for tb in (2 * C + B, 2 * C + 2 * B, 2 * C + 3 * B):
        if 0 <= t - tb < 0.1:
            r, g, b, a = peca.split()
            peca = Image.merge("RGBA", (r.transform(peca.size, Image.AFFINE, (1, 0, -10, 0, 1, 0)), g,
                                        b.transform(peca.size, Image.AFFINE, (1, 0, 10, 0, 1, 0)), a))
    oy = 760 - int(170 * k_cta)
    # brilho atrás da peça para destacá-la do fundo
    if t >= C:
        halo = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        ImageDraw.Draw(halo).ellipse([XS + 20 - 330, oy - 330, XS + 20 + 330, oy + 330], fill=PRIM + (55,))
        im.alpha_composite(halo.filter(ImageFilter.GaussianBlur(90)))
    im.alpha_composite(peca, (int(XS + 20 - L / 2), int(oy - L / 2)))

    # cotas do esboço (texto)
    if 0.95 <= t < C:
        f = fonte(F_MONO, 34)
        a = int(255 * clamp((t - 0.95) / 0.2))
        d.text((XS + 250, 760 - 420), f"Ø {2 * 300 // 10}", font=f, fill=DEST + (a,))
        d.text((XS - 380, 760 + 370), f"z = {n}", font=f, fill=DEST + (a,))
    # HUD do parâmetro
    if 2 * C <= t < 4 * C:
        f = fonte(F_MONO, 40)
        a = int(255 * clamp((t - 2 * C) / 0.2) * (1 - clamp((t - 4 * C + 0.25) / 0.25)))
        caixa = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        dk = ImageDraw.Draw(caixa)
        dk.rounded_rectangle([XS - 290, 1150, XS + 290, 1240], 18, fill=(10, 14, 22, int(a * 0.85)), outline=PRIM + (a,), width=3)
        if t < 3 * C:
            rot = f"dentes = {n}"
        else:
            rot = f"furo Ø = {int(2 * r_furo / 10)}"
        dk.text((XS, 1195), rot, font=f, fill=DEST + (a,), anchor="mm")
        im.alpha_composite(caixa)

    # títulos (área segura: y 220-1500)
    ty = 1360
    colar_pop(im, TXT["h1"], XS, ty, (t - 0.1) / 0.5, (t - C + 0.15) / 0.15)
    colar_pop(im, TXT["h2"], XS, ty, (t - C) / 0.5, (t - 2 * C + 0.15) / 0.15)
    colar_pop(im, TXT["h3"], XS, ty, (t - 2 * C) / 0.5, (t - 3 * C + 0.15) / 0.15)
    colar_pop(im, TXT["h4"], XS, ty, (t - 3 * C) / 0.5, (t - 4 * C + 0.12) / 0.12)
    colar_pop(im, TXT["cta1"], XS, 1060, (t - 4 * C) / 0.5)
    colar_pop(im, TXT["cta2"], XS, 1200, (t - 4 * C - 0.65) / 0.5)
    colar_pop(im, TXT["marca"], XS, 1300, (t - 4 * C - 1.0) / 0.5)
    # flash no corte da extrusão
    if 0 <= t - C < 0.08:
        im.alpha_composite(Image.new("RGBA", (W, H), (255, 255, 255, int(90 * (1 - (t - C) / 0.08)))))
    # barra de progresso no topo da área segura
    d = ImageDraw.Draw(im)
    d.rectangle([60, 228, 60 + int(820 * t / DUR), 234], fill=DEST + (255,))
    return im.convert("RGB")


def main():
    saida = sys.argv[1] if len(sys.argv) > 1 else str(PROJ / "render" / "cad_video.mp4")
    Path(saida).parent.mkdir(parents=True, exist_ok=True)
    bg = fundo_grade()
    so = [float(x) for x in sys.argv[2:]]  # tempos avulsos para conferir quadros
    if so:
        for t in so:
            quadro(t, bg).save(Path(saida).with_name(f"quadro_{t:05.2f}.png"))
        return
    ff = subprocess.Popen(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgb24",
                           "-s", f"{W}x{H}", "-r", str(FPS), "-i", "-", "-c:v", "libx264", "-preset", "medium",
                           "-crf", "16", "-pix_fmt", "yuv420p", saida], stdin=subprocess.PIPE)
    n = int(DUR * FPS)
    for i in range(n):
        ff.stdin.write(quadro(i / FPS, bg).tobytes())
        if i % 60 == 0:
            print(f"  quadro {i}/{n}", flush=True)
    ff.stdin.close()
    ff.wait()
    print(f"vídeo: {saida}")


if __name__ == "__main__":
    main()
