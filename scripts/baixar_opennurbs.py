# SPDX-License-Identifier: LGPL-2.1-or-later
"""Baixa o openNURBS na versão fixada e guarda só os fontes que o crate fenkra-3dm compila (B211, ADR 0090).

O fonte não fica no repositório: são cerca de 18 MB de C++. Este script clona o repositório oficial
(github.com/mcneel/opennurbs) no commit fixado abaixo, copia os fontes da biblioteca, o zlib que vem junto,
o android_uuid (só fora do Windows), o LICENSE e o CMakeLists.txt (de onde o build.rs tira a lista de
fontes), e apaga o clone. Sem exemplos, binários nem arquivos de teste.

Uso:
    python -I scripts/baixar_opennurbs.py                 # baixa para third_party/opennurbs
    python -I scripts/baixar_opennurbs.py --destino D:/x  # outra pasta (aponte FENKRA_OPENNURBS para ela)
    python -I scripts/baixar_opennurbs.py --de <clone>    # usa um clone que já existe, no commit fixado

O build.rs do fenkra-3dm procura em FENKRA_OPENNURBS e depois em third_party/opennurbs desta pasta ou de
qualquer pasta acima (um worktree dentro do repositório acha a cópia da raiz).
"""

import argparse
import pathlib
import shutil
import subprocess
import sys
import tempfile

URL = "https://github.com/mcneel/opennurbs.git"
# ramo 8.x, versão 8.35.26251.13001 (mesclada em 15/09/2026)
COMMIT = "eb92af3ba1806b0a34a99aba0d3bda83e3d46083"
RAIZ = pathlib.Path(__file__).resolve().parent.parent


def git(*args, cwd=None):
    subprocess.run(["git", *args], cwd=cwd, check=True)


def clonar(pasta: pathlib.Path) -> pathlib.Path:
    destino = pasta / "opennurbs"
    destino.mkdir()
    git("init", "-q", cwd=destino)
    git("remote", "add", "origin", URL, cwd=destino)
    git("fetch", "-q", "--depth", "1", "origin", COMMIT, cwd=destino)
    git("checkout", "-q", "FETCH_HEAD", cwd=destino)
    return destino


def commit_de(clone: pathlib.Path) -> str:
    r = subprocess.run(["git", "rev-parse", "HEAD"], cwd=clone, capture_output=True, text=True)
    return r.stdout.strip()


def copiar(clone: pathlib.Path, destino: pathlib.Path) -> int:
    if destino.exists():
        shutil.rmtree(destino)
    destino.mkdir(parents=True)
    n = 0
    for f in sorted(clone.iterdir()):
        if f.is_file() and f.name.startswith("opennurbs") and f.suffix in (".h", ".cpp"):
            shutil.copy2(f, destino / f.name)
            n += 1
    for nome in ("LICENSE", "CMakeLists.txt", "README.md"):
        shutil.copy2(clone / nome, destino / nome)
    for sub in ("zlib", "android_uuid"):
        (destino / sub).mkdir()
        for f in sorted((clone / sub).iterdir()):
            if f.is_file() and f.suffix in (".c", ".h"):
                shutil.copy2(f, destino / sub / f.name)
                n += 1
    (destino / "FENKRA_VERSAO").write_text(f"{URL}\n{COMMIT}\n", encoding="utf-8")
    return n


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--destino", type=pathlib.Path, default=RAIZ / "third_party" / "opennurbs")
    p.add_argument("--de", type=pathlib.Path, help="clone já existente do repositório oficial")
    a = p.parse_args()
    if a.de:
        if commit_de(a.de) != COMMIT:
            print(f"o clone em {a.de} não está no commit fixado {COMMIT}", file=sys.stderr)
            return 1
        n = copiar(a.de, a.destino)
    else:
        with tempfile.TemporaryDirectory() as tmp:
            clone = clonar(pathlib.Path(tmp))
            n = copiar(clone, a.destino)
            # o Git deixa arquivos só de leitura no .git: o Windows recusa apagar sem tirar a marca
            for f in (clone / ".git").rglob("*"):
                f.chmod(0o666)
    print(f"openNURBS {COMMIT[:12]}: {n} fontes em {a.destino}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
