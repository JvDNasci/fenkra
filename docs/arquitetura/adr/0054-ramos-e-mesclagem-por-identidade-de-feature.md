# ADR 0054: ramos e mesclagem por identidade de feature

Data: 2026-11-06. Item: B130. Liga com B214 (comparador por feature), B305 (checador de qualidade),
B265 (papéis) e B94 (relações).

## Contexto

Travar a peça para editar impede o trabalho em paralelo. A alternativa de copiar o arquivo e juntar à
mão perde a história e não detecta conflito. O documento já tem `Id` estável em cada feature,
parâmetro e coletor, o que permite juntar por identidade.

## Decisão

1. **Ramo é uma linha de versões paralela.** O ramo nasce de uma versão da principal (a base) e guarda
   as próprias versões em `.versoes/`, como as demais. A principal só muda por mesclagem.
2. **Mesclagem de 3 vias sobre o JSON do documento.** Base, principal de hoje e ponta do ramo são
   comparados campo a campo; listas de objetos com `id` casam por identidade. Um lado mudou: vale.
   Os dois mudaram igual: vale. Os dois mudaram diferente: conflito, que a pessoa resolve escolhendo um
   lado. O resultado é igual nas duas ordens de mesclagem (testado). O comparador genérico faz com que
   um tipo novo de comando entre sem código novo.
3. **Faixa de ids por ramo.** Cada ramo reserva 1.000.000 de ids ao nascer, então features criadas em
   ramos diferentes não colidem.
4. **Principal protegida.** Entrar exige pedido de mesclagem aprovado por quem tem o papel
   `aprovador`. A aprovação fica presa à ponta do ramo e à versão da principal: se um deles mudar, o
   pedido volta a ser aberto.
5. **Gancho `verificar`.** O núcleo (`fenkra-project`) não conhece o kernel; recebe a conferência por
   parâmetro. O app liga `verificar_recalculo` (`fenkra-app/src/app/ramos_ui.rs`), que recalcula o
   documento mesclado e barra: comando ativo com erro, referência perdida, parâmetro com erro e corpo
   com volume inválido. Roda ao aprovar e de novo ao mesclar, no resultado final.

## Consequências

- A tela de conflito mostra os dois valores lado a lado por feature, sem prévia 3D ainda; a sobreposição
  colorida do diff no 3D (B214) fica como trabalho futuro.
- O ramo ativo é lembrado em `OpenDoc.ramo`; Salvar grava no ramo, nunca na principal.
- `verificar_recalculo` roda o kernel na thread da interface ao aprovar ou mesclar; para peças muito
  grandes convém mover para o `Worker`.
