# Asa enflechada, nervura e caverna: teste com mouse e teclado reais (23/11/2026)

Teste de modelagem no Fenkra 0.1.0-alpha.42 (`dist/Fenkra-0.1.0-alpha.42-windows-x64/fenkra.exe`), como uma
pessoa faria, com o equipamento `scripts/teste_interface/fk.py` (SendInput real, sem `FENKRA_DEMO`), com
`APPDATA` e `FK_ESTADO` próprios em pasta temporária e a trava `scratchpad\mouse.lock`. Cada passo foi
conferido por captura antes do seguinte. Tela 1920x1080, janela maximizada. O Fenkra testado foi aberto e
fechado pelo próprio PID.

Arquivos salvos com o diálogo do próprio Fenkra (`.fkp`):

- `asa-enflechada.fkp`: só a asa.
- `nervura.fkp`: a asa e o corpo `Nervura` (chapa) na mesma peça.
- `caverna.fkp`: a fuselagem oval e o corpo `Caverna` (chapa, com a aba em erro).

Capturas reduzidas em `capturas/`.

## Resumo

| Peça | Veredito | Observação |
|---|---|---|
| Asa enflechada (NACA 2412, corda 2000 → 1500, envergadura 4000, recuo 600) | **feita** | Loft sólido entre duas splines. Entrada de pontos muito trabalhosa |
| Nervura (chapa na seção da estação 1500) | **parcial** | Parede base ligada à seção, dobrado e planificado ok. A aba por superfície só sai em parte do contorno. Na estação 2800 o recálculo quebra |
| Caverna (anel de chapa na fuselagem oval) | **parcial** | Fuselagem oval feita por loft entre splines. A parede base sai sem o furo, a aba por superfície falha e o corpo inteiro some |

## 1. Asa enflechada

### Passo a passo

1. Tela inicial › Nova peça. Plano XZ › Sketch.
2. Perfil NACA 2412 com 21 pontos (cosseno), calculado fora do Fenkra. **Não há como colar nem importar uma
   lista de pontos**: o caminho foi clicar os 21 pontos aproximados com a Spline (restrições automáticas
   e snaps desligados) e depois corrigir ponto a ponto. Selecionar o ponto mostra os campos `x` e `y` no
   painel. Cada ponto leva clique no ponto, clique no campo, digitação, Tab, digitação e Enter. Os campos
   aceitam vírgula decimal.
3. O bordo de fuga do NACA é aberto (espessura de 5 mm). Fechei com uma Linha entre as pontas. Precisou
   religar os snaps, que tinham ficado desligados por causa da spline.
4. Plano deslocado de XZ, 4000 mm (Geometria › Plano). Sketch nele, perfil de corda 1500 com x + 600
   (enflechamento), pelo mesmo processo, mais a linha do bordo de fuga.
5. Sólidos › Multi-seções (loft) com Sketch1 e Sketch2. Recálculo sem erro (`asa_iso.png`).
6. Ctrl+S numa peça nova abre o Salvar como (bom). Salvo como `asa-enflechada.fkp`.

### O que funcionou

- A Spline por pontos com edição exata de `x`/`y` resolve a precisão.
- O loft sólido entre perfis em planos paralelos e deslocados saiu de primeira, liso (`asa_iso.png`).
- O plano deslocado aceita o valor digitado e mostra a seta e o rótulo "Deslocamento" no 3D.

### O que travou ou foi pouco intuitivo

- Sem colar, importar ou gerar pontos (CSV ou NACA), um perfil de 21 pontos custa cerca de 150 ações por
  seção. É o maior obstáculo para quem modela aeronáutica.
- Os snaps e as restrições automáticas ficam desligados de uma ferramenta para a outra. Desligar na spline
  também desliga na linha.
- Com o zoom muito perto da peça escura, a árvore transparente fica ilegível por cima da geometria
  (`arvore_ilegivel.png`).

## 2. Nervura

### Passo a passo

1. Plano deslocado de XZ, 1500 mm (estação). Sketch nele › Projetar no sketch › Converter ▾ › Interseção
   com plano › clique na asa. A seção vem associativa (Sketch3).
2. O bordo de fuga da seção também sai aberto. Fechei com uma Linha manual, que não é associativa (ver item
   6).
3. Novo coletor de corpos pelo menu da raiz da peça, renomeado para `Nervura` (F2). Chapa metálica ›
   Parede base de sketch com Sketch3 (`parede_base.png`).
4. Chapa › Aba ▾ › Aba › Modo da aba: Superfície. O campo Superfície nasce com **Origem · YZ** e o 3D acusa
   o suporte inválido (`aba_sup_padrao_yz.png`). Clicar na face da asa no 3D não preenche o campo
   (`aba_sup_face.png`, `aba_sup_nao_aceita_face.png`): a lista só oferece planos e superfícies.
5. Contorno: Superfícies › Extrair a face da asa (`Superficie1`) e escolha dela na lista do campo
   Superfície.
6. Com comprimento 12 e 20 aparece o aviso de planificação: a curva do bordo de ataque tem raio de cerca de
   13 mm, menor que o desenvolvimento da aba, e a mensagem sugere um comprimento menor ou o tipo por
   aproximação (`aba_aviso_raio.png`). Ao concluir: erro (`aba_erro.png`). O tipo Aproximação também deu erro
   (`aba_aprox_erro.png`).
7. Com comprimento 8, tipo Exato: a prévia parece contornar o perfil (`aba_previa_8mm.png`), mas o resultado
   só tem aba **num trecho do intradorso perto do bordo de fuga** (`aba_final_parcial.png`,
   `aba_parcial_cima.png`, `aba_parcial_baixo.png`).
8. Planificar (botão da barra de chapa): a chapa plana mostra o perfil com a faixa da aba só nesse trecho
   (`nervura_planificada.png`). Voltar a dobrar funciona.
9. Teste de associatividade: Plano2 de 1500 para 2800. A Parede base acompanha a seção nova, mas o contorno
   vira um crescente perto do bordo de ataque e a Aba1 entra em erro (`estacao_2800.png`,
   `estacao_2800_anel.png`). Ao abrir o Sketch3 aparece o painel de reparo (`reparo_aba.png`). Causa provável:
   a linha manual do bordo de fuga ficou na posição da estação antiga (a corda e o recuo mudam ao longo da
   envergadura), e o perfil deixou de fechar. Com o plano de volta em 1500, tudo recalcula sem erro
   (`nervura_final.png`).
10. Salvo como `nervura.fkp` (Arquivo › Salvar como; o submenu fecha se o mouse passa na diagonal).

### Erros de cálculo vistos

- Aba1: aviso de planificação "raio menor que o desenvolvimento da aba" (comprimentos 12 e 20), depois erro
  ao concluir.
- Aba1 com Aproximação: erro.
- Estação 2800: Aba1 em erro, Parede base com contorno errado e status "1 com erro – reparar".

## 3. Caverna da fuselagem oval

### Passo a passo

1. Peça nova. Primeiro tentei a elipse por Superfícies › Curva › Cônica (elipse 600 x 800 no YZ). A curva
   aparece na árvore, mas **não aparece no 3D** (`conica_invisivel.png`), e o sketch não tem ferramenta de
   elipse. Apaguei a curva.
2. Alternativa: dois círculos (R700 no YZ e R450 num plano a 3000 mm) › Multi-seções. A prévia mostrou a
   torção entre as costuras, e um clique no vértice certo desfez a torção (`loft_torcao.png`). Bom.
3. Para deixar oval: Sólidos › Transformar ▾ › Afinidade, fator Z 1,3. Digitar "1,3" deixou o fator X em
   "13" e o Z em "1,", com erro de sintaxe na árvore (`afinidade_virgula.png`). Com "1.3" funcionou
   (`fuselagem.png`), mas a árvore avisa "sólido inválido depois deste comando"
   (`afinidade_invalida.png`), e a face extraída dele dá "a face não existe mais no modelo". Apaguei a
   Afinidade.
4. Troquei os círculos por splines fechadas de 16 pontos em elipse (600 x 800 e 380 x 520). O loft saiu
   oval e com variação de tamanho (`fuselagem_oval.png`). Problemas da spline fechada: fechar clicando no
   primeiro ponto deixa uma quina na costura (`spline_fechada_quina.png`), e editar as coordenadas do ponto
   da costura **abriu** a curva (`spline_fechada_abriu.png`).
5. Plano a 1500 mm. Superfícies › Extrair a face lateral. O painel abre à direita e empurra a vista 3D: o
   primeiro clique caiu fora da peça (`extrair_vista_desloca.png`).
6. Novo coletor `Caverna`. Sketch no plano da estação › Interseção com a superfície (contorno externo).
   Offset da curva para o contorno interno: o Offset não aceita a spline projetada e fica em "1 de 2"
   (`offset_nao_aceita_spline.png`). Desenhei o contorno interno como spline à mão (`caverna_sketch.png`).
7. Parede base: a prévia mostra os dois contornos (`caverna_parede.png`), mas a chapa sai cheia, **sem o
   furo** (`caverna_sem_furo.png`).
8. Aba por superfície com Superficie1 e comprimento 8: "o suporte não encontra a borda da web", e o **corpo
   Caverna inteiro some** da vista enquanto a aba está em erro (`caverna_corpo_some.png`).
9. O Sketch3 também ganhou o aviso "geometria externa: referência perdida: a face de MultiSeções1 (lateral
   1) não existe mais", depois que os perfis do loft foram trocados.
10. Salvo como `caverna.fkp` com a aba em erro. Dobrado e planificado da caverna **não conferidos**: não houve
    resultado válido.

### Outros tropeços

- Quando a barra de status mostra "Próximo:" com sugestões, os botões de vista da barra inferior mudam de
  lugar. Um clique em "Ajustar" caiu numa sugestão e abriu o Filete sozinho (`filete_aberto_sozinho.png`).
- A Parede base nasce com o campo "Afastamento do…" vazio e um triângulo vermelho, sem dizer o que falta
  (`caverna_parede.png`).

## Itens de melhoria sugeridos para o backlog

| Bancada | Prioridade | Descrição |
|---|---|---|
| Chapa | P0 | Aba por superfície num perfil fechado curvo (aerofólio, elipse) deve contornar todo o perímetro. Hoje sai só num trecho, ou erra |
| Chapa | P0 | Parede base de sketch com contorno interno fechado deve furar a chapa (anel). Hoje sai cheia |
| Chapa | P0 | Feature de chapa em erro não pode esconder o corpo inteiro: mostrar o último estado bom e marcar o erro |
| Chapa | P1 | Campo Superfície da aba por superfície: aceitar clique numa face de sólido no 3D, sem obrigar a Extrair antes, e não nascer com Origem · YZ |
| Chapa | P1 | Aba por aproximação deve resolver os trechos de raio pequeno (bordo de ataque) em vez de falhar |
| Chapa | P1 | Mensagem de erro da aba deve dizer qual trecho do contorno falhou e por quê |
| Sketch | P1 | Spline por tabela de pontos: colar ou importar uma lista x,y (CSV ou texto) |
| Sketch | P1 | Interseção com plano de perfil de bordo de fuga aberto: opção de fechar automaticamente, de forma associativa |
| Sketch | P1 | Offset deve aceitar spline e geometria projetada (interseção) |
| Sketch | P1 | Spline fechada: costura sem quina (tangente contínua), e editar o ponto da costura não pode abrir a curva |
| Sketch | P1 | Ferramenta de elipse no sketch |
| Superfícies | P1 | Curva cônica (elipse) criada precisa aparecer no 3D |
| Sólidos | P1 | Afinidade (escala por eixo) não pode gerar sólido inválido num loft simples |
| Interface | P1 | Campos numéricos da Afinidade aceitam vírgula decimal como os outros. Hoje "1,3" vira erro de sintaxe e altera o campo vizinho |
| Sólidos | P2 | Gerador de perfil NACA 4 dígitos (corda, espessura e curvatura como parâmetros) |
| Interface | P2 | Barra inferior de vista com posição fixa: as sugestões "Próximo:" não podem deslocar os botões |
| Interface | P2 | Painel do Extrair (à direita) não pode deslocar a vista 3D na hora do clique |
| Interface | P2 | Árvore transparente legível sobre geometria escura (fundo ou contorno no texto) |
| Interface | P2 | Submenus do menu principal não podem fechar quando o mouse passa na diagonal |
| Sketch | P2 | Snaps e restrições automáticas: lembrar por ferramenta, ou deixar claro que a opção vale para todas |
| Chapa | P2 | Parede base: explicar no campo "Afastamento do…" o que falta, em vez de só mostrar um triângulo vermelho |
