#!/usr/bin/env python3
"""Gera a biblioteca de efeitos sonoros original do estúdio (sem direitos de terceiros).

Uso: python ferramentas/gerar_sfx.py [--pasta biblioteca/sfx] [--variacoes 3]
Cada efeito sai em três intensidades (suave, medio, forte) e em variações de altura,
para casar com o nível de efeitos escolhido no projeto (skill video-estilo).
Saída: WAV 48 kHz estéreo + catalogo.csv (nome, categoria, intensidade, duração, uso).
"""
import argparse
import csv
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sintese as s  # noqa: E402

# intensidade → (ganho de pico em dB, fator de duração, brilho)
# correção de ganho por efeito (dB), para efeitos de som cheio como o glitch
GANHO = {"glitch": -9, "digitacao": -3}
INTENSIDADES = {"suave": (-9, 0.75, 0.6), "medio": (-4, 1.0, 1.0), "forte": (-1, 1.25, 1.5)}


def whoosh(dur, brilho, rng, reverso=False):
    n = int(s.SR * dur)
    x = s.ruido(dur, rng)
    c = np.sin(np.linspace(0, np.pi, n)) ** 2
    corte = 300 + 5000 * brilho * c
    y = s.passa_baixa(s.passa_baixa(x, corte), corte) * (c ** 1.5)
    y = s.passa_alta_fixo(y, 120)
    if reverso:
        y = y[::-1]
    pan = np.linspace(-0.7, 0.7, n)
    return y, (y * (1 - pan) / 2 * 2, y * (1 + pan) / 2 * 2)


def riser(dur, brilho, rng):
    n = int(s.SR * dur)
    sobe = np.linspace(0, 1, n) ** 2
    tom = s.seno_varredura(120, 900 * brilho + 300, dur) * 0.5
    chi = s.passa_baixa(s.ruido(dur, rng), 400 + 7000 * sobe) * 0.6
    y = (tom + chi) * sobe
    y[-int(0.01 * s.SR):] *= np.linspace(1, 0, int(0.01 * s.SR))
    return y, None


def hit(dur, brilho, rng):
    corpo = s.seno_varredura(140, 38, dur) * s.env(int(s.SR * dur), 0.001, dur * 0.5)
    ataque = s.passa_baixa_fixo(s.ruido(dur, rng), 2500 * brilho) * s.env(int(s.SR * dur), 0.0005, 0.03)
    y = np.tanh(2.2 * (corpo + 0.6 * ataque))
    return s.reverb(y, 0.3, 0.2)[: int(s.SR * dur * 1.6)], None


def boom(dur, brilho, rng):
    corpo = s.seno_varredura(70, 28, dur) * s.env(int(s.SR * dur), 0.002, dur * 0.8)
    estr = s.passa_baixa_fixo(s.ruido(dur, rng), 900 * brilho) * s.env(int(s.SR * dur), 0.001, 0.25)
    return s.reverb(np.tanh(2.5 * corpo + 0.5 * estr), 0.6, 0.35), None


def pop(dur, brilho, rng):
    y = s.seno_varredura(900 * brilho + 300, 220, dur) * s.env(int(s.SR * dur), 0.001, 0.04)
    return y, None


def clique(dur, brilho, rng):
    n = int(s.SR * dur)
    y = s.passa_alta_fixo(s.ruido(dur, rng), 1500) * s.env(n, 0.0002, 0.006)
    y += np.sin(2 * np.pi * 2600 * brilho * s.t(dur)) * s.env(n, 0.0002, 0.01) * 0.4
    return y, None


def swipe(dur, brilho, rng):
    n = int(s.SR * dur)
    y = s.passa_alta_fixo(s.ruido(dur, rng), 2000) * np.sin(np.linspace(0, np.pi, n)) ** 3
    y = s.passa_baixa_fixo(y, 6000 * brilho + 2000)
    return y, None


def ding(dur, brilho, rng):
    tt = s.t(dur)
    f = 1046.5 * (0.75 + 0.25 * brilho)
    y = sum(a * np.sin(2 * np.pi * f * m * tt) for a, m in ((1, 1), (0.4, 2.76), (0.2, 5.4)))
    return s.reverb(y * s.env(len(tt), 0.002, dur * 0.4), 0.4, 0.3), None


def brilho_magico(dur, brilho, rng):
    tt = s.t(dur)
    y = np.zeros_like(tt)
    for k in range(14):
        ini = rng.uniform(0, dur * 0.7)
        f = rng.choice([1318.5, 1568, 1760, 2093, 2637]) * (0.8 + 0.2 * brilho)
        nota = np.sin(2 * np.pi * f * tt) * np.exp(-np.maximum(tt - ini, 0) * 9) * (tt >= ini)
        y += nota * rng.uniform(0.3, 1)
    return s.reverb(y, 0.5, 0.4), None


def glitch(dur, brilho, rng):
    n = int(s.SR * dur)
    y = np.zeros(n)
    pos = 0
    while pos < n:
        bloco = int(rng.uniform(0.008, 0.04) * s.SR)
        tipo = rng.integers(3)
        seg = np.arange(bloco) / s.SR
        if tipo == 0:
            pedaco = np.sign(np.sin(2 * np.pi * rng.uniform(80, 1200) * seg))
        elif tipo == 1:
            pedaco = rng.uniform(-1, 1, bloco)
        else:
            pedaco = np.zeros(len(seg))
        y[pos:pos + bloco] = pedaco[: n - pos] * 0.6 * brilho
        pos += bloco
    return np.round(y * 6) / 6, None


def digitacao(dur, brilho, rng):
    n = int(s.SR * dur)
    y = np.zeros(n)
    tclique = 0.0
    while tclique < dur - 0.03:
        c, _ = clique(0.03, brilho * rng.uniform(0.7, 1.1), rng)
        i = int(tclique * s.SR)
        y[i:i + len(c)] += c[: n - i] * rng.uniform(0.5, 1)
        tclique += rng.uniform(0.06, 0.14)
    return y, None


def obturador(dur, brilho, rng):
    a, _ = clique(0.04, brilho, rng)
    b, _ = clique(0.05, brilho * 0.8, rng)
    y = np.zeros(int(s.SR * dur))
    y[: len(a)] += a
    j = int(0.07 * s.SR)
    y[j:j + len(b)] += b[: len(y) - j]
    return y, None


def queda_grave(dur, brilho, rng):
    y = s.seno_varredura(180, 35, dur) * s.env(int(s.SR * dur), 0.01, dur * 0.9)
    return np.tanh(2 * y), None


def transicao_tom(dur, brilho, rng):
    y = s.seno_varredura(2000 * brilho + 500, 200, dur) * np.sin(np.linspace(0, np.pi, int(s.SR * dur)))
    return s.reverb(y * 0.6, 0.3, 0.3), None


EFEITOS = {
    # nome: (função, duração base, categoria, uso)
    "whoosh": (whoosh, 0.55, "transicao", "corte com movimento, whip pan, entrada de texto grande"),
    "whoosh-reverso": (lambda d, b, r: whoosh(d, b, r, True), 0.6, "transicao", "antes da revelação; sugado para o corte"),
    "swipe": (swipe, 0.25, "transicao", "deslizar de tela, troca rápida de card"),
    "riser": (riser, 1.6, "tensao", "construir expectativa antes da virada ou do drop"),
    "hit": (hit, 0.45, "impacto", "zoom de impacto, palavra-chave, corte seco forte"),
    "boom": (boom, 1.4, "impacto", "revelação grande, título, logo"),
    "queda-grave": (queda_grave, 0.9, "impacto", "drop, mudança de cena dramática"),
    "pop": (pop, 0.12, "interface", "texto ou ícone aparecendo, emoji, bolha"),
    "clique": (clique, 0.05, "interface", "botão, cursor, seleção na tela"),
    "obturador": (obturador, 0.18, "interface", "foto, congelar quadro, print"),
    "digitacao": (digitacao, 1.2, "interface", "texto sendo digitado, busca, código"),
    "ding": (ding, 1.0, "acento", "acerto, dica, item de lista, check"),
    "brilho": (brilho_magico, 1.2, "acento", "antes e depois, revelação charmosa, produto brilhando"),
    "glitch": (glitch, 0.35, "acento", "erro, virada tecnológica, quebra de padrão"),
    "transicao-tom": (transicao_tom, 0.5, "transicao", "troca de assunto, fim de bloco"),
}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--pasta", default=str(Path(__file__).resolve().parent.parent / "biblioteca" / "sfx"))
    ap.add_argument("--variacoes", type=int, default=2)
    ap.add_argument("--so", nargs="*", help="gerar só estes efeitos")
    a = ap.parse_args()
    pasta = Path(a.pasta)
    linhas = []
    for nome, (fn, dur, cat, uso) in EFEITOS.items():
        if a.so and nome not in a.so:
            continue
        for intens, (pico, fdur, brilho) in INTENSIDADES.items():
            pico = pico + GANHO.get(nome, 0)
            for v in range(1, a.variacoes + 1):
                rng = np.random.default_rng(hash((nome, intens, v)) % 2**32)
                br = brilho * (1 + 0.15 * (v - 1) * (-1) ** v)
                y, est = fn(dur * fdur, br, rng)
                if est is not None:
                    l, r = est
                    g = 10 ** (pico / 20) / (max(np.abs(l).max(), np.abs(r).max()) + 1e-9)
                    l, r = s.fade(l * g), s.fade(r * g)
                    arq = pasta / cat / f"{nome}_{intens}_{v}.wav"
                    s.salvar(arq, None, (l, r))
                    n = len(l)
                else:
                    y = s.fade(s.normalizar(y, pico))
                    arq = pasta / cat / f"{nome}_{intens}_{v}.wav"
                    s.salvar(arq, y)
                    n = len(y)
                linhas.append([arq.relative_to(pasta).as_posix(), nome, cat, intens, f"{n / s.SR:.2f}", uso])
    cat_arq = pasta / "catalogo.csv"
    if a.so and cat_arq.exists():  # geração parcial: mantém o resto do catálogo
        with open(cat_arq, encoding="utf-8") as f:
            antigos = [r for r in list(csv.reader(f))[1:] if r and r[1] not in a.so]
        linhas = antigos + linhas
    with open(cat_arq, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["arquivo", "efeito", "categoria", "intensidade", "duracao_s", "uso"])
        w.writerows(linhas)
    (pasta / "LICENCA.txt").write_text(
        "Efeitos sintetizados por ferramentas/gerar_sfx.py. Obra original do estúdio, sem amostras de\n"
        "terceiros. Uso livre em projetos próprios e de clientes.\n", encoding="utf-8")
    print(f"{len(linhas)} arquivos em {pasta}")


if __name__ == "__main__":
    main()
