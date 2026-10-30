# Benchmark e backlog

Geradores da documentação de benchmark e backlog (rodar nesta ordem):

1. `python -I scripts/benchmark/gerar.py` → `docs/benchmark/` e `docs/modulos/` a partir de `dados_a.py`, `dados_b.py` e `fontes.py`
2. `python -I scripts/benchmark/backlog.py` → `docs/backlog/backlog.{json,md}`
3. `python -I scripts/benchmark/pagina.py` → `docs/backlog/painel.html` (kanban + comparador, a partir de `pagina_modelo.html`)

Para mudar o estado de um item do backlog, altere `status` em `backlog.py` (`a fazer`, `em andamento`, `feito`) e rode 2 e 3.
