"""Confere os 3MF do Fenkra com a lib3mf, a biblioteca de referência do 3MF Consortium (BSD-2).

O gravador é nosso, em zip e XML; este script existe para um leitor que não escrevemos dizer se o
pacote é válido. A lib3mf entra só aqui, em Python, e nunca vai no programa.

Uso: pip install lib3mf
     FENKRA_3MF_SAIDA=<pasta> cargo test -p fenkra-conversor --test impressao amostras
     python scripts/validar_3mf.py <pasta>
"""
import pathlib
import sys

import lib3mf


def conferir(caminho, wrapper):
    modelo = wrapper.CreateModel()
    leitor = modelo.QueryReader("3mf")
    leitor.SetStrictModeActive(True)
    leitor.ReadFromFile(str(caminho))
    avisos = [leitor.GetWarning(i) for i in range(leitor.GetWarningCount())]
    if avisos:
        raise SystemExit(f"{caminho.name}: a lib3mf avisou: {avisos}")

    unidade = modelo.GetUnit()
    if unidade != lib3mf.ModelUnit.MilliMeter:
        raise SystemExit(f"{caminho.name}: unidade {unidade}, esperava milímetro")

    malhas = modelo.GetMeshObjects()
    objetos = triangulos = 0
    while malhas.MoveNext():
        malha = malhas.GetCurrentMeshObject()
        objetos += 1
        triangulos += malha.GetTriangleCount()
        if not malha.IsManifoldAndOriented():
            raise SystemExit(f"{caminho.name}: objeto {malha.GetResourceID()} não é fechado e orientado")
    if objetos == 0 or triangulos == 0:
        raise SystemExit(f"{caminho.name}: pacote sem malha")

    itens = modelo.GetBuildItems()
    n_itens = 0
    while itens.MoveNext():
        n_itens += 1
    if n_itens == 0:
        raise SystemExit(f"{caminho.name}: sem item de construção")

    print(f"{caminho.name}: {objetos} objetos, {triangulos} triângulos, {n_itens} itens, miniatura: {modelo.HasPackageThumbnailAttachment()}")


def main():
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    arquivos = sorted(pathlib.Path(sys.argv[1]).glob("*.3mf"))
    if not arquivos:
        raise SystemExit("nenhum .3mf na pasta")
    wrapper = lib3mf.get_wrapper()
    print("lib3mf", wrapper.GetLibraryVersion())
    for a in arquivos:
        conferir(a, wrapper)


if __name__ == "__main__":
    main()
