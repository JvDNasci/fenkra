// SPDX-License-Identifier: LGPL-2.1-or-later
//! B149: comando Balões automáticos com prévia. Antes de confirmar, escolhe-se o lado (o mais perto,
//! direita, esquerda, acima, abaixo), a distância da vista e o passo entre balões; dois cliques na
//! folha fazem uma janela (só as peças com a ponta dentro dela) e Shift+clique tira ou devolve um balão
//! da prévia. Na mesma barra: alinhar na horizontal ou na vertical os balões que estão na janela, e
//! Atualizar, que renumera os balões pela ordem da lista de materiais.

use super::*;
use fenkra_desenho::baloes::{self, DwgBalao, Eixo, Lado};

/// Estado do comando (as escolhas ficam para a próxima vez).
#[derive(Clone)]
pub struct Auto {
    pub lado: Lado,
    pub distancia: f64,
    pub passo: f64,
    /// cantos da janela (0, 1 ou 2 cliques)
    pub janela: Vec<V2>,
    /// peças tiradas da prévia com Shift+clique
    pub fora: Vec<String>,
    /// Shift apertado no último quadro (o clique não tem o teclado)
    pub shift: bool,
    /// onde o painel do comando está (livre ou encaixado)
    pub painel: crate::paineis::Posicao,
    /// peças já lidas da montagem (ler a montagem a cada quadro pesa): folha e o resultado
    pub cache: Option<(Id, CandidatosVista)>,
}

/// A vista, o retângulo dela na folha e as peças.
pub type CandidatosVista = (View, V2, V2, Vec<Candidato>);

impl Default for Auto {
    fn default() -> Self {
        Auto { lado: Lado::Perto, distancia: 12., passo: 8., janela: vec![], fora: vec![], shift: false, painel: Default::default(), cache: None }
    }
}

/// Uma peça que pode ganhar balão: número do item, chave na lista, ponta na folha e ponto no modelo.
#[derive(Clone, Debug)]
pub struct Candidato {
    pub item: String,
    pub peca: String,
    pub ponta: V2,
    pub ponto3: V3,
}

/// A vista de onde saem os balões e as peças dela: a primeira vista de montagem da folha ou, sem
/// ela, a primeira peça estrutural (um balão por item da lista de corte).
pub fn candidatos(st: &mut DwgState) -> Result<CandidatosVista, &'static str> {
    if let Some((folha, c)) = &st.baloes.cache {
        if *folha == st.sheet {
            return Ok(c.clone());
        }
    }
    let c = ler_candidatos(st)?;
    st.baloes.cache = Some((st.sheet, c.clone()));
    Ok(c)
}

fn ler_candidatos(st: &mut DwgState) -> Result<CandidatosVista, &'static str> {
    if let Some(v) = st.d.views.iter().find(|v| v.sheet == st.sheet && v.source.path.to_lowercase().ends_with(".fka")).cloned() {
        let full = st.models.full(&v.source.path);
        let a = std::fs::read_to_string(&full).map_err(|e| e.to_string()).and_then(|t| fenkra_montagem::Assembly::from_json(&t)).map_err(|_| "Não consegui ler a montagem")?;
        let p = st.models.proj(&st.d, v.id);
        let ax = p.axes.ok_or("A vista ainda não tem projeção")?;
        let dir = full.parent().map(|p| p.to_path_buf());
        let mut cache = fenkra_montagem::load::PartCache::default();
        let (lo, hi) = layout::view_rect(&v, &p);
        let itens = fenkra_montagem::bom::bom(&a, dir.as_deref(), fenkra_montagem::bom::BomLevel::Topo);
        let mut out: Vec<Candidato> = vec![];
        for n in &a.nodes {
            let (fenkra_montagem::NodeKind::Part { path } | fenkra_montagem::NodeKind::SubAssembly { path }) = &n.kind else { continue };
            // B102: a mesma peça em duas configurações é dois itens, com dois balões
            let cfg = a.config_de(n.id).to_string();
            let chave = if cfg.is_empty() { path.clone() } else { format!("{path}#{cfg}") };
            if out.iter().any(|c| c.peca == chave) || !fenkra_montagem::view::shown(&a, n.id) {
                continue;
            }
            let fp = fenkra_montagem::load::full_path(dir.as_deref(), path);
            let item = itens.iter().find(|r| r.path == fp && r.config == cfg).map(|r| r.item.clone()).unwrap_or_else(|| (out.len() + 1).to_string());
            let Some(sh) = fenkra_montagem::view::placed_shape(&a, n.id, dir.as_deref(), &mut cache) else { continue };
            let Some((l, h)) = sh.bbox() else { continue };
            let c3 = [0, 1, 2].map(|i| (l[i] + h[i]) / 2.);
            out.push(Candidato { item, peca: chave, ponta: layout::to_sheet(&v, &p, ax.project(c3)), ponto3: c3 });
        }
        if out.is_empty() {
            return Err("A montagem não tem peças visíveis");
        }
        return Ok((v, lo, hi, out));
    }
    let partes: Vec<View> = st.d.views.iter().filter(|v| v.sheet == st.sheet && v.source.path.to_lowercase().ends_with(".fkp")).cloned().collect();
    for v in partes {
        let centros = match &st.models.load(&v.source).model {
            fenkra_desenho::source::Model::Part(doc) => fenkra_solidos::item_centers(doc),
            _ => vec![],
        };
        if centros.is_empty() {
            continue;
        }
        let p = st.models.proj(&st.d, v.id);
        let Some(ax) = p.axes else { continue };
        let (lo, hi) = layout::view_rect(&v, &p);
        let out = centros.into_iter().map(|(item, c3)| Candidato { item: item.to_string(), peca: format!("item {item}"), ponta: layout::to_sheet(&v, &p, ax.project(c3)), ponto3: c3 }).collect();
        return Ok((v, lo, hi, out));
    }
    Err("Coloque antes uma vista da montagem ou da peça estrutural na folha")
}

/// Os balões que o Confirmar vai criar: (candidato, posição).
pub fn previa(st: &mut DwgState) -> Result<(View, Vec<(Candidato, V2)>), &'static str> {
    let (v, lo, hi, cands) = candidatos(st)?;
    let a = &st.baloes;
    let na_janela = |c: &Candidato| a.janela.len() < 2 || baloes::dentro(c.ponta, a.janela[0], a.janela[1]);
    let escolhidos: Vec<Candidato> = cands.into_iter().filter(|c| na_janela(c) && !a.fora.contains(&c.peca)).collect();
    let pontas: Vec<V2> = escolhidos.iter().map(|c| c.ponta).collect();
    let pos = baloes::posicoes(&pontas, lo, hi, a.lado, a.distancia, a.passo);
    Ok((v, escolhidos.into_iter().zip(pos).collect()))
}

/// Clique na folha: canto da janela, ou com Shift, tira ou devolve o balão mais perto.
pub fn clique(st: &mut DwgState, m: V2) {
    if st.baloes.shift {
        let Ok((_, todos)) = candidatos(st).map(|(v, lo, hi, c)| {
            let pontas: Vec<V2> = c.iter().map(|c| c.ponta).collect();
            let pos = baloes::posicoes(&pontas, lo, hi, st.baloes.lado, st.baloes.distancia, st.baloes.passo);
            (v, c.into_iter().zip(pos).collect::<Vec<_>>())
        }) else { return };
        let d = |a: V2, b: V2| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt();
        if let Some((c, _)) = todos.iter().map(|(c, p)| (c, d(*p, m).min(d(c.ponta, m)))).filter(|(_, x)| *x < 8.).min_by(|a, b| a.1.total_cmp(&b.1)) {
            let f = &mut st.baloes.fora;
            if let Some(i) = f.iter().position(|p| *p == c.peca) {
                f.remove(i);
            } else {
                f.push(c.peca.clone());
            }
        }
        return;
    }
    if st.baloes.janela.len() >= 2 {
        st.baloes.janela.clear();
    }
    st.baloes.janela.push(m);
}

/// Cria os balões da prévia. Devolve quantos.
pub fn confirmar(st: &mut DwgState) -> Result<usize, &'static str> {
    let (v, lista) = previa(st)?;
    if lista.is_empty() {
        return Err("Nenhuma peça dentro da janela");
    }
    let n = lista.len();
    for (c, pos) in lista {
        let id = st.d.alloc();
        st.d.notes.push(Note { id, sheet: st.sheet, kind: NoteKind::Balloon, pos, leader: Some(c.ponta), text: c.item, size: st.d.text_height, view: Some(v.id), anchor3: Some(c.ponto3), balao: Some(DwgBalao { peca: c.peca, ..Default::default() }), ..Default::default() });
    }
    st.modified = true;
    st.items = None;
    Ok(n)
}

/// Balões da folha dentro da janela (ou todos, sem janela).
fn baloes_na_janela(st: &DwgState) -> Vec<Id> {
    let j = &st.baloes.janela;
    st.d.notes.iter().filter(|n| n.sheet == st.sheet && n.kind == NoteKind::Balloon && (j.len() < 2 || baloes::dentro(n.pos, j[0], j[1]))).map(|n| n.id).collect()
}

/// Alinha os balões da janela. Devolve quantos se moveram.
pub fn alinhar(st: &mut DwgState, eixo: Eixo) -> usize {
    let ids = baloes_na_janela(st);
    let k = baloes::alinhar(&mut st.d.notes, &ids, eixo);
    if k > 0 {
        st.modified = true;
        st.items = None;
    }
    k
}

/// Atualizar: renumera os balões pela lista de materiais de agora.
pub fn atualizar(st: &mut DwgState) -> Result<usize, &'static str> {
    // a lista pode ter mudado desde que o comando abriu
    st.baloes.cache = None;
    let (_, _, _, cands) = candidatos(st)?;
    let lista: Vec<(String, String)> = cands.into_iter().map(|c| (c.peca, c.item)).collect();
    let k = baloes::renumerar(&mut st.d.notes, &lista);
    if k > 0 {
        st.modified = true;
        st.items = None;
    }
    Ok(k)
}

/// Fantasma: os balões da prévia, a janela e as peças tiradas (só a ponta, riscada).
pub fn fantasma(st: &mut DwgState) -> Vec<layout::Item> {
    let mut out = vec![];
    let Ok((_, lista)) = previa(st) else { return out };
    let mut m = Models::new(None);
    for (c, pos) in lista {
        let n = Note { id: Id(ghost::SCRATCH), sheet: st.sheet, kind: NoteKind::Balloon, pos, leader: Some(c.ponta), text: c.item, size: st.d.text_height, ..Default::default() };
        out.extend(layout::note_items(&st.d, &n, &mut m));
    }
    if let [a, b] = st.baloes.janela[..] {
        let pts = vec![a, [b[0], a[1]], b, [a[0], b[1]], a];
        out.push(layout::Item { owner: None, prim: Prim::Line { pts, style: Style::Hidden }, color: None, width: None, bold: false, alert: false });
    }
    out
}

/// A barra do comando, na folha.
pub fn barra(ctx: &egui::Context, st: &mut DwgState, rect: Rect) -> Option<(String, bool)> {
    let mut aviso = None;
    let painel = crate::paineis::Painel { nome: "dwg_baloes".into(), titulo: tr("Balões automáticos").to_string(), padrao: crate::paineis::abaixo_do_cubo(rect, 280.), largura: 280., redimensiona: false };
    let pos = st.baloes.painel.clone();
    let saida = crate::paineis::mostrar(ctx, rect, &painel, pos, |ui| {
        let a = &mut st.baloes;
        egui::ComboBox::from_label(tr("Lado")).selected_text(tr(a.lado.label())).show_ui(ui, |ui| {
            for l in Lado::TODOS {
                ui.selectable_value(&mut a.lado, l, tr(l.label()));
            }
        }).response.on_hover_text(tr("De que lado da vista os balões ficam. O mais perto escolhe para cada peça o lado mais próximo dela."));
        ui.horizontal(|ui| {
            ui.label(tr("Distância (mm)"));
            ui.add(crate::ui_ext::dv(&mut a.distancia).range(2.0..=80.).speed(0.5)).on_hover_text(tr("Distância entre a borda da vista e o centro dos balões."));
        });
        ui.horizontal(|ui| {
            ui.label(tr("Passo (mm)"));
            ui.add(crate::ui_ext::dv(&mut a.passo).range(4.0..=40.).speed(0.5)).on_hover_text(tr("Espaço mínimo entre dois balões vizinhos na mesma fila."));
        });
        ui.label(if a.janela.len() < 2 { tr("Janela: a vista toda (dois cliques na folha limitam)") } else { tr("Janela: só as peças com a ponta dentro") }).on_hover_text(tr("Clique dois cantos na folha para escolher as peças. Shift+clique num balão da prévia tira ou devolve."));
        if !a.fora.is_empty() {
            ui.label(format!("{}: {}", tr("Tirados com Shift"), a.fora.len()));
        }
        if (a.janela.len() >= 2 || !a.fora.is_empty()) && ui.button(tr("Limpar janela")).clicked() {
            a.janela.clear();
            a.fora.clear();
        }
        ui.separator();
        ui.label(tr("Balões já na janela"));
        ui.horizontal(|ui| {
            if ui.button(tr("Alinhar na horizontal")).on_hover_text(tr("Põe os balões da janela na altura do primeiro.")).clicked() {
                aviso = Some((format!("{}: {}", tr("Balões alinhados"), alinhar(st, Eixo::Horizontal)), false));
            }
            if ui.button(tr("Na vertical")).on_hover_text(tr("Põe os balões da janela na coluna do primeiro.")).clicked() {
                aviso = Some((format!("{}: {}", tr("Balões alinhados"), alinhar(st, Eixo::Vertical)), false));
            }
        });
        if ui.button(tr("Atualizar números")).on_hover_text(tr("Renumera os balões pela ordem atual da lista de materiais.")).clicked() {
            aviso = Some(match atualizar(st) {
                Ok(k) => (format!("{}: {k}", tr("Balões renumerados")), false),
                Err(e) => (tr(e).to_string(), true),
            });
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(tr("Confirmar")).on_hover_text(tr("Cria os balões da prévia (Enter).")).clicked() {
                aviso = Some(terminar(st));
            }
            if ui.button(tr("Cancelar")).clicked() {
                st.tool = None;
            }
        });
    });
    if let Some(p) = saida.movido {
        st.baloes.painel = p;
    }
    aviso
}

/// Confirma e fecha o comando.
pub fn terminar(st: &mut DwgState) -> (String, bool) {
    match confirmar(st) {
        Ok(n) => {
            st.tool = None;
            st.baloes.janela.clear();
            st.baloes.fora.clear();
            (format!("{}: {n}", tr("Balões criados")), false)
        }
        Err(e) => (tr(e).to_string(), true),
    }
}

/// Forma do balão: as molduras que servem a um número de item.
const FORMAS: [(fenkra_core::pmi::PmiFrame, &str); 7] = [
    (fenkra_core::pmi::PmiFrame::Padrao, "Círculo"),
    (fenkra_core::pmi::PmiFrame::Quadrado, "Quadrado"),
    (fenkra_core::pmi::PmiFrame::Triangulo, "Triângulo"),
    (fenkra_core::pmi::PmiFrame::Losango, "Losango"),
    (fenkra_core::pmi::PmiFrame::Caixa, "Retângulo"),
    (fenkra_core::pmi::PmiFrame::CirculoRiscado, "Círculo dividido"),
    (fenkra_core::pmi::PmiFrame::Nenhuma, "Sem moldura"),
];

/// Seção do balão nas propriedades da nota: forma, texto de baixo, quantidade, largura e nós.
pub fn propriedades(ui: &mut egui::Ui, n: &mut Note) {
    let atual = FORMAS.iter().find(|(f, _)| *f == n.style.frame).map(|(_, l)| *l).unwrap_or("Círculo");
    egui::ComboBox::from_label(tr("Forma do balão")).selected_text(tr(atual)).show_ui(ui, |ui| {
        for (f, l) in FORMAS {
            ui.selectable_value(&mut n.style.frame, f, tr(l));
        }
    });
    let b = n.balao.get_or_insert_with(Default::default);
    ui.horizontal(|ui| {
        ui.label(tr("Texto de baixo"));
        ui.text_edit_singleline(&mut b.abaixo).on_hover_text(tr("Com texto aqui o balão se divide: número em cima, este texto embaixo."));
    });
    ui.horizontal(|ui| {
        ui.label(tr("Quantidade"));
        ui.add(egui::TextEdit::singleline(&mut b.quantidade).desired_width(60.).hint_text("2X")).on_hover_text(tr("Escrita ao lado do balão, fora dele."));
    });
    ui.horizontal(|ui| {
        ui.label(tr("Largura fixa (mm)"));
        let r = ui.add(crate::ui_ext::dv(&mut n.style.frame_fixed).range(0.0..=60.).speed(0.2)).on_hover_text(tr("0 ajusta ao texto. Use o mesmo valor em todos para balões do mesmo tamanho."));
        crate::ui_ext::wheel(ui, &r, &mut n.style.frame_fixed);
    });
    // nós das chamadas: dobras no caminho até a ponta
    let pontas = fenkra_desenho::setas_nota::pontas(n).len();
    for k in 0..pontas {
        let nos = n.balao.as_ref().and_then(|b| b.nos.get(k)).map(|v| v.len()).unwrap_or(0);
        ui.horizontal(|ui| {
            ui.label(format!("{} {}: {} {}", tr("Chamada"), k + 1, nos, tr("nós")));
            if ui.small_button("+").on_hover_text(tr("Acrescenta um nó no meio do último trecho")).clicked() {
                if let Some(c) = baloes::caminho(n, k) {
                    let (a, b) = (c[c.len() - 2], c[c.len() - 1]);
                    baloes::adicionar_no(n, k, [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2. + 4.]);
                }
            }
            if nos > 0 && ui.small_button("−").on_hover_text(tr("Tira o último nó")).clicked() {
                baloes::remover_no(n, k, nos - 1);
            }
            if nos > 0 && ui.small_button(tr("Nova chamada")).on_hover_text(tr("Nova chamada a partir do último nó desta")).clicked() {
                if let Some(c) = baloes::caminho(n, k) {
                    let no = c[c.len() - 2];
                    baloes::chamada_do_no(n, k, nos - 1, [no[0] + 10., no[1] - 10.]);
                }
            }
        });
    }
}

/// Desenvolvimento: `FENKRA_DEMO_DWG_BALOES=1` põe na folha de exemplo os balões do B149 (dividido,
/// com quantidade, com nós e duas chamadas, de largura fixa e nas outras formas), uma meia cota, uma
/// cota de arco e uma rugosidade em várias linhas, e seleciona o balão dividido. `auto` arma o comando
/// Balões automáticos. Nada grava no disco.
pub fn demo(st: &mut DwgState) {
    let modo = std::env::var("FENKRA_DEMO_DWG_BALOES").unwrap_or_default();
    if modo.is_empty() {
        return;
    }
    if modo == "auto" {
        st.tool = Some(c::AUTO_BALLOONS);
        return;
    }
    use fenkra_core::pmi::PmiFrame;
    let (sheet, th) = (st.sheet, st.d.text_height);
    let novo = |st: &mut DwgState, pos: V2, ponta: V2, texto: &str, f: &dyn Fn(&mut Note)| {
        let id = st.d.alloc();
        let mut n = Note { id, sheet, kind: NoteKind::Balloon, pos, leader: Some(ponta), text: texto.into(), size: th, ..Default::default() };
        f(&mut n);
        st.d.notes.push(n);
        id
    };
    let dividido = novo(st, [300., 230.], [270., 205.], "1", &|n| n.balao = Some(DwgBalao { abaixo: "A".into(), ..Default::default() }));
    novo(st, [330., 230.], [300., 200.], "2", &|n| n.balao = Some(DwgBalao { quantidade: "4X".into(), ..Default::default() }));
    novo(st, [360., 230.], [320., 190.], "3", &|n| {
        baloes::adicionar_no(n, 0, [360., 205.]);
        baloes::chamada_do_no(n, 0, 0, [340., 185.]);
    });
    novo(st, [390., 230.], [350., 195.], "12", &|n| n.style.frame_fixed = 14.);
    for (k, f) in [PmiFrame::Quadrado, PmiFrame::Triangulo, PmiFrame::Losango, PmiFrame::Caixa].into_iter().enumerate() {
        novo(st, [300. + 30. * k as f64, 265.], [290. + 30. * k as f64, 250.], &(5 + k).to_string(), &|n| n.style.frame = f);
    }
    // rugosidade em várias linhas
    let id = st.d.alloc();
    st.d.notes.push(Note { id, sheet, kind: NoteKind::Roughness, pos: [60., 250.], leader: Some([45., 235.]), text: "Ra 1,6".into(), size: th, rough: Some(fenkra_desenho::simbolos::DwgRough { metodo: "Retificado".into(), outras: vec!["Rz 6,3".into(), "Rmax 10".into()], ..Default::default() }), ..Default::default() });
    // meia cota e cota de arco a partir das cotas da folha, quando houver
    let lineares: Vec<Id> = st.d.dims.iter().filter(|d| matches!(d.kind, DimKind::Horizontal | DimKind::Vertical)).map(|d| d.id).collect();
    if let Some(dm) = lineares.first().and_then(|id| st.d.dims.iter_mut().find(|d| d.id == *id)) {
        dm.opcoes.meia = true;
        dm.opcoes.como_diametro = true;
    }
    if let Some(v) = st.d.views.iter().find(|v| v.sheet == sheet && matches!(v.kind, ViewKind::Standard(Dir::Frente))).cloned() {
        let p = st.models.proj(&st.d, v.id);
        if let Some(ax) = p.axes {
            let (lo, hi) = (p.lo, p.hi);
            let c = [lo[0], lo[1]];
            let r = ((hi[0] - lo[0]).min(hi[1] - lo[1])) * 0.6;
            let id = st.d.alloc();
            let mut dm = Dim { id, view: v.id, kind: DimKind::Angle, anchors: vec![ax.unproject(c), ax.unproject([c[0] + r, c[1]]), ax.unproject([c[0] + r * 0.7071, c[1] + r * 0.7071])], ..Default::default() };
            dm.opcoes.arco = true;
            st.d.dims.push(dm);
        }
    }
    st.sel = Some(dividido);
    st.items = None;
}
