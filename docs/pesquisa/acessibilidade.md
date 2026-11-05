# Acessibilidade e modo daltônico (B320)

> Gerado por IA, sem revisão jurídica. Resumo em palavras próprias de normas e documentação públicas.
> As fontes estão em [procedência](../arquitetura/procedencia.md). Os endereços foram registrados de
> memória das publicações e devem ser reabertos na revisão humana antes de qualquer citação formal.

## 1. O que a pessoa precisa

| Limitação | O que quebra num CAD | O que precisamos |
|---|---|---|
| Daltonismo (cerca de 8% dos homens e 0,5% das mulheres de ascendência europeia; a maioria é deuteranomalia ou protanomalia) | Vermelho x verde em estado, erro, mapas de análise, prévia de adicionar x remover | Paletas seguras e nunca só cor |
| Baixa visão | Texto pequeno, linhas finas, contraste baixo, ícones de 16 px | Escala, contraste mínimo, espessura de traço ajustável |
| Motora | Alvos pequenos, arrastar com precisão, atalhos com duas mãos | Alvos de 24 px ou mais, alternativa por teclado, tempo sem limite |
| Sensibilidade a movimento | Animação de câmera, pulsos e transições | Opção de reduzir movimento |
| Cegueira | Tudo | Leitor de tela nos painéis e menus; o 3D em si fica fora de alcance |

## 2. Daltonismo

### 2.1 Como se modela

As três formas de dicromacia (protanopia, deuteranopia, tritanopia) são simuladas projetando a cor no
espaço dos cones (LMS) e removendo o cone ausente. Os métodos publicados mais usados são o de Brettel,
Viénot e Mollon (1997), que opera sobre planos de confusão, e a matriz linear de Machado, Oliveira e
Fernandes (2009), que também cobre anomalias parciais com severidade de 0 a 1. Para a verificação de
paletas basta a matriz de Machado em RGB linear: aplicar a matriz às cores da paleta e medir a distância
entre pares (Delta E em Lab, mínimo sugerido de 10 entre cores que precisam ser distintas).

Casos de borda: simular em RGB linear, não em sRGB codificado; clarear não resolve confusão, que é de
matiz; achromatopsia (monocromacia) exige que a luminância sozinha distinga os estados.

### 2.2 Paleta segura

A paleta qualitativa de Okabe e Ito (8 cores, 2008) foi desenhada para as três dicromacias e é a base
usual. Resumo das tonalidades: laranja, azul-celeste, verde-azulado, amarelo, azul, vermelhão, roxo-rosado
e preto. Regra para o Fenkra: estados semânticos usam pares que se separam por **luminância e matiz
azul-laranja**, não vermelho-verde. Sugestão inicial (a validar com a matriz de Machado e com Delta E):

| Semântica | Hoje (a conferir em `theme.rs`) | Proposta |
|---|---|---|
| Acrescenta (prévia) | azul-água | azul-celeste, traço contínuo |
| Remove (prévia) | vermelho translúcido | laranja-vermelhão, hachura diagonal |
| Referência | cinza | cinza, tracejado |
| OK / erro / aviso | verde / vermelho / amarelo | azul-esverdeado com "✓", vermelhão com "■", amarelo com "▲" |

### 2.3 Nunca só cor

A WCAG 2.2, critério 1.4.1 (Uso de cor), exige que cor não seja o único meio de transmitir informação.
O código já faz isso em um lugar (`dwg_ui.rs`, símbolos ●, ▲, ■ no estado da vista). A regra passa a valer
em todos os pontos: estado de restrição do sketch (livre, restrito, em conflito), erro de recompute na
árvore, seleção x pré-seleção, prévia de acrescentar x remover, mapas de análise (curvatura, desvio,
interferência) e cores de camada no desenho. Receita por tipo: símbolo, hachura, espessura, tracejado ou
texto curto, além da cor.

### 2.4 Mapas de análise

Mapas de zebra, curvatura e desvio costumam usar arco-íris, que é ruim para daltônicos e para todos
(não é perceptualmente uniforme). Usar rampas sequenciais de luminância monótona (famílias viridis e
cividis, esta última pensada para daltonismo) e deixar a escala com valores numéricos e marcas de nível
legíveis em escala de cinza.

### 2.5 Verificação no produto

Um modo "simular daltonismo" nas configurações aplicaria a matriz de Machado ao quadro final (viewport e
interface) em um passe de pós-processamento, para o próprio projetista conferir seus desenhos. Fica como
ferramenta de teste e de apresentação, separada do "tema daltônico", que muda a paleta de verdade.

## 3. Contraste, tamanho e escala

- **Texto:** WCAG 1.4.3, contraste mínimo de 4,5:1 (3:1 para texto grande, a partir de 18 pt ou 14 pt em
  negrito). Nível AAA (1.4.6) pede 7:1. Componentes e gráficos, 1.4.11: 3:1 contra o fundo adjacente
  (contornos de campo, ícones, foco).
- **Estado hoje:** `theme.rs` tem testes de contraste mínimo 4,5 para texto e 2,2 para a base de ícones.
  O limite de 2,2 fica abaixo do 3:1 de componentes gráficos: ícones são informação e deveriam atingir 3:1.
- **Escala:** `ui_scale` existe nas configurações. Verificar a faixa (de 0,75 a pelo menos 2,0, que cobre
  o redimensionamento de 200% do 1.4.4) e se alvos, ícones e viewport acompanham.
- **Alvos:** 2.5.8 (WCAG 2.2) pede alvos de pelo menos 24 por 24 px CSS. Ícones de 16 px com espaçamento
  podem passar, mas o círculo de pega das alças 3D (22 px, `interface.md`) fica abaixo.
- **Espessura de traço:** linhas de sketch e de seleção com opção de engrossar, pensando em baixa visão.

## 4. Teclado, foco e movimento

- **Teclado:** 2.1.1, todas as funções por teclado. No CAD, o gesto no 3D é inerentemente de ponteiro; a
  alternativa é digitar o valor no campo e mover o foco por Tab nos painéis. Comandos pela busca (B256 e a
  paleta de comandos) já são caminho por teclado.
- **Foco visível:** 2.4.7 e 2.4.11 (WCAG 2.2, foco não obscurecido). Painéis translúcidos sobre a viewport
  (ADR 0012) não podem cobrir o elemento focado; o contorno de foco precisa de 3:1.
- **Sem armadilha de teclado:** 2.1.2, Esc sempre sai (alinhado ao ADR 0015).
- **Redução de movimento:** respeitar a preferência do sistema e uma chave própria: sem animação de
  câmera (vista cúbica salta em vez de deslizar), sem pulso em destaques, sem transições de painel.

## 5. Leitor de tela no egui

O egui integra o AccessKit, que expõe uma árvore de acessibilidade às APIs do sistema (UI Automation no
Windows, NSAccessibility no macOS, AT-SPI no Linux). No eframe 0.37 a feature `accesskit` faz parte do
conjunto padrão; o `Cargo.lock` do Fenkra já resolve `accesskit` 0.25, então a base está compilada.
Cada widget padrão informa papel e nome. O que falta é nosso:

- botões só com ícone precisam de nome acessível (hoje o texto está no tooltip, que o leitor não lê);
- widgets pintados à mão (árvore em `tree_ui.rs`, painéis em `paineis.rs`, ViewCube, alças) não aparecem
  na árvore e precisam de informação de papel, nome e estado;
- a viewport 3D é uma imagem; oferecer uma descrição textual do que está selecionado e do estado do
  comando na barra de status, marcada como região viva.

Depuração: ferramentas de inspeção da árvore de acessibilidade do sistema (Accessibility Insights no
Windows, Accessibility Inspector no macOS) mostram o que o leitor enxerga; sintoma de falha é o botão
anunciado como "botão" sem nome.

## 6. O que falta no Fenkra

| # | Lacuna | Evidência | Prioridade |
|---|---|---|---|
| A1 | Sem paleta nem tema seguro para daltonismo; estados usam verde x vermelho | `theme.rs`, cores semânticas | Alta |
| A2 | "Nunca só cor" só vale na vista do desenho | `dwg_ui.rs` tem; sketch, árvore, prévia e análise não | Alta |
| A3 | Mapas de análise sem rampa perceptual e sem escala legível em cinza | a conferir em `render.rs` | Média |
| A4 | Teste de contraste de componentes (3:1) e de pares de estado após simulação | `theme.rs` testa só texto e base de ícone | Alta |
| A5 | Botões só de ícone sem nome acessível; widgets pintados fora da árvore | `icons.rs`, `tree_ui.rs`, `paineis.rs` | Média |
| A6 | Sem opção de reduzir movimento | animação de câmera e destaques | Média |
| A7 | Sem foco visível garantido sob painéis translúcidos | ADR 0012 | Média |
| A8 | Alças 3D com pega de 22 px e sem alternativa por teclado | `handles.rs` | Média |
| A9 | Faixa de `ui_scale` e espessura de traço não medidas | `settings.rs` | Baixa |
| A10 | Sem modo de simulação de daltonismo para conferência | novo | Baixa |

## 7. Itens de implementação propostos

1. **Tema com paleta segura** (A1, A4): semânticas azul-laranja, validadas por teste que simula as três
   dicromacias e exige Delta E mínimo entre pares de estado.
2. **Regra "nunca só cor"** (A2): tabela única de estado para símbolo e hachura, usada por sketch, árvore,
   prévia, desenho e análise; teste que falha se um estado semântico novo não tiver símbolo.
3. **Rampas de análise perceptuais** (A3) com escala numérica.
4. **Nomes acessíveis e árvore AccessKit** nos widgets pintados (A5) e região viva na barra de status.
5. **Reduzir movimento e foco visível** (A6, A7), com chave nas configurações.
6. **Simulador de daltonismo** como passe de pós-processamento (A10).
7. **Revisão de alvos e escala** (A8, A9), com alternativa por teclado para as alças.

Estes itens devem ser levados ao `scripts/benchmark/backlog.py` pela líder; este relatório não edita os
arquivos gerados do backlog.
