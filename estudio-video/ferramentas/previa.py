#!/usr/bin/env python3
"""Pré-renderiza uma prévia (animatic) do vídeo a partir do storyboard, para aprovar ritmo, textos,
cores e sons antes do render final.

Uso: python ferramentas/previa.py planos.csv saida.mp4 [--formato vertical|horizontal] [--trilha audio/trilha.wav]
     [--paleta graficos/marca/paleta.json] [--alta]   (--alta = resolução final em vez de meia resolução)

planos.csv (cabeçalho obrigatório; tempos em segundos; campos vazios são permitidos):
  inicio,fim,texto,cor,midia,sfx
  0,1.5,VOCÊ ERRA ISSO,#101820,,impacto/hit_forte_1.wav
  1.5,4,Contexto do problema,,brutos/A001.mp4@12.0,transicao/whoosh_medio_1.wav
- midia: imagem ou vídeo (vídeo aceita @segundo de início). Sem mídia, entra um cartão de cor.
- sfx: caminho relativo a biblioteca/sfx ou absoluto; toca no início do plano.
"""
import argparse
import csv
import json
import subprocess
import sys
import tempfile
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
FONTE = RAIZ / "biblioteca" / "fontes" / "limpo" / "poppins" / "Poppins-ExtraBold.ttf"
IMAGENS = {".png", ".jpg", ".jpeg", ".webp"}


def ff(*args):
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", *args], check=True)


def esc(caminho):
    return str(caminho).replace("\\", "/").replace(":", "\\:").replace("'", "\\'")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("planos")
    ap.add_argument("saida")
    ap.add_argument("--formato", choices=["vertical", "horizontal"], default="vertical")
    ap.add_argument("--trilha")
    ap.add_argument("--paleta")
    ap.add_argument("--alta", action="store_true")
    a = ap.parse_args()

    W, H = (1080, 1920) if a.formato == "vertical" else (1920, 1080)
    if not a.alta:
        W, H = W // 2, H // 2
    pal = {"fundo": "#14161A", "texto": "#FFFFFF", "destaque": "#FFD400"}
    if a.paleta:
        pal.update(json.loads(Path(a.paleta).read_text(encoding="utf-8")))
    fonte = FONTE if FONTE.exists() else None
    if not fonte:
        print("aviso: rode ferramentas/baixar_fontes.py para usar a fonte do estúdio", file=sys.stderr)

    with open(a.planos, encoding="utf-8") as f:
        planos = list(csv.DictReader(f))
    if not planos:
        sys.exit("planos.csv vazio")
    base = Path(a.planos).resolve().parent
    tmp = Path(tempfile.mkdtemp(prefix="previa_"))
    lista, sons = [], []
    for i, p in enumerate(planos):
        ini, fim = float(p["inicio"]), float(p["fim"])
        dur = max(0.04, fim - ini)
        seg = tmp / f"p{i:03}.mp4"
        txt = tmp / f"t{i:03}.txt"
        txt.write_text((p.get("texto") or "").strip(), encoding="utf-8")
        cor = (p.get("cor") or pal["fundo"]).strip()
        midia = (p.get("midia") or "").strip()
        fontopt = f":fontfile='{esc(fonte)}'" if fonte else ""
        rotulo = (f"drawtext=text='PRÉVIA  plano {i + 1}  %{{pts\\:hms}}'{fontopt}:fontsize={H // 60}:"
                  f"fontcolor=white@0.8:x=20:y=20:box=1:boxcolor=black@0.5:boxborderw=8")
        texto = (f"drawtext=textfile='{esc(txt)}'{fontopt}:fontsize={W // 11}:fontcolor={pal['texto']}:"
                 f"borderw={max(2, W // 180)}:bordercolor=black:x=(w-tw)/2:y=h*0.62-th/2:line_spacing=10")
        ajuste = f"scale={W}:{H}:force_original_aspect_ratio=increase,crop={W}:{H},setsar=1,fps=30"
        vf = f"{ajuste},{texto},{rotulo},format=yuv420p"
        if midia:
            caminho, _, inicio_m = midia.partition("@")
            m = Path(caminho)
            m = m if m.is_absolute() else base / m
            if m.suffix.lower() in IMAGENS:
                ff("-loop", "1", "-t", f"{dur}", "-i", str(m),
                   "-vf", f"{ajuste},zoompan=z='min(zoom+0.0012,1.15)':d=1:s={W}x{H}:fps=30,{texto},{rotulo},format=yuv420p",
                   "-an", "-c:v", "libx264", "-preset", "veryfast", "-crf", "26", str(seg))
            else:
                ff("-ss", inicio_m or "0", "-t", f"{dur}", "-i", str(m), "-vf", vf, "-an",
                   "-c:v", "libx264", "-preset", "veryfast", "-crf", "26", str(seg))
        else:
            ff("-f", "lavfi", "-i", f"color=c={cor}:s={W}x{H}:r=30:d={dur}", "-vf", vf, "-an",
               "-c:v", "libx264", "-preset", "veryfast", "-crf", "26", str(seg))
        lista.append(f"file '{seg.as_posix()}'")
        sfx = (p.get("sfx") or "").strip()
        if sfx:
            sp = Path(sfx)
            sp = sp if sp.is_absolute() else RAIZ / "biblioteca" / "sfx" / sp
            if sp.exists():
                sons.append((sp, ini))
            else:
                print(f"aviso: sfx não encontrado: {sp}", file=sys.stderr)
    (tmp / "lista.txt").write_text("\n".join(lista), encoding="utf-8")
    video = tmp / "video.mp4"
    ff("-f", "concat", "-safe", "0", "-i", str(tmp / "lista.txt"), "-c", "copy", str(video))
    total = float(planos[-1]["fim"])

    entradas, filtros, rot = ["-i", str(video)], [], []
    k = 1
    if a.trilha:
        entradas += ["-i", a.trilha]
        filtros.append(f"[{k}:a]atrim=0:{total},volume=0.6[t]")
        rot.append("[t]")
        k += 1
    for sp, t0 in sons:
        entradas += ["-i", str(sp)]
        ms = int(t0 * 1000)
        filtros.append(f"[{k}:a]adelay={ms}|{ms}[s{k}]")
        rot.append(f"[s{k}]")
        k += 1
    if rot:
        filtros.append("".join(rot) + f"amix=inputs={len(rot)}:normalize=0:duration=longest,atrim=0:{total}[a]")
        ff(*entradas, "-filter_complex", ";".join(filtros), "-map", "0:v", "-map", "[a]",
           "-c:v", "copy", "-c:a", "aac", "-b:a", "160k", "-shortest", a.saida)
    else:
        ff("-i", str(video), "-c", "copy", a.saida)
    print(f"prévia: {a.saida} ({len(planos)} planos, {total:.1f} s, {W}x{H})")


if __name__ == "__main__":
    main()
