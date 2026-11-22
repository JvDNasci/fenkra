# ADR 0109: encaixe de tubo com corte normal à parede (boca de lobo)

Data: 2026-11-22. Estado: aceita (parte do B574; o resto está em "Pendente"). Estende o ADR 0058 (B155) e
cumpre a onda 5 estrutural do ADR 0107.

## Contexto

O tratamento Encaixe cortava o membro que chega por booleana: o tubo que recebe é subtraído e a face de
corte cola na superfície dele. Em ângulo diferente de 90° a face corta a parede na diagonal e a borda vira
ponta de faca (nos testes, 0,035 mm de parede a 0,05 mm da borda, num T de 60,3x3,6 sobre 88,9x4 a 45°); a
solda derrete a quina. O corte real (plasma, laser de 3 eixos ou traçado à mão) é normal à parede: a face de
corte contém a normal da parede em cada ponto, e a borda mantém a espessura inteira. O ADR 0058 já calculava
essa curva para o desenho planificado, mas o sólido 3D continuava booleano.

## Decisão

1. **Opções no documento** (`fenkra-core`, `doc.rs`): `FitOptions { cut, wall, face }` em
   `StructMember::fit` (padrão do membro) e em `StructJoint::fit` (`Option`, o canto manda). `cut` é
   `Boolean` (de sempre) ou `Normal`; `wall` é a parede do tubo cortado que a curva segue (`Auto`, `Outer`,
   `Inner`, `Middle`); `face` é a superfície do tubo que recebe usada como encosto (`Outer`, `Middle`,
   `Inner`). Os dois campos novos saem do arquivo quando valem o padrão: arquivo antigo abre e recalcula
   igual (teste). O recompute resolve por `Ctx::joint_fit` (canto, senão o membro do posterior, como
   `joint_rule`).
2. **A curva** continua em `st_encaixe::calcular`, agora com `Opcoes::normal` e `Opcoes::face`. No corte normal
   os pares de pontos das duas peles seguem a normal da parede (no tubo retangular, a diagonal dos cantos
   encurta o par), e o modo automático vale o alcance máximo da espessura inteira (a saída de um sólido
   convexo é côncava na posição: o máximo pode estar no meio do segmento). A face do tubo que recebe entra
   como deslocamento do hospedeiro (meio e face interna ficam para dentro).
3. **O sólido** (`st_boca_lobo.rs`). Redondo: uma superfície B-spline passa pela grade raio × ângulo com
   `λ(θ)` constante ao longo do raio e divide o tubo (`clip_until`); a face de corte é uma só, lisa.
   Retangular: cada face plana tem o perfil `λ(c)`, extrudado pela normal da face e limitado à cunha que
   vai do canto externo à diagonal; as quatro peças se encontram na diagonal com o mesmo `λ`.
4. **Vários vizinhos** (nó de 3 a 5 tubos). Todos os vizinhos de uma ponta que cortam este membro entram na
   mesma curva: em cada ponto vale o de maior alcance. A ordem de montagem é a da árvore (o membro posterior
   é cortado pelos anteriores), a mesma de `joint_rule`.
5. **Validação** (`st_boca_lobo::validar`): vão de raiz e interferência medidos na normal do vizinho, espessura
   da borda e ângulo mínimo da ponta, com avisos no status do comando (`⚠ encaixe normal à parede`). A
   tolerância de vão é o orçamento da curva (1,5 mm) ou a parede do tubo cortado, o que for maior: hipótese de
   trabalho (a solda de ângulo fecha um vão de uma parede), a conferir em peça real. Interferência acima de
   0,05 mm e parede abaixo de 1 mm também avisam.
6. **Sem silêncio**: onde o corte normal não vale (barra maciça, membro curvo, seção que não é tubo) ou o
   kernel falha, a ponta cai na booleana e o aviso diz o motivo.
7. **Gabarito**: `encaixes_da_estrutura` passou a usar as opções do canto, então o contorno planificado da
   lista de corte (DXF/SVG, camada CORTE) é a mesma curva do sólido. A lista de corte ganhou a coluna
   `encaixe_normal` (vão, interferência e borda por ponta) e o botão "Gabaritos de encaixe…" grava um DXF por
   tubo.

## Conferência

`crates/fenkra-solidos/tests/estrutural_boca_de_lobo_b574.rs` monta os documentos em código e mede no
sólido do kernel (corte do sólido pelo plano do eixo): espessura da parede na borda, vão e interferência
contra o cilindro do tubo que recebe, volume contra a integral da própria curva e caixa envolvente.

## Pendente (B574)

- Tubo de passagem com rasgo na parede do tubo que recebe (diâmetro externo, interno ou externo menos a
  parede) e a opção "só encostar".
- Tubo contra superfície qualquer (não plana) e conjunto de superfícies, com a ponte automática quando as
  superfícies escolhidas não se tocam. O hospedeiro do cálculo é convexo (cilindro ou caixa).
- Escolher, na caixa de seleção universal, as faces e os tubos que cortam num nó (hoje vale a ordem da
  árvore) e o descritor dessas entradas em `contrato.rs`.
- Junta em T no painel "Cantos da estrutura" (`member_corners` só lista pontas que se encontram); o T usa a
  opção do membro.
- Barras maciças (sem parede) e canto de raio no tubo retangular; GIF do tooltip; conferir o vão aceito e o
  corte em peça real.
