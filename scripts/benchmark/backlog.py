# Backlog consolidado (design review + benchmark). Gera docs/backlog/backlog.{json,md}.
# Uso: python -I scripts/benchmark/backlog.py
import json, os, sys
sys.path.insert(0, os.path.dirname(__file__))
from dados_a import F as FA
from dados_b import F as FB

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
OUT = os.path.join(ROOT, 'docs', 'backlog')
CAT = {x['id']: x for x in FA + FB}

ONDAS = {
    0: ('Onda 0 — fundação: filosofia robusta (contexto, publicações, salvar, testes)', '0.1.0-alpha.28 a .30'),
    1: ('Onda 1 — bugs do design review', '0.1.0-alpha.31'),
    2: ('Onda 2 — filosofia base + VPM local', '0.1.0-alpha.32'),
    3: ('Onda 3 — módulos novos e lacunas do benchmark', '0.1.0-alpha.33+'),
}

def c(id, onda, prio, titulo, oque, origem, cat, tokens_k, teste, area, status='a fazer'):
    return dict(id=id, onda=onda, prio=prio, titulo=titulo, oque=oque, origem=origem, catalogo=cat,
                tokens_k=tokens_k, teste=teste, area=area, status=status)

B = [
# ---------------------------------------------------------------- onda 1
c('B01', 1, 'P0', 'Chapa: abas em sequência, múltiplas e vizinhas', 'Corrigir dobras em sequência, várias dobras + outra lateral, abas compostas lado a lado (com validação de colisão) e aba em cadeia reta + curva.',
  ['DR-54', 'DR-55', 'DR-56', 'DR-57'], ['CHP-03', 'CHP-04'], 200, 'Auto + Visual', 'Chapa metálica'),
c('B02', 1, 'P0', 'Chapa: recorte planificado → dobrado no raio', 'O recorte feito no planificado deve voltar ao dobrado mesmo atravessando o raio de dobra.',
  ['DR-20'], ['CHP-11'], 60, 'Auto', 'Chapa metálica'),
c('B03', 1, 'P0', 'Chapa: dobra por linha em spline', 'Dobra por linha aceitando spline e emendando os raios.', ['DR-17', 'DR-22'], ['CHP-10'], 70, 'Auto', 'Chapa metálica'),
c('B04', 1, 'P0', 'Chapa: aba por superfície (nervura de asa)', 'Fazer a aba por superfície funcionar nos casos do usuário, incluindo a nervura de asa.', ['DR-19', 'DR-23'], ['CHP-06'], 90, 'Auto + Visual', 'Chapa metálica'),
c('B05', 1, 'P0', 'Rosca: preview, personalização e início', 'Preview ao vivo; passo, perfil e tamanho personalizados; deslocamento inicial e chanfro de entrada; corrigir o tap.',
  ['DR-02', 'DR-03'], ['SOL-07', 'GER-02'], 140, 'Auto', 'Sólidos'),
c('B06', 1, 'P0', 'Casca: spline, furo escareado, opção reta', 'Casca funcionando com spline e furo escareado; para fora com opção reta/raio editável; espessura por face.',
  ['DR-02', 'DR-07'], ['SOL-11'], 110, 'Auto', 'Sólidos'),
c('B07', 1, 'P1', 'Engrossar face de sólido', 'Espessura aplicada a face de sólido.', ['DR-12'], ['SOL-14'], 40, 'Auto', 'Sólidos'),
c('B08', 1, 'P0', 'Sketch: tangência da spline e interseção', 'Spline tangente a outros elementos; interseção com elementos de outro plano devolvendo todos os pontos.',
  ['DR-02', 'DR-06'], ['SKT-04', 'SKT-21'], 120, 'Auto', 'Sketch'),
c('B09', 1, 'P0', 'Fill, Join e Preencher com sketch', 'Aceitar linhas de sketch e seleção em cadeia em Fill, Join e Preencher fronteiras.', ['DR-13'], ['SUP-12', 'SUP-15'], 140, 'Auto', 'Superfícies e wireframe'),
c('B10', 1, 'P1', 'Render: linhas indevidas e filete de superfície', 'Remover riscos no render (como o modo sem arestas, mas com as arestas principais) e corrigir arestas em filetes de superfície.',
  ['DR-08', 'DR-10'], ['GER-06', 'SUP-19'], 60, 'Visual', 'Geral'),
c('B11', 1, 'P1', 'Nervura com parede dos dois lados', 'Espessura para um lado, para o outro ou simétrica.', ['DR-05'], ['SOL-13'], 40, 'Auto', 'Sólidos'),
c('B12', 1, 'P1', 'Loft com restrições e tensão', 'Restrição inicial/final por elemento, tensão de concordância, coletor com seleção múltipla.', ['DR-07'], ['SOL-05'], 110, 'Auto', 'Sólidos'),
c('B13', 1, 'P1', 'Planos de face e plano na curva', 'Plano de face entra em erro quando a face some; plano na curva por ponto contido ou projetado.', ['DR-21', 'DR-04'], ['SUP-03'], 60, 'Auto', 'Superfícies e wireframe'),
c('B14', 1, 'P1', 'Superfícies: "até elemento" e dois limites', 'Extensões de superfície com até elemento; distâncias inicial e final juntas.', ['DR-24', 'DR-16'], ['SUP-09'], 60, 'Auto', 'Superfícies e wireframe'),
# ---------------------------------------------------------------- onda 2
c('B15', 2, 'P0', 'Referências v2 (ADR 0010)', 'Referências por dono + papel + assinatura, com o ponto como reserva; re-anexar sozinho ou mostrar reparo com candidatos no 3D.',
  ['DR-11'], ['SOL-24', 'GER-05'], 400, 'Auto', 'Sólidos', 'em andamento'),
c('B16', 2, 'P0', 'Ocultar hierárquico', 'O pai oculta os filhos e cada um guarda o próprio estado; ocultar a origem e os menus da árvore no próprio item.', ['DR-15', 'DR-30'], ['GER-01'], 80, 'Auto + Visual', 'Geral'),
c('B17', 2, 'P1', 'Câmera no estilo CATIA', 'Duplo clique do meio recentra por raycast; pivô sob o cursor (SW/Rhino); zoom no cursor; turntable (Inventor).',
  ['CTX-CAM'], ['NAV-02', 'NAV-03', 'NAV-04'], 100, 'Visual', 'Navegação'),
c('B18', 2, 'P1', 'Relimitação do sketch', 'Quick trim (borracha) em todas as curvas, break, trim-extend, close arc e complement.', ['DR-50', 'DR-51', 'DR-52', 'DR-53'],
  ['SKT-10', 'SKT-11', 'SKT-12', 'SKT-13', 'SKT-14'], 220, 'Auto', 'Sketch'),
c('B19', 2, 'P1', 'Seleção por janela e geometria externa no sketch', 'Janela/cruzada pelo sentido do arrasto dentro do sketch; política de seleção externa (publicar ou bloquear).',
  ['DR-05', 'DR-18'], ['SKT-22', 'SKT-23'], 60, 'Visual', 'Sketch'),
c('B20', 2, 'P1', 'Filosofia dos painéis de comando', 'Ordem suporte → modo → parâmetros → avançado em todos os comandos; caixas quando o menu é curto e abas quando é longo; direção normal a plano/superfície.',
  ['DR-09', 'DR-25', 'DR-26', 'DR-27', 'DR-28', 'DR-29'], ['SUP-23', 'GER-03'], 120, 'Visual', 'Geral'),
c('B21', 2, 'P1', 'Extract com propagação gravada', 'Propagação recalculada a cada atualização; extract disponível no botão direito de outros comandos.', ['DR-13', 'DR-14'], ['SUP-08'], 80, 'Auto', 'Superfícies e wireframe'),
c('B22', 2, 'P1', 'Montagem: origem, seleção e contexto', 'Origem da montagem; Ctrl/Alt; ocultar/apagar como nas peças (edição no contexto foi para o B54).', ['DR-31', 'DR-32'],
  ['MON-11', 'MON-13', 'MON-14'], 120, 'Visual', 'Montagem'),
c('B23', 2, 'P1', 'PMI: realce e chamadas', 'Realce das entidades da anotação, várias setas por anotação e seta que não atravessa o quadro.', ['DR-31', 'DR-47', 'DR-48'], ['PMI-04'], 70, 'Visual', 'PMI 3D'),
c('B24', 2, 'P2', 'Pastas e setas de referência', '"Selecionados → nova pasta" escolhendo o pai; setas 3D do referencial nos pontos.', ['DR-46', 'DR-49'], ['GER-04', 'SUP-01'], 60, 'Visual', 'Geral'),
c('B25', 2, 'P1', 'VPM fase 1 (local e em pasta)', 'SQLite + cofre em pasta: produto, revisão, documento, estrutura (BOM) e histórico; migração do índice JSON do fenkra-project; modo pasta compartilhada (LAN/OneDrive) com lock.json e detecção de conflito; VPMReader/Writer/Environment e painel lateral.',
  ['CTX-PLM', 'CTX-PN'], ['PLM-02', 'PLM-03', 'PLM-04', 'PLM-05', 'PLM-07'], 340, 'Auto', 'PDM / PLM'),
c('B47', 2, 'P1', 'PNEngine: regras de part-number e ambientes', 'Editor de regras com campos ([NORMA], [CATEGORIA], [MATERIAL], [DIM], [SERIAL:n], [REV]), interpretador que preenche pela peça, contadores por ambiente/categoria, ambientes (Industrial, Aeroespacial, Cliente X) salvos em JSON.',
  ['CTX-PN'], ['PLM-06'], 120, 'Auto', 'PDM / PLM'),
# ---------------------------------------------------------------- onda 3
c('B26', 3, 'P1', 'Estrutura Pai → montagem dinâmica', 'Corpos iguais viram uma peça com instâncias; subpeças linkadas; submontagens de solda; balões e lista de corte; desenho por corpo.',
  ['CTX-EST', 'DR-33', 'DR-44', 'DR-45'], ['EST-06', 'EST-07', 'EST-08', 'DES-08', 'DES-11'], 500, 'Auto + Visual', 'Estrutural e soldas', 'feito'),
c('B27', 3, 'P1', 'Catálogo de perfis com dimensões', 'Perfis W/H/U/L Gerdau e Aço Brasil (NBR), IPE/HEA/HEB/HEM/UPN/UPE (ArcelorMittal, EN 10365), W/HP/S/C/MC/L/HSS (AISC), tubos EN 10210/10219, ASTM A500 e Vallourec; JIS e AS/NZS depois. Propriedades calculadas e conferidas com o publicado.',
  ['CTX-PRF', 'CTX-HW'], ['EST-03'], 250, 'Auto', 'Estrutural e soldas'),
c('B28', 3, 'P2', 'Gerador de engrenagens, eixos e chavetas', 'Reta, helicoidal, interna e cremalheira (involuta), eixos e chaveta DIN 6885, com cálculo e inserção com restrições, no estilo do Inventor.',
  ['CTX-ENG'], ['BIB-03', 'BIB-04'], 400, 'Auto', 'Biblioteca de normas', 'feito'),
c('B29', 3, 'P1', 'Fixadores industriais e smart fasteners', 'Parafusos, porcas e arruelas ISO/DIN/ABNT (medidas pelos catálogos Ciser, Brasfixo, Vonder, Belzer) e preenchimento automático de furos com pilhas e restrições.',
  ['CTX-FIX', 'CTX-FXL', 'CTX-HW'], ['BIB-01', 'BIB-02', 'MON-16'], 450, 'Auto + Visual', 'Biblioteca de normas', 'feito'),
c('B42', 3, 'P0', 'Infraestrutura da biblioteca', 'Formato aberto por família (norma, fonte, nível A/B/C, licença), camadas pública/empresa/usuário, gerador de geometria por família, cálculo e conferência de propriedades nos testes.',
  ['CTX-HW', 'ADR-0008'], ['BIB-01', 'EST-03'], 150, 'Auto', 'Biblioteca de normas', 'em andamento'),
c('B43', 3, 'P2', 'Fixadores aeronáuticos e militares', 'AN/MS/MIL de domínio público (ASSIST, Distribution A): AN3–AN10, MS20004, MS20073, MS21250, MS21042, MS20002, MS20392, AN470/AN426, AN310/AN315; NAS só por fontes públicas equivalentes.',
  ['CTX-HW', 'CTX-FXL'], ['BIB-06'], 200, 'Auto', 'Biblioteca de normas'),
c('B44', 3, 'P2', 'Rolamentos', 'Rolamentos por medidas de contorno ISO 15 (iguais em SKF, FAG/INA, NSK); capacidade de carga citada do fabricante.', ['CTX-HW'], ['BIB-07'], 150, 'Auto', 'Biblioteca de normas'),
c('B45', 3, 'P3', 'Pneumática', 'Cilindros ISO 15552/6432 (Festo, Parker, SMC) com cursos e fixações; séries próprias como camada de fabricante.', ['CTX-HW'], ['BIB-08'], 150, 'Auto', 'Biblioteca de normas'),
c('B46', 3, 'P3', 'Guias lineares e fusos', 'Trilhos, patins e fusos por série de fabricante (THK, Hiwin, Bosch Rexroth).', ['CTX-HW'], ['BIB-09'], 200, 'Auto', 'Biblioteca de normas'),
c('B30', 3, 'P2', 'Desenho: atualização, hachura e normas', 'Estado de atualização por vista; propriedades de hachura; 3º diedro; herança norma → folha → vista → elemento; vista interrompida e corte parcial.',
  ['DR-34', 'DR-35', 'DR-36', 'DR-37', 'DR-38', 'DR-39', 'DR-40', 'DR-41', 'DR-42', 'DR-43'], ['DES-02', 'DES-04', 'DES-06', 'DES-07', 'DES-10'], 300, 'Auto + Visual', 'Desenho técnico'),
c('B31', 3, 'P2', 'Comando de aba unificado', 'Um comando com tipo (aresta, cadeia, perfil, superfície, entre perfis) que abre as caixas do tipo.', ['DR-54'], ['CHP-17'], 150, 'Visual', 'Chapa metálica'),
c('B32', 3, 'P2', 'Sketch: entidades e ferramentas que faltam', 'Retângulo por 3 pontos e paralelogramo, elipse/cônicas, rasgo em arco, girar/escalar, padrão, texto, sketch 3D com restrições.',
  ['BM-SKT-01', 'BM-SKT-02', 'BM-SKT-03', 'BM-SKT-05', 'BM-SKT-06', 'BM-SKT-07', 'BM-SKT-08'], ['SKT-02', 'SKT-05', 'SKT-06', 'SKT-17', 'SKT-19', 'SKT-24', 'SKT-25'], 250, 'Auto', 'Sketch'),
c('B33', 3, 'P2', 'Sólidos: relevo, mola, modelagem direta', 'Relevo/gravação, comando de mola (coil) e edição direta de faces.', ['BM-SOL-01', 'BM-SOL-02', 'BM-SOL-03'], ['SOL-21', 'SOL-22', 'SOL-23'], 200, 'Auto', 'Sólidos'),
c('B34', 3, 'P2', 'Superfícies: guias, regrada e análises', 'Loft com várias guias, superfície regrada, análise de desvio e de continuidade.', ['BM-SUP-01', 'BM-SUP-02', 'BM-SUP-03'], ['SUP-11', 'SUP-18', 'SUP-22'], 150, 'Auto', 'Superfícies e wireframe'),
c('B35', 3, 'P2', 'Chapa: entre perfis, desdobrar parcial, converter', 'Aba entre perfis (lofted), desdobrar/redobrar parcial e converter sólido em chapa.', ['BM-CHP-01', 'BM-CHP-02', 'BM-CHP-03'], ['CHP-07', 'CHP-15', 'CHP-16'], 250, 'Auto', 'Chapa metálica'),
c('B36', 3, 'P2', 'Montagem: restrições avançadas', 'Largura, simetria, caminho e limite.', ['BM-MON-01'], ['MON-06'], 100, 'Auto', 'Montagem'),
c('B37', 3, 'P2', 'Estrutural: entalhe, cordões, barras', 'Entalhe (notch) por perfil, cordão de solda 3D e plano de corte otimizado.', ['BM-EST-01', 'BM-EST-02', 'BM-EST-03'], ['EST-02', 'EST-05', 'EST-09'], 150, 'Auto', 'Estrutural e soldas'),
c('B38', 3, 'P3', 'PMI: consultor de tolerância', 'Verificar se a peça está totalmente cotada e tolerada.', ['BM-PMI-01'], ['PMI-05'], 120, 'Auto', 'PMI 3D'),
c('B39', 3, 'P3', 'API Python', 'Automação por scripts (RNF-011).', ['BM-GER-01'], ['GER-07'], 250, 'Auto', 'Geral'),
c('B40', 3, 'P3', 'Herdados: varredura cônica, subdivisão, dobra cega', 'Varredura cônica/adaptativa com várias guias e G2 exato; formas por subdivisão; dobra cega e cônica, cortes flangeados, alívios aeronáuticos.',
  ['histórico'], [], 200, 'Auto', 'Superfícies e wireframe'),
c('B41', 1, 'P1', 'Teste com o mouse (usuário)', 'Pendências antigas que dependem do usuário: desenho por mouse, cotagem smart, menu rápido, arrastar na árvore, manipulador, bancadas.',
  ['histórico'], [], 0, 'Manual', 'Geral'),
# ---------------------------------------------------------------- lacunas restantes do catálogo
c('B48', 2, 'P1', 'Furo: ângulo do escareado e casca com escareado', 'Ângulo do escareado editável (82°, 90°, 100°, 120°) e por norma; casca deixa de falhar quando o corpo tem furo escareado.',
  ['DR-02', 'BM-SOL'], ['SOL-06'], 80, 'Auto', 'Sólidos'),
c('B49', 3, 'P2', 'Filete variável com pontos de controle', 'Pontos intermediários ao longo da aresta com raio próprio, transição linear ou suave, edição pelos pontos na viewport.',
  ['BM-SOL'], ['SOL-09'], 120, 'Auto + Visual', 'Sólidos'),
c('B50', 3, 'P1', 'Estrutura → montagem viva e desenho por item', 'Links vivos (atualizar sem gerar de novo), submontagens de solda, limpeza de arquivos órfãos; desenho com vista própria por item/corpo e detalhe de ponta.',
  ['DR-44', 'DR-45'], ['EST-08', 'DES-11'], 250, 'Auto + Visual', 'Estrutural e soldas'),
c('B51', 3, 'P2', 'Fixadores: mais famílias e edição da pilha', 'Pinos, prisioneiros, porcas autotravantes, polegada/ASTM e tamanhos acima de M20 (dados gratuitos em duas fontes); trocar norma/comprimento de uma pilha inserida; pega medida pela face em vez de sondas.',
  ['CTX-FIX', 'CTX-HW'], ['MON-16', 'BIB-02'], 200, 'Auto', 'Biblioteca de normas'),
c('B52', 3, 'P2', 'Engrenagens e eixos totalmente paramétricos', 'Perfil involuto recalculado ao mudar m/z/α/β no documento; filete de raiz (trocoide); engrenagem cônica e coroa/sem-fim; eixo com ranhuras de anel, roscas e cálculo de resistência; t2 da DIN 6885 em segunda fonte.',
  ['CTX-ENG'], ['BIB-03', 'BIB-04'], 300, 'Auto', 'Biblioteca de normas'),
# ---------------------------------------------------------------- onda 0 — fundação (pesquisa CATIA/SW/Inventor, ADR 0011)
c('B53', 0, 'P0', 'Corpus de peças de referência e regressão', '16 peças típicas (suporte usinado, flange, carcaça com casca, eixo, chapa, quadro soldado, garrafa, superfícies, engrenagem, mola, montagens com fixadores e mecanismo, PMI+desenho) com valores esperados; cada uma: recompute sem erro e igual ao cache, volume/área/bbox/nº de faces, malha fechada, mudar cada cota e voltar (referências seguem, nada some calado), salvar/abrir idêntico; script de cobertura feature → teste; captura visual FENKRA_DEMO_CAPTURA comparada por pixels. Toda feature nova entra com peça no corpus.',
  ['PESQ-TST'], [], 250, 'Auto + Visual', 'Geral'),
c('B54', 0, 'P0', 'Editar peça no contexto da montagem', 'Duplo clique ativa a peça SEM sair da montagem: resto sombreado e esmaecido, selecionável para referência; um componente ativo por vez; Voltar / Voltar ao pai / Voltar ao topo; descartar ou salvar ao sair; criar componente no contexto (nome e pasta); links contextuais com estado na árvore (-> em contexto, ->? fora, ->* travado, ->x quebrado), Travar/Destravar/Quebrar e lista de links por peça; aviso quando a peça tem várias instâncias.',
  ['PESQ-CAT', 'PESQ-SW', 'PESQ-INV', 'DR-32'], ['MON-11', 'MON-19'], 450, 'Auto + Visual', 'Montagem'),
c('B55', 0, 'P0', 'Montagem com sketch, referências e publicações', 'Sketch, ponto, eixo, plano e sistema de eixos no nível da montagem; peça esqueleto (origem comum) e Fazer peça a partir dele; publicação com nome de ponto, vetor/eixo, plano, face e sketch em peças e montagens; seleção em contexto usa só publicados (opção padrão); substituir componente religa por nome e lista o que não casou; substituir a geometria de um nome publicado mantém os links.',
  ['PESQ-CAT', 'PESQ-INV'], ['MON-17'], 350, 'Auto + Visual', 'Montagem'),
c('B56', 0, 'P0', 'Salvar unificado e Salvar tudo', 'Um só grupo Salvar / Salvar tudo / Salvar como na barra, igual em peça, montagem e desenho; Salvar tudo abre lista de arquivos modificados com o motivo (editado, novo, dependência) e exige nome/pasta para os novos; peças modificadas marcadas na árvore da montagem; Ctrl+S salva o documento ativo e Ctrl+Shift+S salva tudo; Empacotar (Pack and Go) com verificação de referências.',
  ['PESQ-SW', 'PESQ-INV', 'AUD-FNK'], ['GER-08'], 150, 'Auto + Visual', 'PDM / PLM'),
c('B57', 0, 'P0', 'Furo entre peças como nos CADs de referência', 'Furo de montagem pelo eixo (Série de furos): atravessa os componentes que o eixo cruza (escopo editável), tipo por peça (folga na de cima, roscado/cego na de baixo), posição por ponto publicado, sketch da montagem ou cotas, padrão; grava um furo real em cada peça com link -> editável só pela montagem; opção "só na montagem" com participantes; hole_table e Fixadores passam a enxergar esses furos; verificação de que toda peça do escopo recebeu o furo.',
  ['PESQ-SW', 'PESQ-INV', 'AUD-FNK'], ['MON-12'], 350, 'Auto + Visual', 'Montagem'),
# ---------------------------------------------------------------- onda 0 — P1
c('B58', 0, 'P1', 'Conexão parafusada e rebites guiados por pontos', 'Assistente tipo Bolted Connection: planos de início/fim, posição linear, concêntrica, por furo (seguir padrão) e por ponto publicado; cria os furos em todas as peças da pilha; pilhas superior/inferior ordenadas; comprimentos só da série do catálogo; editar depois pelo mesmo assistente; apagar a conexão apaga a pilha. Rebites: MS20470/AN470, MS20426/AN426 e cegos ISO 15983 (dados gratuitos em duas fontes, AC 43.13-1B para furos), inserção em massa por pontos, furos ou padrão, comprimento pela espessura do pacote.',
  ['PESQ-INV', 'PESQ-REB', 'CTX-HW'], ['MON-18', 'BIB-10'], 400, 'Auto + Visual', 'Biblioteca de normas'),
c('B59', 0, 'P1', 'Árvore robusta: objeto em trabalho e diagnóstico de atualização', 'Objeto em trabalho visível (onde entra o próximo comando), barra de fim de peça para rollback e inserir no meio; contêineres ordenado (consome) e não ordenado (referencia) com regras de aninhamento; atualização manual/automática com estado desatualizado/falhou por comando; diálogo de diagnóstico com Editar (realça a referência perdida), Substituir, Isolar, Desativar e Reordenar; pais/filhos por comando; planos/eixos/pontos de referência oferecidos antes de faces ao criar sketch.',
  ['PESQ-CAT', 'PESQ-SW', 'PESQ-INV'], ['GER-11', 'GER-05'], 300, 'Auto + Visual', 'Geral'),
c('B60', 0, 'P1', 'Ícones que mostram o resultado e tooltips com mini fluxo', 'Revisar todos os ícones para mostrar o resultado do comando (ex.: extrusão = perfil + seta + sólido); tooltip em dois estágios (500 ms: nome, atalho e frase; ~1,2 s ou Shift: mini fluxograma entrada → operação → resultado com ícones internos); linha na barra de status com o próximo passo do comando; registro automático de ícones e teste que falha se um comando não tiver ícone, tooltip ou tradução.',
  ['PESQ-UX', 'AUD-FNK'], ['GER-10'], 300, 'Auto + Visual', 'Geral'),
c('B61', 0, 'P1', 'Configurações de gráficos e qualidade de renderização', 'Presets rascunho/normal/alta + controle deslizante (desvio relativo à peça com piso na tolerância e ângulo 30°/20°/12°) e modo de desvio fixo com limites; aplicar às peças da montagem; MSAA desligado/2×/4×/8×; simplificar só durante a navegação (taxa mínima de quadros) e voltar à qualidade total ao parar; referências nunca somem por culling; constante única de tesselação no lugar das 7 espalhadas; cache de malha invalidado ao trocar a qualidade.',
  ['PESQ-CAT', 'PESQ-SW', 'PESQ-INV', 'PESQ-UX', 'AUD-FNK'], ['GER-09'], 200, 'Auto + Visual', 'Navegação'),
]
def main():
    for x in B:
        for k in x['catalogo']:
            assert k in CAT, (x['id'], k)
    tot = sum(x['tokens_k'] for x in B)
    for x in B:
        x['pct'] = round(100 * x['tokens_k'] / tot, 1)
    ondas = {o: dict(titulo=t, versao=v, tokens_k=sum(x['tokens_k'] for x in B if x['onda'] == o)) for o, (t, v) in ONDAS.items()}
    for o in ondas.values():
        o['pct'] = round(100 * o['tokens_k'] / tot, 1)
    json.dump(dict(gerado_em='2026-10-29', tokens_total_k=tot, ondas=ondas, itens=B), open(os.path.join(OUT, 'backlog.json'), 'w', encoding='utf-8'), ensure_ascii=False, indent=1)

    L = ['# Backlog', '', 'Consolidado do [design review](../contextos/design-review.md), dos [contextos](../contextos/) e do [benchmark](../benchmark/).',
         'Versão visual (kanban + comparador): [painel.html](painel.html), gerada por `scripts/benchmark/pagina.py`.',
         'Gerado por `scripts/benchmark/backlog.py`; o estado de cada item é atualizado aqui a cada entrega.', '',
         f'**Estimativa total:** ≈ {tot/1000:.1f} M tokens (≈ {tot*1000/300/1000:.0f} mil linhas de código, pela média das últimas versões).', '',
         '| Onda | Versão | Itens | Tokens | % |', '|---|---|---|---|---|']
    for o, d in ondas.items():
        L.append(f'| {d["titulo"]} | `{d["versao"]}` | {sum(1 for x in B if x["onda"] == o)} | ≈ {d["tokens_k"]} k | {d["pct"]}% |')
    L += ['', 'Prioridade: **P0** quebra o uso · **P1** importante · **P2** desejável · **P3** futuro.',
          'Teste: **Auto** = `cargo test` com documento montado em código · **Visual** = captura do app com `FENKRA_DEMO_*` · **Manual** = usuário.', '']
    for o, d in ondas.items():
        L += [f'## {d["titulo"]} → `{d["versao"]}`', '', '| ID | P | Item | Origem | Catálogo | Tokens | % | Teste | Estado |', '|---|---|---|---|---|---|---|---|---|']
        for x in sorted([x for x in B if x['onda'] == o], key=lambda x: (x['prio'], x['id'])):
            cat = ', '.join(x['catalogo'])
            L.append(f'| {x["id"]} | {x["prio"]} | **{x["titulo"]}** — {x["oque"]} | {", ".join(x["origem"])} | {cat} | {x["tokens_k"]} k | {x["pct"]}% | {x["teste"]} | {x["status"]} |')
        L.append('')
    L += ['## Versionamento', '',
          '- Trabalho em `develop`; um commit por item citando o ID (`B05 Rosca: …`).',
          '- Fim de cada onda: versão no `Cargo.toml`, tag `v0.1.0-alpha.N`, `scripts\\release.ps1`, commit do `Cargo.lock`.',
          '- Push para `origin` só com confirmação do dono do repositório.', '']
    open(os.path.join(OUT, 'backlog.md'), 'w', encoding='utf-8').write('\n'.join(L))
    print(len(B), 'itens', tot, 'k tokens', {o: (d['tokens_k'], d['pct']) for o, d in ondas.items()})


if __name__ == '__main__':
    main()
