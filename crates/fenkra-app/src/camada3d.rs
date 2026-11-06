// SPDX-License-Identifier: LGPL-2.1-or-later
//! Camadas de comparação no 3D (B214 sobreposição colorida de versões, B130 prévia lado a lado de
//! uma mesclagem).
//!
//! Como funciona:
//! 1. o cálculo (recalcular as duas versões e separar o que existe só de um lado, ver
//!    `fenkra_project::sobreposicao`) roda numa linha de execução própria e deixa o resultado numa vaga
//!    global; o laço da interface o recolhe com [`receber`];
//! 2. o resultado é um [`Conjunto`] de camadas de triângulos, cada uma com um **papel**. A cor vem do
//!    papel e do tema em uso na hora de montar a cena (`na_cena`), nunca de cor fixa;
//! 3. as camadas entram na camada translúcida da cena (a mesma da prévia de comandos): não entram na
//!    seleção e não mudam o documento. A janela com a legenda fecha tudo.

use crate::i18n::tr;
use crate::render::{MeshVertex, Scene};
use eframe::egui;
use fenkra_core::Document;
use fenkra_project::sobreposicao::{sobrepor, MalhaRef};
use glam::Vec3;
use std::sync::Mutex;

/// Papel de uma camada: decide a cor pelo tema.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Papel {
    /// só na versão antiga
    Removido,
    /// só na versão nova
    Adicionado,
    /// resultado da mesclagem ficando com a principal
    Principal,
    /// resultado da mesclagem ficando com o ramo
    Ramo,
}

impl Papel {
    pub fn cor32(self, v: &egui::Visuals) -> egui::Color32 {
        match self {
            Papel::Removido => v.error_fg_color,
            Papel::Adicionado => v.hyperlink_color,
            Papel::Principal => v.hyperlink_color,
            Papel::Ramo => v.warn_fg_color,
        }
    }

    pub fn cor(self, v: &egui::Visuals) -> [u8; 3] {
        let c = self.cor32(v);
        [c.r(), c.g(), c.b()]
    }
}

#[derive(Clone, Debug)]
pub struct Camada {
    pub papel: Papel,
    /// triângulos soltos, 9 números cada
    pub tris: Vec<f32>,
    pub alfa: f32,
    pub deslocamento: [f32; 3],
}

#[derive(Clone, Debug, Default)]
pub struct Conjunto {
    pub titulo: String,
    pub camadas: Vec<Camada>,
    /// papel e o que ele significa, para a legenda
    pub legenda: Vec<(Papel, String)>,
    /// linhas de resumo (áreas, volumes) mostradas na janela
    pub resumo: Vec<String>,
    pub visivel: bool,
}

static ATIVO: Mutex<Option<Conjunto>> = Mutex::new(None);
static PRONTO: Mutex<Option<Result<Conjunto, String>>> = Mutex::new(None);

fn trava<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn ativo() -> bool {
    trava(&ATIVO).is_some()
}

pub fn limpar() {
    *trava(&ATIVO) = None;
}

pub fn definir(c: Conjunto) {
    *trava(&ATIVO) = Some(c);
}

/// Recolhe o que a linha de execução terminou. `Some(Err)` quando o cálculo falhou.
pub fn receber() -> Option<Result<(), String>> {
    let r = trava(&PRONTO).take()?;
    Some(r.map(definir))
}

fn malhas_de(d: &Document) -> Vec<(Vec<f32>, Vec<u32>)> {
    fenkra_solidos::recompute(d).bodies.into_iter().map(|b| (b.mesh.positions, b.mesh.indices)).collect()
}

fn refs(m: &[(Vec<f32>, Vec<u32>)]) -> Vec<MalhaRef<'_>> {
    m.iter().map(|(p, i)| MalhaRef { positions: p, indices: i }).collect()
}

/// Monta a sobreposição de duas versões (B214). `rot_*` são os rótulos das versões (`v2`, `v5`).
pub fn sobreposicao_de(antes: &Document, depois: &Document, rot_a: &str, rot_b: &str) -> Conjunto {
    let (ma, mb) = (malhas_de(antes), malhas_de(depois));
    let s = sobrepor(&refs(&ma), &refs(&mb), None);
    let mut c = Conjunto { titulo: format!("{rot_a} ↔ {rot_b}"), visivel: true, ..Default::default() };
    c.legenda = vec![(Papel::Removido, format!("{} {rot_a}", tr("só na"))), (Papel::Adicionado, format!("{} {rot_b}", tr("só na")))];
    c.resumo = vec![
        format!("{}: {:.1} mm²", tr("Área removida"), s.area_removida),
        format!("{}: {:.1} mm²", tr("Área adicionada"), s.area_adicionada),
        format!("{}: {:.2} mm", tr("Tolerância"), s.tolerancia),
    ];
    if s.e_igual() {
        c.resumo.insert(0, tr("As superfícies coincidem: nada para mostrar.").to_string());
    }
    c.camadas.push(Camada { papel: Papel::Removido, tris: s.removido, alfa: 0.55, deslocamento: [0.; 3] });
    c.camadas.push(Camada { papel: Papel::Adicionado, tris: s.adicionado, alfa: 0.55, deslocamento: [0.; 3] });
    c
}

/// Triângulos de cada corpo, separados em "do comando em conflito" e "o resto", pela dona de cada face.
fn por_dona(d: &Document, destaque: Option<u64>) -> (Vec<f32>, Vec<f32>, ([f32; 3], [f32; 3])) {
    let r = fenkra_solidos::recompute(d);
    let (mut dest, mut resto) = (vec![], vec![]);
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for b in &r.bodies {
        let m = &b.mesh;
        for p in m.positions.chunks_exact(3) {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        for fi in 0..m.face_ranges.len() {
            let (ini, fim) = (m.face_ranges[fi] as usize, m.face_ranges.get(fi + 1).map_or(m.indices.len(), |x| *x as usize));
            let da_feature = destaque.is_some_and(|f| b.face_owner.get(fi).is_some_and(|o| o.0 == f));
            let alvo = if da_feature { &mut dest } else { &mut resto };
            for &i in &m.indices[ini..fim] {
                let k = i as usize * 3;
                alvo.extend_from_slice(&m.positions[k..k + 3]);
            }
        }
    }
    (dest, resto, (lo, hi))
}

/// Prévia lado a lado de uma mesclagem (B130): o resultado ficando com a principal e o resultado
/// ficando com o ramo, um ao lado do outro, com as faces da feature em conflito mais fortes.
pub fn lado_a_lado_de(nossa: &Document, deles: &Document, destaque: Option<u64>, rotulo: &str) -> Conjunto {
    let ((dn, rn, cn), (dd, rd, cd)) = (por_dona(nossa, destaque), por_dona(deles, destaque));
    let largura = (cn.1[0] - cn.0[0]).max(cd.1[0] - cd.0[0]).max(1.);
    let passo = largura * 1.25;
    let mut c = Conjunto { titulo: rotulo.to_string(), visivel: true, ..Default::default() };
    c.legenda = vec![(Papel::Principal, tr("Principal").to_string()), (Papel::Ramo, tr("Ramo").to_string())];
    c.resumo = vec![tr("À direita do modelo: a principal e, depois dela, o ramo. As faces do conflito aparecem mais fortes.").to_string()];
    for (papel, forte, fraco, dx) in [(Papel::Principal, dn, rn, passo), (Papel::Ramo, dd, rd, passo * 2.)] {
        c.camadas.push(Camada { papel, tris: fraco, alfa: 0.28, deslocamento: [dx, 0., 0.] });
        c.camadas.push(Camada { papel, tris: forte, alfa: 0.7, deslocamento: [dx, 0., 0.] });
    }
    c
}

fn em_segundo_plano(f: impl FnOnce() -> Conjunto + Send + 'static) {
    std::thread::spawn(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|_| tr("o cálculo da comparação falhou").to_string());
        *trava(&PRONTO) = Some(r);
    });
}

/// Calcula a sobreposição de duas versões sem travar a interface.
pub fn calcular_sobreposicao(antes: Document, depois: Document, rot_a: String, rot_b: String) {
    em_segundo_plano(move || sobreposicao_de(&antes, &depois, &rot_a, &rot_b));
}

/// Calcula a prévia lado a lado de uma mesclagem sem travar a interface.
pub fn calcular_lado_a_lado(nossa: Document, deles: Document, destaque: Option<u64>, rotulo: String) {
    em_segundo_plano(move || lado_a_lado_de(&nossa, &deles, destaque, &rotulo));
}

/// Acrescenta as camadas à cena (camada translúcida) e as inclui no enquadramento.
pub fn na_cena(v: &egui::Visuals, s: &mut Scene) {
    let g = trava(&ATIVO);
    let Some(c) = g.as_ref().filter(|c| c.visivel) else { return };
    for cam in &c.camadas {
        let [r, gr, b] = cam.papel.cor(v).map(|x| f32::from(x) / 255.);
        let d = Vec3::from(cam.deslocamento);
        for t in cam.tris.chunks_exact(9) {
            let p = [Vec3::new(t[0], t[1], t[2]) + d, Vec3::new(t[3], t[4], t[5]) + d, Vec3::new(t[6], t[7], t[8]) + d];
            let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
            for q in p {
                s.ghost.push(MeshVertex { pos: q.into(), nrm: n.into(), col: [r, gr, b, cam.alfa], mat: [1., 0., 1., 0.] });
                s.bounds = Some(match s.bounds {
                    Some((lo, hi)) => (lo.min(q), hi.max(q)),
                    None => (q, q),
                });
            }
        }
    }
}

/// Janela da legenda. Devolve `true` quando algo mudou e a cena precisa ser refeita.
pub fn janela(ctx: &egui::Context) -> bool {
    let Some(mut c) = trava(&ATIVO).clone() else { return false };
    let (mut aberta, antes, mut fechar) = (true, c.visivel, false);
    egui::Window::new(format!("{}: {}", tr("Comparação no 3D"), c.titulo)).id(egui::Id::new("fenkra_camada3d")).open(&mut aberta).default_pos([60., 120.]).default_width(300.).show(ctx, |ui| {
        for (papel, texto) in &c.legenda {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(14., 14.), egui::Sense::hover());
                let cor = papel.cor32(ui.visuals());
                ui.painter().rect_filled(r, 3., cor);
                ui.label(texto);
            });
        }
        for l in &c.resumo {
            ui.label(egui::RichText::new(l).weak().small());
        }
        ui.checkbox(&mut c.visivel, tr("Mostrar no 3D"));
        if ui.button(tr("Fechar comparação")).on_hover_text(tr("Tira as camadas coloridas do 3D")).clicked() {
            fechar = true;
        }
    });
    if !aberta || fechar {
        limpar();
        return true;
    }
    let mudou = c.visivel != antes;
    if mudou {
        definir(c);
    }
    mudou
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{BoolOp, FeatureKind, OriginPlane, PlaneRef, PrimKind, Primitive};

    fn caixa(lado: &str) -> Document {
        let s = |x: &str| x.to_string();
        let mut d = Document::new("Placa");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s(lado), b: s("20"), c: s("5"), d: s("0"), op: BoolOp::Add }));
        d
    }

    /// O estado das camadas é global: os testes que o usam rodam um por vez.
    static VEZ: Mutex<()> = Mutex::new(());

    fn cena_com(c: Conjunto) -> Scene {
        let _vez = VEZ.lock().unwrap_or_else(|e| e.into_inner());
        definir(c);
        let mut s = Scene::default();
        na_cena(&egui::Visuals::dark(), &mut s);
        limpar();
        s
    }

    #[test]
    fn sobreposicao_pinta_o_acrescentado_com_a_cor_do_papel_no_tema() {
        let c = sobreposicao_de(&caixa("40"), &caixa("60"), "v1", "v2");
        let adicionado = c.camadas.iter().find(|x| x.papel == Papel::Adicionado).unwrap();
        assert!(!adicionado.tris.is_empty(), "a ponta nova existe só na versão nova");
        assert!(!c.camadas.iter().find(|x| x.papel == Papel::Removido).unwrap().tris.is_empty(), "a tampa velha, agora por dentro, existe só na antiga");
        let s = cena_com(c);
        assert!(!s.ghost.is_empty() && s.ghost.len() % 3 == 0);
        let v = egui::Visuals::dark();
        let cor = |p: Papel| p.cor(&v).map(|x| f32::from(x) / 255.);
        let tem = |c: [f32; 3]| s.ghost.iter().any(|g| (g.col[0] - c[0]).abs() < 1e-6 && (g.col[1] - c[1]).abs() < 1e-6 && (g.col[2] - c[2]).abs() < 1e-6 && (g.col[3] - 0.55).abs() < 1e-6);
        assert!(tem(cor(Papel::Adicionado)) && tem(cor(Papel::Removido)), "as cores vêm do tema, uma por papel");
        assert!(s.bounds.is_some(), "a sobreposição entra no enquadramento");
        assert_ne!(Papel::Removido.cor(&v), Papel::Adicionado.cor(&v), "o que saiu e o que entrou têm cores diferentes");
    }

    #[test]
    fn lado_a_lado_afasta_a_principal_e_o_ramo_e_reforca_a_feature_em_conflito() {
        let nossa = caixa("40");
        let deles = caixa("60");
        let id_caixa = nossa.features[0].id.0;
        let c = lado_a_lado_de(&nossa, &deles, Some(id_caixa), "Furo 1");
        let desloc: Vec<f32> = c.camadas.iter().map(|x| x.deslocamento[0]).collect();
        assert!(desloc[0] > 0. && desloc[2] > desloc[0], "principal à direita do modelo e o ramo depois dela: {desloc:?}");
        let forte = c.camadas.iter().filter(|x| x.alfa > 0.5 && !x.tris.is_empty()).count();
        assert_eq!(forte, 2, "as faces da caixa em conflito ficam fortes nos dois lados");
        let s = cena_com(c);
        assert!(s.ghost.len() >= 36 * 2, "cada caixa são 12 triângulos de 3 vértices");
    }

    #[test]
    fn camada_escondida_nao_vai_para_a_cena() {
        let mut c = sobreposicao_de(&caixa("40"), &caixa("60"), "v1", "v2");
        c.visivel = false;
        assert!(cena_com(c).ghost.is_empty());
    }
}
