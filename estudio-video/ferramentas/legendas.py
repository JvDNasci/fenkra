#!/usr/bin/env python3
"""Transcreve com faster-whisper e gera legendas .json (palavras), .srt e .ass com destaque
palavra a palavra, no estilo das legendas animadas de vídeo vertical.

Uso:
  python ferramentas/legendas.py brutos/A001.mp4 --saida render/A001
  python ferramentas/legendas.py --palavras render/A001.json --saida render/A001   # sem retranscrever
  opções: --modelo small|medium|large-v3  --idioma pt  --por-vez 3  --formato vertical|horizontal
          --estilo limpo|impacto|podcast|caixa|charme|manuscrito|divertido|tech  --fonte "Nome"
          --paleta graficos/marca/paleta.json  --maiusculas
As fontes vêm de biblioteca/fontes (ferramentas/baixar_fontes.py); video.sh legendar já as encontra.
paleta.json: {"texto": "#FFFFFF", "destaque": "#FFD400", "contorno": "#000000", "fonte": "Poppins ExtraBold"}
"""
import argparse
import json
import sys
from pathlib import Path

PADRAO = {"texto": "#FFFFFF", "destaque": "#FFD400", "contorno": "#000000", "fonte": "Poppins ExtraBold"}

# Estilos de legenda. fonte_enfase troca a fonte só da palavra falada (variar sem perder leitura).
# borda: 1 = contorno + sombra, 3 = caixa sólida atrás do texto (cor = contorno).
ESTILOS = {
    "limpo": dict(fonte="Poppins ExtraBold", fonte_enfase=None, maiusculas=False, borda=1, contorno_px=5, sombra=2, por_vez=3, escala=112),
    "impacto": dict(fonte="Anton", fonte_enfase=None, maiusculas=True, borda=1, contorno_px=7, sombra=3, por_vez=2, escala=120),
    "podcast": dict(fonte="Archivo Black", fonte_enfase=None, maiusculas=True, borda=1, contorno_px=8, sombra=0, por_vez=3, escala=108),
    "caixa": dict(fonte="Poppins ExtraBold", fonte_enfase=None, maiusculas=False, borda=3, contorno_px=14, sombra=0, por_vez=4, escala=100),
    "charme": dict(fonte="Poppins SemiBold", fonte_enfase="DM Serif Display", maiusculas=False, borda=1, contorno_px=3, sombra=2, por_vez=4, escala=125),
    "manuscrito": dict(fonte="Poppins ExtraBold", fonte_enfase="Permanent Marker", maiusculas=False, borda=1, contorno_px=5, sombra=2, por_vez=3, escala=118),
    "divertido": dict(fonte="Lilita One", fonte_enfase="Bangers", maiusculas=True, borda=1, contorno_px=7, sombra=3, por_vez=2, escala=125),
    "tech": dict(fonte="Space Mono", fonte_enfase=None, maiusculas=True, borda=3, contorno_px=10, sombra=0, por_vez=3, escala=100),
}


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


def escrever_ass(blocos, destino, paleta, formato, maiusculas, estilo):
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
Style: Fala,{estilo['fonte']},{tam},{cor_ass(paleta['texto'])},{cor_ass(paleta['destaque'])},{cor_ass(paleta['contorno'])},&H80000000,0,0,0,0,100,100,0,0,{estilo['borda']},{estilo['contorno_px']},{estilo['sombra']},2,80,160,{margem},1

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
                t = q["palavra"].upper() if (maiusculas or estilo["maiusculas"]) else q["palavra"]
                t = t.replace("{", "(").replace("}", ")")
                if j == i:
                    # palavra falada: cor de destaque e um pop de escala
                    fn = "\\fn" + estilo["fonte_enfase"] if estilo["fonte_enfase"] else ""
                    esc = estilo["escala"]
                    # pop: entra 8% maior e assenta na escala do estilo em 80 ms
                    t = ("{\\c" + dest + fn + f"\\fscx{esc + 8}\\fscy{esc + 8}\\t(0,80,\\fscx{esc}\\fscy{esc})" + "}"
                         + t + "{\\r}")
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
    ap.add_argument("--por-vez", type=int, help="palavras por vez (padrão: do estilo)")
    ap.add_argument("--estilo", choices=ESTILOS, default="limpo")
    ap.add_argument("--fonte", help="troca a fonte base do estilo")
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
    estilo = dict(ESTILOS[a.estilo])
    if a.fonte:
        estilo["fonte"] = a.fonte
    elif a.paleta and "fonte" in paleta:
        estilo["fonte"] = paleta["fonte"]
    blocos = agrupar(palavras, a.por_vez or estilo["por_vez"])
    escrever_srt(blocos, pref.with_suffix(".srt"))
    escrever_ass(blocos, pref.with_suffix(".ass"), paleta, a.formato, a.maiusculas, estilo)
    print(f"{len(palavras)} palavras, {len(blocos)} blocos -> {pref}.srt / {pref}.ass")


if __name__ == "__main__":
    main()
