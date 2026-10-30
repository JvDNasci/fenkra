# Monta a página visual (backlog + comparador) com os dados embutidos: docs/backlog/painel.html.
# Uso: python -I scripts/benchmark/pagina.py   (depois de gerar.py e backlog.py)
import json, os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
modelo = open(os.path.join(os.path.dirname(__file__), 'pagina_modelo.html'), encoding='utf-8').read()
backlog = json.load(open(os.path.join(ROOT, 'docs', 'backlog', 'backlog.json'), encoding='utf-8'))
catalogo = json.load(open(os.path.join(ROOT, 'docs', 'benchmark', 'catalogo.json'), encoding='utf-8'))
dump = lambda d: json.dumps(d, ensure_ascii=False, separators=(',', ':')).replace('</', '<\\/')
html = modelo.replace('/*BACKLOG*/null', dump(backlog)).replace('/*CATALOGO*/null', dump(catalogo))
out = os.path.join(ROOT, 'docs', 'backlog', 'painel.html')
open(out, 'w', encoding='utf-8').write(html)
print('ok', out, len(html) // 1024, 'KB')
