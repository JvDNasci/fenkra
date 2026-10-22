# ADR 0007: Vistas de desenho como blocos ligados ou travados

- **Status:** proposta
- **Data:** 2026-10-21

## Contexto
Desenhos que recalculam todas as vistas a cada mudança no 3D ficam lentos e instáveis, principalmente
em montagens grandes. O usuário precisa decidir quando o desenho acompanha o modelo.

## Decisão
- Cada vista é um **bloco** que guarda seu próprio resultado 2D (linhas, arestas ocultas, hachuras)
  junto com cotas e anotações.
- Cada vista tem um **link com o 3D** no mesmo modelo do ADR 0006:
  **ligada** (atualiza quando o modelo muda), **travada** (mantém o último resultado), **desatualizada**
  (o modelo mudou; a vista mostra um indicador e atualiza sob comando).
- A geração das linhas é feita em background, com pré-visualização aproximada antes do resultado exato.
- Cotas e anotações se prendem às arestas por referência persistente (ADR 0004); se a aresta sumir,
  a cota fica marcada como órfã.

## Alternativas consideradas
- **Sempre ligado:** simples, mas lento e imprevisível.
- **Sempre manual:** rápido, mas o usuário esquece de atualizar e libera desenho desatualizado.
  Mitigado aqui pelo indicador "desatualizada" e por um aviso na liberação de revisão.

## Consequências
- ✅ Abrir desenhos é rápido (vistas travadas não recalculam, RNF-017).
- ✅ Revisões congeladas guardam o desenho exatamente como foi liberado.
- ⚠️ Arquivo de desenho maior (guarda o resultado 2D).
