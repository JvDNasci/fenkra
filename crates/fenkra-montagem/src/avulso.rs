// SPDX-License-Identifier: LGPL-2.1-or-later
//! Item normalizado avulso (B242): um parafuso, porca ou arruela da biblioteca que a pessoa leva do
//! catálogo até um furo da montagem.
//!
//! Como funciona:
//! 1. [`furos_do_mundo`] lista os furos das peças visíveis já no mundo (boca, sentido para dentro,
//!    diâmetro, espessura); é nesses furos que o fantasma pode saltar;
//! 2. [`casa`] diz se um tamanho serve a um furo: a faixa de folga do tamanho contém o diâmetro, ou,
//!    em furo roscado, a rosca do furo é a do tamanho;
//! 3. [`pose`] dá a boca de entrada e o sentido do item. `inverter` (Tab na viewport) entra pela outra
//!    ponta do furo passante;
//! 4. [`inserir_avulso`] grava a peça (compartilhada ou cópia) e a põe na montagem com as mesmas três
//!    restrições do preenchimento de furos: concêntrico, contato na face e trava de giro. As três
//!    tiram os 6 graus de liberdade do item.
//!
//! O modo **padrão** reaproveita a peça `fixadores/<família>_<tamanho>[x<L>].fkp` (todos os itens
//! iguais apontam para o mesmo arquivo); o **personalizado** grava uma cópia só dele, que a pessoa
//! pode editar sem mexer nos outros.

use crate::fixadores::{apoio, arquivo_fixador, concentrico, espessura_ao_longo, mais_proximo, num, orientado, tamanho_da_rosca, trava_de_giro, GRUPO};
use crate::load::{full_path, PartCache};
use crate::*;
use fenkra_solidos::biblioteca::{Biblioteca, Familia, Tamanho};
use std::collections::HashMap;
use std::path::Path;

/// Folga de diâmetro entre o furo e a faixa do tamanho (mm).
const TOL: f64 = 0.05;

/// Peça compartilhada ou cópia própria.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModoInsercao {
    #[default]
    Padrao,
    Personalizado,
}

/// Um furo de uma peça, no mundo, que pode receber um item avulso.
#[derive(Clone, Debug, PartialEq)]
pub struct FuroAlvo {
    pub comp: Id,
    pub nome: String,
    /// centro da boca de entrada
    pub boca: V3,
    /// sentido da boca para dentro do material (unitário)
    pub dir: V3,
    pub diametro: f64,
    /// comprimento útil ao longo de `dir` (espessura, ou profundidade do roscado cego)
    pub espessura: f64,
    /// atravessa a peça: só nele se entra pela outra ponta
    pub passante: bool,
    /// rosca do furo ("M6"); vazia no furo liso
    pub rosca: String,
}

/// Furos das peças visíveis (só as de `comps`, quando não vazio), no mundo. Furo cego liso não entra:
/// um parafuso ali não teria onde prender.
pub fn furos_do_mundo(asm: &Assembly, dir: Option<&Path>, cache: &mut PartCache, comps: &[Id]) -> Vec<FuroAlvo> {
    let mut v = vec![];
    let pecas: Vec<(Id, String, Placement, String)> = asm
        .nodes
        .iter()
        .filter(|n| n.visible && (comps.is_empty() || comps.contains(&n.id)))
        .filter_map(|n| match &n.kind {
            NodeKind::Part { path } => Some((n.id, n.name.clone(), n.placement, path.clone())),
            _ => None,
        })
        .collect();
    for (id, nome, pl, path) in pecas {
        let Ok(lp) = cache.get(&full_path(dir, &path), true) else { continue };
        for r in fenkra_solidos::hole_table(&lp.doc) {
            let roscado = !r.thread.trim().is_empty();
            if !roscado && !r.through {
                continue;
            }
            let espessura = if roscado && !r.through { Some(r.depth) } else { lp.shape.as_ref().and_then(|s| espessura_ao_longo(s, r.pos, r.dir, r.diameter)) };
            let Some(espessura) = espessura.filter(|e| *e > 1e-6) else { continue };
            v.push(FuroAlvo { comp: id, nome: nome.clone(), boca: pl.point(r.pos), dir: unit(pl.dir(r.dir)), diametro: r.diameter, espessura, passante: r.through, rosca: r.thread.trim().to_string() });
        }
    }
    v
}

/// O tamanho serve ao furo? Liso: o diâmetro cai na faixa do tamanho (fina a folgada, ou a faixa que a
/// família declara). Roscado: a rosca do furo é a do tamanho.
pub fn casa(fam: &Familia, t: &Tamanho, f: &FuroAlvo) -> bool {
    if !f.rosca.is_empty() {
        return tamanho_da_rosca(fam, &f.rosca).is_some_and(|d| d == t.designacao);
    }
    fam.faixa_furo(t).is_some_and(|(a, b)| f.diametro >= a - TOL && f.diametro <= b + TOL)
}

/// Tamanhos da família que servem ao furo, do menor para o maior.
pub fn tamanhos_que_casam<'a>(fam: &'a Familia, f: &FuroAlvo) -> Vec<&'a Tamanho> {
    let mut v: Vec<&Tamanho> = fam.tamanhos.iter().filter(|t| casa(fam, t, f)).collect();
    v.sort_by(|a, b| a.diametro_nominal().partial_cmp(&b.diametro_nominal()).unwrap_or(std::cmp::Ordering::Equal));
    v
}

/// Boca de entrada do item e sentido de entrada. Sem `inverter`: a boca do furo e o sentido dele.
/// Com `inverter`: a saída do furo e o sentido oposto. Só furo passante inverte.
pub fn pose(f: &FuroAlvo, inverter: bool) -> Result<(V3, V3), String> {
    if !inverter {
        return Ok((f.boca, f.dir));
    }
    if !f.passante {
        return Err("furo cego: o item só entra pela boca".into());
    }
    let saida = [0, 1, 2].map(|i| f.boca[i] + f.dir[i] * f.espessura);
    Ok((saida, f.dir.map(|x| -x)))
}

/// Comprimento que a série do tamanho oferece para o furo: no passante, o menor L que atravessa a
/// espessura com dois passos de rosca sobrando (ou o maior da série); no roscado cego, o maior L que
/// cabe na profundidade.
pub fn comprimento_sugerido(fam: &Familia, t: &Tamanho, f: &FuroAlvo) -> Option<f64> {
    let serie = fam.comprimentos_de(t);
    let passo = t.valor("P").unwrap_or(0.);
    if !f.rosca.is_empty() && !f.passante {
        return serie.iter().copied().filter(|l| *l <= f.espessura + 1e-9).last().or(serie.first().copied());
    }
    serie.iter().copied().find(|l| *l >= f.espessura + 2. * passo - 1e-9).or(serie.last().copied())
}

/// Onde o item fica no mundo: origem na boca de entrada. Parafuso com Z = -u (cabeça para fora, corpo
/// para dentro); porca e arruela com Z = u (o corpo fica do lado de fora da peça). É a pose do
/// fantasma e a da peça inserida.
pub fn posicao_do_item(parafuso: bool, entrada: V3, u: V3) -> Placement {
    orientado(entrada, if parafuso { u.map(|x| -x) } else { u })
}

#[derive(Clone, Debug)]
pub struct ParamAvulso {
    pub familia: String,
    pub tamanho: String,
    /// comprimento pedido (parafusos); vale o valor da série mais próximo. `None`: o sugerido
    pub comprimento: Option<f64>,
    pub furo: FuroAlvo,
    pub inverter: bool,
    pub modo: ModoInsercao,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Inserido {
    pub comp: Id,
    /// caminho da peça relativo à montagem
    pub arquivo: String,
    pub nome: String,
    /// boca de entrada e sentido (da boca para dentro) usados
    pub entrada: V3,
    pub eixo: V3,
    pub comprimento: Option<f64>,
    /// restrições criadas (concêntrica, trava de giro, contato)
    pub restricoes: Vec<Id>,
}

/// Grava uma cópia própria da peça, com número livre na pasta `fixadores/`.
fn arquivo_copia(dir: &Path, fam: &Familia, t: &Tamanho, l: Option<f64>) -> Result<(String, String), String> {
    let base = match l {
        Some(l) => format!("{}_{}x{}", fam.id, t.designacao, num(l)),
        None => format!("{}_{}", fam.id, t.designacao),
    };
    let pasta = dir.join("fixadores");
    std::fs::create_dir_all(&pasta).map_err(|e| format!("{}: {e}", pasta.display()))?;
    let n = (1..).find(|n| !pasta.join(format!("{base}_copia{n}.fkp")).exists()).unwrap_or(1);
    let mut doc = fenkra_solidos::fixadores::gerar(fam, &t.designacao, l)?;
    doc.props.description = format!("{} (cópia {n})", doc.props.description);
    let nome = doc.props.description.clone();
    let arq = format!("fixadores/{base}_copia{n}.fkp");
    std::fs::write(dir.join(&arq), doc.to_json()).map_err(|e| format!("{arq}: {e}"))?;
    Ok((arq, nome))
}

/// Põe o item no furo, restrito. Erro com a frase do problema (tamanho que não serve, família
/// desconhecida, furo cego invertido).
pub fn inserir_avulso(asm: &mut Assembly, dir: &Path, lib: &Biblioteca, p: &ParamAvulso) -> Result<Inserido, String> {
    let fam = lib.familia(&p.familia).ok_or_else(|| format!("família desconhecida: {}", p.familia))?;
    let t = fam.tamanho(&p.tamanho).ok_or_else(|| format!("{}: não existe o tamanho {}", fam.norma, p.tamanho))?;
    if !casa(fam, t, &p.furo) {
        return Err(format!("{} {} não serve ao furo Ø{} de {}", fam.norma, t.designacao, num(p.furo.diametro), p.furo.nome));
    }
    if asm.node(p.furo.comp).is_none() {
        return Err("a peça do furo não está mais na montagem".into());
    }
    let (entrada, u) = pose(&p.furo, p.inverter)?;
    let comprimento = if fam.tipo.e_parafuso() {
        let serie = fam.comprimentos_de(t);
        Some(match p.comprimento {
            Some(l) => mais_proximo(&serie, l).ok_or("tamanho sem série de comprimentos")?,
            None => comprimento_sugerido(fam, t, &p.furo).ok_or("tamanho sem série de comprimentos")?,
        })
    } else {
        None
    };
    let (arquivo, nome) = match p.modo {
        ModoInsercao::Padrao => arquivo_fixador(dir, fam, t, comprimento, &mut HashMap::new(), &mut vec![])?,
        ModoInsercao::Personalizado => arquivo_copia(dir, fam, t, comprimento)?,
    };
    let grupo = match asm.nodes.iter().find(|n| matches!(n.kind, NodeKind::Group) && n.name == GRUPO).map(|n| n.id) {
        Some(g) => g,
        None => asm.add_group(GRUPO, None),
    };
    let baixo = u.map(|x| -x);
    // parafuso: Z = -u (cabeça para fora, corpo para dentro); porca e arruela: Z = u (corpo para fora da peça)
    let pl = posicao_do_item(fam.tipo.e_parafuso(), entrada, u);
    let comp = asm.add_component(&nome, NodeKind::Part { path: arquivo.clone() }, pl, Some(grupo));
    let antes: Vec<Id> = asm.mates.iter().map(|m| m.id).collect();
    let eixo = (entrada, baixo);
    concentrico(asm, p.furo.comp, comp, eixo);
    trava_de_giro(asm, p.furo.comp, comp, eixo);
    apoio(asm, p.furo.comp, (entrada, baixo), comp, (entrada, u));
    let restricoes = asm.mates.iter().map(|m| m.id).filter(|i| !antes.contains(i)).collect();
    Ok(Inserido { comp, arquivo, nome, entrada, eixo: u, comprimento, restricoes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixadores::tests::{pasta, placa};
    use fenkra_solidos::biblioteca;

    fn montagem(dir: &Path, esp: f64, furo: f64) -> (Assembly, Id) {
        std::fs::write(dir.join("placa.fkp"), placa(esp, furo, None).to_json()).unwrap();
        let mut a = Assembly::new("Avulso");
        let c = a.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement::default(), None);
        (a, c)
    }

    fn param(furo: FuroAlvo, familia: &str, tamanho: &str) -> ParamAvulso {
        ParamAvulso { familia: familia.into(), tamanho: tamanho.into(), comprimento: None, furo, inverter: false, modo: ModoInsercao::Padrao }
    }

    #[test]
    fn furo_da_placa_aparece_no_mundo_e_so_o_m6_serve() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_furos");
        let (a, c) = montagem(&dir, 10., 6.6);
        let furos = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]);
        let [f] = &furos[..] else { panic!("{furos:?}") };
        assert_eq!((f.comp, f.diametro, f.passante), (c, 6.6, true));
        assert!(f.boca[0].hypot(f.boca[1]) < 1e-6 && (f.boca[2] - 10.).abs() < 1e-6 && f.dir[2] < -0.99, "{f:?}");
        assert!((f.espessura - 10.).abs() < 0.01, "{}", f.espessura);
        let fam = biblioteca::familia("iso4762").unwrap();
        let ok: Vec<&str> = tamanhos_que_casam(fam, f).iter().map(|t| t.designacao.as_str()).collect();
        assert_eq!(ok, ["M6"], "só o M6 tem 6,6 na faixa de folga");
        // comprimento sugerido: 10 + 2 passos = 12 → L12 ou o próximo da série
        let l = comprimento_sugerido(fam, fam.tamanho("M6").unwrap(), f).unwrap();
        assert!(l >= 12. && fam.comprimentos_de(fam.tamanho("M6").unwrap()).contains(&l), "{l}");
    }

    #[test]
    fn parafuso_avulso_entra_restrito_com_zero_gdl_na_posicao_e_no_eixo_do_furo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_gdl");
        let (mut a, c) = montagem(&dir, 10., 6.6);
        let f = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]).remove(0);
        let antes = a.clone();
        let r = inserir_avulso(&mut a, &dir, biblioteca::publica(), &param(f, "iso4762", "M6")).unwrap();
        assert_eq!(r.restricoes.len(), 3, "concêntrico, trava de giro e contato");
        assert_eq!(a.nodes.len(), antes.nodes.len() + 2, "grupo Fixadores e a peça");
        let rep = solve::solve(&mut a);
        assert!(rep.converged && rep.dof == 0, "{rep:?}");
        assert_eq!(solve::dof_per_comp(&a).get(&r.comp).copied().unwrap_or(0), 0, "o parafuso fica sem graus de liberdade");
        // posição e eixo: no eixo do furo, cabeça apoiada na face de cima (z = 10), corpo para baixo
        let p = a.node(r.comp).unwrap().placement;
        assert!(p.origin[0].hypot(p.origin[1]) < 0.01 && (p.origin[2] - 10.).abs() < 0.01, "{:?}", p.origin);
        assert!(norm(sub(p.dir([0., 0., 1.]), [0., 0., 1.])) < 1e-6, "Z local aponta para a cabeça (para cima)");
        assert!(a.node(c).unwrap().placement == antes.node(c).unwrap().placement, "a placa não se mexe");
        // o arquivo é o da biblioteca, compartilhado
        assert_eq!(r.arquivo, format!("fixadores/iso4762_M6x{}.fkp", num(r.comprimento.unwrap())));
        assert!(dir.join(&r.arquivo).exists());
        // desfazer = voltar ao estado de antes (a janela guarda a cópia): sem a peça e sem restrições
        assert_eq!((antes.nodes.len(), antes.mates.len()), (1, 0));
        assert!(a.mates.len() == 3 && antes != a);
    }

    #[test]
    fn tab_inverte_o_sentido_e_o_item_entra_pela_saida_do_furo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_inverter");
        let (mut a, _) = montagem(&dir, 10., 6.6);
        let f = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]).remove(0);
        let (entrada, u) = pose(&f, true).unwrap();
        assert!(entrada[2].abs() < 0.01 && u[2] > 0.99, "{entrada:?} {u:?}");
        let mut p = param(f, "iso4762", "M6");
        p.inverter = true;
        let r = inserir_avulso(&mut a, &dir, biblioteca::publica(), &p).unwrap();
        let rep = solve::solve(&mut a);
        assert!(rep.converged && rep.dof == 0, "{rep:?}");
        let pl = a.node(r.comp).unwrap().placement;
        // cabeça embaixo (z = 0), Z local para baixo, corpo para cima
        assert!(pl.origin[2].abs() < 0.01, "{:?}", pl.origin);
        assert!(norm(sub(pl.dir([0., 0., 1.]), [0., 0., -1.])) < 1e-6);
    }

    #[test]
    fn tamanho_errado_e_furo_cego_invertido_dao_erro_sem_mexer_na_montagem() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_erros");
        let (mut a, _) = montagem(&dir, 10., 6.6);
        let f = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]).remove(0);
        let antes = a.clone();
        let e = inserir_avulso(&mut a, &dir, biblioteca::publica(), &param(f.clone(), "iso4762", "M8")).unwrap_err();
        assert!(e.contains("não serve"), "{e}");
        let e = inserir_avulso(&mut a, &dir, biblioteca::publica(), &param(f.clone(), "naoexiste", "M6")).unwrap_err();
        assert!(e.contains("desconhecida"), "{e}");
        assert_eq!(a, antes);
        let cego = FuroAlvo { passante: false, ..f };
        assert!(pose(&cego, true).is_err());
    }

    #[test]
    fn modo_padrao_compartilha_a_peca_e_o_personalizado_grava_copia() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_modos");
        let (mut a, _) = montagem(&dir, 10., 6.6);
        let f = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]).remove(0);
        let lib = biblioteca::publica();
        let r1 = inserir_avulso(&mut a, &dir, lib, &param(f.clone(), "iso4762", "M6")).unwrap();
        let r2 = inserir_avulso(&mut a, &dir, lib, &param(f.clone(), "iso4762", "M6")).unwrap();
        assert_eq!(r1.arquivo, r2.arquivo, "padrão: mesma peça compartilhada");
        let mut p = param(f.clone(), "iso4762", "M6");
        p.modo = ModoInsercao::Personalizado;
        let c1 = inserir_avulso(&mut a, &dir, lib, &p).unwrap();
        let c2 = inserir_avulso(&mut a, &dir, lib, &p).unwrap();
        assert_ne!(c1.arquivo, r1.arquivo);
        assert_ne!(c1.arquivo, c2.arquivo, "cada cópia tem o seu arquivo");
        assert!(c1.arquivo.contains("_copia1") && c2.arquivo.contains("_copia2"), "{} {}", c1.arquivo, c2.arquivo);
        assert!(dir.join(&c1.arquivo).exists() && dir.join(&c2.arquivo).exists());
    }

    #[test]
    fn arruela_avulsa_assenta_na_boca_do_furo() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let dir = pasta("avulso_arruela");
        let (mut a, _) = montagem(&dir, 10., 6.6);
        let f = furos_do_mundo(&a, Some(&dir), &mut PartCache::default(), &[]).remove(0);
        let ar = inserir_avulso(&mut a, &dir, biblioteca::publica(), &param(f, "iso7089", "M6")).unwrap();
        let rep = solve::solve(&mut a);
        assert!(rep.converged && rep.dof == 0, "{rep:?}");
        assert_eq!(ar.comprimento, None);
        let pl = a.node(ar.comp).unwrap().placement;
        // arruela fora da peça: face de apoio em z = 10, Z local para dentro do furo (para baixo), de
        // modo que o corpo (de -h a 0 no eixo dela) fique acima da face
        assert!((pl.origin[2] - 10.).abs() < 0.01 && pl.origin[0].hypot(pl.origin[1]) < 0.01, "{:?}", pl.origin);
        assert!(norm(sub(pl.dir([0., 0., 1.]), [0., 0., -1.])) < 1e-6);
    }
}
