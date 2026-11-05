#!/usr/bin/env python3
"""Cabeçalho SPDX nos fontes do Fenkra (B201).

Aplicar em tudo:           python scripts/spdx.py
Aplicar numa parte:        python scripts/spdx.py crates/fenkra-kernel crates/fenkra-core/src/doc.rs
Só listar o que falta:     python scripts/spdx.py --verificar

Rodar de novo não muda nada. O cabeçalho entra na primeira linha, depois de um BOM se houver, e usa o
fim de linha do arquivo. Quem barra arquivo novo sem cabeçalho é crates/fenkra-app/tests/spdx.rs.
"""

import argparse
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
CABECALHO = b"// SPDX-License-Identifier: LGPL-2.1-or-later"
EXTENSOES = {".rs", ".cpp", ".h", ".inc"}
PASTAS = ("crates", "tools")
IGNORADAS = {"target", ".git", "__pycache__", "node_modules"}
BOM = b"\xef\xbb\xbf"


def fontes(alvos):
    for alvo in alvos:
        if alvo.is_file():
            if alvo.suffix in EXTENSOES:
                yield alvo
            continue
        for p in sorted(alvo.rglob("*")):
            if p.is_file() and p.suffix in EXTENSOES and not IGNORADAS & set(p.parts):
                yield p


def tem_cabecalho(dados):
    return dados.removeprefix(BOM).startswith(CABECALHO)


def com_cabecalho(dados):
    corpo = dados.removeprefix(BOM)
    fim = b"\r\n" if b"\r\n" in corpo else b"\n"
    return (BOM if dados.startswith(BOM) else b"") + CABECALHO + fim + corpo


def main():
    ap = argparse.ArgumentParser(description="Aplica ou confere o cabeçalho SPDX.")
    ap.add_argument("caminhos", nargs="*", type=Path, help="arquivos ou pastas; sem nada, crates e tools")
    ap.add_argument("--verificar", action="store_true", help="não grava; sai com erro se faltar cabeçalho")
    args = ap.parse_args()

    alvos = [(RAIZ / c).resolve() for c in args.caminhos] or [RAIZ / p for p in PASTAS]
    faltando = [p for p in fontes(alvos) if not tem_cabecalho(p.read_bytes())]
    for p in faltando:
        print(p.relative_to(RAIZ).as_posix())
        if not args.verificar:
            p.write_bytes(com_cabecalho(p.read_bytes()))

    acao = "sem cabeçalho" if args.verificar else "atualizados"
    print(f"{len(faltando)} arquivos {acao}", file=sys.stderr)
    return 1 if args.verificar and faltando else 0


if __name__ == "__main__":
    sys.exit(main())
