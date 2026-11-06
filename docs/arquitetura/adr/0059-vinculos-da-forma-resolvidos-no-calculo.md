# ADR 0059: vínculos da forma livre resolvidos no cálculo, sobre uma cópia da gaiola (B303)

## Contexto

O nó da gaiola precisa seguir a peça: um parâmetro muda e o nó anda, a borda da forma acompanha uma
aresta B-rep, e o sólido convertido vem junto. A ADR 0042 deixou isso para depois, com o número
persistente do nó como âncora. O risco é o de sempre em vínculo: referência que some e o modelo mudar
sem ninguém ver.

## Decisão

1. **A gaiola do documento não é tocada pelo cálculo.** `Forma.vinculos` guarda as regras e
   `fenkra_solidos::forma_vinculo` produz, a cada recálculo, uma `FormaResolvida` com a cópia da gaiola.
   Superfície e conversão usam a cópia. Assim o recálculo continua sem efeito colateral no arquivo.
2. **A interface é quem grava de volta.** Fora do arraste, `forma_vinculo_ui::sincronizar` leva os nós
   presos para o documento, para as operações de topologia partirem do lugar certo. Ela só age se
   `FormaResolvida.entrada` for igual à gaiola atual: uma conta atrasada devolveria o nó para antes do
   último gesto.
3. **Coordenada na referência, e não ponto.** Curva guarda a fração do comprimento e plano guarda `u` e
   `v`, para o nó seguir a referência mesmo quando ela anda de lado. Face é exceção: o OpenCASCADE dá
   coordenadas de superfície que mudam entre recálculos, então vale o ponto da face mais perto do nó
   guardado. Sem coordenada (`None`) vale a projeção, até a interface ancorar.
4. **Perdeu a referência, é erro, com a forma parada.** O comando fica em `Status::Error`, e a superfície
   segue na tela com a última gaiola guardada. Por isso a face é resolvida sempre pela forma viva do
   modelo, sem o atalho do ponto guardado que outros comandos aceitam.
5. **A simetria manda mais que o vínculo.** Nó da costura acompanha só dentro do plano, com aviso, e o
   vínculo que levaria um nó para o outro lado do plano é erro.
6. **Borda casada pela fórmula do limite.** Numa borda aberta a superfície é a B-spline cúbica dos nós
   da borda, e através dela vale a fileira fantasma `2·P0 − P1`. Em G0 resolve-se o sistema
   `(P[i-1] + 4·P[i] + P[i+1]) / 6 = C[i]`, porque pôr os nós sobre a aresta encolheria a curva. Em G1 a
   fileira vizinha vai para o plano tangente, já que a derivada através da borda é `P1 − P0`.
7. **G2 honesto.** A mesma fileira fantasma dá derivada segunda nula através da borda: toda borda livre
   tem curvatura normal zero nessa direção, seja qual for a gaiola. G2 vale então só com face que não
   curva através da aresta (plano, ou cilindro casado pela borda circular). Nos outros casos o comando
   fica com aviso e o relatório diz G1.

## Consequências

- Casar G2 em face curva pediria mudar a regra da borda na subdivisão, ou recortar a superfície numa
  fileira de dentro. Fica como item futuro, se a demanda aparecer.
- G1 e G2 só valem em borda aberta. Por dentro da forma, G0 vira vinco cheio.
- A forma com vínculo lê a peça inteira: entra na assinatura de todos os corpos do cache.
- A forma precisa vir depois do pai na árvore. Antes dele, a referência ainda não existe e o estado é erro.
