# ADR 0077: conjunto de fixadores como comando sobre as pilhas (B375)

## Contexto

As pilhas (B98) já sabiam refazer um prendedor guardando ids e restrições, mas cada uma era avulsa, com
uma arruela e uma porca ligadas por caixas de marcar. O pedido é um comando único na árvore, com pilhas
livres de quantos itens forem precisos, alvos sem furo e regras de diâmetro por cor.

## Decisão

1. **O conjunto manda, a pilha executa.** `Assembly.conjuntos` guarda o pedido (parafuso, diâmetro,
   comprimento, pilhas e alvos). Cada alvo gera uma `Pilha` comum, com `conjunto` apontando para ele, e
   `conjuntos::sincronizar` é o único caminho entre os dois. Assim o refazer, a herança de ids e as
   restrições manuais do B98 valem sem cópia, e o prendedor segue sendo peça normal.
2. **Formato aditivo.** Campos novos com `#[serde(default)]`, sem subir `format_version`: `conjuntos` e
   `regras_cor` na montagem; `conjunto`, `superior`, `inferior`, `pega_auto` e `vao` na pilha. A
   assinatura das pilhas antigas continua igual, para abrir um arquivo antigo não refazer nada.
3. **Uma conta só para a pilha.** `arruela` e `porca` viram uma lista equivalente em
   `fixadores::pilhas_pedidas`; `medir` e `inserir_pecas` só conhecem listas. O assento na boca do furo
   olha o item que encosta na peça.
4. **Ponto e vetor são pilhas manuais com pega medida.** A pega vem de `Shape::ray_hits`, função nova do
   kernel que devolve todos os cortes da reta, porque `ray_hit` só dá o mais próximo e peça oca precisa
   dos dois lados de cada parede. O bloco preso são os trechos seguidos com vão de até `vao`.
5. **Interferência de propósito é decidida na hora, não gravada.** `separar_na_montagem` tira da lista o
   achado cuja malha cabe inteira no corpo do parafuso de uma pilha sem furo. Gravar pares em
   `RegrasInterf` deixaria lixo quando o alvo muda, e esconderia a cabeça batendo numa peça vizinha.
6. **Cor vem da aparência do comando** que criou o furo ou o ponto. Nada novo no documento da peça. Um
   ponto de sketch herda a cor do sketch inteiro.
7. **Regras ficam na montagem**, que leva as regras junto com o arquivo; a preferência do usuário só dá o
   ponto de partida de uma montagem nova.

## Consequências

- Regra sem tamanho na família usa o mais próximo, com aviso na linha do alvo. A biblioteca pública ainda
  não tem polegada, M1 nem M2: esses tamanhos entram pelas famílias, sem mudar o conjunto.
- A pega por reta não enxerga peças dentro de subconjuntos.
- Dois pontos do mesmo sketch não podem ter cores diferentes; quem precisa usa dois sketches.
