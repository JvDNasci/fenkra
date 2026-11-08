"""Capturas do app real em lote, com diário cronológico para o artefato e o vídeo do kart.

Uso: python capturar.py <pasta_do_diario> [filtro]
Cada linha de LISTA: arquivo, peça, bancada, etapa, resultado, legenda, extras.
`rollback=n` no extra abre a peça parada no comando n: as etapas intermediárias.
`painel=<nome>` no extra diz que comando deve estar aberto na captura (B431): o programa grava
`<captura>.png.estado.json` com o que a tela mostra, e a captura só vale se bater. Sem isso, a legenda
prometia um painel que a imagem não mostrava (tela inicial, painel fechado).
`FENKRA_EXE` troca o executável.
"""
import json, os, shutil, subprocess, sys, tempfile, time, uuid

RAIZ = os.path.dirname(os.path.abspath(__file__))
MODELO = os.path.join(RAIZ, "modelo")
EXE = os.environ.get("FENKRA_EXE", "C:/cargo-target/kart14/release/fenkra.exe")
DIARIO = sys.argv[1]
FILTRO = sys.argv[2] if len(sys.argv) > 2 else ""
SUFIXO = sys.argv[3] if len(sys.argv) > 3 else ""
os.makedirs(DIARIO, exist_ok=True)
JSONL = os.path.join(DIARIO, "diario.jsonl")


def proximo():
    return 1 + len([f for f in os.listdir(DIARIO) if f.endswith(".png")])


def confere_estado(png, esperado):
    """O que a tela mostrava na hora da captura contra o que a legenda promete. Devolve (ok, motivo)."""
    try:
        e = json.load(open(png + ".estado.json", encoding="utf8"))
    except (OSError, ValueError):
        return False, "o programa não gravou o estado da tela (versão antiga do executável?)"
    if e.get("tela_inicial"):
        return False, "a tela inicial apareceu no lugar da peça"
    # painéis que não são comando de peça: o estado diz se ficaram abertos
    sinais = {"sketch": "sketch_em_edicao", "biblioteca": "biblioteca_aberta", "simulacao": "simulacao_aberta", "transmissao": "transmissao_aberta"}
    if esperado in sinais:
        return (True, "") if e.get(sinais[esperado]) else (False, f"o painel '{esperado}' não abriu (estado: {sinais[esperado]} = {e.get(sinais[esperado])})")
    if esperado and e.get("comando_aberto") != esperado:
        return False, f"o painel '{esperado}' não abriu (aberto: {e.get('comando_aberto')})"
    return True, ""


def captura(arq, peca, bancada, etapa, resultado, legenda, extra=None):
    extra = dict(extra or {})
    painel = extra.pop("painel", None)
    # demos que montam a própria peça ou montagem (FENKRA_DEMO_ASM, _SKETCH...): o arquivo aberto por
    # FENKRA_DEMO_ARQUIVO passaria por cima e o painel não abriria
    sem_arquivo = extra.pop("sem_arquivo", False)
    rollback = extra.pop("rollback", None)
    espera = int(extra.pop("espera", 7))
    fonte = os.path.join(MODELO, arq)
    tmp = tempfile.mkdtemp(prefix="kart14_")
    if rollback is not None:
        d = json.load(open(fonte, encoding="utf8"))
        d["rollback"] = rollback
        fonte = os.path.join(tmp, arq)
        json.dump(d, open(fonte, "w", encoding="utf8"), ensure_ascii=False)
    n = proximo()
    slug = f"{peca}_{etapa}".lower().replace(" ", "_").replace("/", "_")
    nome = f"{n:04d}_{slug}.png"
    png = os.path.join(DIARIO, nome)
    env = dict(os.environ, APPDATA=tmp, FENKRA_DEMO_FOTO=png, FENKRA_DEMO_FOTO_ESPERA=str(espera))
    if not sem_arquivo:
        env["FENKRA_DEMO_ARQUIVO"] = fonte
    env.setdefault("FENKRA_DEMO_CAMERA", "35,25,1")
    env.setdefault("FENKRA_DEMO_APRESENTACAO", "1")
    env.update({k: str(v) for k, v in extra.items()})
    p = subprocess.Popen([EXE], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        p.wait(timeout=espera + 60)
    except subprocess.TimeoutExpired:
        p.kill()
    shutil.rmtree(tmp, ignore_errors=True)
    if os.path.exists(png):
        ok_estado, motivo = confere_estado(png, painel)
        registro = {"arquivo": nome, "peca": peca, "bancada": bancada, "etapa": etapa, "resultado": resultado, "legenda": legenda, "conferida": ok_estado}
        if not ok_estado:
            registro["motivo"] = motivo
        with open(JSONL, "a", encoding="utf8") as f:
            f.write(json.dumps(registro, ensure_ascii=False) + "\n")
        print("ok" if ok_estado else "NÃO CONFERE", nome, motivo, flush=True)
    else:
        print("sem imagem", nome, flush=True)


# etapa de cada peça: mestre, quadro, chapas, eixo, rodas, direção, transmissão, freio, posto, proteções
L = []


def a(arq, peca, banc, etapa, leg, res="ok", **x):
    L.append((arq, peca, banc, etapa, res, leg, x))


a("MESTRE.fkp", "MESTRE", "Sketch", "esqueleto", "Desenho mestre: pontos, planos e linhas do esqueleto")
a("CH-01.fkp", "CH-01", "Estrutural", "longarina", "Longarina esquerda: tubo Ø30 dobrado sobre o esqueleto")
a("CH-01.fkp", "CH-01", "Estrutural", "painel", "Comando Membro aberto na longarina", FENKRA_EDITAR="Longarina")
a("CH-06.fkp", "CH-06", "Estrutural", "arco", "Arco dianteiro dos pedais, três trechos dobrados")
a("QUADRO.fkp", "QUADRO", "Estrutural", "so_longarinas", "Quadro: as duas longarinas", rollback=2)
a("QUADRO.fkp", "QUADRO", "Estrutural", "travessas", "Quadro: travessas e montante", rollback=5)
a("QUADRO.fkp", "QUADRO", "Estrutural", "completo", "Quadro completo com arco e apoios do banco")
a("QUADRO.fkp", "QUADRO", "Estrutural", "topo", "Quadro visto de cima: recortes das travessas", FENKRA_DEMO_CAMERA="0,89,0.7")
for c, n, r in [("CH-08", "Suporte do mancal", 2), ("CH-11", "Apoio do motor", 2), ("CH-12", "Suporte da coluna", 2), ("CH-13", "Orelha de pedal", 2), ("CH-14", "Suporte da pinça", 2)]:
    a(f"{c}.fkp", c, "Chapa", "parede", f"{n}: parede base", rollback=r)
    a(f"{c}.fkp", c, "Chapa", "pronta", f"{n}: abas e furos prontos")
a("CH-08.fkp", "CH-08", "Chapa", "painel_aba", "Comando de aba aberto no suporte do mancal", FENKRA_EDITAR="Aba1")
a("CH-09.fkp", "CH-09", "Sólidos", "pronta", "Suporte em C do pino-mestre com filete e chanfro")
a("CH-10.fkp", "CH-10", "Estrutural", "pronta", "Soquete de para-choque")
a("EX-01.fkp", "EX-01", "Sólidos", "tubo", "Eixo traseiro: tubo Ø50 × 2,5", rollback=2)
a("EX-01.fkp", "EX-01", "Sólidos", "pronta", "Eixo com quatro rasgos de chaveta e chanfros")
a("EX-01.fkp", "EX-01", "PMI", "pmi", "PMI do eixo: diâmetro, referência A e batimento", FENKRA_DEMO_CAMERA="20,20,0.5")
a("EX-02.fkp", "EX-02", "Sólidos", "pronta", "Caixa do mancal, furos em padrão circular")
a("EX-05.fkp", "EX-05", "Sólidos", "pronta", "Cubo traseiro com rasgo, chanfro e filete")
a("EX-06.fkp", "EX-06", "Sólidos", "pronta", "Porta-coroa")
a("EX-07.fkp", "EX-07", "Sólidos", "pronta", "Porta-disco")
a("EX-08.fkp", "EX-08", "Sólidos", "pronta", "Disco de freio com 24 furos em padrão")
a("RD-01.fkp", "RD-01", "Sólidos", "pronta", "Aro dianteiro com pinos do talão")
a("RD-02.fkp", "RD-02", "Sólidos", "pronta", "Aro traseiro")
a("RD-03.fkp", "RD-03", "Superfícies", "pronta", "Pneu dianteiro: perfil em spline girado e engrossado", FENKRA_BANCADA="sup")
a("RD-04.fkp", "RD-04", "Superfícies", "pronta", "Pneu traseiro", FENKRA_BANCADA="sup")
a("DI-01.fkp", "DI-01", "Forma livre", "aro", "Volante: aro em Forma livre convertido em B-rep", rollback=1)
a("DI-01.fkp", "DI-01", "Forma livre", "pronta", "Volante com raios e centro")
a("DI-02.fkp", "DI-02", "Sólidos", "pronta", "Cubo do volante")
a("DI-03.fkp", "DI-03", "Estrutural", "pronta", "Coluna de direção")
a("DI-05.fkp", "DI-05", "Chapa", "pronta", "Placa dos braços")
a("DI-06.fkp", "DI-06", "Estrutural", "pronta", "Tirante")
a("DI-08.fkp", "DI-08", "Sólidos", "ponta", "Manga: ponta de eixo por revolução", rollback=2)
a("DI-08.fkp", "DI-08", "Sólidos", "pronta", "Manga com braço unido por booleana entre coletores")
a("DI-09.fkp", "DI-09", "Sólidos", "pronta", "Pino-mestre")
a("DI-10.fkp", "DI-10", "Sólidos", "pronta", "Cubo dianteiro")
a("MO-01.fkp", "MO-01", "Sólidos", "pronta", "Volume do motor")
a("MO-02.fkp", "MO-02", "Sólidos", "pronta", "Suporte do motor com ângulo de saída")
a("TR-01.fkp", "TR-01", "Transmissão", "disco", "Coroa: disco antes dos dentes", rollback=2)
a("TR-01.fkp", "TR-01", "Transmissão", "pronta", "Coroa de 82 dentes por padrão circular, contorno da falta da 219")
a("TR-02.fkp", "TR-02", "Transmissão", "pronta", "Pinhão de 11 dentes")
a("TR-03.fkp", "TR-03", "Transmissão", "pronta", "Corrente de 106 elos, rolos no caminho das rodas")
a("TR-04.fkp", "TR-04", "Chapa", "pronta", "Protetor de corrente")
a("EX-09.fkp", "EX-09", "Sólidos", "pronta", "Corpo da pinça")
a("EX-10.fkp", "EX-10", "Sólidos", "pronta", "Pastilha")
a("EX-11.fkp", "EX-11", "Sólidos", "pronta", "Cilindro-mestre com rosca cosmética")
a("EX-12.fkp", "EX-12", "Superfícies", "falhou", "Linha de freio: a varredura de superfície na spline 3D falhou", res="falhou", FENKRA_BANCADA="sup")
a("EX-12B.fkp", "EX-12", "Sólidos", "segunda_tentativa", "Linha de freio: varredura sólida na mesma spline")
# peça parada antes do primeiro sólido: a legenda mostra a massa como "—", nunca "-0,000 kg" (B430)
a("EX-12.fkp", "EX-12", "Superfícies", "sem_corpo", "Linha de freio antes do sólido: massa sem corpo na legenda", rollback=0, FENKRA_BANCADA="sup")
a("PO-05.fkp", "PO-05", "Chapa", "pronta", "Pedal de freio")
a("PO-06.fkp", "PO-06", "Chapa", "pronta", "Pedal de acelerador")
a("PO-01.fkp", "PO-01", "Forma livre", "pronta", "Banco em Forma livre convertido em sólido", espera=30)
a("PO-02.fkp", "PO-02", "Chapa", "pronta", "Chapa de reforço do banco")
a("PO-03.fkp", "PO-03", "Estrutural", "pronta", "Escora do banco")
a("PO-04.fkp", "PO-04", "Chapa", "parede", "Assoalho: parede base", rollback=2)
a("PO-04.fkp", "PO-04", "Chapa", "pronta", "Assoalho com 42 furos em padrão")
a("PO-04.fkp", "PO-04", "Chapa", "planificado", "Assoalho planificado", FENKRA_DEMO_PLANA=1)
a("PO-08.fkp", "PO-08", "Superfícies", "pronta", "Tanque: multi-seções, casca e bocal")
for c, n in [("PR-01", "Para-choque superior"), ("PR-02", "Para-choque inferior"), ("PR-04", "Barra lateral inferior"), ("PR-05", "Barra lateral superior"), ("PR-10", "Suporte da proteção traseira")]:
    a(f"{c}.fkp", c, "Estrutural", "pronta", n)
for c, n in [("PR-06", "Bico em Forma livre"), ("PR-07", "Painel frontal em Forma livre"), ("PR-08", "Pontão lateral em Forma livre"), ("PR-09", "Proteção traseira em Forma livre")]:
    a(f"{c}.fkp", c, "Forma livre", "pronta", n)
a("MO-03.fkp", "MO-03", "Superfícies", "pronta", "Escape: multi-seções e casca")
for c, n in [("CV-CHAPA1", "Estampo, veneziana e colar"), ("CV-CHAPA2", "Bainha e desvio"), ("CV-CHAPA3", "Alívio de canto"), ("CV-CHAPA4", "Canal convertido em chapa"), ("CV-SOL1", "Furos, rosca modelada e espessura"), ("CV-SOL2", "Nervura, escala e dividir"), ("CV-EST", "Placa, reforço e solda"), ("CV-SUP1", "Superfície deslocada"), ("CV-SUP2", "Preenchimento")]:
    a(f"{c}.fkp", c, "Cobertura", "pronta", n)
for s_, n in [("SC-QUADRO", "Subconjunto do quadro"), ("SC-EIXO", "Subconjunto do eixo e rodas traseiras"), ("SC-DIRECAO", "Subconjunto da direção"), ("SC-MOTOR", "Subconjunto do motor e transmissão"), ("SC-FREIO", "Subconjunto do freio e pedais"), ("SC-POSTO", "Subconjunto do posto do piloto"), ("SC-CARROCERIA", "Subconjunto da carroceria")]:
    a(f"{s_}.fka", s_, "Montagem", "pronta", n, espera=25)
a("KART-PLANO.fka", "KART", "Montagem", "geral", "Montagem geral do kart", espera=60)
# o kart corre ao longo de X (assim foi montado em conjunto.rs), então "de lado" é olhar por -Y, a vista
# Frente do cubo: yaw -90. Com yaw 0 o olho fica em +X (vista Direita) e o kart aparece de frente (B430).
a("KART-PLANO.fka", "KART", "Montagem", "lateral", "Kart de lado (vista Frente do cubo)", espera=60, FENKRA_DEMO_CAMERA="-90,5,0.9")
a("KART.fka", "KART", "Montagem", "subconjuntos", "Montagem de topo com o mestre e os subconjuntos", espera=60)
# estados que só o mouse abre, pelas variáveis FENKRA_DEMO (B431): Biblioteca, gerador, simulação e Sketch
a("SC-QUADRO.fka", "BIBLIOTECA", "Montagem", "painel", "Biblioteca de normas com o M6 escolhido", espera=30, painel="biblioteca", sem_arquivo=True, FENKRA_DEMO_ASM="biblioteca")
a("SC-QUADRO.fka", "GERADOR", "Montagem", "painel", "Gerador de transmissão (engrenagens e correntes)", espera=30, painel="transmissao", sem_arquivo=True, FENKRA_DEMO_ASM="transmissao")
a("SC-QUADRO.fka", "SIMULACAO", "Montagem", "painel", "Janela de simulação de movimento da cadeia", espera=30, painel="simulacao", sem_arquivo=True, FENKRA_DEMO_ASM="cadeia")
a("CV-SOL1.fkp", "SKETCH", "Sketch", "retangulo", "Sketch de retângulo em edição", espera=12, painel="sketch", FENKRA_DEMO_SKETCH="retangulo")
a("DW-KART.fkd", "DW-KART", "Desenho", "folha", "Folha de conjunto com lista de peças", espera=150)
a("DW-QUADRO.fkd", "DW-QUADRO", "Desenho", "folha", "Quadro com lista de corte", espera=15)
a("DW-EX01.fkd", "DW-EX01", "Desenho", "folha", "Eixo: vista e corte", espera=12)
a("DW-EX08.fkd", "DW-EX08", "Desenho", "folha", "Disco com tabela de furos", espera=12)
a("DW-DI08.fkd", "DW-DI08", "Desenho", "folha", "Manga: vista e corte", espera=12)
a("DW-PO-04.fkd", "DW-PO-04", "Desenho", "folha", "Assoalho planificado com tabela de dobras", espera=12)
a("DW-CH09.fkd", "DW-CH09", "Desenho", "folha", "Peça filha com legenda", espera=12)

for arq, peca, banc, etapa, res, leg, x in L:
    if FILTRO and FILTRO not in arq:
        continue
    if not os.path.exists(os.path.join(MODELO, arq)):
        print("sem arquivo", arq, flush=True)
        continue
    if SUFIXO:
        etapa, leg = f"{etapa}_{SUFIXO}", f"{leg} ({SUFIXO})"
    captura(arq, peca, banc, etapa, res, leg, x)
print("fim", flush=True)
