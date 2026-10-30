# Benchmark — Chapa metálica

> Texto próprio, a partir das fontes públicas em [fontes.md](fontes.md). Catálogo completo do módulo em
> [catalogo.md](catalogo.md#chapa-metálica).

## Como cada ferramenta conduz o fluxo

- **CATIA V5:** Parâmetros globais (espessura, raio, tabela) → Wall → Wall On Edge / Swept Walls (Flange, Hem, Tear Drop, User Flange) → recortes e estampos → Fold/Unfold. A bancada aeroespacial adiciona abas apoiadas em superfície (OML/EOP).
- **SOLIDWORKS:** Base Flange → Edge/Miter Flange com posição da dobra explícita (material dentro, material fora, dobra fora, virtual sharp) → Hem/Jog/Sketched Bend → Flatten. Unfold/Fold para editar entre dobras.
- **Inventor:** Regras e estilos de chapa reaproveitáveis → Face → Flange/Contour Flange/Lofted Flange → Cut com "Cut Across Bend" → Flat Pattern com orientação e linhas de dobra.

## O que adotar no Fenkra

- Um comando de aba com tipo (aresta, cadeia, perfil, superfície, entre perfis) que abre as caixas do tipo (pedido DR-54).
- Posição da dobra explícita como no SW e regras de chapa como no Inventor.
- Validação das abas vizinhas antes de gerar (DR-57).

## O que evitar

- Abas que precisam de recuo manual para não colidir (resolvido com recuo mínimo; manter teste).

## Paridade

| ID | Feature | Fenkra | Falta / observação | Backlog |
|---|---|---|---|---|
| CHP-01 | Parâmetros de chapa | ✅ | SheetParams: material, bitolas, método/fator K, sugestão por tabela. |  |
| CHP-02 | Parede base | ✅ | SmWall (sketch ou superfície, offset, mid). |  |
| CHP-03 | Aba em aresta | 🟡 | SmFlange com lado, meia-esquadria, recuo mínimo; bugs em dobras em sequência, múltiplas dobras e abas vizinhas compostas (DR-55..57). | DR-55 |
| CHP-04 | Aba em cadeia (reta + curva) | 🟡 | SmCurveFlange (alpha.24–26); bug ao gerar em aresta com curva e reta (DR-54). | DR-54 |
| CHP-05 | Aba por perfil (contorno) | ✅ | SmProfileFlange. |  |
| CHP-06 | Aba por superfície (suporte) | 🟡 | SmSurfFlange com 3 modos (alpha.26); falhou nos testes do usuário, inclusive nervura de asa (DR-19, DR-23). | DR-19 |
| CHP-07 | Aba entre perfis (lofted) | ❌ |  | BM-CHP-01 |
| CHP-08 | Bainha e gota | ✅ | SmHem (tipos). |  |
| CHP-09 | Joggle | ✅ | SmJoggle (ângulo, plano, keep_length). |  |
| CHP-10 | Dobra por linha | 🟡 | SmBend só com reta a–b; não gera em spline e não emenda os raios (DR-17, DR-22). | DR-22 |
| CHP-11 | Recorte na chapa | 🟡 | SmCut; planificado→dobrado falha quando atravessa o raio (DR-20). | DR-20 |
| CHP-12 | Alívios de canto e dobra | ✅ | SmCornerRelief; relief_kind na aba. |  |
| CHP-13 | Estampos, veneziana e furo flangeado | ✅ | SmStamp, SmLouver, SmFlangedHole. |  |
| CHP-14 | Planificação e DXF | ✅ | SmUnfold; bend_lines; tabela de dobras; DXF no desenho. |  |
| CHP-15 | Desdobrar parcial | ❌ |  | BM-CHP-02 |
| CHP-16 | Converter sólido em chapa | ❌ |  | BM-CHP-03 |
| CHP-17 | Comando de aba unificado | 🟡 | Hoje são comandos separados (DR-54). | DR-54 |
