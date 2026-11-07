#!/usr/bin/env python3
"""Gera LUTs 3D (.cube) dos looks de cor do estúdio, para aplicar com `video.sh cor`.

Uso: python ferramentas/lut.py [--pasta biblioteca/luts] [--tamanho 33]
Looks: natural-vivo, quente, frio, teal-laranja, vintage, pastel, pb-contraste, noturno-neon, tech-limpo
São transformações matemáticas próprias (curvas, matiz e saturação): livres para qualquer uso.
"""
import argparse
from pathlib import Path

import numpy as np


def luma(rgb):
    return rgb @ np.array([0.2126, 0.7152, 0.0722])


def saturar(rgb, k):
    y = luma(rgb)[..., None]
    return y + (rgb - y) * k


def curva_s(x, forca):
    """Curva em S suave: forca 0 = linear."""
    return (1 - forca) * x + forca * (3 * x ** 2 - 2 * x ** 3)


def tons(rgb, sombras, altas):
    """Tinge sombras e altas-luzes com cores (deltas RGB)."""
    y = luma(rgb)[..., None]
    return rgb + (1 - y) ** 2 * np.array(sombras) + y ** 2 * np.array(altas)


def levantar_pretos(rgb, k):
    return k + rgb * (1 - k)


LOOKS = {
    "natural-vivo": lambda c: saturar(curva_s(c, 0.25), 1.15),
    "quente": lambda c: saturar(tons(curva_s(c, 0.2), [0.02, 0.0, -0.03], [0.05, 0.02, -0.04]), 1.08),
    "frio": lambda c: saturar(tons(curva_s(c, 0.2), [-0.02, 0.0, 0.04], [-0.03, 0.0, 0.03]), 0.95),
    "teal-laranja": lambda c: saturar(tons(curva_s(c, 0.3), [-0.04, 0.03, 0.06], [0.06, 0.02, -0.05]), 1.1),
    "vintage": lambda c: saturar(tons(levantar_pretos(curva_s(c, 0.15), 0.06), [0.03, 0.02, -0.02], [0.04, 0.02, -0.06]), 0.8),
    "pastel": lambda c: saturar(levantar_pretos(c, 0.08) * 0.97 + 0.03, 0.75),
    "pb-contraste": lambda c: np.repeat(curva_s(luma(c), 0.45)[..., None], 3, axis=-1),
    "noturno-neon": lambda c: saturar(tons(curva_s(c, 0.35), [0.02, -0.02, 0.07], [0.05, -0.02, 0.06]), 1.3),
    "tech-limpo": lambda c: saturar(tons(curva_s(c, 0.2), [-0.01, 0.01, 0.03], [0.0, 0.01, 0.02]), 1.0),
}


def escrever_cube(caminho, nome, fn, n):
    g = np.linspace(0, 1, n)
    # ordem do .cube: R varia mais rápido, depois G, depois B
    b, gg, r = np.meshgrid(g, g, g, indexing="ij")
    rgb = np.stack([r, gg, b], axis=-1).reshape(-1, 3)
    out = np.clip(fn(rgb), 0, 1)
    with open(caminho, "w", encoding="ascii") as f:
        f.write(f'TITLE "{nome}"\nLUT_3D_SIZE {n}\nDOMAIN_MIN 0 0 0\nDOMAIN_MAX 1 1 1\n')
        for v in out:
            f.write(f"{v[0]:.6f} {v[1]:.6f} {v[2]:.6f}\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--pasta", default=str(Path(__file__).resolve().parent.parent / "biblioteca" / "luts"))
    ap.add_argument("--tamanho", type=int, default=33)
    a = ap.parse_args()
    pasta = Path(a.pasta)
    pasta.mkdir(parents=True, exist_ok=True)
    for nome, fn in LOOKS.items():
        escrever_cube(pasta / f"{nome}.cube", nome, fn, a.tamanho)
    print(f"{len(LOOKS)} LUTs em {pasta}")


if __name__ == "__main__":
    main()
