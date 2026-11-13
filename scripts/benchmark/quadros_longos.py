"""B259: explica os quadros longos de uma medição `FENKRA_MEDIR_QUADROS=<arquivo.csv>`.

Uso: python -I scripts/benchmark/quadros_longos.py <arquivo.csv> [limite_ms]

Para cada quadro acima do limite (100 ms por padrão), junta o tempo de parede da passada da interface,
o tempo de CPU que a thread rodou nela, a fração ociosa da máquina e a fase
mais longa registrada no quadro, e classifica a causa como em `medir_quadros::causa`:

- programa: a thread trabalhou mais da metade do tempo (código nosso);
- sistema ocupado: a thread esperou e a máquina estava com menos de 10% ociosa;
- espera: a thread esperou com processador livre (paginação, disco, placa de vídeo ou trava).
"""

import csv
import sys
from collections import defaultdict


def ler(caminho):
    quadros = defaultdict(dict)
    fases = defaultdict(list)
    with open(caminho, encoding="utf-8") as f:
        for linha in csv.reader(f):
            if len(linha) < 4:
                continue
            tipo, n, v, nome = linha[0], int(linha[1]), float(linha[2]), linha[3]
            if tipo == "quadro":
                quadros[n]["quadro"] = v
            elif tipo == "valor" and nome in ("parede", "cpu_thread", "ocioso_sistema"):
                quadros[n][nome] = v
            elif tipo == "fase":
                fases[n].append((v, nome))
    return quadros, fases


def causa(q):
    if q["cpu_thread"] >= 0.5 * q["parede"]:
        return "programa"
    if q["ocioso_sistema"] < 0.1:
        return "sistema ocupado"
    return "espera"


def main():
    caminho = sys.argv[1]
    limite = float(sys.argv[2]) if len(sys.argv) > 2 else 100.0
    quadros, fases = ler(caminho)
    com_relogio = {n: q for n, q in quadros.items() if {"parede", "cpu_thread", "ocioso_sistema"} <= q.keys()}
    longos = sorted((n for n, q in com_relogio.items() if q["parede"] > limite), key=lambda n: -com_relogio[n]["parede"])
    total = len(com_relogio)
    print(f"{total} quadros com relógio; {len(longos)} acima de {limite:.0f} ms")
    contagem = defaultdict(int)
    for n in longos:
        q = com_relogio[n]
        c = causa(q)
        contagem[c] += 1
        # a medida e as fases da passada levam o mesmo número de quadro
        f = max(fases.get(n, [(0.0, "-")]))
        print(f"quadro {n}: parede {q['parede']:.0f} ms, CPU da thread {q['cpu_thread']:.0f} ms, máquina {100 * q['ocioso_sistema']:.0f}% ociosa, maior fase {f[1]} {f[0]:.0f} ms -> {c}")
    for c, k in sorted(contagem.items()):
        print(f"{c}: {k}")


if __name__ == "__main__":
    main()
