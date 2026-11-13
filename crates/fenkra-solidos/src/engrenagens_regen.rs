// SPDX-License-Identifier: LGPL-2.1-or-later
//! Perfil da engrenagem recalculado a partir dos parâmetros do documento (B52).
//!
//! **Causa do problema antigo.** O gerador desenha o contorno do dente como coordenadas fixas no
//! sketch (pontos da evolvente, da trocoide e dos arcos). Os parâmetros `m`, `z`, `alpha`, `beta`,
//! `x`, `folga` e `L` ficavam na tabela do documento, mas nenhum comando os lia: mudar `z` de 20
//! para 30 refazia `d = m*z` e deixava os 20 dentes como estavam, e a configuração da tabela de
//! projeto que trocava o módulo não mudava a peça.
//!
//! **Como funciona agora.** O gerador grava na peça as entradas com que desenhou o perfil
//! (`props.custom["engrenagem"]`, em JSON). Antes de cada recálculo, [`perfil_atualizado`] lê os
//! parâmetros do documento (já com a configuração ativa); se algum difere do gravado, redesenha os
//! sketches do perfil pelo mesmo cálculo do gerador e devolve o documento efetivo. Os comandos
//! acrescentados pela pessoa depois do gerador ficam como estão; só o desenho dos sketches
//! `Perfil_dentes`, `Perfil_anel` e `Secao_i` é trocado (o id do comando é mantido, então a
//! extrusão e o loft continuam apontando para eles). Na helicoidal, se o número de seções muda (a
//! torção cresceu com β ou com a largura), as seções e seus planos são refeitos no lugar das antigas,
//! antes do loft.
//!
//! Entradas inválidas (por exemplo `z = 3`) não estragam a peça: o recálculo segue com o perfil
//! gravado e o motivo vai para o erro de parâmetro.

use crate::engrenagens::{calculo, dente_externo, dente_interno, passos_cremalheira, pontos_flanco, Helice, ParamEngrenagem, TipoEngrenagem};
use crate::fixadores::Montador;
use fenkra_core::*;

const CHAVE: &str = "engrenagem";

/// Grava na peça as entradas com que o perfil foi desenhado.
pub fn gravar_gerador(doc: &mut Document, p: &ParamEngrenagem) {
    doc.props.custom.retain(|(k, _)| k != CHAVE);
    doc.props.custom.push((CHAVE.into(), serde_json::to_string(p).unwrap_or_default()));
}

/// Entradas gravadas pelo gerador; `None` se a peça não veio do gerador de engrenagens.
pub fn gerador_gravado(doc: &Document) -> Option<ParamEngrenagem> {
    let v = &doc.props.custom.iter().find(|(k, _)| k == CHAVE)?.1;
    serde_json::from_str(v).ok()
}

/// Entradas que os parâmetros atuais do documento pedem (parte do que foi gravado).
pub fn entradas_do_documento(doc: &Document, gravado: &ParamEngrenagem) -> Result<ParamEngrenagem, String> {
    let vars = doc.eval_params().map_err(|e| e.to_string())?;
    let le = |nome: &str, atual: f64| vars.get(nome).copied().unwrap_or(atual);
    let mut p = gravado.clone();
    p.modulo = le("m", p.modulo);
    p.angulo_pressao = le("alpha", p.angulo_pressao);
    p.largura = le("b", p.largura);
    p.folga_dentes = le("folga", p.folga_dentes);
    p.folga_impressao = le("folga_impressao", p.folga_impressao);
    match p.tipo {
        TipoEngrenagem::Cremalheira => p.comprimento_cremalheira = le("L", p.comprimento_cremalheira),
        _ => {
            let z = le("z", p.dentes as f64);
            if !(z.is_finite() && z >= 1. && (z - z.round()).abs() < 1e-6) {
                return Err(format!("z = {z}: o número de dentes precisa ser inteiro"));
            }
            p.dentes = z.round() as u32;
        }
    }
    if matches!(p.tipo, TipoEngrenagem::Reta | TipoEngrenagem::Helicoidal) {
        p.coef_correcao = le("x", p.coef_correcao);
        p.furo = le("furo", p.furo);
    }
    if p.tipo == TipoEngrenagem::Helicoidal {
        p.angulo_helice = le("beta", p.angulo_helice);
    }
    Ok(p)
}

/// Documento efetivo com o perfil redesenhado, quando os parâmetros mudaram desde o gerador.
/// `None`: não é engrenagem do gerador ou nada mudou. `Some(Err)`: as entradas novas são inválidas.
pub fn perfil_atualizado(doc: &Document) -> Option<Result<Document, String>> {
    let gravado = gerador_gravado(doc)?;
    let novo = match entradas_do_documento(doc, &gravado) {
        Ok(p) if p == gravado => return None,
        Ok(p) => p,
        Err(e) => return Some(Err(e)),
    };
    let mut d = doc.clone();
    Some(redesenhar(&mut d, &novo).map(|_| d))
}

/// Grava no próprio documento o perfil que os parâmetros pedem. Devolve se algo mudou.
pub fn sincronizar_perfil(doc: &mut Document) -> Result<bool, String> {
    let Some(gravado) = gerador_gravado(doc) else { return Ok(false) };
    let novo = entradas_do_documento(doc, &gravado)?;
    if novo == gravado {
        return Ok(false);
    }
    redesenhar(doc, &novo)?;
    Ok(true)
}

/// Troca o desenho do perfil pelo das entradas `p` e grava `p` como novas entradas do gerador.
fn redesenhar(doc: &mut Document, p: &ParamEngrenagem) -> Result<(), String> {
    calculo(p).map_err(|e| format!("engrenagem: {e}"))?;
    let mut m = Montador { doc: std::mem::replace(doc, Document::new("")) };
    let r = redesenhar_em(&mut m, p);
    *doc = m.doc;
    r?;
    let chave = p.chave();
    doc.props.custom.retain(|(k, _)| k != "catalogo");
    doc.props.custom.push(("catalogo".into(), chave));
    if doc.props.description == gerador_gravado(doc).map(|g| g.descricao()).unwrap_or_default() {
        doc.props.description = p.descricao();
    }
    gravar_gerador(doc, p);
    Ok(())
}

fn redesenhar_em(m: &mut Montador, p: &ParamEngrenagem) -> Result<(), String> {
    let externa = matches!(p.tipo, TipoEngrenagem::Reta | TipoEngrenagem::Helicoidal);
    match p.tipo {
        TipoEngrenagem::Cremalheira => {
            let (inicio, passos) = passos_cremalheira(p);
            let data = m.dados_contorno(inicio, &passos, 0., None);
            trocar_desenho(&mut m.doc, "Perfil_dentes", data)
        }
        TipoEngrenagem::Interna => {
            let g = dente_interno(p);
            let (inicio, passos) = g.contorno(pontos_flanco(g.z))?;
            let anel = calculo(p)?.diametro_anel;
            let data = m.dados_contorno(inicio, &passos, 0., Some(anel));
            trocar_desenho(&mut m.doc, "Perfil_anel", data)
        }
        _ if externa && sketch_por_nome(&m.doc, "Perfil_dentes").is_some() => {
            // reta, ou helicoidal gerada com β = 0: perfil sem torção
            let g = dente_externo(p);
            let (inicio, passos) = g.contorno(pontos_flanco(g.z))?;
            let data = m.dados_contorno(inicio, &passos, 0., None);
            trocar_desenho(&mut m.doc, "Perfil_dentes", data)
        }
        _ => redesenhar_helice(m, p),
    }
}

fn sketch_por_nome(doc: &Document, nome: &str) -> Option<usize> {
    doc.features.iter().position(|f| f.name == nome && matches!(f.kind, FeatureKind::Sketch(_)))
}

fn trocar_desenho(doc: &mut Document, nome: &str, data: SketchData) -> Result<(), String> {
    let i = sketch_por_nome(doc, nome).ok_or_else(|| format!("o sketch {nome} do gerador não está mais na peça"))?;
    if let FeatureKind::Sketch(sk) = &mut doc.features[i].kind {
        sk.data = data;
    }
    Ok(())
}

/// Helicoidal: troca o desenho de cada seção; se a quantidade de seções muda, refaz seções e planos.
fn redesenhar_helice(m: &mut Montador, p: &ParamEngrenagem) -> Result<(), String> {
    let g = dente_externo(p);
    let (inicio, passos) = g.contorno(pontos_flanco(g.z))?;
    let h = Helice::nova(&g, p.angulo_helice.to_radians(), p.largura, p.espinha_peixe);
    let i_loft = m.doc.features.iter().position(|f| f.name == "Dentes_helicoidais" && matches!(f.kind, FeatureKind::Loft(_))).ok_or("o loft Dentes_helicoidais do gerador não está mais na peça")?;
    let FeatureKind::Loft(loft) = &m.doc.features[i_loft].kind else { unreachable!() };
    let antigas = loft.sections.clone();
    if antigas.len() == h.trechos + 1 {
        for (i, id) in antigas.iter().enumerate() {
            let data = m.dados_contorno(inicio, &passos, h.giro(i), None);
            if let Some(FeatureKind::Sketch(sk)) = m.doc.features.iter_mut().find(|f| f.id == *id).map(|f| &mut f.kind) {
                sk.data = data;
            }
        }
        return Ok(());
    }
    // quantidade nova de seções: tira as antigas (e seus planos) e põe as novas antes do loft
    let planos_antigos: Vec<Id> = m
        .doc
        .features
        .iter()
        .filter(|f| antigas.contains(&f.id))
        .filter_map(|f| match &f.kind {
            FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(d), .. }) => Some(*d),
            _ => None,
        })
        .collect();
    m.doc.features.retain(|f| !antigas.contains(&f.id));
    m.doc.references.retain(|r| !planos_antigos.contains(&r.id));
    let rollback_no_fim = m.doc.rollback >= m.doc.features.len() + antigas.len();
    let mut novas = vec![];
    for i in 0..=h.trechos {
        let plano = if i == 0 { PlaneRef::Origin(OriginPlane::XY) } else { m.plano(&format!("Plano_secao_{i}"), &format!("b*{i}/{}", h.trechos))? };
        novas.push(m.sketch_contorno(&format!("Secao_{i}"), plano, inicio, &passos, h.giro(i), None));
    }
    // `add_feature` põe no ponto de retrocesso: as seções vão para logo antes do loft
    let mut tiradas: Vec<Feature> = vec![];
    for id in &novas {
        if let Some(k) = m.doc.features.iter().position(|f| f.id == *id) {
            tiradas.push(m.doc.features.remove(k));
        }
    }
    let at = m.doc.features.iter().position(|f| f.name == "Dentes_helicoidais" && matches!(f.kind, FeatureKind::Loft(_))).ok_or("loft perdido")?;
    for (k, f) in tiradas.into_iter().enumerate() {
        m.doc.features.insert(at + k, f);
    }
    m.doc.rollback = if rollback_no_fim { m.doc.features.len() } else { m.doc.rollback.min(m.doc.features.len()) };
    if let Some(FeatureKind::Loft(l)) = m.doc.features.iter_mut().find(|f| f.name == "Dentes_helicoidais").map(|f| &mut f.kind) {
        l.sections = novas;
        l.torcao.clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engrenagens::engrenagem_documento;
    use crate::{recompute, Status};

    fn medir(doc: &Document) -> (f64, [f64; 3], [f64; 3]) {
        let r = recompute(doc);
        assert!(r.param_error.is_none(), "{:?}", r.param_error);
        for (id, s) in &r.status {
            assert!(matches!(s, Status::Ok | Status::Warning(_)), "{id:?}: {s:?}");
        }
        assert_eq!(r.bodies.len(), 1);
        corpo(&r.bodies[0])
    }

    fn corpo(b: &crate::BodyResult) -> (f64, [f64; 3], [f64; 3]) {
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for q in b.mesh.positions.chunks_exact(3) {
            for i in 0..3 {
                lo[i] = lo[i].min(q[i] as f64);
                hi[i] = hi[i].max(q[i] as f64);
            }
        }
        (b.mass.volume, lo, hi)
    }

    fn mudar(doc: &mut Document, nome: &str, expr: &str) {
        doc.params.iter_mut().find(|p| p.name == nome).unwrap_or_else(|| panic!("sem {nome}")).expr = expr.into();
    }

    fn igual(a: (f64, [f64; 3], [f64; 3]), b: (f64, [f64; 3], [f64; 3])) {
        assert!((a.0 - b.0).abs() < 1e-3 * b.0, "volume {} x {}", a.0, b.0);
        for k in 0..3 {
            assert!((a.1[k] - b.1[k]).abs() < 1e-2 && (a.2[k] - b.2[k]).abs() < 1e-2, "caixa {a:?} x {b:?}");
        }
    }

    /// Mudar z e m no documento redesenha os dentes: a peça fica igual à que o gerador faria com as
    /// entradas novas (antes, a caixa continuava a dos 20 dentes).
    #[test]
    fn reta_muda_z_e_m_pelos_parametros() {
        let p = ParamEngrenagem { dentes: 20, modulo: 2., largura: 8., ..Default::default() };
        let mut doc = engrenagem_documento(&p).unwrap();
        let antes = medir(&doc);
        assert!((antes.2[0] - 22.).abs() < 1e-2, "da/2 = 22: {antes:?}");
        mudar(&mut doc, "z", "30");
        mudar(&mut doc, "m", "2.5");
        let depois = medir(&doc);
        let alvo = medir(&engrenagem_documento(&ParamEngrenagem { dentes: 30, modulo: 2.5, ..p.clone() }).unwrap());
        igual(depois, alvo);
        assert!((depois.2[0] - 40.).abs() < 1e-2, "da/2 = 2,5·32/2: {depois:?}");
        // gravar no próprio documento dá o mesmo resultado e deixa as entradas novas
        assert!(sincronizar_perfil(&mut doc).unwrap());
        assert_eq!(gerador_gravado(&doc).unwrap().dentes, 30);
        assert!(doc.props.description.contains("z30"));
        assert!(perfil_atualizado(&doc).is_none());
        igual(medir(&doc), alvo);
    }

    /// α e x também redesenham o flanco; a cremalheira acompanha L e a interna acompanha z.
    #[test]
    fn alfa_x_cremalheira_e_interna() {
        let p = ParamEngrenagem { dentes: 18, modulo: 2., largura: 6., ..Default::default() };
        let mut doc = engrenagem_documento(&p).unwrap();
        mudar(&mut doc, "alpha", "25");
        mudar(&mut doc, "x", "0.3");
        igual(medir(&doc), medir(&engrenagem_documento(&ParamEngrenagem { angulo_pressao: 25., coef_correcao: 0.3, ..p.clone() }).unwrap()));

        let c = ParamEngrenagem { tipo: TipoEngrenagem::Cremalheira, modulo: 2., largura: 6., comprimento_cremalheira: 60., ..Default::default() };
        let mut doc = engrenagem_documento(&c).unwrap();
        mudar(&mut doc, "L", "90");
        let r = medir(&doc);
        assert!((r.2[0] - 90.).abs() < 1e-3, "{r:?}");
        igual(r, medir(&engrenagem_documento(&ParamEngrenagem { comprimento_cremalheira: 90., ..c.clone() }).unwrap()));

        let i = ParamEngrenagem { tipo: TipoEngrenagem::Interna, modulo: 1.5, dentes: 40, largura: 6., ..Default::default() };
        let mut doc = engrenagem_documento(&i).unwrap();
        mudar(&mut doc, "z", "50");
        igual(medir(&doc), medir(&engrenagem_documento(&ParamEngrenagem { dentes: 50, ..i.clone() }).unwrap()));
    }

    /// Helicoidal: β maior pede mais seções do loft; elas são refeitas antes do loft e a peça bate com a
    /// do gerador.
    #[test]
    fn helicoidal_beta_muda_quantidade_de_secoes() {
        let p = ParamEngrenagem { tipo: TipoEngrenagem::Helicoidal, dentes: 20, modulo: 2., largura: 12., angulo_helice: 5., ..Default::default() };
        let mut doc = engrenagem_documento(&p).unwrap();
        let secoes = |d: &Document| d.features.iter().filter(|f| f.name.starts_with("Secao_")).count();
        let n0 = secoes(&doc);
        mudar(&mut doc, "beta", "30");
        let novo = perfil_atualizado(&doc).unwrap().unwrap();
        assert!(secoes(&novo) > n0, "{} seções", secoes(&novo));
        let i_loft = novo.features.iter().position(|f| f.name == "Dentes_helicoidais").unwrap();
        assert!(novo.features.iter().take(i_loft).filter(|f| f.name.starts_with("Secao_")).count() == secoes(&novo));
        let alvo = engrenagem_documento(&ParamEngrenagem { angulo_helice: 30., ..p.clone() }).unwrap();
        igual(medir(&doc), medir(&alvo));
    }

    /// Entrada inválida não estraga a peça: o perfil gravado fica e o motivo aparece.
    #[test]
    fn entrada_invalida_mantem_perfil_e_avisa() {
        let p = ParamEngrenagem { dentes: 20, modulo: 2., largura: 8., ..Default::default() };
        let mut doc = engrenagem_documento(&p).unwrap();
        let antes = medir(&engrenagem_documento(&p).unwrap());
        mudar(&mut doc, "z", "3");
        let r = recompute(&doc);
        assert!(r.param_error.as_deref().unwrap_or("").contains("dentes"), "{:?}", r.param_error);
        assert!((corpo(&r.bodies[0]).0 - antes.0).abs() < 1e-3 * antes.0);
        mudar(&mut doc, "z", "20.5");
        assert!(perfil_atualizado(&doc).unwrap().is_err());
    }

    /// Peça sem o gerador não muda.
    #[test]
    fn peca_comum_nao_muda() {
        assert!(perfil_atualizado(&crate::example_document()).is_none());
    }
}
