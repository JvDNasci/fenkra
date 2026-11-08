# ADR 0079: projeção das vistas de montagem em segundo plano, com progresso e cancelamento (B427, B393)

## Contexto

O desenho de conjunto do kart (84 peças) levava 126 s para quatro vistas e a exportação `.fkv` 1170 s, sempre
dentro de um quadro da interface: janela parada, sem barra, sem como desistir. Medido na máquina de
desenvolvimento, com a máquina ocupada por outros processos: 94 s para carregar as formas e de 11 a 31 s de
linhas ocultas (HLR) por vista.

## Decisão

1. **A vista de montagem (`.fka`) é calculada numa thread própria.** `Models::ativar_fundo` liga uma
   `fenkra_core::trabalho::Fila` com o seu `Models`: a forma do OpenCASCADE não atravessa threads, então só a
   `Proj` (listas de números) volta. Até chegar, `Models::proj` devolve uma `Proj` vazia com
   `calculando = true`; `sondar` troca pela pronta e a interface refaz a folha. Peça isolada continua
   síncrona: leva décimos de segundo.
2. **Um pedido por vez, na ordem.** `Fila` cancela o pedido anterior ao receber outro, o que mataria as
   outras vistas; por isso a fila de vistas fica em `fundo::Fundo` e só um pedido vai à thread. A vista filha
   (projetada, corte, detalhe) espera a mãe chegar.
3. **A chave do cache não carrega a montagem.** Vista de montagem usa os arquivos que a thread leu e as datas
   de quando leu (`Fundo::arquivos`). `refresh` compara com as datas de agora e esquece a projeção que ficou
   velha. A interface só lê o modelo leve (`Models::read_leve`: a montagem sem formas), que serve à árvore,
   às tabelas e aos nomes das peças; volume e nomes vêm da thread.
4. **Cancelar** esvazia a fila, solta o pedido em andamento (o resultado é descartado) e marca as vistas como
   canceladas até a pessoa pedir de novo ou mudar a vista, para não serem refeitas a cada quadro. O cache da
   thread guarda só o que terminou. O HLR do OpenCASCADE é uma chamada só e não é interrompível: a thread o
   termina e descarta; a interface já está livre.
5. **O `.fkv` também roda numa thread**, com `Opcoes::controle` (passos por componente, mais análise e fecho,
   e cancelamento entre componentes). O arquivo é escolhido antes de gerar. Cancelado devolve `CANCELADO` e
   não grava nada.

## Medição (B393)

A carga era de duas formas por peça: `recompute` (com malha) e `final_shape` (de novo, sem cache). Agora:
`forma_cache::forma_do_cache` monta a forma final dos corpos do cache do mesmo cálculo, e o desenho pede só a
forma (`PartCache::so_forma`), sem malha nem níveis de detalhe. Carga do kart: 94 s para 50 s. O HLR
(11 a 31 s por vista) não mudou: continua sendo o maior custo e está fora da thread da interface.

## Consequências

- `Proj::calculando` é campo novo: quem constrói uma `Proj` à mão usa `..Default::default()`.
- Teste de montagem lê a vista sem a thread (`Models::new` sem `ativar_fundo`), então não muda.
- A forma tirada do cache perde a malha (`discard_mesh`) para a caixa envolvente sair exata como a de `final_shape`.
