---
name: fenkra-chapa
description: Diretrizes da bancada Chapa metálica do Fenkra. Use ao criar, corrigir ou pesquisar fator K, tolerância de dobra, planificação, alívios e mapa de dobras, ou ao tocar em crates/fenkra-kernel/cpp/sheet.inc.
---

# Chapa metálica

Antes de criar ou mudar um comando, siga a skill `fenkra-comando` (contrato do ADR 0107).

## Domínio

crates/fenkra-kernel/cpp/sheet.inc; trechos de chapa em crates/fenkra-solidos/src/lib.rs e crates/fenkra-app/src/feat_ui.rs

## Antes de agir

1. Leia a seção da área em `docs/modulos/chapa-metalica.md` e, se precisar de contexto, `docs/pesquisa/chapa.md`.
2. Consulte `F:\Obsidian Maestro\Maestro\Projetos\Fenkra\Conhecimento\Chapa metálica\` antes de pesquisar de novo.
3. Veja os itens abertos: `python ~/.claude/skills/maestro/scripts/maestro.py backlog list --project Fenkra`.

## Pesquisar como funciona de verdade

Quando faltar entendimento de fator K, tolerância de dobra, planificação, alívios e mapa de dobras, busque primeiro documentação oficial do OpenCASCADE, normas,
livros e artigos técnicos, e depois documentação pública de outros CADs. Guarde uma nota por assunto
na pasta de conhecimento, com: como funciona, fórmulas, casos de borda, sintomas de falha e como
depurar. Escreva com suas palavras, sem copiar texto ou imagem, e registre as fontes em
`docs/arquitetura/procedencia.md`. Pesquise só o que a tarefa pede: pesquisa tem custo.

## Diretrizes

Preencha com o que for aprendido. Cada regra leva o motivo.

- Aba em perfil fechado (B579, nervura e caverna): a cadeia fechada exige quatro cuidados. (1) Lado de fora pela sonda no material (`chapa_fechada::lado_fora`), nunca pela caixa da peça (contorno de furo aponta para o centro). (2) Faixa planificada é um anel: `chapa_fechada::anel` (prisma da curva maior menos o da menor, cada laço em duas metades); a face do kernel com dois laços decide o furo pela área integrada, que erra em spline longa, e saía cheia. (3) Volume de conferência pela malha (`chapa_fechada::volume`): `mass()` erra até 13 % em face plana de spline longa. (4) A varredura de laço vai pela polilinha da aresta, não pela curva exata (saía inválida). Na costura da spline fechada a linha de fora usa a tangente que atravessa o começo.
- O comando de aba curva acha a aresta pelo elemento do meio da lista (`edge[len/2]`) e leva a cadeia inteira pelo quanto ele andou até o ponto médio do kernel: numa aresta fechada o ponto do meio tem de ser o ponto médio que o kernel dá (teste: `aresta_da_web`), senão a aba sai deslocada.
- Aba de linha de dobra curva: a planificação segue a conformação (prensa com borracha, hidroconformação), analisada em `chapa_hidro.rs`: ε = (1 + κ·x_f)/(1 + κ·D) − 1, convexo contrai e enruga, côncavo estica e trinca; `Status::Warning` com o trecho, nunca erro. Trecho reto continua pela dobradeira com fator K. Os limites (`Limites`) são hipóteses a calibrar; o alongamento limite está em Chapa › Parâmetros.
- Aba dobrada de perfil fechado (nervura de aerofólio): a união com a chapa é costura das arestas (`costura_t` em `sheet.inc`, `apply_aceita_x` com `costura = Some(tol)`), não a booleana geral: a lateral da chapa é B-spline longa e a dobra encosta nela de raspão, e a booleana leva 50 a 500 s. Fusão difusa não serve (só um valor passou e perdia material), cola também gasta 50 s. A costura só vale se `chapa_fechada::volume_estavel` (malha em duas flechas) concorda: a costura de loft saía com volume do kernel certo e malha de dezenas de vezes o volume nas flechas finas. Falhando, o comando cai na booleana de antes (loft: 80 a 437 s, anotado no ADR 0107).
- Mapa de deformação ("Mostrar deformação"): `RecomputeResult::conformacoes` guarda a `Analise` de cada aba curva e `scene::build` pinta o vértice (`chapa_deformacao.rs`); a cor vai em `MeshVertex::col`, sem mexer no wgpu. Opção nova de `Settings` entra em `config_camadas::CLASSIFICACAO`.

## Como verificar

`cargo test -p <crate> <filtro>`, com documento montado em código conferindo status, volume e caixa
envolvente. Mudança visual: capture com as variáveis `FENKRA_DEMO*`.

## Vizinhos

Consulte solidos, desenho quando a mudança atravessar a fronteira da área.

## Armadilhas

Registre aqui erros que se repetiram.

- Dobra por linha nasce sem linha (`a == b`): o recálculo trata isso como "esperando a face fixa" e o clique na face põe a linha no meio dela (`chapa_linha::linha_no_meio`). Valor padrão com `a != b` faz a dobra sair na ponta da peça (B469).
- Comando de chapa na lista `previa_leve::so_no_fim` mostra só a caixa da zona durante a edição. Só entra nela o que é caro de construir; a dobra cônica saiu (B470) e a parede base também (sem corpo com o painel aberto, a pessoa achava que o comando não fez nada).
- Aba, bainha, laço e aba curva nascem com a aresta em (0,0,0) quando nada estava selecionado. Esse ponto é "a escolher" (`aresta_a_escolher`): nunca resolva a aresta mais perto da origem, que dava erro ou aba na aresta errada.
- Teste de chapa que só monta o documento pronto não pega bug de interface. Use `chapa_fluxo_tests.rs`: `Cache::leve` com o comando aberto, aresta chegando depois, ponto em `f32` e chave promovida gravada.
- No reconhecimento de espessura, faces paralelas só contam se estão de costas (normais opostas, material entre elas): a fenda de um rasgo também tem faces que se enfrentam, com ar no meio.

- Comando que tira o corpo do coletor antes de montar o novo e falha no meio deixava a peça sem corpo (B580). O recálculo devolve os corpos ao estado anterior em qualquer erro (`erro_corpo.rs`); não restaure à mão dentro do comando.
- Parede base sem furo costuma ser contorno externo aberto, não bug de furo: `make_face` fura por profundidade. Para a nervura/caverna a partir de um corpo use a Interseção do sketch com "Corpo inteiro" (`LinkSrc::Body`), que dá o contorno fechado e acompanha a estação.

- Edição por script Python no Windows: abra e grave com `newline=''`; sem isso o arquivo vira CRLF e os testes que leem o próprio fonte (`espessar_paridade_tests`) falham em "não achei }".
- Volume pela malha de uma forma costurada ou com arestas de tolerância alta pode variar com a flecha (o do kernel não): confira em duas flechas antes de aceitar.

## Ao terminar

Comando criado ou alterado gera o material simples em `Comandos/`, como manda o `CLAUDE.md`.

## Lições do kart (B353)

- Para mostrar planificação, o 3D deve estar na chapa plana: tabela de dobras certa no desenho não vale como captura.
- Depois de abrir o painel, confira que ele aparece na captura antes de dizer que foi visto.
