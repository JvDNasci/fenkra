# ADR 0071: varredura por escada de tentativas, com portão no erro da superfície (B337)

## Contexto

Uma guia suave vira uma aresta só. Num sorteio de 120 splines de 8 a 25 pontos, com perfil normal no
início da guia, só 47 varreduras saíram boas em `BRepCheck`: 55 derrubavam o processo e 18 saíam
inválidas. Dividir a guia ajuda, mas não resolve: com 8 arestas ainda sobram 8 quedas.

A medida mostrou a causa. O Frenet corrigido do OpenCASCADE gira até 172° entre duas amostras vizinhas
(1/4000 da guia), enquanto o triedro discreto fica abaixo de 0,2°. A aproximação não acompanha uma lei
que salta, e o erro da superfície chega a 1356 mm com perfil de raio 1. Pelo fonte, o Frenet corrigido
amostra cerca de 10 passos por aresta e o discreto amostra 10 por vão de nós, o que combina com a melhora
ao dividir. Essa ligação foi lida no fonte; o salto foi medido.

`is_valid` não serve de critério: há sólidos aprovados com 4 mm de erro. O volume do kernel também não:
nas superfícies muito onduladas ele chegou a 1,5 vez o valor que a malha da mesma superfície dá.

## Decisão

1. **O caminho de sempre vem primeiro.** Guia que já varria mantém as mesmas faces e o mesmo triedro.
2. **Toda tentativa passa por um portão:** `IsDone`, `ErrorOnSurface` até 1e-3 mm e `BRepCheck`. Um
   resultado bom fica perto de 5e-5 mm; um ruim passa de 1 mm.
3. **Se falhar, a guia suave sobe a escada:** triedro discreto com 4, 8 e 16 arestas. As arestas apontam
   para a mesma B-spline, então as emendas têm a tangente e a curvatura da própria curva. Frenet pedido
   pela pessoa continua Frenet.
4. **Curva mais fechada que o braço do perfil é erro nomeado.** A parede passaria sobre si mesma, o kernel
   aprovaria e nenhum triedro corrigiria. Em guia plana só conta o braço no plano da curva, para uma
   parede alta sobre uma curva suave não ser recusada.
5. **Nervura de chapa tem limite de 60 s.** O tempo está na booleana (370 de 372 s medidos), que aceita
   interrupção. O prazo fica no indicador de progresso, porque a booleana o consulta nos fios paralelos.

## Limites conhecidos

- **Guia com cantos continua errada, com aviso.** `MakePipe` não gira a seção nos cantos: cada trecho sai
  como prisma inclinado. Guia em L dá metade do volume e guia em U dá zero, e o sólido é aprovado.
  Corrigir muda documentos salvos, então a decisão foi pedida à parte; até lá o recompute avisa na
  varredura e na nervura de chapa. Filete e membro estrutural usam o mesmo caminho e não avisam.
- `varredura` e filete não aceitam interrupção: o prazo só alcança booleana e malha.
- A proteção contra violação de acesso só existe no Windows. Sem ela, a guia suave pula a primeira
  tentativa. A saída no Linux é `OSD::SetSignal` com `OCC_CATCH_SIGNALS`, ainda sem teste.

## Consequências

Guia que falhava passa a varrer com mais faces do que uma guia que passa de primeira. Parte das nervuras
de chapa sorteadas agora dá erro de dobra em vez de um sólido com parede sobreposta: 21 de 30, num sorteio
de curvas fechadas de propósito.
