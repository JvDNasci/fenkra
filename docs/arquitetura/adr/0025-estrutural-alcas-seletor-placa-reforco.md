# ADR 0025: Estrutural, alças 3D do membro, seletor de perfil, placa e reforço

Estado: aceita. Itens B253, B254 e B255.

## Contexto

O comando Membro tinha cinco números no painel (rotação, deslocamentos X e Y, prolongar início e fim)
sem nenhuma manipulação no 3D, a escolha do perfil era dois combos de texto e a placa e o reforço
guardavam só um ponto, uma espessura e uma sobra. Arquivos já salvos precisam continuar iguais.

## Decisões

1. **As alças gravam o mesmo campo do painel.** Arrastar escreve o texto do campo (`StructMember.off_x`,
   `angle`, `ext1`...) pela mesma função de formatação do painel (`st_alcas::gravar`). Digitar e
   arrastar dão o mesmo texto e o mesmo documento. Expressão ou parâmetro no campo trava a alça (a
   leitura continua), como nas demais alças do 3D. A geometria vem de `fenkra_solidos::st_frames`, que
   calcula só os eixos das linhas (sem sólidos), então o quadro da interface não refaz booleanas.
2. **Rotação com ímã de 45°.** O arco mede o ângulo do mouse no plano da seção, no mesmo sentido em que
   o comando gira a seção; arredonda a 1° e gruda nos múltiplos de 45° a até 4°. Shift solta o ímã e usa
   décimos de grau.
3. **Espelhar é um campo novo, `StructMember.mirror`** (`serde(default)`, falso nos arquivos antigos).
   Ele espelha os laços da seção em x e o centróide; a caixa e o volume não mudam.
4. **Alinhar a uma aresta** calcula o ângulo que deixa o X da seção paralelo à projeção da aresta no
   plano da seção, pela menor volta, em (-90°, 90°].
5. **Seletor de perfil por cascata** (grupo, tipo, tamanho) com busca. `Family` ganhou `norm`, o grupo
   da cascata: as famílias de fábrica agrupam por tipo de produto, as do usuário ficam em "Minhas
   famílias". Não é uma tabela normativa: as medidas continuam sendo as comerciais usuais.
6. **A seção desenhada vem dos mesmos laços do kernel.** A miniatura, as cotas anotadas e os nove
   pontos mais o centróide usam `structural::section`, então o desenho nunca diverge do sólido. As
   cotas anotadas (`dimension_marks`) medem exatamente o valor da dimensão que nomeiam; um teste confere
   todas as medidas de fábrica, e outro confere que todo laço fecha.
7. **Placa e reforço crescem só com campos novos com `serde(default)`.** Um arquivo que guarda só o
   ponto, a espessura e a sobra (ou as abas e o chanfro) volta ao que era: placa externa, sobra em mm,
   sem furos; reforço triangular ou chanfrado, de dois lados, no meio da aresta, uma cópia. Um teste lê
   o JSON antigo e confere a igualdade.
8. **A face clicada escolhe a ponta.** O ponto da face clicada vai em `StructPlate.at`; `member_end`
   prefere a ponta cujo plano contém o ponto e cuja seção o cobre, e só cai na mais próxima quando
   nenhuma tem a face. Assim clicar a face do topo de um membro não escolhe o começo do membro vizinho.
9. **Faixas do reforço ao longo da aresta** (`st_placas::faixas_do_reforco`): início e fim encostam a
   face no afastamento; no meio, o lado da espessura (interno, dois lados, externo) decide de que lado
   do plano a espessura cresce. Quantidade espalha as cópias por igual entre as pontas úteis;
   espaçamento repete a cada passo a partir da posição (nos dois sentidos no meio). Cópias que se
   sobrepõem ou que saem da aresta são erro, nunca ajuste em silêncio.

## Consequências

- `StructMember`, `StructPlate` e `StructGusset` ganham campos; os construtores `StructPlate::nova` e
  `StructGusset::novo` reproduzem o comportamento antigo.
- As alças aparecem na primeira linha guia do comando; o valor vale para todas as linhas.
- O ímã de 45° e o passo vêm do zoom da câmera (`passo_3d`), então o mesmo arrasto dá passos diferentes
  em peças de tamanhos diferentes.
