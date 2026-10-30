# Requisitos — Biblioteca pública de hardware e perfis

Ver [ADR 0008](../arquitetura/adr/0008-biblioteca-publica.md).

## Estrutura

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-BIB-001 | Biblioteca em formato aberto e legível (texto estruturado), versionada em repositório público. | M | 1 | rascunho |
| RF-BIB-002 | Edição pela comunidade com revisão (contribuições propostas, revisadas e aceitas). | M | 2 | rascunho |
| RF-BIB-003 | Camadas: **pública** → **empresa** → **projeto/usuário**; camadas de cima sobrepõem ou estendem as de baixo. | M | 1 | rascunho |
| RF-BIB-004 | Cada item é uma **família parametrizada** (geometria gerada por parâmetros) + **tabela de tamanhos**. | M | 1 | rascunho |
| RF-BIB-005 | Cadastro de novos itens pela interface, sem programar (desenhar o perfil/peça e preencher a tabela). | M | 2 | rascunho |
| RF-BIB-006 | Cada item tem norma de referência, fonte dos dados e licença. | M | 1 | rascunho |
| RF-BIB-007 | Funciona offline com cópia local; sincroniza atualizações quando houver conexão. | M | 2 | rascunho |
| RF-BIB-008 | Itens da biblioteca usados em projetos ficam fixados na versão usada; atualizar é decisão do usuário. | M | 2 | rascunho |

## Conteúdo inicial

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-BIB-010 | **Perfis estruturais** com dimensões, área da seção, **peso por metro**, momentos de inércia, módulos resistentes e raios de giração. | M | 2 | rascunho |
| RF-BIB-011 | Famílias de perfis: I, H, W, U, L (abas iguais e desiguais), T, tubos redondos, quadrados e retangulares, barras chatas, redondas e quadradas, perfis formados a frio (U enrijecido, Z). | M | 2 | rascunho |
| RF-BIB-012 | Normas/catálogos: perfis brasileiros (NBR e tabelas de fabricantes nacionais), EN/ISO, AISC. | M | 2 | rascunho |
| RF-BIB-013 | **Fixadores**: parafusos (sextavado, Allen, cabeça chata, prisioneiros), porcas, arruelas, pinos, anéis elásticos, insertos. | M | 2 | rascunho |
| RF-BIB-014 | **Roscas**: métrica ISO grossa e fina, UNC/UNF, BSP/BSPT, NPT, trapezoidal, com classes de tolerância. | M | 1 | rascunho |
| RF-BIB-015 | Elementos de máquina (fase posterior): rolamentos, chavetas, polias, engrenagens simples, perfis de alumínio estrutural. | S | 4 | rascunho |
| RF-BIB-016 | Materiais (densidade, módulo de elasticidade, aparência, hachura). | M | 1 | rascunho |
| RF-BIB-017 | Tabelas de chapas: espessuras comerciais, raio de dobra mínimo e fator K por material. | M | 2 | rascunho |
