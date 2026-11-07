#!/usr/bin/env python3
"""Baixa a biblioteca de fontes livres do estúdio (SIL OFL ou Apache 2.0, do repositório público
google/fonts) para biblioteca/fontes/, com a licença de cada família e um catálogo por estilo.

Uso: python ferramentas/baixar_fontes.py [--pasta biblioteca/fontes] [--estilo impacto ...]
Todas as fontes desta lista permitem uso comercial em vídeo, inclusive embutidas na imagem.
"""
import argparse
import sys
import urllib.request
from pathlib import Path

BASE = "https://raw.githubusercontent.com/google/fonts/main/"

# (pasta no google/fonts, arquivo, estilo, uso sugerido)
CATALOGO = [
    ("ofl/anton", "Anton-Regular.ttf", "impacto", "gancho e título em maiúsculas, legenda de impacto"),
    ("ofl/bebasneue", "BebasNeue-Regular.ttf", "impacto", "títulos condensados, números, listas"),
    ("ofl/archivoblack", "ArchivoBlack-Regular.ttf", "impacto", "legenda grossa e larga, estilo podcast"),
    ("ofl/poppins", "Poppins-ExtraBold.ttf", "limpo", "legenda padrão moderna, títulos"),
    ("ofl/poppins", "Poppins-SemiBold.ttf", "limpo", "texto de apoio, créditos, chamadas para redes"),
    ("ofl/poppins", "Poppins-Black.ttf", "limpo", "palavra de ênfase dentro da legenda"),
    ("ofl/inter", "Inter[opsz,wght].ttf", "limpo", "interface, dados, legendas longas no YouTube (variável)"),
    ("ofl/montserrat", "Montserrat[wght].ttf", "limpo", "marca corporativa, títulos (variável)"),
    ("ofl/lilitaone", "LilitaOne-Regular.ttf", "divertido", "legenda arredondada, infantil e culinária"),
    ("ofl/bangers", "Bangers-Regular.ttf", "divertido", "estilo quadrinho, reação, humor"),
    ("ofl/righteous", "Righteous-Regular.ttf", "divertido", "retrô futurista, games"),
    ("ofl/shrikhand", "Shrikhand-Regular.ttf", "charme", "título charmoso e gordinho, moda e confeitaria"),
    ("ofl/abrilfatface", "AbrilFatface-Regular.ttf", "charme", "editorial elegante, revista"),
    ("ofl/dmserifdisplay", "DMSerifDisplay-Italic.ttf", "charme", "palavra de ênfase elegante em itálico"),
    ("ofl/playfairdisplay", "PlayfairDisplay-Italic[wght].ttf", "charme", "luxo, beleza, gastronomia (variável)"),
    ("ofl/pacifico", "Pacifico-Regular.ttf", "manuscrito", "assinatura, vinheta, toque pessoal"),
    ("ofl/lobster", "Lobster-Regular.ttf", "manuscrito", "retrô charmoso, cafeteria"),
    ("ofl/greatvibes", "GreatVibes-Regular.ttf", "manuscrito", "casamento, convite, só em palavras curtas"),
    ("apache/permanentmarker", "PermanentMarker-Regular.ttf", "manuscrito", "anotação de marcador, setas e destaques"),
    ("ofl/caveat", "Caveat[wght].ttf", "manuscrito", "nota escrita à mão, bastidores (variável)"),
    ("ofl/spacemono", "SpaceMono-Bold.ttf", "tech", "código, dados, CAD e engenharia"),
    ("ofl/spacegrotesk", "SpaceGrotesk[wght].ttf", "tech", "tecnologia moderna, startups (variável)"),
    ("ofl/rubikmonoone", "RubikMonoOne-Regular.ttf", "tech", "títulos geométricos pesados"),
    ("ofl/syne", "Syne[wght].ttf", "tech", "design, arte, portfólio (variável)"),
    ("ofl/monoton", "Monoton-Regular.ttf", "neon", "neon e anos 80, só títulos grandes"),
    ("ofl/oswald", "Oswald[wght].ttf", "impacto", "condensada versátil, esportes (variável)"),
]


def baixar(url, destino):
    req = urllib.request.Request(url, headers={"User-Agent": "estudio-video/1.0"})
    with urllib.request.urlopen(req, timeout=60) as r:
        destino.write_bytes(r.read())


def nome_familia(arquivo):
    try:
        from PIL import ImageFont
        fam, estilo = ImageFont.truetype(str(arquivo), 20).getname()
        return f"{fam} ({estilo})"
    except Exception:
        return "?"


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--pasta", default=str(Path(__file__).resolve().parent.parent / "biblioteca" / "fontes"))
    ap.add_argument("--estilo", nargs="*")
    a = ap.parse_args()
    pasta = Path(a.pasta)
    linhas = ["# Biblioteca de fontes", "",
              "Geradas por `ferramentas/baixar_fontes.py`. Licença de cada família na pasta dela.", "",
              "| Estilo | Família (nome interno) | Arquivo | Uso |", "|---|---|---|---|"]
    falhas = 0
    for repo, arq, estilo, uso in CATALOGO:
        if a.estilo and estilo not in a.estilo:
            continue
        fam_dir = pasta / estilo / repo.split("/")[1]
        fam_dir.mkdir(parents=True, exist_ok=True)
        destino = fam_dir / arq
        try:
            if not destino.exists():
                baixar(BASE + repo + "/" + urllib.request.quote(arq), destino)
            lic = "OFL.txt" if repo.startswith("ofl/") else "LICENSE.txt"
            if not (fam_dir / lic).exists():
                baixar(BASE + repo + "/" + lic, fam_dir / lic)
        except Exception as e:  # rede, arquivo renomeado no repositório
            print(f"falhou {arq}: {e}", file=sys.stderr)
            falhas += 1
            continue
        linhas.append(f"| {estilo} | {nome_familia(destino)} | `{destino.relative_to(pasta).as_posix()}` | {uso} |")
    (pasta / "catalogo.md").write_text("\n".join(linhas) + "\n", encoding="utf-8")
    print(f"{len(linhas) - 6} fontes em {pasta} ({falhas} falhas); catálogo: {pasta / 'catalogo.md'}")


if __name__ == "__main__":
    main()
