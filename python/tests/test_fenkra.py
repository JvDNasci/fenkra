# SPDX-License-Identifier: LGPL-2.1-or-later
"""Testes da API Python. Rodam pelo `cargo test -p fenkra-cli`, que passa o programa em FENKRA_CLI."""

import os
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

from fenkra import API_VERSAO, Fenkra, FenkraErro  # noqa: E402

RETANGULO = {
    "pontos": [
        {"id": "a", "x": 0, "y": 0},
        {"id": "b", "x": 40, "y": 0},
        {"id": "c", "x": 40, "y": 30},
        {"id": "d", "x": 0, "y": 30},
    ],
    "curvas": [
        {"id": "l1", "geometria": {"tipo": "reta", "a": "a", "b": "b"}},
        {"id": "l2", "geometria": {"tipo": "reta", "a": "b", "b": "c"}},
        {"id": "l3", "geometria": {"tipo": "reta", "a": "c", "b": "d"}},
        {"id": "l4", "geometria": {"tipo": "reta", "a": "d", "b": "a"}},
    ],
    "restricoes": [
        {"tipo": "fixa", "entidades": ["a"]},
        {"tipo": "horizontal", "entidades": ["l1"]},
        {"tipo": "vertical", "entidades": ["l2"]},
        {"tipo": "horizontal", "entidades": ["l3"]},
        {"tipo": "vertical", "entidades": ["l4"]},
        {"tipo": "comprimento", "entidades": ["l1"], "valor": "L"},
        {"tipo": "comprimento", "entidades": ["l2"], "valor": "30"},
    ],
}


def montar(f):
    f.nova_peca("Suporte")
    f.definir_parametro("L", 40)
    esboco = f.criar_esboco("XY", **RETANGULO)
    f.extrudar(esboco["id"], {"tipo": "cego", "distancia": "20"})
    plano = f.criar_plano(20)
    furo = f.criar_esboco(
        {"referencia": plano["id"]},
        [{"id": "p", "x": 20, "y": 15}],
        [{"id": "c", "geometria": {"tipo": "ponto", "ponto": "p"}}],
    )
    return f.furar(furo["id"], 8, {"tipo": "atravessar"})


class Api(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="fenkra-py-peças-")
        self.addCleanup(self.tmp.cleanup)
        self.pasta = pathlib.Path(self.tmp.name)

    def test_abre_muda_parametro_e_exporta_step(self):
        with Fenkra(self.pasta) as f:
            self.assertEqual(montar(f)["status"], "ok")
            f.salvar("suporte.fkx")
        with Fenkra(self.pasta) as f:
            f.abrir("suporte.fkx")
            antes = f.medir()["massa"]["volume"]
            f.definir_parametro("L", "80")
            self.assertEqual(f.parametros()[0]["valor"], 80)
            depois = f.medir()["massa"]["volume"]
            self.assertAlmostEqual(depois - antes, 40 * 30 * 20, places=3)
            f.exportar("suporte-80.step")
            self.assertTrue((self.pasta / "suporte-80.step").is_file())
            self.assertAlmostEqual(f.caixa("suporte-80.step")["tamanho"][0], 80, places=4)

    def test_consulta_de_topologia(self):
        with Fenkra(self.pasta) as f:
            montar(f)
            f.exportar("s.step")
            furos = f.furos("s.step")
            self.assertEqual(len(furos), 1)
            self.assertAlmostEqual(furos[0]["diametro"], 8, places=5)
            self.assertEqual(len(f.planos("s.step")), 6)
            self.assertEqual(len(f.faces()), 7)
            self.assertEqual(len(f.cilindros()), 1)
            self.assertTrue(f.comparar("s.step")["aprovado"])

    def test_erro_vira_excecao_com_codigo_estavel(self):
        with Fenkra(self.pasta) as f:
            with self.assertRaises(FenkraErro) as e:
                f.abrir("../fora.fkx")
            self.assertEqual(e.exception.codigo, "caminho_fora")
            with self.assertRaises(FenkraErro) as e:
                f.chamar("executar_python", codigo="print(1)")
            self.assertEqual(e.exception.codigo, "comando_desconhecido")
            montar(f)
            f.exportar("a.step")
            with self.assertRaises(FenkraErro) as e:
                f.exportar("a.step")
            self.assertEqual(e.exception.codigo, "ja_existe")
            f.exportar("a.step", sobrescrever=True)

    def test_programa_ausente_diz_o_que_fazer(self):
        guardado = os.environ.pop("FENKRA_CLI")
        self.addCleanup(os.environ.__setitem__, "FENKRA_CLI", guardado)
        caminho = os.environ.get("PATH", "")
        os.environ["PATH"] = ""
        self.addCleanup(os.environ.__setitem__, "PATH", caminho)
        with self.assertRaises(FenkraErro) as e:
            Fenkra(self.pasta)
        self.assertEqual(e.exception.codigo, "programa_ausente")

    def test_chapa_e_montagem(self):
        with Fenkra(self.pasta) as f:
            f.nova_peca("Chapa")
            f.definir_chapa(espessura="2", raio="2", fator_k="0.44")
            esboco = f.criar_esboco("XY", **{k: RETANGULO[k] for k in ("pontos", "curvas")})
            self.assertEqual(f.adicionar("parede_chapa", esboco=esboco["id"])["status"], "ok")
            aba = f.adicionar("aba", aresta=[40, 15, 2], comprimento="10", angulo="90")
            self.assertEqual(aba["status"], "ok")
            self.assertGreater(aba["volume"], 40 * 30 * 2)
            f.salvar("chapa.fkp")
            f.nova_montagem("Conjunto")
            f.inserir_componente("chapa.fkp")
            f.inserir_componente("chapa.fkp", origem=[0, 0, 50])
            r = f.restringir("contato", {"componente": "c1", "plano": "XY"}, {"componente": "c2", "plano": "XY"})
            self.assertEqual(r["status"], "ok")
            self.assertEqual(f.bom()["linhas"][0]["quantidade"], 2)
            f.salvar_montagem("conjunto.fka")
            self.assertTrue((self.pasta / "conjunto.fka").is_file())

    def test_versao(self):
        with Fenkra(self.pasta) as f:
            self.assertEqual(f.chamar("versao")["api"], API_VERSAO)


if __name__ == "__main__":
    unittest.main()
