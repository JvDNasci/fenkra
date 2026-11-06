# ADR 0058: corte de encaixe de tubo por duas peles e pele de maior alcance

Data: 2026-11-06. Estado: aceita. Item: B155.

## Contexto
O tratamento Encaixe do canto estrutural já recorta o membro pelo contorno do vizinho no 3D
(`clip_until`), o que dá uma face de corte inclinada de uma pele à outra. Um laser ou plasma de
tubo de 3 eixos corta perpendicular à parede: uma curva só vale para a espessura inteira.
Fabricar exige essa curva, planificada, no mesmo DXF e SVG do tubo aberto.

## Decisão
1. O cálculo vive em `fenkra-solidos/src/st_encaixe.rs`, puro (seção, eixos e hospedeiro entram
   como dados) e testável sem kernel; `st_encaixe_doc.rs` só lê o documento. Não toca o recompute.
2. Cada ponto do perímetro tem dois raios paralelos ao eixo (pele externa e pele interna) que saem
   do hospedeiro, convexo e engordado pela folga, em `λ_ext` e `λ_int`. O modo automático corta na
   pele de maior `λ`: nunca invade, e dá a menor abertura. A troca de pele acontece com abertura
   zero, então a curva não tem degrau e dispensa histerese.
3. A abertura de raiz é medida na normal do hospedeiro (`axial × |cos|`), que é o espaço que a solda
   preenche. Acima do orçamento o trecho é marcado e o chanfro `atan(Δλ/parede)` é sugerido.
4. O desenho reaproveita `Desenho`, `Camada` e as saídas de `st_dobra_saida` (SVG, DXF): X é o `λ`,
   Y o perímetro da pele externa, que preserva comprimento.

## Consequências
Seção só redonda, quadrada, retangular e barra; sem raio de canto nem ovalização (B176). O tubo
dobrado ainda não leva o encaixe no seu planificado. Os parâmetros do painel (parede, orçamento,
perpendicular) ficam na sessão; a folga é a do canto, que já está no documento. Tudo "a validar em
peça real".
