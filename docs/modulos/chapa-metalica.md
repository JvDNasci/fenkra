# Chapa metálica

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/chapa-metalica.md](../benchmark/chapa-metalica.md).

## ✅ Pronto

- **Parâmetros de chapa** (Parâmetros de chapa) — SheetParams: material, bitolas, método/fator K, sugestão por tabela.
- **Parede base** (Parede base) — SmWall (sketch ou superfície, offset, mid).
- **Aba por perfil (contorno)** (Aba por perfil) — SmProfileFlange.
- **Bainha e gota** (Bainha) — SmHem (tipos).
- **Joggle** (Joggle) — SmJoggle (ângulo, plano, keep_length).
- **Alívios de canto e dobra** (Alívio de canto) — SmCornerRelief; relief_kind na aba.
- **Estampos, veneziana e furo flangeado** (Estampo, veneziana, furo flangeado) — SmStamp, SmLouver, SmFlangedHole.
- **Planificação e DXF** (Dobrar/Desdobrar) — SmUnfold; bend_lines; tabela de dobras; DXF no desenho.

## 🟡 Parcial

- **Aba em aresta** (Aba) — SmFlange com lado, meia-esquadria, recuo mínimo; bugs em dobras em sequência, múltiplas dobras e abas vizinhas compostas (DR-55..57). `DR-55`
- **Aba em cadeia (reta + curva)** (Aba curva) — SmCurveFlange (alpha.24–26); bug ao gerar em aresta com curva e reta (DR-54). `DR-54`
- **Aba por superfície (suporte)** (Aba por superfície) — SmSurfFlange com 3 modos (alpha.26); falhou nos testes do usuário, inclusive nervura de asa (DR-19, DR-23). `DR-19`
- **Dobra por linha** (Dobra por linha) — SmBend só com reta a–b; não gera em spline e não emenda os raios (DR-17, DR-22). `DR-22`
- **Recorte na chapa** (Recorte) — SmCut; planificado→dobrado falha quando atravessa o raio (DR-20). `DR-20`
- **Comando de aba unificado** (vários comandos) — Hoje são comandos separados (DR-54). `DR-54`

## ❌ Ainda não existe

- **Aba entre perfis (lofted)** (—) —  `BM-CHP-01`
- **Desdobrar parcial** (—) —  `BM-CHP-02`
- **Converter sólido em chapa** (—) —  `BM-CHP-03`
