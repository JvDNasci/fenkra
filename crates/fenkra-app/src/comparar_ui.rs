// SPDX-License-Identifier: LGPL-2.1-or-later
//! B237: painel "Comparar com um alvo". Usa o checador de similaridade do `fenkra-cli` (o mesmo da
//! CLI e do servidor MCP) para comparar a peça aberta com um corpo alvo lido de um STEP: volume, área,
//! centroide, caixa, inércia, furos por diâmetro e distância entre as superfícies, com a tolerância
//! escolhida no painel. No 3D, a superfície da peça fica pintada pela distância até o alvo (perto na
//! cor de destaque, longe na cor de erro) e cada ponto que diverge ganha uma marca: o furo fora do
//! lugar, o que falta, o que sobra e o ponto mais afastado.
//!
//! O painel é livre e encaixável, abre pelo botão do painel Verificar requisitos e fecha no ✕. Os dois
//! corpos valem no mesmo sistema de coordenadas: não há alinhamento automático.

use crate::app::App;
use crate::i18n::tr;
use crate::render::{LineVertex, MeshVertex, Scene};
use crate::sup_analise::{paradas, rampa};
use eframe::egui::{self, Rect, RichText};
use fenkra_cli::checador::{self, Tolerancias};
use fenkra_cli::topologia::analisar;
use fenkra_kernel::{Mesh, Shape};
use fenkra_malha::{Bvh, Malha};
use serde_json::Value;
use std::cell::RefCell;

/// Resultado de uma comparação, pronto para o painel e para a cena.
pub struct Comparacao {
    /// o JSON do checador, igual ao da CLI
    pub json: Value,
    /// malha da peça (o modelo) e a distância de cada vértice até a superfície do alvo, em mm
    pub malha: Mesh,
    pub distancias: Vec<f32>,
    /// pontos onde diverge, com o motivo
    pub marcas: Vec<([f64; 3], String)>,
}

impl Comparacao {
    pub fn aprovado(&self) -> bool {
        self.json["aprovado"] == true
    }

    pub fn maior_distancia(&self) -> f32 {
        self.distancias.iter().copied().fold(0., f32::max)
    }
}

/// Tolerâncias com uma tolerância relativa única para as medidas e a distância; os furos ficam no padrão.
pub fn tolerancias(rel: f64) -> Tolerancias {
    Tolerancias { volume: rel, area: rel, centroide: rel, caixa: rel, inercia: rel, distancia: rel, ..Default::default() }
}

fn malha_de(m: &Mesh) -> Malha {
    let pos: Vec<[f64; 3]> = m.positions.chunks_exact(3).map(|p| [p[0] as f64, p[1] as f64, p[2] as f64]).collect();
    let tris: Vec<[u32; 3]> = m.indices.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect();
    Malha { nome: String::new(), pos, tris, nrm: vec![], cor_v: vec![], material: None }
}

fn ponto(f: &Value, k: &str) -> Option<[f64; 3]> {
    Some([f[k][0].as_f64()?, f[k][1].as_f64()?, f[k][2].as_f64()?])
}

/// As duas bocas do furo: o meio fica dentro do material e a marca não apareceria.
fn bocas(f: &Value) -> Vec<[f64; 3]> {
    ["inicio", "fim"].iter().filter_map(|k| ponto(f, k)).collect()
}

/// Uma marca em cada boca; o texto só na primeira, para a lista citar o furo uma vez.
fn marcar(marcas: &mut Vec<([f64; 3], String)>, pontos: Vec<[f64; 3]>, texto: String) {
    for (k, p) in pontos.into_iter().enumerate() {
        marcas.push((p, if k == 0 { texto.clone() } else { String::new() }));
    }
}

fn d_txt(f: &Value) -> String {
    f["diametro"].as_f64().map(|d| format!("Ø{}", crate::app::fmt(d))).unwrap_or_default()
}

/// Compara `modelo` com `alvo`.
pub fn comparar(alvo: &Shape, modelo: &Shape, t: &Tolerancias) -> Result<Comparacao, String> {
    let ra = analisar(alvo).map_err(|e| e.to_string())?;
    let rm = analisar(modelo).map_err(|e| e.to_string())?;
    let json = checador::comparar(&ra, &rm, t);
    let bvh = Bvh::nova(&malha_de(&ra.malha));
    let distancias = rm
        .malha
        .positions
        .chunks_exact(3)
        .map(|p| bvh.mais_proximo([p[0] as f64, p[1] as f64, p[2] as f64]).map_or(0., |r| r.distancia as f32))
        .collect();
    let mut marcas = vec![];
    let furos = &json["furos"];
    for x in furos["deslocados"].as_array().into_iter().flatten() {
        marcar(&mut marcas, bocas(&x["modelo"]), format!("{} {} {} mm", tr("furo fora do lugar"), d_txt(&x["modelo"]), crate::app::fmt(x["desvio"].as_f64().unwrap_or(0.))));
    }
    for x in furos["diametro_errado"].as_array().into_iter().flatten() {
        marcar(&mut marcas, bocas(&x["modelo"]), format!("{} {} → {}", tr("diâmetro errado"), d_txt(&x["alvo"]), d_txt(&x["modelo"])));
    }
    for x in furos["faltam"].as_array().into_iter().flatten() {
        marcar(&mut marcas, bocas(x), format!("{} {}", tr("furo que falta"), d_txt(x)));
    }
    for x in furos["sobram"].as_array().into_iter().flatten() {
        marcar(&mut marcas, bocas(x), format!("{} {}", tr("furo que sobra"), d_txt(x)));
    }
    let dist = &json["distancia"];
    if dist["ok"] == false {
        if let Some(o) = dist["onde"].as_array().filter(|o| o.len() == 3) {
            let p = [0, 1, 2].map(|i| o[i].as_f64().unwrap_or(0.));
            marcas.push((p, format!("{} {} mm", tr("ponto mais afastado"), crate::app::fmt(dist["maxima"].as_f64().unwrap_or(0.)))));
        }
    }
    Ok(Comparacao { json, malha: rm.malha, distancias, marcas })
}

struct Estado {
    aberto: bool,
    alvo: Option<std::path::PathBuf>,
    rel: f64,
    mapa: bool,
    res: Option<Comparacao>,
    erro: Option<String>,
    demo_lido: bool,
}

impl Default for Estado {
    fn default() -> Self {
        Estado { aberto: false, alvo: None, rel: 1e-6, mapa: true, res: None, erro: None, demo_lido: false }
    }
}

thread_local! {
    static ST: RefCell<Estado> = RefCell::new(Estado::default());
}

/// Abre o painel (botão do painel Verificar requisitos).
pub fn abrir() {
    ST.with(|s| s.borrow_mut().aberto = true);
}

/// Alvo de exemplo para a captura sem mouse (`FENKRA_DEMO_COMPARAR=1`): a mesma peça com o primeiro
/// parâmetro 5% maior, para o mapa mostrar onde as duas se afastam.
fn alvo_demo(doc: &fenkra_core::Document) -> Option<Shape> {
    let mut d = doc.clone();
    if let Some(p) = d.params.first_mut() {
        p.expr = format!("({})*1.05", p.expr);
    }
    fenkra_solidos::final_shape(&d)
}

fn rodar(app: &App, e: &mut Estado, alvo: Option<Shape>) {
    e.erro = None;
    e.res = None;
    let alvo = match alvo {
        Some(s) => s,
        None => {
            let Some(p) = &e.alvo else { return };
            match fenkra_kernel::Shape::import_step(&p.to_string_lossy()) {
                Ok(s) => s,
                Err(x) => {
                    e.erro = Some(format!("{}: {}", tr("Não consegui ler o alvo"), x.0));
                    return;
                }
            }
        }
    };
    let Some(modelo) = fenkra_solidos::final_shape(&app.open.doc) else {
        e.erro = Some(tr("A peça não tem sólido para comparar.").to_string());
        return;
    };
    match comparar(&alvo, &modelo, &tolerancias(e.rel)) {
        Ok(c) => e.res = Some(c),
        Err(m) => e.erro = Some(m),
    }
}

fn nome_medida(n: &str) -> &'static str {
    match n {
        "volume" => "Volume",
        "area" => "Área",
        "centroide" => "Centroide",
        "caixa" => "Caixa envolvente",
        "inercia" => "Momentos de inércia",
        _ => "Medida",
    }
}

/// Painel livre da comparação.
pub fn painel(app: &mut App, ctx: &egui::Context, area: Rect) {
    use crate::paineis::{self, Painel};
    let demo = ST.with(|st| {
        let mut e = st.borrow_mut();
        let ler = !e.demo_lido && ctx.input(|i| i.time) > 2.;
        if ler {
            e.demo_lido = true;
        }
        ler && std::env::var_os("FENKRA_DEMO_COMPARAR").is_some()
    });
    if demo {
        let alvo = alvo_demo(&app.open.doc);
        ST.with(|st| {
            let mut e = st.borrow_mut();
            e.aberto = true;
            rodar(app, &mut e, alvo);
        });
        app.scene_dirty = true;
    }
    if !ST.with(|s| s.borrow().aberto) || app.sem_peca_em_edicao() {
        return;
    }
    let cfg = Painel { nome: "comparar_alvo".into(), titulo: tr("Comparar com um alvo").to_string(), padrao: paineis::abaixo_do_cubo(area, 360.), largura: 360., redimensiona: false };
    let pal = paradas(&ctx.global_style().visuals);
    let (mut pedir, mut fechar) = (false, false);
    let mut mapa_mudou = false;
    ST.with(|st| {
        let mut e = st.borrow_mut();
        let mapa_antes = e.mapa;
        let posicao = app.settings.painel(&cfg.nome, paineis::Encaixe::Livre);
        let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
            ui.label(RichText::new(tr("Compara a peça com um corpo alvo no mesmo sistema de coordenadas: medidas, furos e distância entre as superfícies.")).small().weak());
            ui.horizontal(|ui| {
                if ui.button(tr("Escolher alvo (STEP)…")).on_hover_text(tr("Arquivo STEP com o corpo que a peça deve reproduzir.")).clicked() {
                    if let Some(p) = rfd::FileDialog::new().add_filter("STEP", &["step", "stp"]).pick_file() {
                        e.alvo = Some(p);
                        pedir = true;
                    }
                }
                if let Some(p) = &e.alvo {
                    ui.label(RichText::new(p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()).small());
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("Tolerância relativa"));
                ui.add(crate::ui_ext::dv(&mut e.rel).speed(1e-6).range(1e-9..=0.5).custom_formatter(|v, _| format!("{v:.0e}")).dica(tr("Vale para volume, área, centroide, caixa, inércia e distância (fração da diagonal do alvo). Furos: 0,01 mm no diâmetro e 0,05 mm na posição.")));
                if ui.add_enabled(e.alvo.is_some(), egui::Button::new(tr("Comparar"))).on_hover_text(tr("Mede de novo a peça contra o alvo.")).clicked() {
                    pedir = true;
                }
            });
            ui.checkbox(&mut e.mapa, tr("Mapa de distância no 3D")).on_hover_text(tr("Pinta a peça pela distância até o alvo: perto na cor de destaque, longe na cor de erro."));
            if let Some(m) = &e.erro {
                ui.colored_label(crate::theme::st_err(), m);
            }
            let Some(c) = &e.res else { return };
            egui::ScrollArea::vertical().max_height(220.).show(ui, |ui| {
            let (txt, cor) = if c.aprovado() { (tr("Igual ao alvo dentro da tolerância."), crate::theme::st_ok()) } else { (tr("Diferente do alvo."), crate::theme::st_err()) };
            ui.label(RichText::new(txt).strong().color(cor));
            let marca = |ok: bool| if ok { ("✔", crate::theme::st_ok()) } else { ("✖", crate::theme::st_err()) };
            for m in c.json["medidas"].as_array().into_iter().flatten() {
                let (s, cor) = marca(m["ok"] == true);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(s).color(cor));
                    ui.label(tr(nome_medida(m["nome"].as_str().unwrap_or(""))));
                    ui.label(RichText::new(format!("{} {:.2e}", tr("desvio"), m["desvio"].as_f64().unwrap_or(0.))).small().weak());
                });
            }
            let d = &c.json["distancia"];
            let (s, cor) = marca(d["ok"] == true);
            ui.horizontal(|ui| {
                ui.label(RichText::new(s).color(cor));
                ui.label(tr("Distância entre superfícies"));
                ui.label(RichText::new(format!("{} {} mm", tr("máx."), crate::app::fmt(d["maxima"].as_f64().unwrap_or(0.)))).small().weak());
            });
            let f = &c.json["furos"];
            let (s, cor) = marca(f["ok"] == true);
            ui.horizontal(|ui| {
                ui.label(RichText::new(s).color(cor));
                ui.label(tr("Furos por diâmetro"));
            });
            for l in f["por_diametro"].as_array().into_iter().flatten() {
                let (s, cor) = marca(l["ok"] == true);
                ui.label(RichText::new(format!("   {s} Ø{}: {} {} · {} {}", crate::app::fmt(l["diametro"].as_f64().unwrap_or(0.)), l["alvo"], tr("no alvo"), l["modelo"], tr("na peça"))).small().color(cor));
            }
            let mut vistos: Vec<(&String, usize)> = vec![];
            for (_, m) in c.marcas.iter().filter(|(_, m)| !m.is_empty()) {
                match vistos.iter_mut().find(|v| v.0 == m) {
                    Some(v) => v.1 += 1,
                    None => vistos.push((m, 1)),
                }
            }
            for (m, n) in vistos {
                let vezes = if n > 1 { format!("  ×{n}") } else { String::new() };
                ui.label(RichText::new(format!("• {m}{vezes}")).small().color(crate::theme::st_err()));
            }
            if e.mapa {
                let max = c.maior_distancia();
                ui.horizontal(|ui| {
                    for k in 0..=4 {
                        let (rc, _) = ui.allocate_exact_size(egui::vec2(14., 10.), egui::Sense::hover());
                        ui.painter().rect_filled(rc, 2., crate::theme::cor_da_escala(rampa(&pal, k as f32 / 4.)));
                    }
                    ui.label(RichText::new(format!("0 → {} mm", crate::app::fmt(max as f64))).small());
                });
            }
            ui.horizontal(|ui| {
                if ui.button(tr("Copiar JSON")).on_hover_text(tr("O mesmo relatório da CLI, para colar num script.")).clicked() {
                    ui.ctx().copy_text(c.json.to_string());
                }
                if ui.button(tr("Fechar")).clicked() {
                    fechar = true;
                }
            });
            });
        });
        if let Some(m) = saida.movido {
            app.settings.guardar_painel(&cfg.nome, m);
        }
        if fechar {
            *e = Estado { demo_lido: true, ..Estado::default() };
        }
        mapa_mudou = e.mapa != mapa_antes;
    });
    if fechar || mapa_mudou {
        app.scene_dirty = true;
    }
    if pedir {
        ST.with(|st| {
            let mut e = st.borrow_mut();
            rodar(app, &mut e, None);
        });
        app.scene_dirty = true;
    }
}

/// Mapa de distância e marcas, na cena.
pub fn decorar(v: &egui::Visuals, s: &mut Scene) {
    let pal = paradas(v);
    ST.with(|st| {
        let e = st.borrow();
        let Some(c) = e.res.as_ref().filter(|_| e.aberto) else { return };
        let m = &c.malha;
        let tam = tamanho(m);
        if e.mapa {
            let max = c.maior_distancia().max(1e-9);
            // um fio acima da superfície, para o mapa não brigar com a peça na profundidade
            let folga = tam * 2e-4;
            for &vi in &m.indices {
                let k = vi as usize * 3;
                let t = c.distancias.get(vi as usize).copied().unwrap_or(0.) / max;
                let col = rampa(&pal, t);
                s.ghost.push(MeshVertex { pos: [0, 1, 2].map(|i| m.positions[k + i] + m.normals[k + i] * folga), nrm: [m.normals[k], m.normals[k + 1], m.normals[k + 2]], col: [col[0], col[1], col[2], 0.92], mat: [1., 0., 1., 0.] });
            }
        }
        let tam = tam * 0.03;
        let c_err = pal[3];
        for (p, _) in &c.marcas {
            let p = p.map(|x| x as f32);
            for eixo in 0..3 {
                let (mut a, mut z) = (p, p);
                a[eixo] -= tam;
                z[eixo] += tam;
                // por cima de tudo: a marca fica dentro do furo e o mapa cobriria uma linha comum
                let col = [c_err[0], c_err[1], c_err[2], 1.];
                s.overlay.push(LineVertex { pos: a, col });
                s.overlay.push(LineVertex { pos: z, col });
            }
        }
    });
}

fn tamanho(m: &Mesh) -> f32 {
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in m.positions.chunks_exact(3) {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    if hi[0] < lo[0] {
        return 1.;
    }
    ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2) + (hi[2] - lo[2]).powi(2)).sqrt().max(1e-3)
}


#[cfg(test)]
mod tests {
    #[path = "../../../../fenkra-solidos/tests/common/mod.rs"]
    mod common;

    use super::*;
    use common::*;
    use fenkra_core::{BoolOp, Document, OriginPlane, PlaneRef};

    fn placa(furo_x: f64) -> Shape {
        let mut d = Document::new("placa");
        let base = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-50., -30.], [50., 30.]);
        d.add_feature(ext(base, BoolOp::Add, "8"));
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "8");
        let sk = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[furo_x, 0.], [-20., 10.]]);
        d.add_feature(furo_passante(sk, "6"));
        fenkra_solidos::final_shape(&d).expect("placa")
    }

    #[test]
    fn peca_igual_passa_em_1e6_e_o_mapa_fica_zerado() {
        if !kernel_disponivel() {
            return;
        }
        let a = placa(20.);
        let c = comparar(&a, &placa(20.), &tolerancias(1e-6)).unwrap();
        assert!(c.aprovado(), "{}", c.json);
        assert!(c.maior_distancia() < 1e-4, "{}", c.maior_distancia());
        assert!(c.marcas.is_empty());
        assert_eq!(c.distancias.len() * 3, c.malha.positions.len());
    }

    #[test]
    fn furo_deslocado_meio_milimetro_falha_e_aponta_o_furo() {
        if !kernel_disponivel() {
            return;
        }
        let c = comparar(&placa(20.), &placa(20.5), &tolerancias(1e-6)).unwrap();
        assert!(!c.aprovado());
        let (p, m) = c.marcas.iter().find(|(_, m)| m.contains(tr("furo fora do lugar"))).expect("marca do furo");
        assert!((p[0] - 20.5).abs() < 1e-3 && p[1].abs() < 1e-3, "{p:?} {m}");
        // o mapa acende perto do furo, e longe dele a placa coincide com o alvo
        let (mut perto, mut longe) = (0f32, 0f32);
        for (i, d) in c.distancias.iter().enumerate() {
            let x = c.malha.positions[i * 3];
            let y = c.malha.positions[i * 3 + 1];
            if ((x - 20.5).powi(2) + y * y).sqrt() < 4. {
                perto = perto.max(*d);
            } else if ((x - 20.).powi(2) + y * y).sqrt() > 8. {
                longe = longe.max(*d);
            }
        }
        assert!(perto > 0.3, "perto {perto}");
        assert!(longe < 1e-3, "longe {longe}");
    }
}
