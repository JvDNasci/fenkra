# Benchmark de detalhe: estilo visual (B311)

> Texto próprio, gerado por IA, sem revisão jurídica; resumo em palavras próprias de fontes públicas.
> Compara linguagens visuais de softwares CAD, de criação 3D e de engenharia, sem copiar telas, ícones
> ou textos. Fontes gerais em `procedencia.md` (linha de 2026-11-05, "Estilo visual"); este arquivo não
> traz endereços nem nomes de produtos proprietários.

## 0. Decisão aprovada

O usuário escolheu duas famílias de tema, **Prancheta** (claras e uma noturna, fonte de texto humanista e
fonte mono para valores) e **Cianotipia** (fundos azulados escuros, destaque quente, fonte condensada nos
títulos). Estão implementadas em `crates/fenkra-app/src/theme.rs` (B313): Prancheta Fria, Papel e
Noturna; Cianotipia Azul, Petróleo, Noturna e Sépia. O que segue registra o porquê e o que ainda medir.

## 1. Como cada família de software resolve o visual

| Grupo | Densidade | Hierarquia | Cor | O que esconde |
|---|---|---|---|---|
| CADs avançados comerciais (A a J) | Alta: faixas de abas com dezenas de botões | Abas por contexto, grupos com rótulo pequeno | Fundo cinza médio, ícones coloridos, muitos azuis | Comandos raros em listas recolhidas e em abas contextuais |
| Criação 3D livre (Blender) | Média a alta, ajustável por escala | Editores com cabeçalho; propriedades em abas laterais | Cinza escuro neutro, destaque laranja, eixos X/Y/Z em vermelho, verde e azul | Painéis recolhíveis, menus de busca e radiais |
| CAD livre (FreeCAD) | Média, barras soltas | Bancadas que trocam as barras | Tema do sistema, ícones coloridos de estilos misturados | Barras por bancada |
| CADs de tablet e de modelagem direta | Baixa: poucos botões grandes | Um painel por vez, modelo no centro | Fundos neutros, uma cor de ação | Quase tudo some até selecionar algo |
| Ferramentas de engenharia de dados e simulação | Média | Gráfico no centro, painéis estreitos | Escuro, alto contraste em valores | Opções avançadas em seção "mais" |

Padrões que descansam o olho, comuns aos que são elogiados em temas escuros da comunidade:

1. **Poucas cores fortes.** Uma cor de ação e uma de alerta; o resto é família neutra com saturação baixa.
2. **Revelar aos poucos.** O que é raro só aparece depois do gesto (seleção, comando). Reduz ruído sem
   cortar função.
3. **Contraste suficiente, não máximo.** Texto a 7:1 ou mais cansa em fundo escuro puro; fundos levemente
   acinzentados e texto levemente suave reduzem o halo.
4. **Valores em fonte mono**, rótulos em fonte de texto: o olho acha números sem ler o painel inteiro.
5. **Ícones de um único traço**, uma cor, preenchimento só para estado. Famílias mistas cansam.

## 2. O que hoje cansa no Fenkra (a medir)

| Causa | Evidência | Como medir |
|---|---|---|
| Ícones com a mesma matiz azul e variantes quase iguais | `interface.md`, seção 5 | Teste de 1 segundo (4 de 5 acertos) |
| Painéis grandes cobrindo o modelo | ADR 0012 já os torna translúcidos e livres | Área média coberta por painéis em 1080p |
| Muitas faixas coloridas na barra de comandos | `band_a` e `band_b` do tema | Número de cores distintas na barra |
| Texto pequeno com densidade alta | escala de interface e densidade em `theme.rs` | Altura x mínima em pixels nas três densidades |
| Contraste de componentes gráficos abaixo de 3:1 | `docs/pesquisa/acessibilidade.md`, lacuna A4 | Teste de contraste de ícones e contornos |

## 3. Direções propostas e escolhidas

| Direção | Ideia | Situação |
|---|---|---|
| Prancheta | Papel técnico: neutros quentes ou frios, uma cor de ação, contornos finos, fonte humanista e mono para valores | Escolhida; 3 temas prontos |
| Cianotipia | Cópia heliográfica: azuis profundos, linhas claras, destaque âmbar, títulos condensados | Escolhida; 4 temas prontos |
| Grafite | Escuro neutro e sóbrio, destaque único | Descartada: ficou parecida com os escuros comuns |
| Modelo | Interface mínima de poucos botões grandes | Descartada como tema; a ideia de revelar aos poucos vai para o ADR 0015 |

Regra em todo tema: a peça tem cor própria, sempre distinta do fundo (teste `temas_prontos_tem_contraste_e_peca_distinta`).

## 4. Pendências

- Medir a lista da seção 2 antes e depois, com capturas por `FENKRA_DEMO*`, em cada tema.
- Levar a seção 1 (ícones de um traço e uma cor) para a família de ícones nova.
- Cruzar com o modo daltônico: as semânticas azul e laranja da pesquisa de acessibilidade combinam com as
  duas famílias, mas os pares de estado precisam passar pelo teste de simulação (B320, item 1).
