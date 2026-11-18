// SPDX-License-Identifier: LGPL-2.1-or-later
//! B505: comandos pelo elemento que resulta. A barra tem um botão só para Ponto, Linha, Plano, Curva e
//! Superfície; o tipo (coordenadas, sobre curva, por três pontos, por offset...) é escolhido dentro do
//! painel, que troca os campos conforme o tipo. O botão lembra o último tipo usado.
//!
//! Interseção, projeção, extração e junção ficam à parte, como operações. Quando a operação dá vários
//! pedaços, o painel diz quantos são e oferece "mais perto", "mais longe" e "extrair todos": nada é
//! escolhido em silêncio.
//!
//! Repetir pontos é uma função do Ponto (quantidade ou espaçamento ao longo de uma curva).
//!
//! Compatibilidade: as famílias antigas da barra (`fam:wf_ponto`, `fam:sup_basica`...) e os botões
//! soltos que viraram tipo (`Repetir pontos`, `Polilinha`...) são levados para o botão novo por
//! `chave_atual`, nas caixas próprias, nos ocultos e na ordem gravada. Nenhum tipo saiu do programa e
//! os arquivos antigos abrem iguais: o documento guarda o tipo (`RefKind`), não o botão.

use crate::app::{App, Cmd};
use crate::geo_ui::{self, GeoKind, Seed};
use crate::i18n::tr;
use crate::toolbar::{Act, Entry, Item};
use eframe::egui::{self, RichText};
use fenkra_core::{CurveRef, Document, Id, RefKind};

type V3 = [f64; 3];

/// Elemento da barra: chave da família, tipos, ícone, nome, frase e passo a passo.
pub struct Elemento {
    pub chave: &'static str,
    pub tipos: &'static [GeoKind],
    pub icone: &'static str,
    pub nome: &'static str,
    pub frase: &'static str,
    pub passos: &'static str,
}

/// Os cinco elementos, na ordem da barra.
pub const ELEMENTOS: [Elemento; 5] = [
    Elemento { chave: "el_ponto", tipos: &geo_ui::POINTS, icone: "ponto_coord", nome: "Ponto", frase: "Ponto: o tipo é escolhido no painel", passos: "Crie o ponto e escolha o tipo no painel: coordenadas, sobre curva, sobre plano, sobre superfície, centro, tangente, entre dois pontos ou deslocado. O painel também repete pontos ao longo de uma curva." },
    Elemento { chave: "el_linha", tipos: &geo_ui::LINES, icone: "linha_pontos", nome: "Linha", frase: "Linha: o tipo é escolhido no painel", passos: "Crie a linha e escolha o tipo no painel: dois pontos, ponto e direção, ângulo, tangente, normal, entre planos, bissetriz, eixo ou polilinha." },
    Elemento { chave: "el_plano", tipos: &geo_ui::PLANES, icone: "plano", nome: "Plano", frase: "Plano: o tipo é escolhido no painel", passos: "Crie o plano e escolha o tipo no painel: offset, paralelo por ponto, ângulo ou normal, três pontos, duas linhas, ponto e linha, pela curva, normal à curva, tangente, equação ou médio." },
    Elemento { chave: "el_curva", tipos: &geo_ui::CURVES, icone: "spline", nome: "Curva", frase: "Curva: o tipo é escolhido no painel", passos: "Crie a curva e escolha o tipo no painel: círculo, círculo por 3 pontos, cônica, spline, spline por controle, conexão, canto, hélice, espiral, espinha ou paralela." },
    Elemento { chave: "el_superficie", tipos: &geo_ui::SURFACES, icone: "sup_extrudada", nome: "Superfície", frase: "Superfície: o tipo é escolhido no painel", passos: "Crie a superfície e escolha o tipo no painel: extrudada, revolução, esfera, cilindro, deslocada, varredura, tubo, multi-seções, regrada, concordância, conectar ou preenchimento." },
];

/// Operações à parte (com as variantes na seta, porque o tipo depende do que se cruza).
pub const OPERACOES: [(&str, &[GeoKind]); 4] = [("op_intersecao", &geo_ui::INTERSECOES), ("op_projecao", &geo_ui::PROJECOES), ("op_extrair", &geo_ui::EXTRACOES), ("op_juntar", &geo_ui::JUNCOES)];

/// A chave é de um elemento (botão sem seta: o tipo fica no painel)?
pub fn e_elemento(chave: &str) -> bool {
    ELEMENTOS.iter().any(|e| e.chave == chave)
}

pub fn elemento(chave: &str) -> Option<&'static Elemento> {
    ELEMENTOS.iter().find(|e| e.chave == chave)
}

fn item_geo(k: GeoKind) -> Item {
    let (i, n, b, a) = k.info();
    Item { icon: i, name: n, brief: b, advanced: a, act: Act::Cmd(Cmd::Geo(k)) }
}

/// Item de Repetir pontos: função do Ponto, no fim da lista de tipos dele.
pub const REPETIR: Item = Item {
    icon: "wf_repetir",
    name: "Repetir pontos",
    brief: "Repetir pontos na curva",
    advanced: "Selecione uma curva e crie N pontos associativos ao longo dela (por quantidade ou espaçamento), com planos normais opcionais em cada ponto.",
    act: Act::Cmd(Cmd::PointRepeat),
};

/// Item de Extrair a seleção (faces, arestas e vértices), primeiro da operação Extrair.
pub const EXTRAIR: Item = Item {
    icon: "extrair",
    name: "Extrair",
    brief: "Extrair a seleção (faces, arestas, vértices)",
    advanced: "Faces, arestas e vértices viram superfície, curva ou ponto de referência. Simples = um elemento por item; múltipla = um só. Propagação por ponto, tangente ou curvatura; o resultado pode ser publicado.",
    act: Act::Cmd(Cmd::Extract),
};

/// Entrada da barra do elemento (um botão; os tipos ficam no painel).
pub fn entrada_do_elemento(chave: &str) -> Entry {
    let e = elemento(chave).expect("elemento conhecido");
    let mut v: Vec<Item> = e.tipos.iter().map(|k| item_geo(*k)).collect();
    if e.chave == "el_ponto" {
        v.push(REPETIR);
    }
    Entry::Family(e.chave, v)
}

/// Entrada da barra da operação (botão com as variantes na seta).
pub fn entrada_da_operacao(chave: &str) -> Entry {
    let &(k, tipos) = OPERACOES.iter().find(|(k, _)| *k == chave).expect("operação conhecida");
    let mut v: Vec<Item> = vec![];
    if k == "op_extrair" {
        v.push(EXTRAIR);
    }
    v.extend(tipos.iter().map(|t| item_geo(*t)));
    Entry::Family(k, v)
}

/// Família da barra e posição do tipo nela (para o botão lembrar o tipo escolhido no painel).
pub fn familia_do_tipo(k: GeoKind) -> Option<(&'static str, usize)> {
    for e in &ELEMENTOS {
        if let Some(i) = e.tipos.iter().position(|x| *x == k) {
            return Some((e.chave, i));
        }
    }
    for &(c, tipos) in &OPERACOES {
        if let Some(i) = tipos.iter().position(|x| *x == k) {
            return Some((c, i + usize::from(c == "op_extrair")));
        }
    }
    None
}

fn chave_solta(k: GeoKind) -> String {
    let (i, n, _, _) = k.info();
    format!("{i}|{n}")
}

/// Chave atual de um comando gravado numa versão anterior (caixa própria, ocultos, ordem). A chave que
/// já é atual volta igual.
pub fn chave_atual(k: &str) -> String {
    let fam = match k {
        "fam:plano" | "fam:wf_plano" => "el_plano",
        "fam:ponto" | "fam:wf_ponto" | "wf_repetir|Repetir pontos" => "el_ponto",
        "fam:linha" | "fam:wf_linha" => "el_linha",
        "fam:curva" | "fam:wf_circulo" | "fam:wf_spline" | "fam:wf_helice" => "el_curva",
        "fam:superficie" | "fam:sup_basica" | "fam:sup_varrer" | "fam:sup_multi" => "el_superficie",
        "fam:wf_projecao" => "op_projecao",
        "extrair|Extrair" => "op_extrair",
        _ => {
            let soltos: [(GeoKind, &'static str); 6] = [
                (GeoKind::Polyline, "el_linha"),
                (GeoKind::CurveParallel, "el_curva"),
                (GeoKind::SurfOffset, "el_superficie"),
                (GeoKind::SurfFill, "el_superficie"),
                (GeoKind::CurveIntersect, "op_intersecao"),
                (GeoKind::CurveJoin, "op_juntar"),
            ];
            match soltos.iter().find(|(g, _)| chave_solta(*g) == k) {
                Some(&(_, f)) => f,
                None => return k.to_string(),
            }
        }
    };
    format!("fam:{fam}")
}

/// Leva uma lista de chaves gravadas para as atuais, sem repetir (duas famílias antigas viram uma).
pub fn migrar_lista(itens: &mut Vec<String>) {
    let mut out: Vec<String> = vec![];
    for k in itens.drain(..) {
        let n = chave_atual(&k);
        if !out.contains(&n) {
            out.push(n);
        }
    }
    *itens = out;
}

// ---------------------------------------------------------------- pedidos do painel

fn id_tipo() -> egui::Id {
    egui::Id::new("b505_tipo_escolhido")
}
fn id_repetir() -> egui::Id {
    egui::Id::new("b505_repetir")
}
fn id_extrair() -> egui::Id {
    egui::Id::new("b505_extrair")
}

/// O painel trocou o tipo: o botão do elemento passa a abrir esse tipo.
pub fn tipo_escolhido(ctx: &egui::Context, k: GeoKind) {
    ctx.data_mut(|d| d.insert_temp(id_tipo(), Some(k)));
}

/// Curva a usar para repetir pontos: a do próprio ponto (ponto na curva) ou a da seleção.
pub fn curva_para_repetir(kind: &RefKind, seed: &Seed) -> Option<CurveRef> {
    match kind {
        RefKind::PointOnCurve { curve, .. } if !matches!(curve, CurveRef::Ref(id) if crate::slots::is_unset(*id)) => Some(curve.clone()),
        _ => seed.curves.first().cloned(),
    }
}

/// Botão "Repetir na curva" do painel do Ponto.
pub fn botao_repetir(ui: &mut egui::Ui, kind: &RefKind, seed: &Seed) {
    let curva = curva_para_repetir(kind, seed);
    let r = ui.add_enabled(curva.is_some(), egui::Button::new(tr("⋯ Repetir pontos na curva")));
    let r = if curva.is_some() { r.on_hover_text(tr("Cria vários pontos ao longo da curva, por quantidade ou espaçamento.")) } else { r.on_disabled_hover_text(tr("Use o tipo Na curva ou selecione uma curva para repetir pontos ao longo dela.")) };
    if r.clicked() {
        if let Some(c) = curva {
            ui.ctx().data_mut(|d| d.insert_temp(id_repetir(), Some(c)));
        }
    }
}

/// Pedaço mais perto (ou mais longe) de `referencia`: (número da solução, ponto dela).
pub fn escolher_pedaco(solucoes: &[(usize, V3)], referencia: V3, longe: bool) -> Option<(usize, V3)> {
    let d = |p: &V3| (0..3).map(|i| (p[i] - referencia[i]).powi(2)).sum::<f64>();
    let it = solucoes.iter().copied();
    if longe { it.max_by(|a, b| d(&a.1).total_cmp(&d(&b.1))) } else { it.min_by(|a, b| d(&a.1).total_cmp(&d(&b.1))) }
}

/// Grava a escolha do pedaço no tipo: o número e, quando o tipo tem âncora, o ponto dele (a ordem dos
/// pedaços é do kernel e muda com o modelo; a âncora não).
pub fn fixar_pedaco(kind: &mut RefKind, num: usize, p: V3) -> bool {
    match kind {
        RefKind::CurveIntersect { which, near, .. } => {
            *which = num;
            *near = Some(p);
            true
        }
        RefKind::PointCurvePlane { which, .. } => {
            *which = num;
            true
        }
        _ => false,
    }
}

/// Extrair todos: o elemento fica com o primeiro pedaço e cada outro vira um elemento próprio, na
/// mesma pasta. Devolve os novos.
pub fn extrair_pedacos(d: &mut Document, id: Id, solucoes: &[(usize, V3)]) -> Vec<Id> {
    let Some(r) = d.reference(id).cloned() else { return vec![] };
    let mut novos = vec![];
    for (i, (num, p)) in solucoes.iter().enumerate() {
        let mut k = r.kind.clone();
        if !fixar_pedaco(&mut k, *num, *p) {
            return novos;
        }
        if i == 0 {
            if let Some(x) = d.references.iter_mut().find(|x| x.id == id) {
                x.kind = k;
            }
            continue;
        }
        let n = d.add_geometry(k);
        if let Some(x) = d.references.iter_mut().find(|x| x.id == n) {
            x.folder = r.folder;
        }
        novos.push(n);
    }
    novos
}

/// Painel da operação com vários pedaços: quantos são, qual está em uso e as três saídas.
pub fn pedacos_ui(ui: &mut egui::Ui, me: Id, note: Option<&fenkra_solidos::RefNote>, which: &mut usize, near: Option<&mut Option<V3>>) {
    let Some(n) = note.filter(|n| n.solucoes.len() > 1) else { return };
    let total = n.solucoes.len();
    let em_uso = n.solucao.map_or_else(|| "?".to_string(), |k| (k + 1).to_string());
    ui.label(RichText::new(format!("{total} {} · {} {em_uso}", tr("pedaços"), tr("em uso:"))).color(ui.visuals().warn_fg_color));
    ui.label(RichText::new(tr("Escolha qual pedaço fica ou extraia todos; nada é escolhido em silêncio.")).weak().small());
    let mut escolha = None;
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Mais perto da origem")).on_hover_text(tr("Fica o pedaço mais perto da origem da peça; a escolha fica presa à posição.")).clicked() {
            escolha = escolher_pedaco(&n.solucoes, [0.; 3], false);
        }
        if ui.button(tr("Mais longe da origem")).on_hover_text(tr("Fica o pedaço mais longe da origem da peça; a escolha fica presa à posição.")).clicked() {
            escolha = escolher_pedaco(&n.solucoes, [0.; 3], true);
        }
        if ui.button(tr("Extrair todos")).on_hover_text(tr("Cada pedaço vira um elemento próprio.")).clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(id_extrair(), Some((me, n.solucoes.clone()))));
        }
    });
    if let Some((num, p)) = escolha {
        *which = num;
        if let Some(nr) = near {
            *nr = Some(p);
        }
    }
}

impl App {
    /// Pedidos do painel dos elementos (B505): lembrar o tipo, repetir pontos e extrair pedaços.
    pub(crate) fn pedidos_dos_elementos(&mut self, ctx: &egui::Context) {
        if let Some(k) = ctx.data_mut(|d| d.remove_temp::<Option<GeoKind>>(id_tipo())).flatten() {
            if let Some((fam, i)) = familia_do_tipo(k) {
                self.settings.set_family(fam, i);
            }
        }
        if let Some(c) = ctx.data_mut(|d| d.remove_temp::<Option<CurveRef>>(id_repetir())).flatten() {
            self.settings.set_family("el_ponto", geo_ui::POINTS.len());
            self.repeat = Some(crate::app::Repeat { curve: c, count: "5".into(), spacing: "10".into(), by_spacing: false, ends: true, planes: false });
        }
        if let Some((id, sol)) = ctx.data_mut(|d| d.remove_temp::<Option<(Id, Vec<(usize, V3)>)>>(id_extrair())).flatten() {
            let n = extrair_pedacos(&mut self.open.doc, id, &sol);
            self.scene_dirty = true;
            self.toast(format!("{} {}", n.len() + 1, tr("pedaços viraram elementos")), false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Bench, Group, Settings};
    use crate::toolbar::{self, entry_key};
    use fenkra_core::{OriginPlane, PlaneRef, PointRef, SurfRef};

    /// Tipos que a barra alcança em todas as bancadas que têm arame (Superfícies e as que têm Referências).
    fn tipos_na_barra(grupos: &[Group]) -> Vec<GeoKind> {
        let mut v = vec![];
        for g in grupos {
            for e in toolbar::entries(*g) {
                let itens = match e {
                    Entry::One(i) => vec![i],
                    Entry::Family(_, v) => v,
                };
                for i in itens {
                    if let Act::Cmd(Cmd::Geo(k)) = i.act {
                        v.push(k);
                    }
                }
            }
        }
        v
    }

    /// B505: cada bancada de arame tem um ícone por elemento, e todos os tipos antigos continuam
    /// alcançáveis pelo painel (o Tipo do painel do elemento ou da operação).
    #[test]
    fn um_icone_por_elemento_e_tipos_alcancaveis() {
        // Superfícies: arame + superfícies + operações; Sólidos (e as outras): Referências
        let bancadas: [(&str, Vec<Group>); 2] = [("superficies", vec![Group::Wireframe, Group::Superficies, Group::Operacoes]), ("referencias", vec![Group::Referencias])];
        for (nome, grupos) in &bancadas {
            let entradas: Vec<Entry> = grupos.iter().flat_map(|g| toolbar::entries(*g)).collect();
            for e in &ELEMENTOS {
                let k = format!("fam:{}", e.chave);
                assert_eq!(entradas.iter().filter(|x| entry_key(x) == k).count(), 1, "{nome}: {} uma vez", e.nome);
            }
            // tipo que cria um elemento só mora no botão daquele elemento (nenhum ícone solto nem em outra seta)
            for x in &entradas {
                let itens = match x {
                    Entry::One(i) => vec![*i],
                    Entry::Family(_, v) => v.clone(),
                };
                for i in itens {
                    let Act::Cmd(Cmd::Geo(k)) = i.act else { continue };
                    if let Some(el) = ELEMENTOS.iter().find(|e| e.tipos.contains(&k)) {
                        assert_eq!(entry_key(x), format!("fam:{}", el.chave), "{nome}: {k:?} fora do botão {}", el.nome);
                    }
                }
            }
        }
        // todo tipo de criação ou operação está no Tipo do painel do seu botão, sem trocar de classe
        let todos: Vec<GeoKind> = geo_ui::FAMILIAS.iter().flat_map(|f| f.iter().copied()).collect();
        for k in &todos {
            let n = geo_ui::FAMILIAS.iter().filter(|f| f.contains(k)).count();
            assert_eq!(n, 1, "{k:?} em {n} famílias");
            let painel = geo_ui::tipos_do_painel(*k);
            assert!(painel.contains(k), "{k:?} fora do próprio painel");
            if !geo_ui::TRANSFORMS.contains(k) {
                assert!(painel.iter().all(|g| g.classe() == k.classe()), "{k:?}: o painel troca a classe");
            }
        }
        // os tipos da barra de arame antes do B505 continuam na barra (no botão ou na seta da operação)
        let antes = [
            GeoKind::PtCoords, GeoKind::PtOnCurve, GeoKind::PtOnSurface, GeoKind::PtOnPlane, GeoKind::PtCenter, GeoKind::PtBetween, GeoKind::PtProject, GeoKind::PtCurvePlane, GeoKind::PtCurveCurve, GeoKind::PtOffset, GeoKind::PtTangent, GeoKind::PtProjectCurve,
            GeoKind::LinePts, GeoKind::LineDir, GeoKind::LineNormal, GeoKind::LineAngle, GeoKind::LineTangent, GeoKind::LinePlanes, GeoKind::LineBisector, GeoKind::LineAxis, GeoKind::Polyline, GeoKind::CurveJoin,
            GeoKind::Circle, GeoKind::Circle3, GeoKind::Conic, GeoKind::Spline, GeoKind::SplineControl, GeoKind::Connect, GeoKind::Corner, GeoKind::Helix, GeoKind::Spiral, GeoKind::Spine,
            GeoKind::CurveProjSurf, GeoKind::CurveProject, GeoKind::CurveIntersect, GeoKind::CurveParallel,
            GeoKind::SurfExtrude, GeoKind::SurfRevolve, GeoKind::SurfSphere, GeoKind::SurfCylinder, GeoKind::SurfOffset, GeoKind::SurfSweep, GeoKind::SurfSweepX, GeoKind::SurfTube, GeoKind::SurfLoft, GeoKind::SurfRuled, GeoKind::SurfBlend, GeoKind::SurfConnect, GeoKind::SurfFill,
        ];
        let sup = tipos_na_barra(&[Group::Wireframe, Group::Superficies, Group::Operacoes]);
        let refs = tipos_na_barra(&[Group::Referencias]);
        for k in antes.iter().chain(&geo_ui::PLANES) {
            assert!(sup.contains(k), "Superfícies: {k:?} sumiu");
            assert!(refs.contains(k), "Referências: {k:?} sumiu");
        }
        let _ = Bench::Superficies;
    }

    /// B505: caixa própria, ocultos e ordem com chaves antigas vão para o botão novo, sem repetir.
    #[test]
    fn chaves_antigas_vao_para_o_botao_novo() {
        let polilinha = chave_solta(GeoKind::Polyline);
        let mut itens = vec!["fam:wf_ponto".to_string(), "wf_repetir|Repetir pontos".into(), polilinha, "fam:wf_linha".into(), "fam:sup_basica".into(), "fam:sup_varrer".into(), "fam:wf_projecao".into(), "fam:extrusao".into()];
        migrar_lista(&mut itens);
        assert_eq!(itens, ["fam:el_ponto", "fam:el_linha", "fam:el_superficie", "fam:op_projecao", "fam:extrusao"]);
        // toda chave migrada existe na barra: a caixa própria não perde o botão
        let todas: Vec<String> = toolbar::all_entries().iter().map(entry_key).collect();
        for k in &itens {
            assert!(todas.contains(k), "{k} não existe mais");
        }
        // pelas configurações: caixa, ocultos e ordem
        let mut s = Settings::default();
        s.custom.groups.push(crate::settings::CustomGroup { name: "Minha".into(), items: vec!["fam:wf_plano".into(), "fam:plano".into()], ..Default::default() });
        s.custom.hidden.push("fam:wf_circulo".into());
        s.custom.order.push((Group::Wireframe, vec!["fam:wf_plano".into(), "fam:wf_ponto".into()]));
        let s = s.migrar_comandos();
        assert_eq!(s.custom.groups[0].items, ["fam:el_plano"]);
        assert_eq!(s.custom.hidden, ["fam:el_curva"]);
        assert_eq!(s.custom.order[0].1, ["fam:el_plano", "fam:el_ponto"]);
        // e a caixa desenha o botão (não some)
        let list = toolbar::itens_da_caixa(&s.custom.groups[0].items);
        assert_eq!(list.len(), 1);
    }

    /// B505: o botão lembra o tipo escolhido no painel; repetir pontos é o último tipo do Ponto.
    #[test]
    fn botao_lembra_o_tipo_do_painel() {
        assert_eq!(familia_do_tipo(GeoKind::PtOnCurve), Some(("el_ponto", 1)));
        assert_eq!(familia_do_tipo(GeoKind::Plane3P).map(|x| x.0), Some("el_plano"));
        assert_eq!(familia_do_tipo(GeoKind::SurfExtract), Some(("op_extrair", 2)));
        let Entry::Family(_, v) = entrada_do_elemento("el_ponto") else { panic!() };
        assert_eq!(v[geo_ui::POINTS.len()].act, Act::Cmd(Cmd::PointRepeat));
        let Entry::Family(_, v) = entrada_da_operacao("op_extrair") else { panic!() };
        assert_eq!(v[2].act, Act::Cmd(Cmd::Geo(GeoKind::SurfExtract)));
    }

    /// B505 (R5.8): interseção de plano com cilindro dá dois pedaços; perto e longe escolhem o
    /// esperado e extrair gera um elemento por pedaço. Arquivo antigo (sem âncora) abre igual.
    #[test]
    fn pedacos_perto_longe_e_extrair() {
        let sol = vec![(0usize, [10., 0., 5.]), (1usize, [-30., 0., 5.])];
        assert_eq!(escolher_pedaco(&sol, [0.; 3], false).map(|x| x.0), Some(0));
        assert_eq!(escolher_pedaco(&sol, [0.; 3], true).map(|x| x.0), Some(1));
        assert_eq!(escolher_pedaco(&sol, [-25., 0., 0.], false).map(|x| x.0), Some(1));
        let mut d = Document::new("pedacos");
        let cil = d.add_geometry(RefKind::SurfCylinder { point: PointRef::Origin, dir: fenkra_core::AxisRef::Origin(fenkra_core::OriginAxis::Z), radius: "10".into(), start: "0".into(), end: "20".into() });
        let id = d.add_geometry(RefKind::CurveIntersect { a: SurfRef::Ref(cil), b: SurfRef::Plane(PlaneRef::Origin(OriginPlane::YZ)), which: 0, near: None });
        let antes = d.references.len();
        let novos = extrair_pedacos(&mut d, id, &sol);
        assert_eq!(novos.len(), 1);
        assert_eq!(d.references.len(), antes + 1);
        assert!(matches!(d.reference(id).unwrap().kind, RefKind::CurveIntersect { which: 0, near: Some(p), .. } if p == [10., 0., 5.]));
        assert!(matches!(d.reference(novos[0]).unwrap().kind, RefKind::CurveIntersect { which: 1, near: Some(p), .. } if p == [-30., 0., 5.]));
        // o documento com o tipo antigo continua lido igual: o tipo é do documento, não do botão
        let json = serde_json::to_string(&d).unwrap();
        let de_novo: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(de_novo.references, d.references);
        assert_eq!(GeoKind::of(&d.reference(id).unwrap().kind), Some(GeoKind::CurveIntersect));
    }
}
