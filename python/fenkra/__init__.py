# SPDX-License-Identifier: LGPL-2.1-or-later
"""API Python do Fenkra: um cliente fino do `fenkra-cli servir`.

Não há código nativo nem interpretação de texto aqui. O script fala JSON com o programa, um pedido por
linha, e o programa só conhece a lista fechada de comandos. Tudo fica dentro da pasta informada, e nada
é sobrescrito sem `sobrescrever=True`. O contrato está em `docs/formatos/api-json.md`.

    from fenkra import Fenkra

    with Fenkra("C:/projetos/suporte") as f:
        f.abrir("suporte.fkx")
        f.definir_parametro("L", "120")
        print(f.medir()["massa"]["volume"])
        f.exportar("suporte-120.step")
"""

import json
import os
import shutil
import subprocess

__all__ = ["API_VERSAO", "Fenkra", "FenkraErro"]

#: Versão do contrato JSON que este cliente entende.
API_VERSAO = 1


class FenkraErro(Exception):
    """Um comando falhou. `codigo` é estável; `mensagem` é para gente."""

    def __init__(self, codigo, mensagem):
        super().__init__(f"{codigo}: {mensagem}")
        self.codigo = codigo
        self.mensagem = mensagem


def _achar_programa(programa):
    achado = programa or os.environ.get("FENKRA_CLI") or shutil.which("fenkra-cli")
    if not achado:
        raise FenkraErro("programa_ausente", "fenkra-cli não encontrado: informe `programa` ou a variável FENKRA_CLI")
    return achado


class Fenkra:
    """Uma sessão: uma peça aberta de cada vez, dentro de `pasta`."""

    def __init__(self, pasta, programa=None):
        comando = [_achar_programa(programa), "--pasta", os.fspath(pasta), "servir"]
        self._processo = subprocess.Popen(
            comando,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            encoding="utf-8",
        )
        self._proximo = 0
        versao = self.chamar("versao")
        if versao["api"] != API_VERSAO:
            self.fechar()
            raise FenkraErro("versao", f"o programa fala a versão {versao['api']} da API; este cliente, a {API_VERSAO}")

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.fechar()

    def fechar(self):
        if self._processo.poll() is None:
            self._processo.stdin.close()
            self._processo.wait(timeout=30)
        self._processo.stdout.close()

    def chamar(self, comando, **args):
        """Roda um comando da lista e devolve o `resultado`; falha vira `FenkraErro`."""
        self._proximo += 1
        pedido = {"id": self._proximo, "comando": comando, "args": args}
        try:
            self._processo.stdin.write(json.dumps(pedido, ensure_ascii=False) + "\n")
            self._processo.stdin.flush()
            linha = self._processo.stdout.readline()
        except (BrokenPipeError, OSError, ValueError) as e:
            raise FenkraErro("queda", f"o programa encerrou: {e}") from e
        if not linha:
            raise FenkraErro("queda", "o programa encerrou sem responder")
        resposta = json.loads(linha)
        if resposta.get("id") != self._proximo:
            raise FenkraErro("protocolo", "resposta fora de ordem")
        if not resposta["ok"]:
            raise FenkraErro(resposta["erro"]["codigo"], resposta["erro"]["mensagem"])
        return resposta["resultado"]

    # peça
    def nova_peca(self, nome="Peça"):
        return self.chamar("nova_peca", nome=nome)

    def abrir(self, arquivo):
        return self.chamar("abrir", arquivo=arquivo)

    def salvar(self, arquivo, sobrescrever=False):
        return self.chamar("salvar", arquivo=arquivo, sobrescrever=sobrescrever)

    def exportar(self, arquivo, sobrescrever=False):
        return self.chamar("exportar", arquivo=arquivo, sobrescrever=sobrescrever)

    # parâmetros
    def parametros(self):
        return self.chamar("parametros")["parametros"]

    def definir_parametro(self, nome, expressao, grandeza="comprimento"):
        return self.chamar("definir_parametro", nome=nome, expressao=str(expressao), grandeza=grandeza)

    # modelagem: os argumentos são os campos do comando no `.fkx`
    def criar_plano(self, deslocamento, base="XY", **campos):
        return self.chamar("criar_plano", deslocamento=str(deslocamento), base=base, **campos)

    def criar_esboco(self, plano, pontos, curvas, **campos):
        return self.chamar("criar_esboco", plano=plano, pontos=pontos, curvas=curvas, **campos)

    def extrudar(self, esboco, termino, operacao="unir", **campos):
        return self.chamar("extrudar", esboco=esboco, operacao=operacao, termino=termino, **campos)

    def furar(self, esboco, diametro, termino, **campos):
        return self.chamar("furar", esboco=esboco, diametro=str(diametro), termino=termino, **campos)

    def filetar(self, raio, arestas, **campos):
        return self.chamar("filetar", raio=str(raio), arestas=arestas, **campos)

    def chanfrar(self, distancia, arestas, **campos):
        return self.chamar("chanfrar", distancia=str(distancia), arestas=arestas, **campos)

    def desfazer(self):
        return self.chamar("desfazer")

    def adicionar(self, tipo, **campos):
        """Qualquer comando do formato da árvore pelo tipo: revolucao, aba, membro, reforco..."""
        return self.chamar("adicionar", tipo=tipo, **campos)

    def definir_chapa(self, **campos):
        return self.chamar("definir_chapa", **campos)

    # montagem
    def nova_montagem(self, nome="Montagem"):
        return self.chamar("nova_montagem", nome=nome)

    def inserir_componente(self, arquivo, **campos):
        return self.chamar("inserir_componente", arquivo=arquivo, **campos)

    def restringir(self, tipo, a, b, **campos):
        return self.chamar("restringir", tipo=tipo, a=a, b=b, **campos)

    def mover_componente(self, componente, **campos):
        return self.chamar("mover_componente", componente=componente, **campos)

    def montagem(self):
        return self.chamar("montagem")

    def bom(self, nivel="topo"):
        return self.chamar("bom", nivel=nivel)

    def salvar_montagem(self, arquivo, sobrescrever=False):
        return self.chamar("salvar_montagem", arquivo=arquivo, sobrescrever=sobrescrever)

    # consulta
    def recalcular(self):
        return self.chamar("recalcular")

    def medir(self):
        return self.chamar("medir")

    def topologia(self, arquivo=None, secoes=None, limite=None):
        args = {k: v for k, v in (("arquivo", arquivo), ("secoes", secoes), ("limite", limite)) if v is not None}
        return self.chamar("topologia", **args)

    def faces(self, arquivo=None):
        return self.topologia(arquivo, ["faces"])["faces"]

    def cilindros(self, arquivo=None):
        return self.topologia(arquivo, ["cilindros"])["cilindros"]

    def furos(self, arquivo=None):
        return self.topologia(arquivo, ["cilindros"])["furos"]

    def planos(self, arquivo=None):
        return self.topologia(arquivo, ["planos"])["planos"]

    def caixa(self, arquivo=None):
        return self.topologia(arquivo, ["caixa"])["caixa"]

    def comparar(self, alvo, arquivo=None, tolerancias=None):
        args = {k: v for k, v in (("arquivo", arquivo), ("tolerancias", tolerancias)) if v is not None}
        return self.chamar("comparar", alvo=alvo, **args)
