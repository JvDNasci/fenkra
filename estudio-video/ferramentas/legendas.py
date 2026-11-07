#!/usr/bin/env python3
"""Transcreve com faster-whisper e gera legendas .json (palavras), .srt e .ass com destaque
palavra a palavra, no estilo das legendas animadas de vídeo vertical.

Uso:
  python ferramentas/legendas.py brutos/A001.mp4 --saida render/A001
  python ferramentas/legendas.py --palavras render/A001.json --saida render/A001   # sem retranscrever
  opções: --modelo small|medium|large-v3  --idioma pt  --por-vez 3  --formato vertical|horizontal
          --paleta graficos/marca/paleta.json  --maiusculas
paleta.json: {"texto": "#FFFFFF", "destaque": "#FFD400", "contorno": "#000000", "fonte": "Montserrat"}
"""
import argparse
import json
import sys
from pathlib import Path

PADRAO = {"texto": "#FFFFFF", "destaque": "#FFD400", "contorno": "#000000", "fonte": "Montserrat"}


def transcrever(arquivo, modelo, idioma):
    try:
        from faster_whisper import WhisperModel
    except ImportError:
        sys.exit("instale com: pip install faster-whisper")
    m = WhisperModel(modelo, device="auto", compute_type="auto")
    segmentos, _ = m.transcribe(str(arquivo), language=idioma, word_timestamps=True, vad_filter=True)
    return [
        {"palavra": w.word.strip(), "ini": round(w.start, 3), "fim": round(w.end, 3)}
        for s in segmentos for w in (s.words or []) if w.word.strip()
    ]


def agrupar(palavras, por_vez):
    """Blocos de até `por_vez` palavras; quebra também em pontuação final e em pausas longas."""
    blocos, atual = [], []
    for i, p in enumerate(palavras):
        atual.append(p)
        prox = palavras[i + 1] if i + 1 < len(palavras) else None
        pausa = prox is not None and prox["ini"] - p["fim"] > 0.6
        if len(atual) >= por_vez or p["palavra"][-1:] in ".!?" or pausa or prox is None:
            blocos.append(atual)
            atual = []
    return blocos


def tempo_srt(t):
    ms = int(round(t * 1000))
    return f"{ms // 3600000:02}:{ms // 60000 % 60:02}:{ms // 1000 % 60:02},{ms % 1000:03}"


def tempo_ass(t):
    cs = int(round(t * 100))
    return f"{cs // 360000}:{cs // 6000 % 60:02}:{cs // 100 % 60:02}.{cs % 100:02}"


def cor_ass(hexa):
    h = hexa.lstrip("#")
    return f"&H00{h[4:6]}{h[2:4]}{h[0:2]}".upper()


def escrever_srt(blocos, destino):
    linhas = []
    for n, b in enumerate(blocos, 1):
        texto = " ".join(p["palavra"] for p in b)
        linhas += [str(n), f"{tempo_srt(b[0]['ini'])} --> {tempo_srt(b[-1]['fim'])}", texto, ""]
    destino.write_text("\n".join(linhas), encoding="utf-8")


def escrever_ass(blocos, destino, paleta, formato, maiusculas):
    w, h = (1080, 1920) if formato == "vertical" else (1920, 1080)
    tam, margem = (78, 560) if formato == "vertical" else (64, 90)
    cab = f"""[Script Info]
ScriptType: v4.00+
PlayResX: {w}
PlayResY: {h}
WrapStyle: 0
ScaledBorderAndShadow: yes

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Fala,{paleta['fonte']},{tam},{cor_ass(paleta['texto'])},{cor_ass(paleta['destaque'])},{cor_ass(paleta['contorno'])},&H80000000,-1,0,0,0,100,100,0,0,1,6,2,2,80,160,{margem},1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
"""
    dest = cor_ass(paleta["destaque"])
    eventos = []
    for b in blocos:
        for i, p in enumerate(b):
            ini = p["ini"]
            fim = b[i + 1]["ini"] if i + 1 < len(b) else p["fim"]
            if fim <= ini:
                fim = ini + 0.05
            partes = []
            for j, q in enumerate(b):
                t = q["palavra"].upper() if maiusculas else q["palavra"]
                t = t.replace("{", "(").replace("}", ")")
                if j == i:
                    # palavra falada: cor de destaque e um pop de escala
                    t = "{\\c" + dest + "\\fscx112\\fscy112}" + t + "{\\r}"
                partes.append(t)
            eventos.append(f"Dialogue: 0,{tempo_ass(ini)},{tempo_ass(fim)},Fala,,0,0,0,,{' '.join(partes)}")
    destino.write_text(cab + "\n".join(eventos) + "\n", encoding="utf-8")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("entrada", nargs="?", help="vídeo ou áudio a transcrever")
    ap.add_argument("--palavras", help="json de palavras já transcrito")
    ap.add_argument("--saida", required=True, help="prefixo dos arquivos de saída")
    ap.add_argument("--modelo", default="small")
    ap.add_argument("--idioma", default="pt")
    ap.add_argument("--por-vez", type=int, default=3)
    ap.add_argument("--formato", choices=["vertical", "horizontal"], default="vertical")
    ap.add_argument("--paleta")
    ap.add_argument("--maiusculas", action="store_true")
    a = ap.parse_args()

    if a.palavras:
        palavras = json.loads(Path(a.palavras).read_text(encoding="utf-8"))
    elif a.entrada:
        palavras = transcrever(a.entrada, a.modelo, a.idioma)
    else:
        ap.error("informe a entrada ou --palavras")
    if not palavras:
        sys.exit("nenhuma palavra encontrada")

    paleta = dict(PADRAO)
    if a.paleta:
        paleta.update(json.loads(Path(a.paleta).read_text(encoding="utf-8")))

    pref = Path(a.saida)
    pref.parent.mkdir(parents=True, exist_ok=True)
    if not a.palavras:
        pref.with_suffix(".json").write_text(json.dumps(palavras, ensure_ascii=False, indent=1), encoding="utf-8")
    blocos = agrupar(palavras, a.por_vez)
    escrever_srt(blocos, pref.with_suffix(".srt"))
    escrever_ass(blocos, pref.with_suffix(".ass"), paleta, a.formato, a.maiusculas)
    print(f"{len(palavras)} palavras, {len(blocos)} blocos -> {pref}.srt / {pref}.ass")


if __name__ == "__main__":
    main()
