#!/usr/bin/env python
# SPDX-License-Identifier: LGPL-2.1-or-later
"""Varredura de similaridade (B204).

Procura, num documento do Fenkra, frases de N palavras (padrão 8) que também aparecem, na mesma ordem,
nas páginas públicas que o próprio documento cita. Serve para pegar trecho copiado por engano de uma
fonte. Uma coincidência não prova cópia (nomes de norma e termos técnicos repetem), mas toda
coincidência deve ser reescrita ou justificada.

Uso:
    python -I scripts/auditoria/similaridade.py docs/pesquisa/sketch.md
    python -I scripts/auditoria/similaridade.py docs/pesquisa/sketch.md --paginas pasta_com_html_ou_txt
    python -I scripts/auditoria/similaridade.py --autoteste

Sem --paginas, baixa cada endereço http(s) do documento (limite de tempo de 15 s; falhas são listadas
e não contam como "sem coincidência"). Código de saída: 0 sem coincidências, 1 com coincidências,
2 se nenhuma página pôde ser lida.
"""
import argparse
import re
import sys
import urllib.request
from html.parser import HTMLParser
from pathlib import Path

PALAVRAS = 8
URL = re.compile(r"https?://[^\s)>\]\"'`|]+")


class _Texto(HTMLParser):
    def __init__(self):
        super().__init__()
        self.partes = []
        self._ignora = 0

    def handle_starttag(self, tag, attrs):
        if tag in ("script", "style", "nav", "footer"):
            self._ignora += 1

    def handle_endtag(self, tag):
        if tag in ("script", "style", "nav", "footer") and self._ignora:
            self._ignora -= 1

    def handle_data(self, data):
        if not self._ignora:
            self.partes.append(data)


def palavras(texto):
    """Minúsculas, sem pontuação, sem marcas de formatação."""
    return re.findall(r"[0-9a-zà-ÿ]+", texto.lower())


def html_para_texto(bruto):
    p = _Texto()
    try:
        p.feed(bruto)
    except Exception:
        return bruto
    return " ".join(p.partes)


def janelas(pal, n):
    return {" ".join(pal[i : i + n]) for i in range(len(pal) - n + 1)}


def coincidencias(documento, paginas, n=PALAVRAS):
    """Devolve [(linha, trecho, origem)] das frases de n palavras do documento presentes em alguma página."""
    indice = {}
    for origem, texto in paginas.items():
        for j in janelas(palavras(texto), n):
            indice.setdefault(j, origem)
    achados = []
    for num, linha in enumerate(documento.splitlines(), 1):
        sem_url = URL.sub(" ", linha)
        pal = palavras(sem_url)
        vistos = set()
        for i in range(len(pal) - n + 1):
            j = " ".join(pal[i : i + n])
            if j in indice and j not in vistos:
                vistos.add(j)
                achados.append((num, j, indice[j]))
    return achados


def baixar(url):
    req = urllib.request.Request(url, headers={"User-Agent": "fenkra-similaridade/1.0"})
    with urllib.request.urlopen(req, timeout=15) as r:
        bruto = r.read(3_000_000).decode(r.headers.get_content_charset() or "utf-8", "replace")
    return html_para_texto(bruto) if "<" in bruto[:2000] else bruto


def paginas_locais(pasta):
    saida = {}
    for p in Path(pasta).rglob("*"):
        if p.is_file() and p.suffix.lower() in (".html", ".htm", ".txt", ".md"):
            bruto = p.read_text(encoding="utf-8", errors="replace")
            saida[str(p)] = html_para_texto(bruto) if p.suffix.lower() in (".html", ".htm") else bruto
    return saida


def autoteste():
    fonte = "o comando cria um novo elemento geométrico baseado na seleção e nas regras de propagação do usuário"
    copia = "Resumo: ele cria um novo elemento geométrico baseado na seleção e nas regras de propagação, ok."
    livre = "Extrair gera geometria nova e independente a partir de partes de outra peça."
    assert coincidencias(copia, {"fonte": fonte}), "deveria achar a cópia"
    assert not coincidencias(livre, {"fonte": fonte}), "não deveria achar texto próprio"
    assert not coincidencias("veja https://exemplo.org/a/b/c/d/e/f/g/h/i", {"fonte": "a b c d e f g h i exemplo org"}), "URL não conta"
    print("autoteste ok")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("arquivos", nargs="*")
    ap.add_argument("--paginas", help="pasta com páginas já baixadas (modo sem rede)")
    ap.add_argument("-n", type=int, default=PALAVRAS, help="palavras por frase (padrão 8)")
    ap.add_argument("--autoteste", action="store_true")
    a = ap.parse_args()
    if a.autoteste:
        autoteste()
        return 0
    if not a.arquivos:
        ap.error("informe ao menos um arquivo")
    codigo = 0
    for arq in a.arquivos:
        doc = Path(arq).read_text(encoding="utf-8")
        if a.paginas:
            paginas = paginas_locais(a.paginas)
            falhas = []
        else:
            paginas, falhas = {}, []
            for url in sorted(set(u.rstrip(".,;") for u in URL.findall(doc))):
                try:
                    paginas[url] = baixar(url)
                except Exception as e:  # rede, 403, 404, certificado
                    falhas.append(f"{url} ({type(e).__name__})")
        print(f"{arq}: {len(paginas)} página(s) lida(s), {len(falhas)} falha(s)")
        for f in falhas:
            print(f"  não lida: {f}")
        if not paginas:
            codigo = max(codigo, 2)
            continue
        achados = coincidencias(doc, paginas, a.n)
        for num, trecho, origem in achados:
            print(f"  linha {num}: \"{trecho}\" também em {origem}")
        if achados:
            codigo = 1
        else:
            print("  sem coincidências")
    return codigo


if __name__ == "__main__":
    sys.exit(main())
