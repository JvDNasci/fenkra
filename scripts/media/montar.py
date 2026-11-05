#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later
"""Esteira de midia de ajuda (B257): quadros gravados pelo app -> GIF -> manifesto -> pacote.

Sem dependencias externas (so a biblioteca padrao) e sem entrar no executavel do app.

    python -I scripts/media/montar.py gerar-todos --exe C:/cargo-target/ajuda2/release/fenkra.exe
    python -I scripts/media/montar.py montar <comando> <pasta-com-quadros>
    python -I scripts/media/montar.py empacotar
    python -I scripts/media/montar.py verificar

`gerar-todos` roda o app (FENKRA_DEMO_GRAVAR=<comando>) so para os roteiros que mudaram desde o
manifesto (hash do roteiro) e depois monta o pacote. `verificar` sai com erro se algum roteiro nao
tem midia em dia, e e o que o CI roda. Hash: FNV-1a de 64 bits, com CRLF contado como LF, igual ao
`midia::hash` do app.
"""
import argparse
import json
import os
import struct
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
DOCS = RAIZ / "docs" / "media"
LARGURA, ALTURA, FPS, CORES = 360, 220, 12, 64
LIMITE = 150 * 1024
TRANSPARENTE = CORES - 1  # a ultima entrada da paleta de 64 cores fica para a transparencia


def fnv(dados: bytes) -> str:
    dados = dados.replace(b"\r\n", b"\n")
    h = 0xCBF29CE484222325
    for b in dados:
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def ler_ppm(caminho: Path):
    b = caminho.read_bytes()
    partes = b.split(b"\n", 3)
    if partes[0] != b"P6":
        raise ValueError(f"{caminho}: nao e PPM P6")
    w, h = map(int, partes[1].split())
    if (w, h) != (LARGURA, ALTURA):
        raise ValueError(f"{caminho}: {w}x{h}, esperava {LARGURA}x{ALTURA}")
    corpo = partes[3]
    if len(corpo) != w * h * 3:
        raise ValueError(f"{caminho}: dados incompletos")
    return corpo


# ------------------------------------------------------------------ quantizacao

def paleta_mediana(quadros, n):
    """Corte pela mediana sobre o histograma (cores reduzidas a 5 bits por canal)."""
    cont = Counter()
    for q in quadros:
        for i in range(0, len(q), 3 * 2):  # metade dos pixels basta para a paleta
            cont[(q[i] >> 3, q[i + 1] >> 3, q[i + 2] >> 3)] += 1
    caixas = [[(k, c) for k, c in cont.items()]]
    while len(caixas) < n:
        # divide a caixa de maior (populacao x extensao) que ainda tem mais de uma cor
        melhor, pontos = None, -1
        for i, cx in enumerate(caixas):
            if len(cx) < 2:
                continue
            ext = max(max(k[a] for k, _ in cx) - min(k[a] for k, _ in cx) for a in range(3))
            p = sum(c for _, c in cx) * (ext + 1)
            if p > pontos:
                melhor, pontos = i, p
        if melhor is None:
            break
        cx = caixas.pop(melhor)
        eixo = max(range(3), key=lambda a: max(k[a] for k, _ in cx) - min(k[a] for k, _ in cx))
        cx.sort(key=lambda kc: kc[0][eixo])
        total, acc, corte = sum(c for _, c in cx), 0, 1
        for j, (_, c) in enumerate(cx):
            acc += c
            if acc >= total / 2:
                corte = min(max(j + 1, 1), len(cx) - 1)
                break
        caixas += [cx[:corte], cx[corte:]]
    pal = []
    for cx in caixas:
        tot = sum(c for _, c in cx)
        r = sum(k[0] * c for k, c in cx) / tot
        g = sum(k[1] * c for k, c in cx) / tot
        b = sum(k[2] * c for k, c in cx) / tot
        pal.append((min(255, round(r * 8 + 4)), min(255, round(g * 8 + 4)), min(255, round(b * 8 + 4))))
    pal.sort()  # ordem estavel: o mesmo conjunto de cores gera sempre a mesma paleta
    return pal


def indexar(q, pal):
    cache = {}
    out = []
    for i in range(0, len(q), 3):
        cor = q[i : i + 3]
        idx = cache.get(cor)
        if idx is None:
            r, g, b = cor
            idx = min(range(len(pal)), key=lambda k: (pal[k][0] - r) ** 2 + (pal[k][1] - g) ** 2 + (pal[k][2] - b) ** 2)
            cache[cor] = idx
        out.append(idx)
    return out


# ------------------------------------------------------------------ GIF

class Bits:
    def __init__(self):
        self.out = bytearray()
        self.acc = 0
        self.n = 0

    def escrever(self, codigo, largura):
        self.acc |= codigo << self.n
        self.n += largura
        while self.n >= 8:
            self.out.append(self.acc & 0xFF)
            self.acc >>= 8
            self.n -= 8

    def fechar(self):
        if self.n:
            self.out.append(self.acc & 0xFF)
        return bytes(self.out)


def lzw(indices, minimo):
    limpar, fim = 1 << minimo, (1 << minimo) + 1
    b = Bits()
    largura, prox, tabela = minimo + 1, fim + 1, {}
    b.escrever(limpar, largura)
    prefixo = indices[0]
    for px in indices[1:]:
        chave = (prefixo << 8) | px
        c = tabela.get(chave)
        if c is not None:
            prefixo = c
            continue
        b.escrever(prefixo, largura)
        if prox >= (1 << largura) and largura < 12:
            largura += 1
        if prox < 4096:
            tabela[chave] = prox
            prox += 1
        else:
            b.escrever(limpar, largura)
            largura, prox, tabela = minimo + 1, fim + 1, {}
        prefixo = px
    b.escrever(prefixo, largura)
    if prox >= (1 << largura) and largura < 12:
        largura += 1
    b.escrever(fim, largura)
    return b.fechar()


def sub_blocos(dados):
    out = bytearray()
    for i in range(0, len(dados), 255):
        bloco = dados[i : i + 255]
        out.append(len(bloco))
        out += bloco
    out.append(0)
    return bytes(out)


def atraso(k):
    """Centesimos de segundo do quadro k: 100/12 nao e inteiro, entao alterna 8 e 9."""
    return round((k + 1) * 100 / FPS) - round(k * 100 / FPS)


def montar_gif(quadros):
    pal = paleta_mediana(quadros, CORES - 1)
    tabela = bytearray()
    for r, g, b in pal:
        tabela += bytes((r, g, b))
    while len(tabela) < CORES * 3:
        tabela += b"\x00\x00\x00"
    out = bytearray(b"GIF89a")
    out += struct.pack("<HHBBB", LARGURA, ALTURA, 0x80 | 0x70 | 5, 0, 0)  # tabela global de 64 entradas
    out += tabela
    out += b"\x21\xFF\x0BNETSCAPE2.0\x03\x01\x00\x00\x00"  # laco infinito
    anterior = None
    for k, q in enumerate(quadros):
        idx = indexar(q, pal)
        if anterior is not None:
            # delta com perda leve: cor quase igual a do quadro anterior (ruido de borda) nao conta
            # como mudanca; so assim o GIF cabe nos 150 KB sem cortar o tempo do roteiro
            for i in range(len(idx)):
                a, b = pal[idx[i]], pal[anterior[i]]
                if idx[i] != anterior[i] and (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2 <= 192:
                    idx[i] = anterior[i]
        if anterior is None:
            x, y, w, h, dados, transp = 0, 0, LARGURA, ALTURA, idx, False
        else:
            difs = [i for i in range(len(idx)) if idx[i] != anterior[i]]
            if not difs:
                x = y = 0
                w = h = 1
                dados, transp = [TRANSPARENTE], True
            else:
                xs = [i % LARGURA for i in difs]
                ys = [i // LARGURA for i in difs]
                x, y, w, h = min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1
                dados = []
                for yy in range(y, y + h):
                    for xx in range(x, x + w):
                        i = yy * LARGURA + xx
                        dados.append(TRANSPARENTE if idx[i] == anterior[i] else idx[i])
                transp = True
        # disposal 1 (manter), transparencia so nos quadros de diferenca
        out += b"\x21\xF9\x04" + bytes((0b100 | (1 if transp else 0),)) + struct.pack("<H", atraso(k)) + bytes((TRANSPARENTE if transp else 0, 0))
        out += b"\x2C" + struct.pack("<HHHHB", x, y, w, h, 0)
        out += bytes((6,)) + sub_blocos(lzw(dados, 6))
        anterior = idx
    out += b"\x3B"
    return bytes(out)


# ------------------------------------------------------------------ manifesto e pacote

def ler_manifesto():
    p = DOCS / "manifesto.json"
    if p.exists():
        return json.loads(p.read_text(encoding="utf-8"))
    return {"versao": 1, "fps": FPS, "largura": LARGURA, "altura": ALTURA, "cores": CORES, "limite_bytes": LIMITE, "midias": []}


def gravar_manifesto(m):
    m["midias"].sort(key=lambda e: e["id"])
    (DOCS / "manifesto.json").write_text(json.dumps(m, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")


def roteiros():
    return sorted((DOCS / "roteiros").glob("*.json"))


def montar_comando(comando, pasta):
    arqs = sorted(Path(pasta).glob("q*.ppm"))
    if not arqs:
        raise SystemExit(f"{pasta}: sem quadros q*.ppm")
    roteiro = DOCS / "roteiros" / f"{comando}.json"
    quadros_esperados = None
    if roteiro.exists():
        r = json.loads(roteiro.read_text(encoding="utf-8"))
        quadros_esperados = round(r["duracao"] * r.get("fps", FPS))
    if quadros_esperados is not None and len(arqs) != quadros_esperados:
        raise SystemExit(f"{comando}: {len(arqs)} quadros, o roteiro pede {quadros_esperados}")
    gif = montar_gif([ler_ppm(a) for a in arqs])
    if len(gif) > LIMITE:
        raise SystemExit(f"{comando}: GIF com {len(gif)} bytes passa do limite de {LIMITE}; encurte o roteiro ou simplifique a cena")
    (DOCS / "saida").mkdir(parents=True, exist_ok=True)
    (DOCS / "saida" / f"{comando}.gif").write_bytes(gif)
    m = ler_manifesto()
    m["midias"] = [e for e in m["midias"] if e["id"] != comando]
    m["midias"].append({
        "id": comando,
        "hash_roteiro": fnv(roteiro.read_bytes()),
        "gif": f"saida/{comando}.gif",
        "bytes_gif": len(gif),
        "hash_gif": fnv(gif),
        "quadros": len(arqs),
        "largura": LARGURA,
        "altura": ALTURA,
    })
    gravar_manifesto(m)
    print(f"{comando}: {len(arqs)} quadros, {len(gif)} bytes")


def empacotar():
    m = ler_manifesto()
    dados, entradas, off = bytearray(), [], 0
    for e in sorted(m["midias"], key=lambda e: e["id"]):
        b = (DOCS / e["gif"]).read_bytes()
        if fnv(b) != e["hash_gif"]:
            raise SystemExit(f"{e['id']}: GIF no disco nao confere com o manifesto; rode montar de novo")
        entradas.append({"id": e["id"], "offset": off, "len": len(b), "hash": e["hash_gif"]})
        dados += b
        off += len(b)
    indice = json.dumps({"versao": 1, "entradas": entradas}, separators=(",", ":")).encode()
    (DOCS / "ajuda.pack").write_bytes(b"FKAJ" + struct.pack("<I", len(indice)) + indice + bytes(dados))
    print(f"ajuda.pack: {len(entradas)} midias, {len(dados)} bytes")


def em_dia(roteiro, m):
    e = next((x for x in m["midias"] if x["id"] == roteiro.stem), None)
    if not e or e["hash_roteiro"] != fnv(roteiro.read_bytes()):
        return False
    g = DOCS / e["gif"]
    return g.exists() and fnv(g.read_bytes()) == e["hash_gif"]


def gerar_todos(exe, forcar):
    m = ler_manifesto()
    for r in roteiros():
        if not forcar and em_dia(r, m):
            print(f"{r.stem}: em dia")
            continue
        with tempfile.TemporaryDirectory(prefix="fenkra_midia_") as tmp:
            env = dict(os.environ, FENKRA_DEMO_GRAVAR=r.stem, FENKRA_DEMO_GRAVAR_SAIDA=tmp, FENKRA_DEMO_ROTEIROS=str(DOCS / "roteiros"))
            print(f"{r.stem}: gravando...", flush=True)
            subprocess.run([exe], env=env, check=True, timeout=900)
            if not (Path(tmp) / "pronto.txt").exists():
                raise SystemExit(f"{r.stem}: o app terminou sem concluir a gravacao")
            montar_comando(r.stem, tmp)
        m = ler_manifesto()
    empacotar()


def verificar():
    m = ler_manifesto()
    ruim = [r.stem for r in roteiros() if not em_dia(r, m)]
    ids = {r.stem for r in roteiros()}
    orfas = [e["id"] for e in m["midias"] if e["id"] not in ids]
    if ruim or orfas:
        print("midia fora de dia:", ", ".join(ruim) or "-", "| sem roteiro:", ", ".join(orfas) or "-")
        return 1
    print(f"{len(ids)} midias em dia")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sp = ap.add_subparsers(dest="cmd", required=True)
    g = sp.add_parser("gerar-todos")
    g.add_argument("--exe", required=True)
    g.add_argument("--forcar", action="store_true")
    mo = sp.add_parser("montar")
    mo.add_argument("comando")
    mo.add_argument("pasta")
    sp.add_parser("empacotar")
    sp.add_parser("verificar")
    a = ap.parse_args()
    if a.cmd == "gerar-todos":
        gerar_todos(a.exe, a.forcar)
    elif a.cmd == "montar":
        montar_comando(a.comando, a.pasta)
    elif a.cmd == "empacotar":
        empacotar()
    else:
        sys.exit(verificar())


if __name__ == "__main__":
    main()
