#!/usr/bin/env python3
"""Cria a paleta de vídeo de uma marca ou canal e confere o contraste das legendas.

Uso:
  python ferramentas/paleta.py --cor "#1E6FFF" --clima tecnologico --saida graficos/marca
  python ferramentas/paleta.py --imagem logo.png --saida graficos/marca        # extrai do logo ou do site
  climas: tecnologico, charmoso, energetico, calmo, luxo, natural, divertido
Saída: paleta.json (lido por legendas.py e pelos efeitos), paleta.png (amostra) e o relatório de contraste.
Regra 60-30-10: fundo 60%, secundária 30%, destaque 10% (só no que deve chamar atenção).
"""
import argparse
import colorsys
import json
import subprocess
from pathlib import Path

import numpy as np

# clima → (giro de matiz da secundária, giro do destaque, saturação, luminosidade do fundo)
CLIMAS = {
    "tecnologico": (30, 180, 0.85, 0.10),
    "charmoso": (-30, 150, 0.55, 0.95),
    "energetico": (40, 180, 1.0, 0.08),
    "calmo": (25, 160, 0.35, 0.94),
    "luxo": (15, 40, 0.45, 0.07),
    "natural": (35, 200, 0.5, 0.93),
    "divertido": (120, 240, 0.95, 0.97),
}


def hex_rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))


def rgb_hex(c):
    return "#" + "".join(f"{int(round(max(0, min(1, v)) * 255)):02X}" for v in c)


def hls(c, dh=0, l=None, s=None):
    h, ll, ss = colorsys.rgb_to_hls(*c)
    return colorsys.hls_to_rgb((h + dh / 360) % 1, ll if l is None else l, ss if s is None else s)


def lum_rel(c):
    v = [x / 12.92 if x <= 0.03928 else ((x + 0.055) / 1.055) ** 2.4 for x in c]
    return 0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]


def contraste(a, b):
    la, lb = sorted((lum_rel(a), lum_rel(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def extrair(imagem, k=5):
    bruto = subprocess.run(["ffmpeg", "-v", "error", "-i", imagem, "-vf", "scale=64:64", "-f", "rawvideo",
                            "-pix_fmt", "rgb24", "-"], capture_output=True, check=True).stdout
    px = np.frombuffer(bruto, np.uint8).reshape(-1, 3) / 255
    rng = np.random.default_rng(0)
    cent = px[rng.choice(len(px), k, replace=False)]
    for _ in range(20):
        rot = np.argmin(((px[:, None] - cent[None]) ** 2).sum(-1), axis=1)
        cent = np.array([px[rot == i].mean(0) if np.any(rot == i) else cent[i] for i in range(k)])
    conta = np.bincount(rot, minlength=k)
    ordem = np.argsort(-conta)
    return [tuple(cent[i]) for i in ordem]


def montar(primaria, clima, extraidas=None):
    dh2, dh3, sat, l_fundo = CLIMAS[clima]
    escuro = l_fundo < 0.5
    p = hls(primaria, s=max(colorsys.rgb_to_hls(*primaria)[2], sat * 0.8))
    sec = extraidas[1] if extraidas and len(extraidas) > 1 else hls(p, dh2, s=sat * 0.7)
    dest = hls(p, dh3, l=0.58 if escuro else 0.48, s=min(1, sat + 0.1))
    fundo = hls(p, 0, l=l_fundo, s=0.25 if escuro else 0.15)
    texto = (0.98, 0.98, 0.98) if escuro else (0.07, 0.07, 0.09)
    contorno = (0, 0, 0) if escuro else (1, 1, 1)
    # ajusta a luminosidade do destaque até ler bem sobre o fundo (≥ 4,5:1)
    passo = 0.03 if escuro else -0.03
    while contraste(dest, fundo) < 4.5:
        h_, l_, s_ = colorsys.rgb_to_hls(*dest)
        if not 0.05 < l_ + passo < 0.95:
            break
        dest = colorsys.hls_to_rgb(h_, l_ + passo, s_)
    return {"primaria": p, "secundaria": sec, "destaque": dest, "fundo": fundo, "texto": texto,
            "contorno": contorno}


def amostra(paleta, destino):
    cores = list(paleta.values())
    w, h = 180, 180
    img = np.zeros((h, w * len(cores), 3), np.uint8)
    for i, c in enumerate(cores):
        img[:, i * w:(i + 1) * w] = (np.array(c) * 255).astype(np.uint8)
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{img.shape[1]}x{h}",
                    "-i", "-", str(destino)], input=img.tobytes(), check=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--cor", help="cor principal da marca em hex")
    ap.add_argument("--imagem", help="logo, print do site ou foto de referência")
    ap.add_argument("--clima", choices=CLIMAS, default="tecnologico")
    ap.add_argument("--fonte", default="Poppins ExtraBold")
    ap.add_argument("--saida", required=True)
    a = ap.parse_args()
    extraidas = extrair(a.imagem) if a.imagem else None
    if a.cor:
        primaria = hex_rgb(a.cor)
    elif extraidas:
        # a mais saturada entre as dominantes costuma ser a cor da marca
        primaria = max(extraidas, key=lambda c: colorsys.rgb_to_hls(*c)[2] * (0.2 < colorsys.rgb_to_hls(*c)[1] < 0.8))
    else:
        ap.error("informe --cor ou --imagem")
    pal = montar(primaria, a.clima, extraidas)
    saida = Path(a.saida)
    saida.mkdir(parents=True, exist_ok=True)
    js = {k: rgb_hex(v) for k, v in pal.items()}
    js["fonte"] = a.fonte
    js["clima"] = a.clima
    (saida / "paleta.json").write_text(json.dumps(js, indent=2, ensure_ascii=False), encoding="utf-8")
    amostra(pal, saida / "paleta.png")
    print(json.dumps(js, indent=2, ensure_ascii=False))
    print("\nContraste (mínimo 4,5 para legenda, 3,0 para título grande):")
    for a1, a2 in (("texto", "fundo"), ("destaque", "fundo"), ("texto", "contorno"), ("primaria", "fundo")):
        r = contraste(pal[a1], pal[a2])
        print(f"  {a1:9s} sobre {a2:9s} {r:5.2f}  {'ok' if r >= 4.5 else 'só título' if r >= 3 else 'FRACO'}")


if __name__ == "__main__":
    main()
