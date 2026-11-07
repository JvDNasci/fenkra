#!/usr/bin/env python3
"""Remove pausas de um vídeo falado (corte bruto), deixando uma folga em volta da fala.

Uso: python ferramentas/cortar_silencios.py entrada.mp4 saida.mp4 [--limiar -35] [--min 0.35] [--folga 0.08]
  --limiar  nível em dB abaixo do qual conta como silêncio
  --min     duração mínima da pausa para cortar (s)
  --folga   quanto manter antes e depois de cada fala (s)
  --listar  só mostra os trechos mantidos, sem renderizar
"""
import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path


def num(s):
    return float(str(s).replace(",", "."))


def duracao(arquivo):
    r = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", arquivo],
                       capture_output=True, text=True, check=True)
    return float(r.stdout.strip())


def silencios(arquivo, limiar, minimo):
    r = subprocess.run(["ffmpeg", "-hide_banner", "-nostats", "-i", arquivo, "-vn",
                        "-af", f"silencedetect=noise={limiar}dB:d={minimo}", "-f", "null", "-"],
                       capture_output=True, text=True)
    ini = [float(x) for x in re.findall(r"silence_start: (-?[\d.]+)", r.stderr)]
    fim = [float(x) for x in re.findall(r"silence_end: ([\d.]+)", r.stderr)]
    return list(zip(ini, fim + [None] * (len(ini) - len(fim))))


def trechos_mantidos(total, pausas, folga):
    trechos, cursor = [], 0.0
    for a, b in pausas:
        b = total if b is None else b
        corte_ini, corte_fim = max(a + folga, 0.0), min(b - folga, total)
        if corte_fim - corte_ini <= 0.05:
            continue
        if corte_ini > cursor:
            trechos.append((cursor, corte_ini))
        cursor = corte_fim
    if cursor < total:
        trechos.append((cursor, total))
    return [(round(a, 3), round(b, 3)) for a, b in trechos if b - a > 0.05]


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("entrada")
    ap.add_argument("saida", nargs="?")
    ap.add_argument("--limiar", default="-35")
    ap.add_argument("--min", default="0.35")
    ap.add_argument("--folga", default="0.08")
    ap.add_argument("--listar", action="store_true")
    a = ap.parse_args()

    total = duracao(a.entrada)
    trechos = trechos_mantidos(total, silencios(a.entrada, num(a.limiar), num(a.min)), num(a.folga))
    mantido = sum(b - x for x, b in trechos)
    print(f"{len(trechos)} trechos, {mantido:.1f} s de {total:.1f} s ({100 * mantido / total:.0f}%)")
    if a.listar or not a.saida:
        for x, b in trechos:
            print(f"  {x:8.3f} -> {b:8.3f}")
        return
    if not trechos:
        sys.exit("nada sobrou depois do corte; aumente --limiar (ex.: -45)")

    partes, rotulos = [], []
    for i, (x, b) in enumerate(trechos):
        partes.append(f"[0:v]trim={x}:{b},setpts=PTS-STARTPTS[v{i}];"
                      f"[0:a]atrim={x}:{b},asetpts=PTS-STARTPTS,afade=t=in:d=0.01,"
                      f"afade=t=out:st={max(b - x - 0.01, 0)}:d=0.01[a{i}];")
        rotulos.append(f"[v{i}][a{i}]")
    grafo = "".join(partes) + "".join(rotulos) + f"concat=n={len(trechos)}:v=1:a=1[v][a]"
    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False) as f:
        f.write(grafo)
        script = f.name
    try:
        subprocess.run(["ffmpeg", "-hide_banner", "-y", "-i", a.entrada, "-filter_complex_script", script,
                        "-map", "[v]", "-map", "[a]", "-c:v", "libx264", "-preset", "medium", "-crf", "16",
                        "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "320k", "-movflags", "+faststart",
                        a.saida], check=True)
    finally:
        Path(script).unlink(missing_ok=True)


if __name__ == "__main__":
    main()
