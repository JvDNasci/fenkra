// SPDX-License-Identifier: LGPL-2.1-or-later
//! Comando Chaveta (B292): pareia um eixo e um cubo, acha os dois rasgos, confere as medidas com a
//! tabela e insere a chaveta paralela (DIN 6885 forma A) restrita aos rasgos.
//!
//! Como funciona no kernel: cada rasgo é um corte (extrusão cega) numa peça. `chaveta_rasgos` lê
//! esses cortes; aqui eles são levados para o mundo pelas posições dos componentes e comparados:
//!
//! - o **diâmetro** vem do trecho do eixo onde o rasgo está e a tabela dá b, h, t1 e t2;
//! - o **b** do rasgo do eixo e o do cubo têm de ser iguais ao da tabela;
//! - o **t1** (fundo do rasgo do eixo, `d/2 - piso`) e o **t2** (fundo do rasgo do cubo, `teto - furo/2`)
//!   são conferidos com a tabela, e `t1 + t2 >= h` garante folga entre o topo da chaveta e o cubo;
//! - os rasgos precisam estar do mesmo lado do eixo e se sobrepor ao longo dele; a chaveta da série
//!   que cabe nessa sobreposição é a oferecida.
//!
//! A chaveta tem o comprimento no seu X, a largura em Y e a altura em Z (base em z = 0). Entra
//! deitada no piso do rasgo do eixo com três restrições de plano, que dão graus de liberdade zero:
//! contato da base com o piso, plano médio longitudinal coincidente com o plano que passa pelo eixo
//! e pelo rasgo, e plano médio transversal coincidente com a posição escolhida ao longo do eixo.
//! O cubo não ganha restrição: ele já está restrito ao eixo e o rasgo dele acolhe a chaveta.

use crate::load::{full_path, PartCache};
use crate::*;
use fenkra_solidos::biblioteca::{chaveta_para_eixo, comprimentos_chaveta, Chaveta};
use fenkra_solidos::chaveta_rasgos::{diametro_do_trecho, furo_do_cubo, rasgos_do_documento, RasgoLido};
use fenkra_solidos::chaveta_documento;
use std::path::Path;

/// Nome do grupo da árvore que recebe as chavetas.
const GRUPO: &str = "Chavetas";
/// Tolerância das medidas comparadas (mm).
const TOL: f64 = 1e-3;
/// Tolerância das medidas que vêm da tabela (mm): o gerador grava os valores dela com 1 casa.
const TOL_TABELA: f64 = 0.05;
/// Giro máximo (graus) entre os rasgos do eixo e do cubo para contarem como o mesmo lado.
const TOL_GIRO: f64 = 1.;

/// O que a pessoa precisa saber antes de inserir.
#[derive(Clone, Debug, PartialEq)]
pub struct Aviso {
    pub texto: String,
    /// `true`: a chaveta não serviria como está, então a inserção é recusada
    pub bloqueia: bool,
}

/// Par eixo e cubo já pareado: os rasgos, a chaveta da tabela e os comprimentos que cabem.
#[derive(Clone, Debug, PartialEq)]
pub struct ParChaveta {
    pub eixo: Id,
    pub cubo: Id,
    pub diametro: f64,
    pub tabela: Chaveta,
    pub rasgo_eixo: RasgoLido,
    pub rasgo_cubo: RasgoLido,
    /// faixa em que a chaveta pode ficar, em Z local do eixo (sobreposição dos dois rasgos)
    pub z_ini: f64,
    pub z_fim: f64,
    /// comprimentos da série que cabem na faixa, do menor para o maior
    pub comprimentos: Vec<f64>,
    pub avisos: Vec<Aviso>,
}

impl ParChaveta {
    /// Primeiro motivo que impede a inserção.
    pub fn bloqueio(&self) -> Option<&str> {
        self.avisos.iter().find(|a| a.bloqueia).map(|a| a.texto.as_str())
    }
    /// Maior comprimento da série que cabe: o que a chaveta ocupa o rasgo todo.
    pub fn comprimento_padrao(&self) -> Option<f64> {
        self.comprimentos.last().copied()
    }
    /// Quanto o centro da chaveta pode se afastar do centro da faixa com o comprimento `l`.
    pub fn folga_de_posicao(&self, l: f64) -> f64 {
        ((self.z_fim - self.z_ini - l) / 2.).max(0.)
    }
}

fn parte(asm: &Assembly, id: Id) -> Result<&str, String> {
    match asm.node(id).map(|n| &n.kind) {
        Some(NodeKind::Part { path }) => Ok(path),
        Some(_) => Err("escolha uma peça (não um grupo nem um subconjunto)".into()),
        None => Err("componente não encontrado".into()),
    }
}

fn media(a: f64, b: f64) -> f64 {
    (a + b) / 2.
}

/// Pareia o eixo e o cubo. `Err` só quando nem dá para ler os rasgos (peça que não é um eixo do
/// Fenkra, sem rasgo, diâmetro fora da tabela); as demais discordâncias voltam em `avisos`.
pub fn parear(asm: &Assembly, dir: Option<&Path>, cache: &mut PartCache, eixo: Id, cubo: Id) -> Result<ParChaveta, String> {
    if eixo == cubo {
        return Err("escolha duas peças diferentes: o eixo e o cubo".into());
    }
    let (pe, pc) = (parte(asm, eixo)?.to_string(), parte(asm, cubo)?.to_string());
    let doc_e = cache.documento(&full_path(dir, &pe))?.clone();
    let doc_c = cache.documento(&full_path(dir, &pc))?.clone();
    let rasgos_e = rasgos_do_documento(&doc_e);
    let rasgos_c = rasgos_do_documento(&doc_c);
    if rasgos_e.is_empty() {
        return Err(format!("o eixo \"{}\" não tem rasgo de chaveta: crie um rasgo no trecho do eixo", asm.name_of(eixo)));
    }
    if rasgos_c.is_empty() {
        return Err(format!("o cubo \"{}\" não tem rasgo de chaveta: marque Rasgo de chaveta no cubo", asm.name_of(cubo)));
    }
    let (le, lc) = (asm.node(eixo).ok_or("eixo não encontrado")?.placement, asm.node(cubo).ok_or("cubo não encontrado")?.placement);

    // o par de rasgos que mais se encaixa: mesmo lado do eixo primeiro, depois a maior sobreposição
    let faixa_do_cubo = |r: &RasgoLido| {
        let (a, b) = (le.to_local(lc.point([0., 0., r.z0]))[2], le.to_local(lc.point([0., 0., r.z1]))[2]);
        (a.min(b), a.max(b))
    };
    let lado_do_cubo = |r: &RasgoLido| {
        let d = r.direcao();
        let v = le.dir_to_local(lc.dir([d[0], d[1], 0.]));
        v[1].atan2(v[0]).to_degrees()
    };
    let mut melhor: Option<(bool, f64, usize, usize)> = None;
    for (i, e) in rasgos_e.iter().enumerate() {
        for (j, c) in rasgos_c.iter().enumerate() {
            let (c0, c1) = faixa_do_cubo(c);
            let sobre = e.z1.min(c1) - e.z0.max(c0);
            let giro = (lado_do_cubo(c) - e.rotacao + 540.).rem_euclid(360.) - 180.;
            let mesmo_lado = giro.abs() < TOL_GIRO;
            let chave = (mesmo_lado, sobre);
            if melhor.is_none_or(|m| (m.0, m.1) < chave) {
                melhor = Some((mesmo_lado, sobre, i, j));
            }
        }
    }
    let (_, _, i, j) = melhor.expect("há ao menos um par");
    let (re, rc) = (rasgos_e[i].clone(), rasgos_c[j].clone());

    let mut avisos: Vec<Aviso> = vec![];
    let mut avisa = |texto: String, bloqueia: bool| avisos.push(Aviso { texto, bloqueia });

    let zm = media(re.z0, re.z1);
    let d = diametro_do_trecho(&doc_e, zm).ok_or_else(|| format!("o eixo \"{}\" não é um eixo escalonado do Fenkra: não achei o diâmetro do trecho do rasgo", asm.name_of(eixo)))?;
    let tabela = chaveta_para_eixo(d).ok_or_else(|| format!("Ø{d} está fora da tabela DIN 6885 (Ø6 a Ø85): não há chaveta padrão para este eixo"))?;
    let furo = furo_do_cubo(&doc_c).ok_or_else(|| format!("o cubo \"{}\" não tem o parâmetro furo: não sei o diâmetro do furo", asm.name_of(cubo)))?;
    if (furo - d).abs() > TOL {
        avisa(format!("o furo do cubo (Ø{furo}) é diferente do diâmetro do eixo (Ø{d})"), true);
    }

    // eixos coincidentes no mundo: o cubo precisa estar montado sobre o eixo
    let a_e = le.dir([0., 0., 1.]);
    let a_c = lc.dir([0., 0., 1.]);
    let cruz = norm(cross(a_e, a_c));
    let desvio = norm(cross(sub(lc.origin, le.origin), a_e));
    let coaxial = cruz < 1e-3 && desvio < 0.01;
    if !coaxial {
        avisa("o cubo não está alinhado com o eixo: crie antes a restrição de coincidência entre o furo e o eixo".into(), true);
    }

    // largura
    if (re.b - tabela.b).abs() > TOL_TABELA {
        avisa(format!("b do rasgo do eixo ({}) não casa com a tabela para Ø{d} ({})", re.b, tabela.b), true);
    }
    if (rc.b - tabela.b).abs() > TOL_TABELA {
        avisa(format!("b do rasgo do cubo ({}) não casa com a tabela para Ø{d} ({})", rc.b, tabela.b), true);
    }
    if (re.b - rc.b).abs() > TOL_TABELA {
        avisa(format!("b não casa entre o eixo ({}) e o cubo ({})", re.b, rc.b), true);
    }
    // profundidades
    let t1 = d / 2. - re.piso;
    let t2 = rc.teto - furo / 2.;
    if (t1 - tabela.t1).abs() > TOL_TABELA {
        avisa(format!("t1 do eixo ({t1:.2}) difere da tabela ({})", tabela.t1), (t1 - tabela.t1).abs() > 0.5);
    }
    if (t2 - tabela.t2).abs() > TOL_TABELA {
        avisa(format!("t2 do cubo ({t2:.2}) difere da tabela ({})", tabela.t2), (t2 - tabela.t2).abs() > 0.5);
    }
    if t1 + t2 < tabela.h - TOL {
        avisa(format!("t1 + t2 ({:.2}) é menor que a altura h da chaveta ({}): o topo da chaveta bateria no cubo", t1 + t2, tabela.h), true);
    }
    // lado e sobreposição
    let (c0, c1) = faixa_do_cubo(&rc);
    let giro = (lado_do_cubo(&rc) - re.rotacao + 540.).rem_euclid(360.) - 180.;
    if coaxial && giro.abs() > TOL_GIRO {
        avisa(format!("os rasgos estão em lados diferentes do eixo (diferença de {:.0}°): gire o cubo", giro.abs()), true);
    }
    let (z_ini, z_fim) = if coaxial { (re.z0.max(c0), re.z1.min(c1)) } else { (re.z0, re.z1) };
    let comprimentos: Vec<f64> = comprimentos_chaveta().iter().copied().filter(|l| *l > tabela.b + 1e-9 && *l <= z_fim - z_ini + 1e-9).collect();
    if comprimentos.is_empty() {
        avisa(format!("os rasgos se sobrepõem em {:.1} mm: não cabe nenhuma chaveta da série (a menor passa de {} mm)", (z_fim - z_ini).max(0.), tabela.b), true);
    }
    Ok(ParChaveta { eixo, cubo, diametro: d, tabela, rasgo_eixo: re, rasgo_cubo: rc, z_ini, z_fim, comprimentos, avisos })
}

/// Onde a chaveta entra e o que a prende ao eixo.
#[derive(Clone, Debug, PartialEq)]
pub struct Colocacao {
    pub comprimento: f64,
    /// posição do centro da chaveta ao longo do eixo (Z local do eixo)
    pub z_centro: f64,
    /// posição da chaveta no mundo
    pub lugar: Placement,
    /// restrições (a = chaveta, b = eixo): contato com o piso, plano médio longitudinal e transversal
    pub restricoes: [(MateKind, Geom, Geom); 3],
}

/// Conta da colocação, sem tocar na montagem: a inserção e o fantasma usam esta mesma conta.
/// `comprimento` fora da série ou que não cabe é erro; `deslocamento` move a chaveta ao longo do
/// eixo e é limitado ao que cabe nos dois rasgos.
pub fn colocar(asm: &Assembly, par: &ParChaveta, comprimento: f64, deslocamento: f64) -> Result<Colocacao, String> {
    let l = par.comprimentos.iter().copied().find(|x| (x - comprimento).abs() < 1e-9).ok_or_else(|| format!("comprimento {comprimento} fora da série que cabe nos rasgos"))?;
    let le = asm.node(par.eixo).ok_or("eixo não encontrado")?.placement;
    let folga = par.folga_de_posicao(l);
    let zc = media(par.z_ini, par.z_fim) + deslocamento.clamp(-folga, folga);
    let (c, s) = {
        let u = par.rasgo_eixo.direcao();
        (u[0], u[1])
    };
    let f = par.rasgo_eixo.piso;
    // eixos da chaveta no sistema do eixo: X → −Z (comprimento), Y → tangente, Z → radial (altura)
    let u = [c, s, 0.];
    let v = [-s, c, 0.];
    let interna = Placement { origin: [f * c, f * s, zc], rot: [[0., 0., -1.], v, u] };
    let lugar = le.then(&interna);
    let chav = |p: V3, n: V3| Geom::Plane { p, n };
    let restricoes = [
        (MateKind::Contact, chav([0.; 3], [0., 0., -1.]), chav([f * c, f * s, zc], u)),
        (MateKind::Coincident { flip: false }, chav([0.; 3], [0., 1., 0.]), chav([0.; 3], v)),
        (MateKind::Coincident { flip: false }, chav([0.; 3], [1., 0., 0.]), chav([0., 0., zc], [0., 0., -1.])),
    ];
    Ok(Colocacao { comprimento: l, z_centro: zc, lugar, restricoes })
}

/// Resultado da inserção.
#[derive(Clone, Debug, PartialEq)]
pub struct RelatorioChaveta {
    pub chaveta: Id,
    pub comprimento: f64,
    pub tabela: Chaveta,
    pub restricoes: [Id; 3],
    pub avisos: Vec<String>,
}

/// Já existe uma chaveta presa a este rasgo do eixo?
fn ja_tem_chaveta(asm: &Assembly, par: &ParChaveta) -> bool {
    let u = par.rasgo_eixo.direcao();
    asm.mates.iter().filter(|m| !m.suppressed && m.kind == MateKind::Contact && m.b.comp == par.eixo).any(|m| {
        let chaveta = matches!(asm.node(m.a.comp).map(|n| &n.kind), Some(NodeKind::Part { path }) if path.starts_with("chavetas/"));
        let Geom::Plane { n, p } = m.b.geom else { return false };
        chaveta && (n[0] - u[0]).abs() < 1e-6 && (n[1] - u[1]).abs() < 1e-6 && p[2] >= par.z_ini - 1e-6 && p[2] <= par.z_fim + 1e-6
    })
}

/// Insere a chaveta: grava a peça em `<pasta>/chavetas/` (reaproveita se já existe), põe o componente
/// no lugar e cria as três restrições. `comprimento` `None` usa o maior da série que cabe.
pub fn inserir_chaveta(asm: &mut Assembly, dir: Option<&Path>, cache: &mut PartCache, eixo: Id, cubo: Id, comprimento: Option<f64>, deslocamento: f64) -> Result<RelatorioChaveta, String> {
    let dir = dir.ok_or("salve a montagem antes: a peça da chaveta é gravada na pasta dela")?;
    let par = parear(asm, Some(dir), cache, eixo, cubo)?;
    if let Some(b) = par.bloqueio() {
        return Err(b.to_string());
    }
    if ja_tem_chaveta(asm, &par) {
        return Err("já existe uma chaveta presa a este rasgo".into());
    }
    let l = comprimento.or(par.comprimento_padrao()).ok_or("nenhum comprimento da série cabe nos rasgos")?;
    let col = colocar(asm, &par, l, deslocamento)?;
    let doc = chaveta_documento(par.diametro, col.comprimento)?;
    let base = format!("chaveta-din6885a-d{}-l{}", par.diametro, col.comprimento);
    let arq = dir.join("chavetas").join(format!("{base}.fkp"));
    if !arq.exists() {
        std::fs::create_dir_all(dir.join("chavetas")).map_err(|e| format!("{}: {e}", dir.join("chavetas").display()))?;
        std::fs::write(&arq, doc.to_json()).map_err(|e| format!("{}: {e}", arq.display()))?;
    }
    let pasta = match asm.nodes.iter().find(|n| matches!(n.kind, NodeKind::Group) && n.name == GRUPO).map(|n| n.id) {
        Some(g) => g,
        None => asm.add_group(GRUPO, None),
    };
    let id = asm.add_component(&doc.props.description, NodeKind::Part { path: format!("chavetas/{base}.fkp") }, col.lugar, Some(pasta));
    let ref_a = |comp: Id, geom: Geom| MateRef { comp, geom, pick: None };
    let mut ids = [Id(0); 3];
    for (k, (kind, a, b)) in col.restricoes.iter().enumerate() {
        ids[k] = asm.add_mate(kind.clone(), ref_a(id, *a), ref_a(eixo, *b));
    }
    let avisos = par.avisos.iter().map(|a| a.texto.clone()).collect();
    Ok(RelatorioChaveta { chaveta: id, comprimento: col.comprimento, tabela: par.tabela, restricoes: ids, avisos })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interferencia::{analisar, OpcoesInterf};
    use fenkra_solidos::eixos::{eixo_documento, ParamEixo, RasgoChaveta, TrechoEixo};
    use fenkra_solidos::engrenagens::{engrenagem_documento, ParamEngrenagem};

    fn eixo_param(d: f64) -> ParamEixo {
        ParamEixo {
            trechos: vec![
                TrechoEixo { diametro: d - 5., comprimento: 40., chanfro_fim: 0., rasgo: None },
                TrechoEixo { diametro: d, comprimento: 80., chanfro_fim: 0., rasgo: Some(RasgoChaveta { comprimento: 50., desloc_inicio: 10. }) },
            ],
            furo_centro: false,
        }
    }

    fn pasta(nome: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("fenkra_chaveta_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Eixo Ø`d` fixo e engrenagem de furo Ø`furo` com rasgo de cubo; o cubo fica no trecho do rasgo.
    fn conjunto(nome: &str, d: f64, furo: f64, largura: f64, z_cubo: f64) -> (Assembly, std::path::PathBuf, Id, Id) {
        let dir = pasta(nome);
        std::fs::write(dir.join("eixo.fkp"), eixo_documento(&eixo_param(d)).unwrap().to_json()).unwrap();
        let p = ParamEngrenagem { furo, largura, rasgo_cubo: true, dentes: 40, ..Default::default() };
        std::fs::write(dir.join("cubo.fkp"), engrenagem_documento(&p).unwrap().to_json()).unwrap();
        let mut a = Assembly::new("t");
        let e = a.add_component("Eixo", NodeKind::Part { path: "eixo.fkp".into() }, Placement::default(), None);
        a.node_mut(e).unwrap().fixed = true;
        let c = a.add_component("Cubo", NodeKind::Part { path: "cubo.fkp".into() }, Placement { origin: [0., 0., z_cubo], ..Default::default() }, None);
        (a, dir, e, c)
    }

    #[test]
    fn faixa_e_comprimentos_vem_da_sobreposicao_dos_rasgos() {
        let (a, dir, e, c) = conjunto("faixa", 25., 25., 30., 55.);
        let par = parear(&a, Some(&dir), &mut PartCache::default(), e, c).unwrap();
        assert_eq!(par.diametro, 25.);
        assert_eq!((par.tabela.b, par.tabela.h), (8., 7.));
        // rasgo do eixo de z = 50 + 10 = 50 .. 100? (trecho 2 começa em 40): 50 a 100; cubo em 55..85
        assert!((par.rasgo_eixo.z0 - 50.).abs() < 1e-9 && (par.rasgo_eixo.z1 - 100.).abs() < 1e-9, "{:?}", par.rasgo_eixo);
        assert!((par.z_ini - 54.5).abs() < 1e-9 && (par.z_fim - 85.5).abs() < 1e-9, "{} {}", par.z_ini, par.z_fim);
        assert!(par.bloqueio().is_none(), "{:?}", par.avisos);
        assert_eq!(par.comprimento_padrao(), Some(28.), "{:?}", par.comprimentos);
        assert!(par.comprimentos.iter().all(|l| *l > 8. && *l <= 31.));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn avisa_quando_o_diametro_o_furo_ou_o_alinhamento_nao_casam() {
        // furo diferente do eixo
        let (a, dir, e, c) = conjunto("furo", 25., 22., 30., 55.);
        let par = parear(&a, Some(&dir), &mut PartCache::default(), e, c).unwrap();
        let b = par.bloqueio().expect("bloqueia");
        assert!(b.contains("furo") || b.contains('b'), "{b}");
        assert!(par.avisos.iter().any(|x| x.texto.contains("diferente do diâmetro")), "{:?}", par.avisos);
        // b do cubo (furo 22 → 6) difere do do eixo (8)
        assert!(par.avisos.iter().any(|x| x.texto.contains("b não casa")), "{:?}", par.avisos);
        // cubo fora do eixo
        let (mut a, dir2, e, c) = conjunto("fora", 25., 25., 30., 55.);
        a.node_mut(c).unwrap().placement.origin = [3., 0., 55.];
        let par = parear(&a, Some(&dir2), &mut PartCache::default(), e, c).unwrap();
        assert!(par.bloqueio().unwrap().contains("alinhado"), "{:?}", par.avisos);
        // cubo girado 90°: rasgos em lados diferentes
        let (mut a, dir3, e, c) = conjunto("giro", 25., 25., 30., 55.);
        a.node_mut(c).unwrap().placement.rot = [[0., 1., 0.], [-1., 0., 0.], [0., 0., 1.]];
        let par = parear(&a, Some(&dir3), &mut PartCache::default(), e, c).unwrap();
        assert!(par.bloqueio().unwrap().contains("lados diferentes"), "{:?}", par.avisos);
        // eixo sem trecho no cubo: sobreposição curta
        let (a, dir4, e, c) = conjunto("curto", 25., 25., 6., 55.);
        let par = parear(&a, Some(&dir4), &mut PartCache::default(), e, c).unwrap();
        assert!(par.bloqueio().unwrap().contains("não cabe nenhuma"), "{:?}", par.avisos);
        for d in [dir, dir2, dir3, dir4] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    #[test]
    fn erros_de_leitura() {
        // eixo sem rasgo
        let (a, dir, e, c) = conjunto("tabela", 25., 25., 30., 55.);
        let grande = ParamEixo { trechos: vec![TrechoEixo { diametro: 90., comprimento: 60., chanfro_fim: 0., rasgo: None }], furo_centro: false };
        std::fs::write(dir.join("eixo.fkp"), eixo_documento(&grande).unwrap().to_json()).unwrap();
        let err = parear(&a, Some(&dir), &mut PartCache::default(), e, c).unwrap_err();
        assert!(err.contains("não tem rasgo"), "{err}");
        // o mesmo componente duas vezes
        assert!(parear(&a, Some(&dir), &mut PartCache::default(), e, e).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn chaveta_entra_com_gdl_zero_e_sem_interferencia() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut a, dir, e, c) = conjunto("gdl", 25., 25., 30., 55.);
        let mut cache = PartCache::default();
        let r = inserir_chaveta(&mut a, Some(&dir), &mut cache, e, c, Some(25.), 0.).unwrap();
        assert_eq!(r.comprimento, 25.);
        assert_eq!((r.tabela.b, r.tabela.h), (8., 7.));
        assert!(dir.join("chavetas").join("chaveta-din6885a-d25-l25.fkp").exists());
        // o cubo fica livre (a montagem do exemplo não o prende); a chaveta fica sem graus de liberdade
        let graus = crate::solve::dof_per_comp(&a);
        assert_eq!(graus[&r.chaveta], 0, "chaveta com GDL zero");
        // resolver não a move: a posição calculada já satisfaz as restrições
        let antes = a.node(r.chaveta).unwrap().placement;
        crate::solve::solve(&mut a);
        let depois = a.node(r.chaveta).unwrap().placement;
        assert!(norm(sub(antes.origin, depois.origin)) < 1e-4, "{:?} {:?}", antes.origin, depois.origin);
        // base no piso do rasgo (x = 12,5 - 4 = 8,5), topo a 7 mm acima, centro em z = (54,5 + 85,5)/2 = 70
        assert!((antes.origin[0] - 8.5).abs() < 1e-9 && (antes.origin[2] - 70.).abs() < 1e-9, "{:?}", antes.origin);
        // o conjunto passa na verificação de interferência
        let achados = analisar(&a, Some(&dir), &mut cache, &OpcoesInterf::default());
        assert!(achados.is_empty(), "interferências: {:?}", achados.iter().map(|x| (a.name_of(x.a.comp), a.name_of(x.b.comp), x.volume)).collect::<Vec<_>>());
        // controle: afundar a chaveta 1 mm no eixo tem de aparecer como interferência (o teste não é vazio)
        let mut b = a.clone();
        b.node_mut(r.chaveta).unwrap().placement.origin[0] -= 1.;
        assert!(!analisar(&b, Some(&dir), &mut cache, &OpcoesInterf::default()).is_empty(), "o controle precisa interferir");
        // inserir de novo no mesmo rasgo é recusado
        assert!(inserir_chaveta(&mut a, Some(&dir), &mut cache, e, c, Some(25.), 0.).unwrap_err().contains("já existe"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn chaveta_acompanha_o_eixo_movido_e_o_deslocamento_e_limitado() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut a, dir, e, c) = conjunto("mover", 25., 25., 30., 55.);
        // eixo deslocado e cubo junto: o conjunto inteiro anda 100 mm em Y
        for id in [e, c] {
            a.node_mut(id).unwrap().placement.origin[1] += 100.;
        }
        let mut cache = PartCache::default();
        // deslocamento pedido maior que a folga é limitado: 28 mm cabe em 31 → folga de 1,5
        let r = inserir_chaveta(&mut a, Some(&dir), &mut cache, e, c, Some(28.), 50.).unwrap();
        let p = a.node(r.chaveta).unwrap().placement;
        assert!((p.origin[1] - 100.).abs() < 1e-9 && (p.origin[2] - (70. + 1.5)).abs() < 1e-9, "{:?}", p.origin);
        assert_eq!(crate::solve::dof_per_comp(&a)[&r.chaveta], 0);
        let achados = analisar(&a, Some(&dir), &mut cache, &OpcoesInterf::default());
        assert!(achados.is_empty(), "{:?}", achados.len());
        // comprimento fora da série
        let (mut a2, dir2, e2, c2) = conjunto("serie", 25., 25., 30., 55.);
        assert!(inserir_chaveta(&mut a2, Some(&dir2), &mut cache, e2, c2, Some(27.), 0.).is_err());
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(dir2);
    }

    #[test]
    fn rasgo_girado_do_eixo_leva_a_chaveta_para_o_lado_certo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        use fenkra_solidos::eixos_recursos::{eixo_documento_com_recursos, Borda, RecursoEixo, TipoRecurso};
        let dir = pasta("girado");
        let p = ParamEixo { trechos: vec![TrechoEixo { diametro: 25., comprimento: 90., chanfro_fim: 0., rasgo: None }], furo_centro: false };
        let rec = RecursoEixo { trecho: 0, borda: Borda::Esquerda, posicao: 20., tipo: TipoRecurso::Rasgo { comprimento: 50., rotacao: 90. } };
        std::fs::write(dir.join("eixo.fkp"), eixo_documento_com_recursos(&p, &[rec]).unwrap().to_json()).unwrap();
        let g = ParamEngrenagem { furo: 25., largura: 30., rasgo_cubo: true, dentes: 40, ..Default::default() };
        std::fs::write(dir.join("cubo.fkp"), engrenagem_documento(&g).unwrap().to_json()).unwrap();
        let mut a = Assembly::new("t");
        let e = a.add_component("Eixo", NodeKind::Part { path: "eixo.fkp".into() }, Placement::default(), None);
        a.node_mut(e).unwrap().fixed = true;
        // o cubo gira 90° para o rasgo dele cair no lado +Y do eixo
        let c = a.add_component("Cubo", NodeKind::Part { path: "cubo.fkp".into() }, Placement { origin: [0., 0., 30.], rot: [[0., 1., 0.], [-1., 0., 0.], [0., 0., 1.]] }, None);
        let mut cache = PartCache::default();
        let r = inserir_chaveta(&mut a, Some(&dir), &mut cache, e, c, None, 0.).unwrap();
        let p = a.node(r.chaveta).unwrap().placement;
        assert!(p.origin[0].abs() < 1e-9 && (p.origin[1] - 8.5).abs() < 1e-9, "a chaveta fica no +Y: {:?}", p.origin);
        assert_eq!(crate::solve::dof_per_comp(&a)[&r.chaveta], 0);
        assert!(analisar(&a, Some(&dir), &mut cache, &OpcoesInterf::default()).is_empty(), "sem interferência com o rasgo girado");
        let mut b = a.clone();
        b.node_mut(r.chaveta).unwrap().placement.origin[1] -= 1.;
        assert!(!analisar(&b, Some(&dir), &mut cache, &OpcoesInterf::default()).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn toda_a_tabela_deixa_folga_entre_a_chaveta_e_o_cubo() {
        for f in fenkra_solidos::biblioteca::tabela_chavetas().faixas.iter() {
            assert!(f.t1 + f.t2 >= f.h - 1e-9, "b{} h{}: t1 + t2 = {} < h", f.b, f.h, f.t1 + f.t2);
        }
    }
}
