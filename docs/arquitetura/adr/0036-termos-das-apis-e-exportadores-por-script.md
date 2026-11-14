# ADR 0036: Termos das APIs públicas dos CADs de origem e exportadores por script

- **Status:** aceita em 13/11/2026 pelo responsável do projeto (decisão abaixo, em "Decisão do responsável").
- **Data:** 2026-11-05
- **Item:** B267. Relacionados: B266 (esquema aberto), [ADR 0010](0010-referencias-v2.md)

> **Isto é análise técnica, não parecer jurídico.** Foi escrita por agente de IA, sem advogado, a partir do que
> se conseguiu ler em páginas públicas. Ela organiza fatos e uma política de engenharia. Não diz o que é lícito,
> não substitui a leitura do contrato e não vale como garantia para quem roda um script. Contratos mudam, e o que
> vincula cada pessoa é o texto que ela aceitou.

## Contexto

O caminho (b) da migração ([relatório de PLM](../../benchmark/detalhe/plm.md), seção 4.2) é um script que a pessoa
roda no CAD de origem, com a licença dela, para gravar a árvore num arquivo aberto. O relatório previa ler o contrato
de cada fabricante antes de publicar qualquer script.

Os produtos aparecem pelos rótulos neutros da legenda de [procedencia.md](../procedencia.md), que também guarda as
URLs consultadas.

## O que foi lido de verdade

A leitura ficou **incompleta**: várias páginas de contrato recusaram o acesso automático (HTTP 403) ou só remetem a
outro documento. Não tentamos contornar esses bloqueios. A coluna "Leitura" diz o nível de cada linha:

- **resumo do texto:** a página abriu, e o texto passou por um resumo automático; ninguém leu o original inteiro;
- **trecho indireto:** só um trecho, vindo de busca, de fórum ou de cópia de terceiros, sem conferir no original;
- **não lido:** nada aproveitável sobre o assunto.

| CAD de origem | API usada | Leitura em 05/11/2026 | O que se viu | Decisão |
|---|---|---|---|---|
| A | Automação local por COM, macros e suplementos | Trecho indireto (fórum de usuários). A página de termos do fabricante é só um índice | Trecho citado veda usar o programa ou a documentação para criar aplicativo distribuído a terceiros sem programa de parceiros ou acordo próprio. Há também veto a contornar travas de licença | **Bloqueado** |
| B | Automação local por COM | Não lido (mesmo fabricante do A) | Relatos de blog sobre a licença básica, sem valor de contrato | **Bloqueado** |
| C | Modelo de objetos local | Trecho indireto (busca). Termos gerais e de desenvolvedor deram 403 | Termos de desenvolvedor com data de 28/04/2026 vedam engenharia reversa. Uso de API para criar software comercial pede licença de desenvolvedor à parte. Falta ver o limite do uso interno | **Bloqueado** |
| D | Scripts e suplementos | Trecho indireto (mesmas páginas do C) | Vale o que se viu no C. Não se achou termo próprio de distribuição de suplemento | **Bloqueado** |
| E | REST em nuvem e lista de features em JSON | Resumo do texto dos termos de uso (vigência 15/07/2020). O contrato de API só aparece no portal do desenvolvedor: não lido | Veda engenharia reversa, acesso por meios automatizados e extração de dados, além de usar o serviço para análise competitiva ou para criar oferta semelhante. Acesso fora dos clientes oficiais segue o contrato de API. O conteúdo da pessoa pode sair em formatos padrão | **Bloqueado** |
| F | Kit de desenvolvimento local | Trecho indireto (busca, contrato de 2010). A página legal deu 403 | Licença para operação interna da pessoa. Fórum diz que o kit é licença extra. A documentação fala em distribuir aplicativo conforme o contrato | **Bloqueado** |
| G | Interface aberta do programa | Trecho indireto (busca, contrato de usuário v8.x). O PDF foi obtido, mas não foi extraído | Veda engenharia reversa. API só as publicadas, como descritas e em apoio ao uso autorizado. Há condições de licença mais novas que prevalecem, não lidas | **Bloqueado** |
| H | Automação local | Não lido (mesmo fabricante do G) | Sem nada próprio do produto | **Bloqueado** |

Todas as linhas estão bloqueadas porque nenhum contrato vigente foi lido por inteiro. O bloqueio é o estado da
leitura, não uma conclusão de que o script seja proibido.

## O que a leitura mudou

A hipótese do relatório era que entregar o script como código-fonte aberto reduziria o risco. Os trechos vistos
mostram que isso **não resolve sozinho** três pontos, e nenhum deles depende de haver binário:

1. **Quem escreve o script também usa o programa.** O trecho do A fala em usar o programa ou a documentação para
   criar aplicativo distribuído a terceiros. Código-fonte pode caber nessa frase. O risco fica com quem escreve e
   publica, não só com quem roda.
2. **Finalidade concorrente.** Os termos do E citam análise competitiva e oferta semelhante. Um exportador para outro
   CAD pode ser lido assim, ainda que a pessoa só esteja levando os próprios dados.
3. **Software comercial.** O texto do C trata de software comercial feito com a API. Falta saber se um script aberto
   ligado a um projeto com versão paga entra nisso.

Em todos os trechos, a restrição recai sobre o programa do fabricante. Nenhum deles fala do direito da pessoa sobre o
próprio modelo, e o E prevê a saída do conteúdo em formatos padrão. Isso aponta um caminho, sem confirmá-lo.

## Decisão

Política do projeto para exportadores:

1. **Nunca binário ligado à API.** O projeto não distribui executável, biblioteca ou suplemento compilado que ligue à
   API de um CAD de origem, nem arquivos do kit do fabricante.
2. **Só código-fonte legível.** Se um script sair, será texto aberto, que a pessoa lê e roda na própria instalação e
   com a própria licença. Ele grava um arquivo local e não envia nada para fora.
3. **Só interface documentada.** Sem chamada não documentada, sem contornar trava de licença.
4. **Sem engenharia reversa** de formato fechado, de programa ou de protocolo. O Fenkra não lê formato nativo fechado.
5. **Trava antes do primeiro script.** Nenhum script de um CAD entra no repositório enquanto a linha dele não estiver
   com leitura integral do contrato vigente, feita por pessoa, com data e cópia guardada, e decisão "liberado" dada
   pelo responsável do projeto. Os pontos 1 a 3 da seção anterior precisam de resposta de advogado antes disso.
6. **Quem escreve o script** declara sob qual licença trabalhou. Quem aceitou contrato com veto não contribui com aquele
   exportador, como já prevê a checklist da pesquisa de propriedade intelectual.
7. **Reler a cada versão pública** do script: a linha da tabela ganha nova data.

**O que a pessoa pode fazer na própria licença:** esta ADR não afirma. A hipótese de trabalho, a confirmar, é que rodar
um script próprio com a API documentada, em uso interno e sobre os próprios modelos, é o uso que essas APIs existem
para atender. Quem tiver dúvida deve ler o contrato que aceitou.

**O que não pode ser distribuído pelo projeto:** binários ligados à API, arquivos do kit do fabricante, exemplos e
documentação de terceiros, e, até a revisão, qualquer script para os CADs da tabela.

O formato aberto do B266 **não depende desta trava**: é uma especificação nossa, e o leitor do Fenkra não contém código
de nenhum fabricante. Qualquer pessoa pode gravar nele por conta própria.

## Decisão do responsável (13/11/2026)

O responsável do projeto liberou a integração por script, com estas condições, que passam a valer junto com os
pontos 1 a 4, 6 e 7 acima:

- **O caminho recomendado é o STEP.** A documentação e a interface do importador dizem que exportar em STEP no CAD de
  origem é o caminho ideal e suficiente para a geometria. O script serve só para quem também quer a árvore, os
  parâmetros e os nomes.
- **Só pela API própria do CAD de origem.** Importar e exportar dados nativos exige a API documentada que o próprio
  fabricante oferece, rodada pela pessoa na própria licença. O Fenkra nunca lê o formato nativo.
- **O script avisa antes de rodar** que usa a licença e a API de quem roda e que cabe a essa pessoa conferir o contrato
  que aceitou.
- O ponto 5 (trava) fica atendido para o primeiro CAD de origem, que o responsável tem instalado para teste (B298). Os
  demais continuam travados até nova decisão.

## Alternativas consideradas

- **Publicar o script já, confiando em ser código aberto:** descartada pelo ponto 1 acima.
- **Binário que ligue à API:** descartada. É o caso de maior exposição e o que os trechos citam.
- **Ler o formato nativo direto:** descartada. É engenharia reversa, contra a regra do projeto.
- **Publicar só a especificação do formato:** fica como saída se os contratos não permitirem script do projeto.

## Consequências

- O caminho (b) segue sem exportador. A migração com histórico só existe, por ora, entre arquivos no formato aberto.
- A próxima leitura é humana: abrir os contratos no navegador, guardar cópia datada e atualizar a tabela.
- Manter a tabela tem custo recorrente, somado à manutenção de um script por CAD.
