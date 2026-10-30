# Gera o benchmark a partir dos dados: docs/benchmark/{catalogo.json, catalogo.md, <modulo>.md, fontes.md}
# e docs/modulos/<modulo>.md.   Uso: python -I scripts/benchmark/gerar.py
import json, os, re, sys, unicodedata
sys.path.insert(0, os.path.dirname(__file__))
from dados_a import F as FA
from dados_b import F as FB
from fontes import FONTES

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
BM = os.path.join(ROOT, 'docs', 'benchmark')
MOD = os.path.join(ROOT, 'docs', 'modulos')
F = FA + FB
ST = {'ok': '✅', 'parcial': '🟡', 'nao': '❌'}
ST_TXT = {'ok': 'completo', 'parcial': 'parcial', 'nao': 'não existe'}

# Workflow comparado de cada módulo (texto próprio, a partir das fontes listadas em fontes.md).
WORKFLOW = {
'Sketch': dict(
  catia='Sketch num plano/face → perfil desenhado com restrições automáticas → restrições pela caixa de diálogo (lista as possíveis para a seleção) → ferramentas de relimitação (Trim, Quick Trim, Break, Close, Complement) → Sketch Analysis antes de sair. O sketch é visto como sistema de equações e o objetivo é ficar totalmente restrito.',
  sw='Sketch → entidades com relações automáticas → Smart Dimension deduz o tipo de cota → Power Trim (arrastar cruzando) para limpar → cores indicam sub/totalmente/sobre-definido → Convert/Offset trazem geometria do modelo.',
  inv='Sketch com restrições automáticas por inferência → Dimension → Trim/Extend/Split alternáveis com Shift ou menu → Project Geometry / Project Cut Edges trazem o 3D → contador de graus de liberdade.',
  adotar=['Relimitação completa do CATIA (Quick Trim, Break, Close, Complement) com o arrasto do Power Trim do SW.',
          'Caixa de restrições possíveis para a seleção (CATIA) junto com a cota inteligente (SW/Inventor).',
          'Análise de sketch antes de sair: perfis abertos, sobrepostos e isolados.'],
  evitar=['Projeções sem associatividade (o Inventor não deixa aparar o projetado sem quebrar o vínculo).',
          'Perfis automáticos que criam restrições demais (alerta do contexto do sketcher).']),
'Sólidos': dict(
  catia='Separação clara: features por sketch (Pad, Pocket, Shaft, Groove, Rib, Slot, Multi-sections) e de acabamento (fillet, chamfer, draft, shell, thickness). Cada uma pede suporte → tipo de limite (Dimension, Up to next/last/plane/surface) → valores → "More" com opções avançadas. Corpos (Bodies) são combinados por operações booleanas explícitas.',
  sw='PropertyManager único por feature com grupos recolhíveis: From (início) → Direction 1/2 (fim) → Thin feature → Selected contours. Adicionar e remover são variantes do mesmo comando. Hole Wizard guiado por norma.',
  inv='Painel de propriedades com entradas, comportamento (distância, até, entre) e saída (unir, cortar, interseção, novo corpo). Thread com comprimento total ou parcial com offset. Coil e Emboss dedicados.',
  adotar=['O modelo de limites do CATIA (até próximo/último/plano/superfície) disponível em todos os comandos com extensão.',
          'Grupo "Início" do SW (offset/de superfície) e "parede fina" como opção de qualquer extrusão/revolução.',
          'Thread do Inventor: comprimento parcial com offset e cosmético/modelado na mesma feature.',
          'Espessura diferente por face na casca (os três).'],
  evitar=['Referências por índice/ponto que quebram ao mudar o perfil (DR-11): ver ADR 0010.']),
'Superfícies e wireframe': dict(
  catia='Workbench dedicada (GSD): wireframe (pontos, linhas, planos, projeções, interseções), superfícies (extrude, revolve, sweep, fill, multi-sections, blend), operações (join, split, trim, extract, healing, extrapolate) e análises. Todo comando segue suporte → modo → parâmetros → avançado, com propagação e tolerâncias explícitas.',
  sw='Ferramentas de superfície no mesmo ambiente das peças: Boundary/Lofted/Filled/Knit/Trim/Extend/Ruled/Offset/Thicken; sólidos e superfícies se misturam livremente.',
  inv='Patch, Stitch, Ruled, Trim, Extend e Sculpt; Boundary Patch aceita sketch fechado ou bordas. Stitch com tolerância cria sólido quando fecha.',
  adotar=['Propagação gravada (ponto/tangência/curvatura) e Extract usável dentro de qualquer comando (CATIA).',
          'Fill/Boundary Patch aceitando sketch, curvas e arestas misturados (SW/Inventor).',
          '"Até elemento" em toda extensão de superfície.'],
  evitar=['Comandos que só aceitam faces e ignoram linhas de sketch (falha de Fill/Join no review).']),
'Chapa metálica': dict(
  catia='Parâmetros globais (espessura, raio, tabela) → Wall → Wall On Edge / Swept Walls (Flange, Hem, Tear Drop, User Flange) → recortes e estampos → Fold/Unfold. A bancada aeroespacial adiciona abas apoiadas em superfície (OML/EOP).',
  sw='Base Flange → Edge/Miter Flange com posição da dobra explícita (material dentro, material fora, dobra fora, virtual sharp) → Hem/Jog/Sketched Bend → Flatten. Unfold/Fold para editar entre dobras.',
  inv='Regras e estilos de chapa reaproveitáveis → Face → Flange/Contour Flange/Lofted Flange → Cut com "Cut Across Bend" → Flat Pattern com orientação e linhas de dobra.',
  adotar=['Um comando de aba com tipo (aresta, cadeia, perfil, superfície, entre perfis) que abre as caixas do tipo (pedido DR-54).',
          'Posição da dobra explícita como no SW e regras de chapa como no Inventor.',
          'Validação das abas vizinhas antes de gerar (DR-57).'],
  evitar=['Abas que precisam de recuo manual para não colidir (resolvido com recuo mínimo; manter teste).']),
'Estrutural e soldas': dict(
  catia='Structure Design cria perfis sobre esqueleto e trata pontas por endcuts (snipe, trim, weld); Weld Design define os cordões. A organização em Product é manual.',
  sw='Weldment: um part multicorpo com membros estruturais, Trim/Extend com folga de solda, gussets, end caps e cordões; a Cut List agrupa corpos iguais e alimenta os desenhos (vistas por corpo e balões ligados à lista).',
  inv='Frame Generator gera uma montagem com um arquivo por membro, a partir de um esqueleto; tratamentos de ponta (miter, notch, corner joint, trim/extend) e informações por membro.',
  adotar=['O "Estrutura Pai" do contexto do usuário: um modelo mestre multicorpo (como o SW) que gera montagem com instâncias (como o Inventor) e reconhece corpos iguais.',
          'Desenho por corpo com balões ligados à lista de corte (SW).'],
  evitar=['Um arquivo por membro sem agrupamento de iguais (explosão de arquivos do Inventor).']),
'Montagem': dict(
  catia='Product com componentes; restrições (coincidência, contato, offset, ângulo, fix, fix together) e Quick Constraint; mecanismos ficam na bancada de cinemática.',
  sw='Mates padrão, avançados (largura, simetria, caminho, limite) e mecânicos (engrenagem, cremalheira, came, parafuso); SmartMates ao arrastar; padrões guiados pela feature.',
  inv='Constraints para posicionar e Joints para movimento, com tipo automático pela geometria; Design Accelerator insere conexões parafusadas e engrenagens com restrições.',
  adotar=['Juntas com tipo automático pela geometria (Inventor) e restrição rápida (CATIA/SW).',
          'Mates avançados de largura/simetria/caminho (SW).',
          'Origem própria da montagem e seleção/ocultar iguais às peças (DR-31, DR-32).'],
  evitar=['Dois painéis concorrentes (árvore + editor): já unificado na alpha.25.']),
'Desenho técnico': dict(
  catia='Folha com fundo (carimbo) separado do trabalho; vistas como snapshot paramétrico do 3D com estado (atualizada/desatualizada, links quebrados); propriedades herdadas norma → folha → vista → elemento.',
  sw='Vistas de modelo/projetadas/seção/detalhe/quebra; Model Items importam cotas do 3D; tabelas (BOM, furos, dobras, revisões, lista de corte); AutoBalloon.',
  inv='Base/Projected/Section/Detail/Break Out; Retrieve traz anotações 3D; Parts List e balões automáticos; estilos e normas.',
  adotar=['Estado de atualização visível por vista (CATIA).',
          'Cadeia de herança de propriedades norma → folha → vista → elemento (CATIA).',
          'Vistas por corpo para estrutura soldada (SW).'],
  evitar=['Recarregar silenciosamente sem avisar o que mudou.']),
'PMI 3D': dict(
  catia='FT&A: planos de anotação, vistas e capturas; tolerâncias e datums ligados às faces.',
  sw='MBD/DimXpert: esquema de cotas automático, vistas de anotação, vistas 3D com câmera e estado.',
  inv='Anotações 3D com Tolerance Advisor que verifica se a peça está totalmente definida.',
  adotar=['Realce das entidades da anotação e várias chamadas (DR-48).', 'Verificação de cotagem completa (Tolerance Advisor / DimXpert).'],
  evitar=['Chamadas que atravessam o próprio quadro (DR-47).']),
'Navegação': dict(
  catia='Botão do meio arrasta (pan); meio + outro botão gira; clique do meio num ponto recentra a órbita. Alvo estável: nunca "foge" do modelo.',
  sw='Meio gira; clique do meio num vértice/aresta/face gira em torno dele; Ctrl+meio = pan; Shift+meio = zoom; roda aproxima no ponteiro.',
  inv='Free Orbit (eixos da tela) e Constrained Orbit (eixo vertical, turntable); o centro pode ser escolhido clicando durante a órbita ou pelo SteeringWheels; ViewCube.',
  adotar=['CATIA como padrão: alvo fixo + duplo clique do meio recentra por raycast (contexto da câmera).',
          'Zoom no ponteiro (SW) e turntable opcional (Inventor).'],
  evitar=['Pivô que pula quando não há geometria sob o cursor (usar o centro da caixa como reserva).']),
'Biblioteca de normas': dict(
  catia='Catálogos de peças padrão; sem gerador de engrenagens nativo.',
  sw='Toolbox: fixadores por norma, Hole Wizard ligado aos mesmos dados, Smart Fasteners preenchendo furos com pilhas de porca/arruela.',
  inv='Content Center (biblioteca) + Design Accelerator: geradores de engrenagem, eixo, chaveta e conexão parafusada com cálculo e restrições de movimento.',
  adotar=['Uma base de dados única para furo, rosca e fixador (SW), em formato aberto (ADR 0008).',
          'Geradores com cálculo e inserção restrita (Inventor) — o usuário pediu "o mais próximo do Inventor".'],
  evitar=['Editar peças da biblioteca por sketch/feature sem atualizar a base (alerta do próprio SW).']),
'PDM / PLM': dict(
  catia='ENOVIA/VPM: produto, revisões, maturidade e estrutura num banco central.',
  sw='SOLIDWORKS PDM: cofre com check-in/out, fluxos e estados, busca por metadados.',
  inv='Vault: check-in/out, esquemas de revisão e estados de ciclo de vida.',
  adotar=['Fase 1 local (SQLite + cofre em pasta) do contexto PLM, evoluindo o fenkra-project.', 'Onde é usado e comparação de revisões.'],
  evitar=['Servidor obrigatório para o uso individual.']),
'Geral': dict(
  catia='Ocultar é hierárquico e local ao mesmo tempo; toda feature tem preview e diagnóstico de atualização.',
  sw='Pastas na árvore, "What\'s wrong" com reparo de referências, cores de estado.',
  inv='Design Doctor para reparar; pastas e visibilidade por objeto.',
  adotar=['Ocultar hierárquico que preserva o estado do filho (DR-30).', 'Reparo com candidatos no 3D (ADR 0010).'],
  evitar=['Ações de pasta que sobrescrevem o estado de cada item.']),
}

ORDEM = ['Sketch', 'Sólidos', 'Superfícies e wireframe', 'Chapa metálica', 'Estrutural e soldas', 'Montagem',
         'Desenho técnico', 'PMI 3D', 'Navegação', 'Biblioteca de normas', 'PDM / PLM', 'Geral']
SLUG = {m: re.sub(r'[^a-z0-9]+', '-', unicodedata.normalize('NFKD', m.lower()).encode('ascii', 'ignore').decode()).strip('-') for m in ORDEM}


def ancora(t):
    """Âncora no padrão do GitHub: minúsculas, sem pontuação, espaço vira hífen (acentos ficam)."""
    return re.sub(r'[^\w\- ]', '', t.lower()).replace(' ', '-')


def fonte_link(k, rel='.'):
    rot, url = FONTES[k]
    return f'[{rot}]({url})'


def main():
    ids = [x['id'] for x in F]
    assert len(ids) == len(set(ids)), 'IDs repetidos'
    for x in F:
        assert x['modulo'] in ORDEM, x
        for k in x['fontes']:
            assert k in FONTES, (x['id'], k)
    os.makedirs(BM, exist_ok=True)
    os.makedirs(MOD, exist_ok=True)
    json.dump({'gerado_em': '2026-10-29', 'versao_fenkra': '0.1.0-alpha.26', 'fontes': {k: {'rotulo': v[0], 'url': v[1]} for k, v in FONTES.items()},
               'features': F}, open(os.path.join(BM, 'catalogo.json'), 'w', encoding='utf-8'), ensure_ascii=False, indent=1)

    tot = {s: sum(1 for x in F if x['status'] == s) for s in ST}
    # ---- catálogo
    L = ['# Catálogo de features', '',
         f'{len(F)} features em {len(ORDEM)} módulos, com o nome em cada ferramenta, como funcionam em geral e o status no Fenkra '
         f'(alpha.26): ✅ {tot["ok"]} completas · 🟡 {tot["parcial"]} parciais · ❌ {tot["nao"]} inexistentes.', '',
         '> Gerado por `scripts/benchmark/gerar.py` a partir de `scripts/benchmark/dados_*.py`. Não edite à mão.',
         '> Nomes de outros produtos são marcas dos respectivos donos e aparecem só para comparação; textos próprios,',
         '> sem trechos copiados. Fontes em [fontes.md](fontes.md).', '']
    L.append('| Módulo | ✅ | 🟡 | ❌ | Workflow |')
    L.append('|---|---|---|---|---|')
    for m in ORDEM:
        xs = [x for x in F if x['modulo'] == m]
        L.append(f'| [{m}](#{ancora(m)}) | {sum(x["status"]=="ok" for x in xs)} | {sum(x["status"]=="parcial" for x in xs)} | {sum(x["status"]=="nao" for x in xs)} | [{m}]({SLUG[m]}.md) |')
    for m in ORDEM:
        L += ['', f'## {m}', '', '| ID | Feature | CATIA V5 | SOLIDWORKS | Inventor | Fenkra | Backlog |', '|---|---|---|---|---|---|---|']
        for x in [x for x in F if x['modulo'] == m]:
            L.append(f'| {x["id"]} | **{x["nome"]}** | {x["catia"]} | {x["sw"]} | {x["inv"]} | {ST[x["status"]]} {x["fenkra"]} | {x["backlog"] or ""} |')
        L.append('')
        for x in [x for x in F if x['modulo'] == m]:
            L += [f'### {x["id"]} — {x["nome"]}', '', f'- **O que faz:** {x["faz"]}', f'- **Como funciona:** {x["como"]}']
            if x['diferencas']:
                L.append(f'- **Diferenças:** {x["diferencas"]}')
            L.append(f'- **Fenkra:** {ST[x["status"]]} {ST_TXT[x["status"]]}. {x["nota"]}'.rstrip())
            L.append('- **Fontes:** ' + ' · '.join(fonte_link(k) for k in x['fontes']))
            L.append('')
    open(os.path.join(BM, 'catalogo.md'), 'w', encoding='utf-8').write('\n'.join(L))

    # ---- workflow por módulo (benchmark) e estado por módulo (modulos/)
    for m in ORDEM:
        w = WORKFLOW[m]
        xs = [x for x in F if x['modulo'] == m]
        B = [f'# Benchmark — {m}', '', '> Texto próprio, a partir das fontes públicas em [fontes.md](fontes.md). Catálogo completo do módulo em',
             f'> [catalogo.md](catalogo.md#{ancora(m)}).', '', '## Como cada ferramenta conduz o fluxo', '',
             f'- **CATIA V5:** {w["catia"]}', f'- **SOLIDWORKS:** {w["sw"]}', f'- **Inventor:** {w["inv"]}', '',
             '## O que adotar no Fenkra', ''] + [f'- {a}' for a in w['adotar']] + ['', '## O que evitar', ''] + [f'- {a}' for a in w['evitar']] + \
            ['', '## Paridade', '', '| ID | Feature | Fenkra | Falta / observação | Backlog |', '|---|---|---|---|---|']
        for x in xs:
            B.append(f'| {x["id"]} | {x["nome"]} | {ST[x["status"]]} | {x["nota"]} | {x["backlog"] or ""} |')
        open(os.path.join(BM, SLUG[m] + '.md'), 'w', encoding='utf-8').write('\n'.join(B) + '\n')

        M = [f'# {m}', '', f'Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/{SLUG[m]}.md](../benchmark/{SLUG[m]}.md).', '']
        for s, tit in (('ok', 'Pronto'), ('parcial', 'Parcial'), ('nao', 'Ainda não existe')):
            ys = [x for x in xs if x['status'] == s]
            if ys:
                M += [f'## {ST[s]} {tit}', ''] + [f'- **{x["nome"]}** ({x["fenkra"]}) — {x["nota"]}' + (f' `{x["backlog"]}`' if x['backlog'] else '') for x in ys] + ['']
        open(os.path.join(MOD, SLUG[m] + '.md'), 'w', encoding='utf-8').write('\n'.join(M))

    # ---- índices
    R = ['# Benchmark', '', 'Comparação do Fenkra com CATIA V5, SOLIDWORKS e Inventor usando **só material público**.', '',
         f'- [Catálogo de features](catalogo.md): {len(F)} features, nome em cada ferramenta, como funcionam, status no Fenkra.',
         '- Dados em [catalogo.json](catalogo.json) (alimenta o comparador interativo).',
         '- [Fontes](fontes.md) e [metodologia](#metodologia).', '', '## Workflow por módulo', '']
    R += [f'- [{m}]({SLUG[m]}.md)' for m in ORDEM]
    R += ['', '## Metodologia', '',
          '1. Lista mestra montada pela união das barras de comandos públicas das três ferramentas, deduplicada com nome canônico em português.',
          '2. Cada entrada descreve o funcionamento comum na ordem **suporte → modo → parâmetros → avançado → saída** (filosofia do design review, DR-25..29).',
          '3. Status no Fenkra lido no código (`FeatureKind`, `RefKind`, ferramentas do sketch, comandos da barra) e nos testes, na versão 0.1.0-alpha.26.',
          '4. Lacunas viram IDs de backlog: `DR-NN` (design review), `CTX-*` (contextos) ou `BM-<módulo>-NN` (achadas aqui).',
          '5. **Limitação:** a documentação oficial do CATIA V5 exige login; usamos apostilas universitárias abertas, documentação de API pública e tutoriais. As fontes de PDM/PLM são fracas (comparativos).', '']
    open(os.path.join(BM, 'README.md'), 'w', encoding='utf-8').write('\n'.join(R))
    Fo = ['# Fontes do benchmark', '', 'Consultadas em 29/10/2026. Nenhum texto foi copiado: tudo foi reescrito com palavras próprias.', '']
    for grupo, pref in (('SOLIDWORKS', 'sw-'), ('Inventor', 'inv-'), ('CATIA V5', ('catia-', 'niar')), ('PDM/PLM', 'plm-'), ('Material interno', ('dr', 'ctx-'))):
        Fo += [f'## {grupo}', '']
        Fo += [f'- `{k}` — [{v[0]}]({v[1]})' for k, v in FONTES.items() if k.startswith(pref)]
        Fo.append('')
    open(os.path.join(BM, 'fontes.md'), 'w', encoding='utf-8').write('\n'.join(Fo))
    MI = ['# Módulos', '', 'Estado de cada módulo no Fenkra (alpha.26), gerado do [catálogo](../benchmark/catalogo.md).', '',
          '| Módulo | ✅ | 🟡 | ❌ |', '|---|---|---|---|']
    for m in ORDEM:
        xs = [x for x in F if x['modulo'] == m]
        MI.append(f'| [{m}]({SLUG[m]}.md) | {sum(x["status"]=="ok" for x in xs)} | {sum(x["status"]=="parcial" for x in xs)} | {sum(x["status"]=="nao" for x in xs)} |')
    open(os.path.join(MOD, 'README.md'), 'w', encoding='utf-8').write('\n'.join(MI) + '\n')
    print(len(F), 'features', tot)


if __name__ == '__main__':
    main()
