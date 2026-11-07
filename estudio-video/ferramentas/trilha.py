#!/usr/bin/env python3
"""Compõe uma trilha original por algoritmo, no clima do vídeo. Livre de direitos: é obra do estúdio.

Uso: python ferramentas/trilha.py --clima tecnologico --duracao 10 --saida audio/trilha.wav [--bpm 120] [--semente 1]
Climas: tecnologico, energetico, charmoso, calmo, emotivo, epico
Também grava <saida>.batidas.txt com o tempo de cada batida e de cada compasso, para alinhar cortes,
efeitos e animações ao ritmo (os cortes caem na batida; os impactos, no primeiro tempo do compasso).
"""
import argparse
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sintese as s  # noqa: E402

NOTAS = {"C": 0, "C#": 1, "D": 2, "D#": 3, "E": 4, "F": 5, "F#": 6, "G": 7, "G#": 8, "A": 9, "A#": 10, "B": 11}

# clima: bpm, tônica, escala, progressão (graus), timbre do lead, bateria, brilho
CLIMAS = {
    "tecnologico": dict(bpm=118, tonica="A", modo="menor", prog=[0, 5, 2, 6], lead="arpejo", bat="eletronica", brilho=0.9),
    "energetico": dict(bpm=128, tonica="F", modo="maior", prog=[0, 4, 5, 3], lead="arpejo", bat="quatro", brilho=1.0),
    "charmoso": dict(bpm=104, tonica="D", modo="maior", prog=[0, 5, 3, 4], lead="dedilhado", bat="palmas", brilho=0.8),
    "calmo": dict(bpm=84, tonica="E", modo="maior", prog=[0, 3, 5, 4], lead="sino", bat="lofi", brilho=0.5),
    "emotivo": dict(bpm=72, tonica="C", modo="menor", prog=[0, 5, 2, 6], lead="piano", bat="nenhuma", brilho=0.55),
    "epico": dict(bpm=96, tonica="D", modo="menor", prog=[0, 5, 6, 4], lead="metais", bat="tambores", brilho=0.8),
}
ESCALAS = {"maior": [0, 2, 4, 5, 7, 9, 11], "menor": [0, 2, 3, 5, 7, 8, 10]}


def hz(midi):
    return 440.0 * 2 ** ((midi - 69) / 12)


def acorde(tonica_midi, escala, grau, voicing=(0, 2, 4)):
    notas = []
    for v in voicing:
        g = grau + v
        notas.append(tonica_midi + escala[g % 7] + 12 * (g // 7))
    return notas


def colocar(buf, som, t0, ganho=1.0):
    i = int(t0 * s.SR)
    if i >= len(buf):
        return
    n = min(len(som), len(buf) - i)
    buf[i:i + n] += som[:n] * ganho


def kick(dur=0.35):
    return np.tanh(3 * s.seno_varredura(150, 42, dur) * s.env(int(s.SR * dur), 0.001, 0.12))


def caixa(rng, dur=0.22):
    n = int(s.SR * dur)
    corpo = np.sin(2 * np.pi * 190 * s.t(dur)) * s.env(n, 0.001, 0.05)
    chiado = s.passa_alta_fixo(rng.uniform(-1, 1, n), 1500) * s.env(n, 0.001, 0.09)
    return 0.5 * corpo + 0.8 * chiado


def palma(rng, dur=0.2):
    n = int(s.SR * dur)
    x = s.passa_alta_fixo(rng.uniform(-1, 1, n), 900)
    e = np.zeros(n)
    for k, atraso in enumerate((0, 0.011, 0.022)):
        j = int(atraso * s.SR)
        e[j:] += s.env(n - j, 0.0005, 0.03 if k < 2 else 0.12)
    return x * e * 0.6


def chimbal(rng, dur=0.06, aberto=False):
    dur = 0.25 if aberto else dur
    n = int(s.SR * dur)
    return s.passa_alta_fixo(rng.uniform(-1, 1, n), 7000) * s.env(n, 0.0005, dur * 0.4) * 0.5


def nota_lead(tipo, f, dur, brilho):
    n = int(s.SR * dur)
    tt = s.t(dur)
    if tipo == "arpejo":
        y = s.serra(f, dur, 0.004)
        y = s.passa_baixa_fixo(y, 1500 + 3500 * brilho) * s.env(n, 0.002, 0.12)
    elif tipo == "dedilhado":
        y = (np.sin(2 * np.pi * f * tt) + 0.5 * np.sin(4 * np.pi * f * tt) + 0.25 * np.sin(6 * np.pi * f * tt))
        y *= s.env(n, 0.001, 0.18)
    elif tipo == "sino":
        y = (np.sin(2 * np.pi * f * tt) + 0.3 * np.sin(2 * np.pi * f * 2.0 * tt)) * s.env(n, 0.003, 0.5)
    elif tipo == "piano":
        y = sum(np.sin(2 * np.pi * f * k * tt) / k ** 1.6 for k in range(1, 6)) * s.env(n, 0.002, 0.6)
    else:  # metais
        y = s.passa_baixa_fixo(s.serra(f, dur, 0.006), 1200 + 1500 * brilho) * s.adsr(n, 0.04, 0.1, 0.8, 0.15)
    return y


def pad(freqs, dur, brilho):
    n = int(s.SR * dur)
    y = sum(s.serra(f, dur, 0.007) for f in freqs) / len(freqs)
    return s.passa_baixa_fixo(y, 600 + 1400 * brilho) * s.adsr(n, min(0.4, dur / 4), 0.2, 0.8, min(0.5, dur / 4))


def baixo(f, dur, brilho):
    n = int(s.SR * dur)
    y = s.serra(f, dur) * 0.6 + np.sin(2 * np.pi * f * s.t(dur)) * 0.6
    return s.passa_baixa_fixo(y, 260 + 300 * brilho) * s.adsr(n, 0.005, 0.08, 0.7, 0.05)


def compor(clima, duracao, bpm=None, semente=1):
    c = dict(CLIMAS[clima])
    bpm = bpm or c["bpm"]
    rng = np.random.default_rng(semente)
    batida = 60 / bpm
    compasso = 4 * batida
    total = duracao + 1.5  # cauda
    n = int(total * s.SR)
    bat, harm, lead_b, grave = (np.zeros(n) for _ in range(4))
    escala = ESCALAS[c["modo"]]
    tonica = 48 + NOTAS[c["tonica"]]  # C3 + tônica
    n_comp = int(np.ceil(duracao / compasso))
    for k in range(n_comp):
        t0 = k * compasso
        grau = c["prog"][k % len(c["prog"])]
        ac = acorde(tonica + 12, escala, grau)
        intro = k == 0
        colocar(harm, pad([hz(m) for m in ac], compasso + 0.3, c["brilho"]), t0, 0.5 if intro else 0.7)
        if not intro or clima in ("calmo", "emotivo"):
            for b in range(4):
                colocar(grave, baixo(hz(tonica - 12 + escala[grau % 7]), batida * 0.9, c["brilho"]), t0 + b * batida, 0.9)
        # lead: arpejo em colcheias ou semicolcheias
        passo = batida / (4 if c["lead"] == "arpejo" else 2)
        notas_arp = acorde(tonica + 24, escala, grau, (0, 2, 4, 7))
        padrao = [0, 1, 2, 3, 2, 1] if c["lead"] != "piano" else [0, 2, 1, 3]
        j = 0
        tt = t0
        while tt < t0 + compasso - 1e-6:
            if intro and tt < t0 + compasso / 2 and c["lead"] == "arpejo":
                tt += passo
                continue
            m = notas_arp[padrao[j % len(padrao)] % len(notas_arp)]
            dur_n = passo * (1.8 if c["lead"] in ("piano", "sino") else 0.95)
            colocar(lead_b, nota_lead(c["lead"], hz(m), dur_n, c["brilho"]), tt, rng.uniform(0.75, 1.0))
            tt += passo
            j += 1
        # bateria
        for b in range(4):
            tb = t0 + b * batida
            if c["bat"] in ("quatro", "eletronica"):
                if c["bat"] == "quatro" or b in (0, 2):
                    colocar(bat, kick(), tb, 1.0 if not intro else 0.0)
                if b in (1, 3) and not intro:
                    colocar(bat, caixa(rng), tb, 0.7)
                for h in range(2):
                    colocar(bat, chimbal(rng, aberto=(h == 1 and c["bat"] == "quatro")), tb + h * batida / 2, 0.4)
            elif c["bat"] == "palmas":
                if b in (0, 2):
                    colocar(bat, kick(0.25), tb, 0.6)
                if b in (1, 3):
                    colocar(bat, palma(rng), tb, 0.7)
                colocar(bat, chimbal(rng), tb + batida / 2, 0.25)
            elif c["bat"] == "lofi":
                if b == 0 or (b == 2 and k % 2):
                    colocar(bat, kick(0.3), tb, 0.6)
                if b in (1, 3):
                    colocar(bat, caixa(rng) * 0.6, tb + 0.02, 0.5)
            elif c["bat"] == "tambores":
                colocar(bat, kick(0.5), tb, 0.9 if b in (0, 2) else 0.5)
    # batida final: acorde da tônica + impacto
    tf = n_comp * compasso
    if tf < total:
        colocar(harm, pad([hz(m) for m in acorde(tonica + 12, escala, 0)], 1.4, c["brilho"]), min(tf, duracao), 0.6)
    mix = 0.9 * bat + 0.55 * harm + 0.45 * lead_b + 0.7 * grave
    mix = s.reverb(mix, 0.35, 0.18)[:n]
    # fade de saída nos 0,6 s finais da duração pedida
    corte = int((duracao + 0.6) * s.SR)
    mix = mix[:corte]
    mix[-int(0.6 * s.SR):] *= np.linspace(1, 0, int(0.6 * s.SR))
    mix = np.tanh(1.3 * mix / (np.max(np.abs(mix)) + 1e-9)) * 0.9
    batidas = [round(i * batida, 3) for i in range(int(duracao / batida) + 1)]
    return mix, batidas, compasso, bpm


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--clima", choices=CLIMAS, default="tecnologico")
    ap.add_argument("--duracao", type=float, default=15)
    ap.add_argument("--bpm", type=float)
    ap.add_argument("--semente", type=int, default=1)
    ap.add_argument("--saida", required=True)
    a = ap.parse_args()
    mix, batidas, compasso, bpm = compor(a.clima, a.duracao, a.bpm, a.semente)
    s.salvar(a.saida, s.normalizar(mix, -1.0))
    marcas = Path(a.saida).with_suffix(".batidas.txt")
    with open(marcas, "w", encoding="utf-8") as f:
        f.write(f"# bpm={bpm:g} compasso={compasso:.3f}s clima={a.clima} semente={a.semente}\n")
        for i, b in enumerate(batidas):
            f.write(f"{b:.3f}\t{'COMPASSO' if i % 4 == 0 else 'batida'}\n")
    print(f"trilha {a.clima} {bpm:g} bpm, {a.duracao:g} s -> {a.saida} (+ {marcas.name})")


if __name__ == "__main__":
    main()
