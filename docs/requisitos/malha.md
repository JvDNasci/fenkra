# Requisitos — Malha → sólido

| ID | Requisito | Prior. | Fase | Status |
|---|---|---|---|---|
| RF-MAL-001 | Importar malha (STL, OBJ, 3MF) como corpo de malha, visível e medível. | M | 2 | rascunho |
| RF-MAL-002 | Diagnóstico e reparo básico: furos, normais invertidas, faces degeneradas, não-manifold. | M | 2 | rascunho |
| RF-MAL-003 | **Nível 1** — conversão direta: cada triângulo vira face B-rep, costuradas em sólido fechado. | M | 2 | rascunho |
| RF-MAL-004 | **Nível 2** — regiões planas: triângulos coplanares (com tolerância configurável) fundidos em uma única face plana. | M | 2 | rascunho |
| RF-MAL-005 | **Nível 3** — primitivas: detectar planos, cilindros, cones e esferas e reconstruir o sólido com superfícies analíticas. | S | 4 | rascunho |
| RF-MAL-006 | Pré-visualização das regiões detectadas com cores antes de confirmar. | S | 2 | rascunho |
| RF-MAL-007 | O sólido resultante aceita features normais (furos, filetes, extrusões) por cima. | M | 2 | rascunho |
| RF-MAL-008 | Converter malha de 100 mil triângulos (nível 2) em menos de 10 s. | S | 2 | rascunho |
| RF-MAL-009 | Superfícies livres (ajuste NURBS). | W | — | rascunho |
