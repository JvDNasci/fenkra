# ADR 0085: formas das peças calculadas em paralelo e formas e vistas guardadas em disco (B393)

## Contexto

O desenho do conjunto do kart (84 componentes, 57 arquivos de peça) levava de 126 a 135 s para quatro vistas.
Depois dos ADR 0080 e 0081 restavam duas partes caras:

- a carga das formas: cada peça tinha os comandos refeitos, uma depois da outra (cerca de 50 s com a máquina
  ocupada; 22,6 s numa medida com a máquina mais livre);
- as linhas ocultas exatas: de 40 s a mais de um minuto, mesmo em processos paralelos.

Reabrir o mesmo desenho, sem mudar nada, refazia tudo de novo.

## Decisão

1. `PartCache::preparar_formas`: no modo `so_forma`, antes de montar o conjunto, as formas de todas as peças
   visíveis (entrando nos subconjuntos) são calculadas de uma vez, em threads (metade dos núcleos, até seis).
   Cada thread tem o seu documento e a sua topologia; nada é dividido entre elas. A forma pronta passa para a
   thread que pediu (a contagem de referências do OpenCASCADE é atômica).
2. A forma calculada vai para `%LOCALAPPDATA%\Fenkra\formas\<chave>.fkf`, em binário. A chave junta o texto do
   arquivo da peça e a identidade do executável (versão, caminho, tamanho e data): mudar a peça ou o programa
   muda a chave. Peça com malha lida de um arquivo externo a cada cálculo não vai para o disco.
3. As linhas ocultas exatas de cada vista de montagem vão para `<chave>.fkl` na mesma pasta. A chave junta a
   vista (tipo, direção, escala, corte, detalhe, mãe), o conteúdo de cada arquivo lido para montá-la e a
   identidade do executável. A vista achada no disco chega já exata, sem a etapa da prévia.
4. A gravação é atômica (temporário e renomear). Na primeira gravação de cada processo, o que foi gravado há
   mais de 30 dias é apagado. `FENKRA_SEM_CACHE_FORMAS=1` desliga leitura e gravação; `FENKRA_CACHE_FORMAS`
   troca a pasta.

## Segurança das threads

O ADR 0081 deixou em aberto se o recálculo dos comandos é seguro em threads. O teste ignorado
`carga_do_kart_em_paralelo_e_do_disco` calcula as 57 peças do kart numa thread e em várias e confere, peça a
peça, volume e caixa envolvente: iguais. O teste `formas_em_paralelo_iguais_as_de_uma_por_uma_e_depois_lidas_do_disco`
faz o mesmo com cinco peças em todo `cargo test`.

## Medição (kart, release, máquina ocupada por outros agentes)

| Etapa | Antes | Agora |
|---|---|---|
| Formas das 57 peças | 22,6 s (uma thread) | 6,2 s (threads) |
| Carga do conjunto, primeira vez | ~50 s | 8,7 s |
| Carga do conjunto, formas no disco | ~50 s | 2,6 s |
| Quatro vistas, primeira vez | 126 a 135 s | prévia em 16 s, exatas em 67 a 84 s |
| Quatro vistas, desenho reaberto | 126 a 135 s | exatas em 3,0 s |

## Consequências

- A primeira projeção exata de um conjunto grande continua no tempo do ADR 0080; o ganho dela é ao reabrir.
- A pasta cresce com cada versão de peça e de programa; a limpeza dos 30 dias segura o tamanho.
- Uma forma errada gravada por um programa com defeito só some quando o programa muda (a chave muda junto).
