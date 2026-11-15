// SPDX-License-Identifier: LGPL-2.1-or-later
//! B441: no lugar de ferramenta apagada sem aviso, o motivo e o passo que falta; depois de cada
//! comando, os próximos mais prováveis no fluxo da bancada.
//!
//! - **Falta um passo** (`falta`): quando o documento ainda não tem o que o comando usa (chapa sem
//!   parâmetros ou sem parede base, acabamento sem corpo, extrusão sem sketch, ligação sem membro,
//!   restrição com menos de duas peças, cota sem vista), o botão da barra fica esmaecido, mas continua
//!   clicável: o clique abre um quadro pequeno com o motivo, o botão do passo que falta e "usar mesmo
//!   assim" (o comando roda como antes, para quem sabe o que está fazendo). Nada fica apagado sem
//!   explicação.
//! - **Próximos** (`proximos`): o último comando pedido fica guardado; quando ele termina (nenhum
//!   comando nem sketch aberto e o documento ganhou um item, ou na montagem e no desenho, que não
//!   contam itens da peça), a barra de baixo mostra "Próximo:" com até três ícones do fluxo da
//!   bancada. Começar outro comando troca a sugestão.
//!
//! O estado do documento chega pronto em `Contexto`, para as regras serem testadas sem a janela.

use crate::app::Cmd;
use crate::i18n::tr;
use crate::settings::Bench;
use eframe::egui;
use crate::toolbar::Act;
use fenkra_core::FeatureKind as K;
use std::sync::Mutex;

/// O que o documento aberto tem, no que importa para as regras.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Contexto {
    pub bench: Bench,
    pub itens: usize,
    pub sketches: usize,
    pub corpos: usize,
    pub linhas_ref: usize,
    pub chapa_params: bool,
    pub chapa_parede: bool,
    pub membros: usize,
    pub pecas: usize,
    pub desenho_aberto: bool,
    pub vistas: usize,
    /// há comando ou sketch em edição
    pub ocupado: bool,
}

/// Monta o contexto a partir do app.
pub fn contexto(app: &crate::app::App) -> Contexto {
    let d = &app.open.doc;
    let conta = |f: &dyn Fn(&K) -> bool| d.features.iter().filter(|x| f(&x.kind)).count();
    Contexto {
        bench: app.settings.bench,
        itens: d.features.len(),
        sketches: conta(&|k| matches!(k, K::Sketch(_))),
        corpos: app.result.as_ref().map_or(0, |r| r.bodies.len()),
        linhas_ref: d.references.len(),
        chapa_params: d.sheet.is_some(),
        chapa_parede: conta(&|k| matches!(k, K::SmWall(_) | K::SmConvert(_) | K::SmLoftFlange(_) | K::SmProfileFlange(_))) > 0,
        membros: conta(&|k| matches!(k, K::StructMember(_))),
        pecas: app.asm.as_ref().map_or(0, |s| s.asm.nodes.len()),
        desenho_aberto: app.dwg.is_some(),
        vistas: app.dwg.as_ref().map_or(0, |s| s.d.views.len()),
        ocupado: app.session.is_some() || app.sketch.is_some(),
    }
}

/// O que falta para o comando rodar: o motivo e o passo, com o nome do botão do passo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Falta {
    pub motivo: &'static str,
    pub passo: Act,
    pub rotulo: &'static str,
}

const fn f(motivo: &'static str, passo: Cmd, rotulo: &'static str) -> Option<Falta> {
    Some(Falta { motivo, passo: Act::Cmd(passo), rotulo })
}

/// Por que o comando ainda não pode rodar e qual é o passo que falta; `None` quando pode.
pub fn falta(a: Act, c: &Contexto) -> Option<Falta> {
    use Cmd::*;
    let Act::Cmd(cmd) = a else { return None };
    let sketch_ou_corpo = |c: &Contexto| if c.sketches > 0 { f("Ainda não há corpo: este comando trabalha sobre um sólido.", Extrude, "Extrusão") } else { f("Ainda não há corpo: este comando trabalha sobre um sólido, e o sólido nasce de um perfil num sketch.", Sketch, "Novo sketch") };
    match cmd {
        // desenho: primeiro o desenho, depois as vistas
        Dwg(0) | Dwg(1) => None,
        Dwg(_) if !c.desenho_aberto => f("Nenhum desenho aberto: as vistas, as cotas e as tabelas moram numa folha.", Dwg(0), "Novo desenho"),
        Dwg(12..=14) | Dwg(20..=29) | Dwg(31..=37) | Dwg(40..=45) if c.vistas == 0 => f("A folha ainda não tem vistas: este comando parte de uma vista da peça.", Dwg(10), "3 vistas"),
        // chapa: parâmetros, depois a parede base, depois o resto
        SmFlange | SmHem | SmBend | SmJoggle | SmStamp | SmFlangedHole | SmLouver | SmCut | SmCornerRelief | SmCornerClose | SmArcSlot | SmUnfold | SmUnfoldPartial(_) | SmCurveFlange | SmFlangeLoop | SmSurfFlange | SmCone if !c.chapa_params => {
            f("A peça ainda não tem parâmetros de chapa (espessura, raio de dobra e fator K).", SmParams, "Parâmetros da chapa")
        }
        SmFlange | SmHem | SmBend | SmJoggle | SmStamp | SmFlangedHole | SmLouver | SmCut | SmCornerRelief | SmCornerClose | SmArcSlot | SmUnfold | SmUnfoldPartial(_) | SmCurveFlange | SmFlangeLoop | SmSurfFlange if !c.chapa_parede => {
            if c.sketches > 0 {
                f("Falta a parede base: a aba, a dobra e o recorte partem dela.", SmWall, "Parede base")
            } else {
                f("Falta a parede base, e ela nasce de um perfil num sketch.", Sketch, "Novo sketch")
            }
        }
        SmWall | SmProfileFlange if c.sketches == 0 => f("A parede base nasce de um perfil num sketch, e a peça ainda não tem sketch.", Sketch, "Novo sketch"),
        // sólidos: recursos de sketch pedem um sketch; remover pede material
        Extrude | ExtrudeMid | ExtrudeUpTo | ExtrudeDraftFillet | Revolve | Rib | Loft | Stiffener if c.sketches == 0 => {
            f("Este comando parte de um perfil num sketch, e a peça ainda não tem sketch.", Sketch, "Novo sketch")
        }
        Cut | CutBlind | CutUpTo | Groove | Slot | LoftCut if c.corpos == 0 => sketch_ou_corpo(c),
        Fillet | FilletAll | FilletVar | FilletFaceFace | FilletFull | Chamfer | ChamferAll | ChamferTwoDist | ChamferDistAngle | Shell | Draft | Thickness | Thread | PatternLinear | PatternCircular | PatternPoints | Mirror | Scale | MoveBody(_) | DirectEdit(_) | SplitBody
            if c.corpos == 0 && c.bench != Bench::Chapa =>
        {
            sketch_ou_corpo(c)
        }
        Combine(_) if c.corpos < 2 => f("A booleana combina dois corpos, e a peça tem menos de dois.", Collector, "Novo corpo"),
        // estrutural: membros nascem de linhas; ligações, cantos e saídas, de membros
        StMember if c.sketches == 0 && c.linhas_ref == 0 => f("O membro segue linhas: desenhe o arame 3D ou um sketch com as linhas da estrutura.", Geo(crate::geo_ui::GeoKind::LinePts), "Linha"),
        StPlate | StGusset | StWeld | StCorners | StCutList | StAssembly if c.membros == 0 => f("Ainda não há membros estruturais: placas, reforços, soldas, cantos e a lista de corte partem deles.", StMember, "Membro"),
        // montagem: restrições e juntas ligam duas peças
        AsmMate(_) | AsmRigid | AsmInterf | AsmJoint(_) | AsmAutoJoints | AsmSimulate | AsmFeat(_) | AsmChaveta | AsmFasteners if c.pecas < 2 => {
            f("Restrições, juntas e recursos de montagem ligam duas peças, e a montagem tem menos de duas.", AsmInsert, "Inserir peça")
        }
        AsmFix | AsmNumber | AsmBom | AsmReplace | AsmPattern | AsmSolve if c.pecas == 0 => f("A montagem ainda não tem peças.", AsmInsert, "Inserir peça"),
        _ => None,
    }
}

/// Próximos comandos mais prováveis depois de `ultimo`, no fluxo da bancada (até três).
pub fn proximos(b: Bench, ultimo: Cmd) -> Vec<Act> {
    use crate::asm_ui::MateCmd as M;
    use Cmd::*;
    let v: &[Cmd] = match (b, ultimo) {
        (Bench::Chapa, SmParams) => &[Sketch, SmWall],
        (Bench::Chapa, Sketch) => &[SmWall, SmCut, SmBend],
        (Bench::Chapa, SmWall | SmConvert | SmLoftFlange | SmProfileFlange) => &[SmFlange, SmCut, SmUnfold],
        (Bench::Chapa, SmFlange | SmHem) => &[SmFlange, SmCornerRelief, SmUnfold],
        (Bench::Chapa, SmBend | SmJoggle) => &[SmFlange, SmCut, SmUnfold],
        (Bench::Chapa, SmCut | Hole | SmStamp | SmFlangedHole | SmLouver | SmArcSlot) => &[SmCut, SmFlange, SmUnfold],
        (Bench::Chapa, SmCornerRelief | SmCornerClose) => &[SmUnfold, SmCut],
        (Bench::Chapa, SmUnfold | SmUnfoldPartial(_)) => &[Dwg(0), SmUnfold],
        (Bench::Estrutural, Sketch | Geo(_)) => &[StMember, Geo(crate::geo_ui::GeoKind::LinePts)],
        (Bench::Estrutural, StMember) => &[StCorners, StPlate, StGusset],
        (Bench::Estrutural, StCorners) => &[StPlate, StWeld, StCutList],
        (Bench::Estrutural, StPlate | StGusset) => &[StWeld, StCutList],
        (Bench::Estrutural, StWeld) => &[StCutList, StAssembly],
        (Bench::Estrutural, StCutList) => &[StAssembly, Dwg(0)],
        (Bench::Montagem, AsmNew | AsmOpen) => &[AsmInsert, AsmNewPart],
        (Bench::Montagem, AsmInsert | AsmSubAsm | AsmNewPart) => &[AsmMate(M::Auto), AsmMate(M::Coincident), AsmInsert],
        (Bench::Montagem, AsmMate(_) | AsmFix | AsmRigid) => &[AsmMate(M::Auto), AsmSolve, AsmInterf],
        (Bench::Montagem, AsmSolve | AsmInterf) => &[AsmBom, AsmNumber, AsmFasteners],
        (Bench::Montagem, AsmFasteners | AsmConjunto | AsmGears | AsmChaveta | AsmCatalogo) => &[AsmMate(M::Auto), AsmInterf, AsmBom],
        (Bench::Montagem, AsmBom | AsmNumber) => &[Dwg(0), AsmInterf],
        (Bench::Desenho, Dwg(0) | Dwg(1)) => &[Dwg(10), Dwg(46)],
        (Bench::Desenho, Dwg(10..=12) | Dwg(15..=18)) => &[Dwg(46), Dwg(26), Dwg(13)],
        (Bench::Desenho, Dwg(13) | Dwg(14)) => &[Dwg(26), Dwg(34), Dwg(30)],
        (Bench::Desenho, Dwg(46)) => &[Dwg(26), Dwg(40), Dwg(30)],
        (Bench::Desenho, Dwg(20..=26)) => &[Dwg(26), Dwg(36), Dwg(30)],
        (Bench::Desenho, Dwg(30..=37)) => &[Dwg(40), Dwg(3), Dwg(30)],
        (Bench::Desenho, Dwg(40..=45)) => &[Dwg(31), Dwg(3)],
        (_, Sketch) => &[Extrude, Revolve, Cut],
        (_, Extrude | ExtrudeMid | ExtrudeUpTo | ExtrudeDraftFillet | Prim(_)) => &[Sketch, Fillet, Hole],
        (_, Cut | CutBlind | CutUpTo) => &[Fillet, Chamfer, Sketch],
        (_, Revolve | Groove) => &[Sketch, Fillet, PatternCircular],
        (_, Rib | Slot | Loft | LoftCut | Stiffener) => &[Fillet, Shell, Sketch],
        (_, Fillet | FilletAll | FilletVar | FilletFaceFace | FilletFull) => &[Chamfer, Shell, Mirror],
        (_, Chamfer | ChamferAll | ChamferTwoDist | ChamferDistAngle) => &[Fillet, Shell, Hole],
        (_, Shell | Draft | Thickness) => &[Fillet, Hole, Sketch],
        (_, Hole | HoleCounterbore | HoleCountersink | HoleTapped) => &[PatternLinear, PatternCircular, Thread],
        (_, PatternLinear | PatternCircular | PatternPoints) => &[Mirror, Fillet, Sketch],
        (_, Mirror | Scale | MoveBody(_)) => &[Fillet, Sketch],
        (_, Combine(_)) => &[Fillet, Shell],
        _ => &[],
    };
    v.iter().filter(|c| **c != ultimo || matches!(c, SmFlange | SmCut | AsmInsert | Dwg(26) | Dwg(30))).map(|c| Act::Cmd(*c)).take(3).collect()
}

/// O último comando pedido e quantos itens o documento tinha nesse momento.
static ULTIMO: Mutex<Option<(Bench, Cmd, usize)>> = Mutex::new(None);

/// Guarda o comando que começou (chamado pelo app ao despachar `Act::Cmd`).
pub fn registrar(c: Cmd, ctx: &Contexto) {
    if let Ok(mut u) = ULTIMO.lock() {
        *u = Some((ctx.bench, c, ctx.itens));
    }
}

/// Esquece a sugestão (o usuário escolheu um dos próximos, ou fechou a sugestão).
pub fn esquecer() {
    if let Ok(mut u) = ULTIMO.lock() {
        *u = None;
    }
}

/// Sugestões para agora: só depois que o comando terminou e na bancada em que ele foi pedido.
pub fn sugestoes(ctx: &Contexto) -> Vec<Act> {
    let Some((b, c, itens)) = ULTIMO.lock().ok().and_then(|u| *u) else { return vec![] };
    sugestoes_de(b, c, itens, ctx)
}

fn sugestoes_de(b: Bench, c: Cmd, itens: usize, ctx: &Contexto) -> Vec<Act> {
    if ctx.ocupado || b != ctx.bench {
        return vec![];
    }
    // na peça, o comando conta quando deixou um item novo (cancelado não sugere nada)
    let conta_itens = !matches!(b, Bench::Montagem | Bench::Desenho) && !matches!(c, Cmd::SmParams);
    if conta_itens && ctx.itens <= itens {
        return vec![];
    }
    proximos(b, c)
}

/// O item da barra (ícone, nome e dicas) de uma ação.
pub fn item_de(a: Act) -> Option<crate::toolbar::Item> {
    crate::toolbar::all_entries()
        .into_iter()
        .flat_map(|e| match e {
            crate::toolbar::Entry::One(i) => vec![i],
            crate::toolbar::Entry::Family(_, v) => v,
        })
        .find(|i| i.act == a)
}

/// Clique num botão da barra: roda o comando ou, quando falta um passo, esmaece o botão (sem apagar)
/// e abre o quadro com o motivo, o passo que falta e "usar mesmo assim".
pub fn clique(ui: &egui::Ui, r: &egui::Response, act: Act, ctx: &Contexto, acts: &mut Vec<Act>) {
    let Some(x) = falta(act, ctx) else {
        if r.clicked() {
            acts.push(act);
        }
        return;
    };
    if ui.is_rect_visible(r.rect) {
        ui.painter().rect_filled(r.rect, 6., ui.visuals().panel_fill.gamma_multiply(0.55));
    }
    egui::Popup::menu(r).show(|ui| {
        ui.set_max_width(280.);
        ui.label(egui::RichText::new(tr("Falta um passo")).strong());
        ui.label(tr(x.motivo));
        ui.add_space(4.);
        if ui.button(format!("▶ {}", tr(x.rotulo))).clicked() {
            acts.push(x.passo);
            ui.close();
        }
        if ui.small_button(tr("usar mesmo assim")).on_hover_text(tr("Abre o comando como está; ele espera pelo que falta.")).clicked() {
            acts.push(act);
            ui.close();
        }
    });
}

/// "Próximo:" na barra de baixo, com os ícones dos comandos mais prováveis depois do último.
pub fn na_barra(ui: &mut egui::Ui, ctx: &Contexto) -> Vec<Act> {
    let mut out = vec![];
    // captura sem mouse: FENKRA_DEMO_PROXIMO=1 faz de conta que o último item da peça acabou de ser
    // feito pelo comando Sketch (uma vez só)
    static DEMO: std::sync::Once = std::sync::Once::new();
    if std::env::var("FENKRA_DEMO_PROXIMO").is_ok() && ctx.itens > 0 && ui.ctx().cumulative_frame_nr() > 30 {
        DEMO.call_once(|| registrar(Cmd::Sketch, &Contexto { itens: ctx.itens - 1, ..*ctx }));
    }
    let v = sugestoes(ctx);
    if v.is_empty() {
        return out;
    }
    ui.separator();
    ui.label(egui::RichText::new(tr("Próximo:")).small().weak()).on_hover_text(tr("Próximos comandos mais prováveis no fluxo da bancada"));
    for a in v {
        let Some(i) = item_de(a) else { continue };
        let r = crate::icons::tool_button(ui, i.icon, i.name, crate::toolbar::TAMANHO_NA_LINHA, false, false, i.brief, i.advanced);
        clique(ui, &r, a, ctx, &mut out);
    }
    if ui.small_button("✕").on_hover_text(tr("Esconder a sugestão")).clicked() {
        esquecer();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(b: Bench) -> Contexto {
        Contexto { bench: b, ..Default::default() }
    }

    /// B441: a chapa sem parâmetros diz o motivo e oferece os parâmetros; com parâmetros e sem parede,
    /// oferece a parede base (ou o sketch dela); com parede, a aba roda.
    #[test]
    fn chapa_sem_parametros_oferece_o_passo_que_falta() {
        let mut c = ctx(Bench::Chapa);
        let aba = Act::Cmd(Cmd::SmFlange);
        let x = falta(aba, &c).expect("aba sem chapa");
        assert_eq!(x.passo, Act::Cmd(Cmd::SmParams));
        assert!(x.motivo.contains("parâmetros"));
        c.chapa_params = true;
        assert_eq!(falta(aba, &c).unwrap().passo, Act::Cmd(Cmd::Sketch));
        c.sketches = 1;
        assert_eq!(falta(aba, &c).unwrap().passo, Act::Cmd(Cmd::SmWall));
        c.chapa_parede = true;
        assert_eq!(falta(aba, &c), None);
        // parâmetros e parede base nunca ficam presos
        assert_eq!(falta(Act::Cmd(Cmd::SmParams), &ctx(Bench::Chapa)), None);
        // o passo oferecido também pode rodar (não manda para outro passo travado)
        let mut c = ctx(Bench::Chapa);
        c.chapa_params = true;
        c.sketches = 1;
        assert_eq!(falta(Act::Cmd(Cmd::SmWall), &c), None);
    }

    /// B441: em toda bancada, o passo oferecido nunca é o próprio comando, e seguir os passos leva a
    /// um estado em que o comando roda (nenhuma corrente sem fim).
    #[test]
    fn o_passo_oferecido_destrava_o_comando() {
        for b in crate::mapa_macro::BANCADAS {
            for e in crate::toolbar::all_entries() {
                let itens = match e {
                    crate::toolbar::Entry::One(i) => vec![i],
                    crate::toolbar::Entry::Family(_, v) => v,
                };
                for i in itens {
                    let mut c = ctx(b);
                    for _ in 0..6 {
                        let Some(x) = falta(i.act, &c) else { break };
                        assert_ne!(x.passo, i.act, "{b:?} {}: o passo é o próprio comando", i.name);
                        assert!(item_de(x.passo).is_some(), "{b:?} {}: passo sem botão na barra", i.name);
                        assert!(crate::i18n::tem_traducao(x.motivo), "sem tradução: {}", x.motivo);
                        assert!(crate::i18n::tem_traducao(x.rotulo), "sem tradução: {}", x.rotulo);
                        // fazer o passo muda o documento como ele mudaria de verdade
                        match x.passo {
                            Act::Cmd(Cmd::SmParams) => c.chapa_params = true,
                            Act::Cmd(Cmd::Sketch) => c.sketches += 1,
                            Act::Cmd(Cmd::SmWall) => c.chapa_parede = true,
                            Act::Cmd(Cmd::Extrude) => c.corpos += 1,
                            Act::Cmd(Cmd::Collector) => c.corpos += 1,
                            Act::Cmd(Cmd::Geo(_)) => c.linhas_ref += 1,
                            Act::Cmd(Cmd::StMember) => c.membros += 1,
                            Act::Cmd(Cmd::AsmInsert) => c.pecas += 1,
                            Act::Cmd(Cmd::Dwg(0)) => c.desenho_aberto = true,
                            Act::Cmd(Cmd::Dwg(10)) => c.vistas += 4,
                            outro => panic!("passo sem efeito no teste: {outro:?}"),
                        }
                    }
                    assert_eq!(falta(i.act, &c), None, "{b:?} {}: os passos não destravam", i.name);
                }
            }
        }
    }

    /// B441: sólidos, estrutural, montagem e desenho também explicam o que falta.
    #[test]
    fn cada_bancada_explica_o_que_falta() {
        let s = ctx(Bench::Solidos);
        assert_eq!(falta(Act::Cmd(Cmd::Extrude), &s).unwrap().passo, Act::Cmd(Cmd::Sketch));
        assert_eq!(falta(Act::Cmd(Cmd::Fillet), &s).unwrap().passo, Act::Cmd(Cmd::Sketch));
        assert_eq!(falta(Act::Cmd(Cmd::Fillet), &Contexto { sketches: 1, ..s }).unwrap().passo, Act::Cmd(Cmd::Extrude));
        assert_eq!(falta(Act::Cmd(Cmd::Fillet), &Contexto { sketches: 1, corpos: 1, ..s }), None);
        assert_eq!(falta(Act::Cmd(Cmd::Sketch), &s), None);
        let e = ctx(Bench::Estrutural);
        assert_eq!(falta(Act::Cmd(Cmd::StCutList), &e).unwrap().passo, Act::Cmd(Cmd::StMember));
        let m = ctx(Bench::Montagem);
        assert_eq!(falta(Act::Cmd(Cmd::AsmMate(crate::asm_ui::MateCmd::Coincident)), &Contexto { pecas: 1, ..m }).unwrap().passo, Act::Cmd(Cmd::AsmInsert));
        let d = ctx(Bench::Desenho);
        assert_eq!(falta(Act::Cmd(Cmd::Dwg(26)), &d).unwrap().passo, Act::Cmd(Cmd::Dwg(0)));
        assert_eq!(falta(Act::Cmd(Cmd::Dwg(26)), &Contexto { desenho_aberto: true, ..d }).unwrap().passo, Act::Cmd(Cmd::Dwg(10)));
        assert_eq!(falta(Act::Cmd(Cmd::Dwg(10)), &Contexto { desenho_aberto: true, ..d }), None);
        // ações que não são comando (vista, ajustes) nunca ficam presas
        assert_eq!(falta(Act::Fit, &s), None);
    }

    /// B441: depois de cada comando, os próximos do fluxo; só quando o comando terminou e deixou algo.
    #[test]
    fn sugere_os_proximos_depois_do_comando() {
        let mut c = ctx(Bench::Solidos);
        c.itens = 3;
        assert_eq!(proximos(Bench::Solidos, Cmd::Sketch), vec![Act::Cmd(Cmd::Extrude), Act::Cmd(Cmd::Revolve), Act::Cmd(Cmd::Cut)]);
        // comando ainda aberto: nada
        assert!(sugestoes_de(Bench::Solidos, Cmd::Sketch, 2, &Contexto { ocupado: true, ..c }).is_empty());
        // terminou e deixou o sketch: sugere
        assert_eq!(sugestoes_de(Bench::Solidos, Cmd::Sketch, 2, &c).len(), 3);
        // cancelado (nenhum item novo): nada
        assert!(sugestoes_de(Bench::Solidos, Cmd::Sketch, 3, &c).is_empty());
        // trocou de bancada: nada
        assert!(sugestoes_de(Bench::Chapa, Cmd::SmWall, 2, &c).is_empty());
        // cada bancada tem um fluxo
        assert_eq!(proximos(Bench::Chapa, Cmd::SmParams)[1], Act::Cmd(Cmd::SmWall));
        assert_eq!(proximos(Bench::Estrutural, Cmd::StMember)[0], Act::Cmd(Cmd::StCorners));
        assert_eq!(proximos(Bench::Desenho, Cmd::Dwg(0))[0], Act::Cmd(Cmd::Dwg(10)));
        assert!(!proximos(Bench::Montagem, Cmd::AsmInsert).is_empty());
        // toda sugestão tem botão na barra (ícone e dica) e cabe em três
        for b in crate::mapa_macro::BANCADAS {
            for e in crate::toolbar::all_entries() {
                let itens = match e {
                    crate::toolbar::Entry::One(i) => vec![i],
                    crate::toolbar::Entry::Family(_, v) => v,
                };
                for i in itens {
                    if let Act::Cmd(cmd) = i.act {
                        let v = proximos(b, cmd);
                        assert!(v.len() <= 3);
                        for a in v {
                            assert!(item_de(a).is_some(), "{b:?} depois de {}: {a:?} sem botão", i.name);
                        }
                    }
                }
            }
        }
    }
}
