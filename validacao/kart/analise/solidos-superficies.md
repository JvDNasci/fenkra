# Análise pós-kart: Sketch, Sólidos, Superfícies e Forma livre (inclui Cobertura)

Base: `diario/` (imagens abertas: 0001, 0023, 0026, 0030, 0032, 0035, 0041, 0054, 0055, 0058, 0064, 0070,
0072, 0074, 0079, 0080, 0082, 0083, 0116, 0120), `resultados.md`, `bugs.md`, `relatorio.md`. Não abri as
demais capturas "pintado" nem as peças sem comparação nova. Tudo foi modelado em código, não pela interface.

## 1. O que foi usado e o que não foi

- **Sólidos (usado):** Extrusão (35), Revolução (12), Furo (19), Padrão (14), Chanfro (5), Filete (4), Unir
  corpos, Saída, Reforço, Espessura, Rosca, Varredura, Espelho, Escala, Mover corpo e Dividir corpo, cada um
  de 1 a 2 vezes. Peças reais: eixo, cubos, aros, disco, pinça, manga.
- **Superfícies (usado):** Spline, Revolução + Engrossar (pneus), Multi-seções + Casca (tanque, escape),
  Extrudada e Deslocada (CV-SUP1), Preenchimento (CV-SUP2).
- **Forma livre (usado):** só por código (`Forma` 6x) e convertido em B-rep: volante, banco, bico, painel,
  pontão, proteção traseira. Nenhuma ferramenta da bancada foi aberta na interface.
- **Sketch (quase não visto):** 68 sketches criados, mas só há 1 captura da bancada (0001, o esqueleto
  mestre, só pontos e planos). Não vi editor de sketch, restrição, grau de liberdade nem cota. O solver não
  foi exercitado de forma visível.
- **Não usado:** Casca em Forma livre, Junção/Aparar e dividir/Filete/Varredura de superfície (só falhou),
  Hélice, Interseção, Projeção, Paralela e as curvas de continuidade G1/G2. O relatório não diz que foram
  testados e não vi captura.
- **Por quê:** o kart é de usinagem, tubo e chapa; Superfícies entra só em pneu, tanque e escape.

## 2. O que deu certo

- `0026_ex-05_pronta.png` Cubo: revolução, furo com padrão, rasgo de chaveta, chanfro e filete. Parece cubo
  de roda; a árvore de histórico está legível com nomes ("Rasgo").
- `0041_di-08_pronta.png` Manga: dois corpos ("Corpo principal" e "Braço") unidos por booleana, com PMI
  (Ø25, perpendicularidade) bem posicionado sobre a geometria.
- `0023_ex-01_pronta.png` Eixo tubular de 1040 mm com quatro rasgos e chanfro. Nomes como "Tubo do eixo" e
  "Rasgo da coroa" tornam a árvore autoexplicativa.
- `0030_rd-01_pronta.png` Aro com abas e pino do talão: forma reconhecível.
- `0032_rd-03_pronta.png` Pneu: perfil spline com pontos P0..P8 visíveis no 3D e superfície engrossada. É a
  melhor demonstração Superfícies. O pneu é suave e parece pneu.
- `0035_di-01_pronta.png` Volante: aro de Forma livre com raios e cubo, legível e proporcional.
- `0055_ex-12_segunda_tentativa.png` Varredura sólida seguindo spline 3D: tubo contínuo, sem torção visível.
- `0054_ex-12_falhou.png` O Assistente de reparo abre sozinho, marca a feature em vermelho e oferece
  Refazer, Suprimir e Apagar. Bom comportamento em falha, apesar do texto (item 3).
- `0116_ex-08_pronta_pintado.png` Disco escuro com 24 furos: padrão circular correto e a peça se destaca
  bem do fundo.
- `0082_cv-sup1_pronta.png` Superfície extrudada e deslocada: parede curva com espessura e corte colorido.
- Sem erro de comando nas peças de Sólidos. Volumes e caixas batem com a intenção (resultados.md).

## 3. O que falhou

- **K-10 (A)**: sólido inválido passa sem aviso. Na captura `0030_rd-01_pronta.png` o aro parece perfeito
  (tem Furo, PadrãoCircular e "Pino do talão" no histórico) e o status é ok. O defeito só aparece na
  interferência (volume comum maior que a peça). É a pior falha silenciosa da bancada.
- **K-08 (M)**: `0054_ex-12_falhou.png`. O erro "não foi possível varrer o perfil" aparece em Engrossar1, e
  Superfície1 também fica vermelha **sem texto**. O Assistente oferece "Refazer referência", ação que não
  conserta uma falha de geometria. A legenda 3D mostra "-0,000 kg" para peça sem corpo.
- **K-18 (A)**, **K-13 (M)**, **K-14 (B)**, **K-21 (B)**: já em `bugs.md`.
- **Peças em Forma livre ficam maciças**: bico 55,6 L, banco 35,9 L, massa de 1415,9 kg no conjunto
  (relatório). Nada na peça avisa que o valor é implausível. Parte é limite do teste (gaiola por código,
  volumes simples), parte é a falta de alerta de massa.
- **Recálculo lento em Forma livre**: PO-01 leva 12,0 s e a captura padrão pegou a tela de abertura
  (`0058_po-01_pronta.png` é só a tela "Calculando a peça..."). Esta imagem não serve como evidência da peça.

## 4. Deu certo, mas dá para melhorar

- **Bancada errada ao abrir** (estende K-20): peças de Superfícies e Forma livre abrem em "Modelagem Sólida"
  (`0064`, `0074`, `0082`, `0083`, `0070`, `0072`), então a barra de comandos mostrada não é a do conteúdo.
  Só `0032` e `0054` abrem em Superfícies. O diário classifica como "Superfícies" peças que a tela mostra em
  Sólidos.
- **Esqueleto mestre ilegível** (`0001`): 60 itens de geometria e 89 publicações em lista plana, com nomes
  curtos (LGE0..4, TR_MONT_A); a vista mostra três planos pequenos e uma linha laranja que atravessa a tela
  inteira, sem enquadramento. Não dá para entender o kart por ali. A árvore precisa agrupar por tema.
- **Enquadramento**: `0074_mo-03_pronta.png` tem o escape cortado no topo da janela. O rodapé diz
  "duplo clique: ajustar", mas ao abrir a peça não há ajuste automático (pode ser limite da captura).
- **Cor do corpo contra o fundo**: `0120_pr-06_pronta_pintado.png` (bico branco em fundo claro) quase some.
  Quebra a regra "peça sempre distinta do fundo" do padrão de interface.
- **Legenda 3D sobreposta** (K-22): em todas as capturas, rótulo "FK-... Material massa" e "Tolerância geral"
  se escrevem um sobre o outro (ex.: `0023`, `0030`). Ilegível no vídeo.
- **Cobertura fraca visualmente:** `0083_cv-sup2_pronta.png` mostra uma face quase plana; não demonstra
  Preenchimento (tangência). `0080_cv-sol2` não mostra escala nem divisão. `0079_cv-sol1` mostra 4 furos, mas
  a Espessura e a rosca modelada não são distinguíveis; só vejo o aviso de rosca cosmética no Furo1. CV-SOL3
  (padrão por pontos e chanfro geral) não tem captura.
- **Passos:** o pneu usa 9 pontos mais curva mais superfície mais engrossar; para um perfil de revolução
  seriam menos com um sketch de revolução. Não conferi se a interface oferece o atalho.

## 5. Melhores imagens para o vídeo

1. `0026_ex-05_pronta.png` Peça de usinagem completa com histórico na árvore (revolução, padrão, rasgo, filete).
2. `0041_di-08_pronta.png` Dois corpos unidos e cotas/tolerâncias 3D sobre a geometria.
3. `0032_rd-03_pronta.png` Spline com pontos virando pneu: parametria em Superfícies.
4. `0055_ex-12_segunda_tentativa.png` Varredura sólida por uma curva 3D livre.
5. `0054_ex-12_falhou.png` O Assistente de reparo aparecendo com o erro (mostra honestidade do produto).
6. `0035_di-01_pronta.png` Volante em Forma livre convertido em sólido, peça fácil de reconhecer.

## 6. Regras para a skill da bancada

- Peça com padrão (circular/linear) de corte feito em direção não normal pode gerar sólido inválido com status ok: sempre conferir volume contra a peça-base ou rodar verificação de validade (K-10).
- Chanfro/filete por ponto em aresta circular exige o ponto médio exato da aresta; prefira escolher pela interface ou por ID (K-13).
- Superfície de varredura com perfil circular em spline 3D falha: use varredura sólida (Nervura) como contorno até corrigir (K-08).
- Peça de Forma livre vira maciça; sempre aplicar Casca ou checar volume/massa antes de ir para a montagem.
- Captura de peça longa (Forma livre, 3 a 12 s) precisa esperar o recálculo, senão sai a tela de abertura.

## 7. Itens novos de backlog

| Título | Prioridade |
|---|---|
| Verificação de validade do sólido após cada feature, com ícone de aviso na árvore (generaliza K-10) | A |
| Abrir a peça na bancada do conteúdo, incluindo Superfícies e Forma livre (estende K-20) | M |
| Erro em cadeia de superfície: mostrar a causa na Superfície1 e trocar "Refazer referência" pela ação certa | M |
| Alerta de massa/volume implausível (volume maior que X% da caixa envolvente, ou massa fora de faixa) | B |
| Recálculo de Forma livre: cache da conversão em B-rep (PO-01 12 s, DI-01 3 s) e indicador de progresso na peça | M |
| Esqueleto mestre: agrupar geometria publicada por prefixo ou tema na árvore e enquadrar a vista ao abrir | B |
| Legenda 3D: não mostrar "Tolerância geral" colada ao rótulo da peça; evitar sobreposição por padrão | B |
| Massa "-0,000 kg" na legenda de peça sem corpo: mostrar "sem corpo" | B |
| Contraste mínimo entre cor da peça e fundo ao aplicar aparência (aviso ou contorno reforçado) | B |
| Sketch: capturas e testes do editor com restrições, graus de liberdade e cotas (não verificado neste teste) | M |

## Limite deste relatório

Nada foi testado por mouse. As capturas são estáticas; tooltips, prévia, alças e a bancada Forma livre na
interface não foram vistos. Como não recebi run-id do Maestro, não publiquei status no painel.
