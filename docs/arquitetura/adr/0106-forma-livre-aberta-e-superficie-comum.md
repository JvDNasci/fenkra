# ADR 0106 — Forma livre aberta é superfície comum

Estado: aceito (18/11/2026). Complementa o ADR 0010 (referências v2) e o B515. Item: B511.

## Contexto

A forma livre (B153) só virava B-rep quando a pessoa clicava em Converter (B179). Sem isso era malha
de desenho, que nenhum comando enxerga. Convertida com borda, o casco ia composto no corpo de destino:
os comandos de superfície (deslocar, aparar, juntar, filete entre superfícies) só aceitam superfícies
da pasta de geometria (`SurfRef::Ref`) e, pelo corpo, só pegavam uma face de cada vez. Relato de
18/11: a superfície da forma não podia ser escolhida em deslocamento, espessar, furo nem chapa.

## Decisão

1. **Casco aberto não entra no corpo.** A feature `Forma` guarda o casco aberto em `Ctx::tools` (o
   mesmo lugar que guarda a ferramenta de cada comando e já vai e volta do cache). Gaiola fechada
   segue como sólido no corpo (`apply_roles`, nomes das faces da gaiola).
2. **A forma é dona de superfície.** `Ctx::forma_superficie` responde por ela; `surface` (que atende
   `SurfRef::Ref`) e `surface_owner` (que atende as chaves de face e aresta, B515) perguntam primeiro
   por ela. Os papéis das faces são os da superfície de referência (`PAPEL_SUPERFICIE + i`).
3. **Resultado e tela.** A forma aberta entra em `RecomputeResult::surfaces`, com a malha no cache dos
   corpos. Na interface, quem decidia visibilidade e aparência por `doc.reference(id)` passa a usar
   `Document::superficie_mostrada` e `aparencia_da_superficie`; a forma oculta-se como um sketch
   (`hidden`, pasta, corpo) e é consumida (oculta) por deslocar e espessar como as superfícies.
4. **Concluir converte.** Forma nunca convertida converte em G1 ao concluir, salvo a que passa do
   alerta de tempo (`SEGUNDOS_DE_ALERTA`), que espera o botão.
5. **Em edição, espera.** Gaiola aberta (conversão pausada): quem usa a superfície fica inativo, sem
   erro, como os comandos depois dela.

## Consequências

- Deslocar, espessar, aparar, juntar, filete entre superfícies, extrusão até a superfície e a aba por
  superfície da chapa aceitam a forma sem código próprio. Furo: no sólido espessado ou na forma fechada.
- Arquivo antigo com forma aberta convertida e comando de corpo apontando para uma face dela pela chave
  antiga (papel = nome da face da gaiola) perde a referência: o assistente de reparo pede a face de novo.
- O papel por índice segue a face quando um nó anda; mudança de topologia da gaiola cai no desempate por
  tipo, área e ponto. Usar o nome da face da gaiola como papel fica como melhoria.
