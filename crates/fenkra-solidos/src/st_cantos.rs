// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cantos da estrutura (B37): onde as linhas guia de membros se encontram, como cada canto é
//! tratado, e as operações do painel "Cantos da estrutura" (trocar tratamento, quem continua, folga,
//! limpar tudo, agrupar cantos iguais).
//!
//! Como funciona no kernel: o recompute olha cada par de linhas que dividem uma ponta
//! (`member_joints`) e pergunta a `joint_rule` qual regra vale. A regra mora no comando posterior
//! do par (`StructMember.joints`); sem regra própria vale o tratamento padrão do comando posterior
//! (`corner`/`gap`). Este módulo só lê esse resultado e reescreve essas regras; o corte em si
//! continua no recompute.

use fenkra_core::{CornerMode, Document, FeatureKind, Id, StructJoint};

/// Linha guia de um membro: (comando, índice da linha).
pub type Linha = (Id, usize);

/// Como o canto aparece no 3D e na lista.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClasseCanto {
    /// dois membros e nenhuma regra própria: vale o tratamento padrão do comando
    DoisMembros,
    /// três ou mais linhas na mesma ponta
    TresOuMais,
    /// dois membros com regra própria (diferente de "nenhum")
    Tratado,
    /// o tratamento efetivo é "nenhum": as pontas só se tocam
    Ignorado,
}

/// Par de linhas que se encontram, como o recompute o enxerga.
#[derive(Clone, Debug)]
pub struct EntradaCanto {
    pub a: Linha,
    pub b: Linha,
    pub ponto: [f64; 3],
    /// direção de cada linha saindo do canto
    pub dir_a: [f64; 3],
    pub dir_b: [f64; 3],
    /// outra ponta de cada linha (para desenhar a prévia do corte)
    pub fim_a: [f64; 3],
    pub fim_b: [f64; 3],
    pub modo: CornerMode,
    /// `a` é o membro 1 (continua; o outro para nele)
    pub a_continua: bool,
    pub folga: f64,
    /// existe regra própria para este par (ou só vale o padrão do comando)
    pub explicito: bool,
    /// perfil de cada linha ("Tubo quadrado 40x3"), para agrupar cantos iguais
    pub perfil_a: String,
    pub perfil_b: String,
}

#[derive(Clone, Debug)]
pub struct Canto {
    pub ponto: [f64; 3],
    /// todas as linhas que chegam ao ponto
    pub linhas: Vec<Linha>,
    pub classe: ClasseCanto,
    /// tratamento efetivo (do primeiro par; só importa nos cantos de dois membros)
    pub modo: CornerMode,
    pub explicito: bool,
    /// em Aparar/Encaixe, a linha que continua; em meia-esquadria e nenhum, `None`
    pub continua: Option<Linha>,
    pub folga: f64,
    /// ângulo entre os dois membros (graus), lido da geometria; só nos cantos de dois membros
    pub angulo: Option<f64>,
    /// pares do canto (um nos cantos de dois membros)
    pub pares: Vec<EntradaCanto>,
}

impl Canto {
    /// O membro de `linha` participa deste canto.
    pub fn tem(&self, linha: Linha) -> bool {
        self.linhas.contains(&linha)
    }

    /// Assinatura para o modo automático: cantos com a mesma assinatura recebem o mesmo tratamento.
    pub fn assinatura(&self) -> String {
        let mut perfis: Vec<&str> = self.pares.iter().flat_map(|p| [p.perfil_a.as_str(), p.perfil_b.as_str()]).collect();
        perfis.sort_unstable();
        perfis.dedup();
        format!("{}|{:?}|{:?}|{:.1}|{:.2}", perfis.join("+"), self.classe, self.modo, self.angulo.unwrap_or(-1.), self.folga)
    }
}

fn perto(p: [f64; 3], q: [f64; 3]) -> bool {
    (0..3).all(|k| (p[k] - q[k]).abs() < 1e-4)
}

/// Junta os pares que dividem o mesmo ponto num único canto e classifica cada um.
pub fn montar_cantos(entradas: Vec<EntradaCanto>) -> Vec<Canto> {
    let mut grupos: Vec<(Vec<EntradaCanto>, Vec<Linha>)> = vec![];
    for e in entradas {
        let g = grupos.iter().position(|(ps, _)| perto(ps[0].ponto, e.ponto));
        let g = g.unwrap_or_else(|| {
            grupos.push((vec![], vec![]));
            grupos.len() - 1
        });
        for l in [e.a, e.b] {
            if !grupos[g].1.contains(&l) {
                grupos[g].1.push(l);
            }
        }
        grupos[g].0.push(e);
    }
    grupos
        .into_iter()
        .map(|(pares, linhas)| {
            let p0 = &pares[0];
            let duas = linhas.len() == 2;
            let todos_nenhum = pares.iter().all(|p| p.modo == CornerMode::Nenhum);
            let classe = if todos_nenhum {
                ClasseCanto::Ignorado
            } else if !duas {
                ClasseCanto::TresOuMais
            } else if p0.explicito {
                ClasseCanto::Tratado
            } else {
                ClasseCanto::DoisMembros
            };
            let continua = matches!(p0.modo, CornerMode::Aparar | CornerMode::Encaixe).then(|| if p0.a_continua { p0.a } else { p0.b });
            let dot: f64 = (0..3).map(|k| p0.dir_a[k] * p0.dir_b[k]).sum();
            Canto {
                ponto: p0.ponto,
                classe,
                modo: p0.modo,
                explicito: p0.explicito,
                continua,
                folga: p0.folga,
                angulo: duas.then(|| dot.clamp(-1., 1.).acos().to_degrees()),
                linhas,
                pares,
            }
        })
        .collect()
}

/// Posição do comando na árvore (para saber quem é o posterior de um par).
fn posicao(doc: &Document, id: Id) -> usize {
    doc.features.iter().position(|f| f.id == id).unwrap_or(usize::MAX)
}

/// Ordena o par: (anterior, posterior), como o recompute (por posição do comando, depois linha).
fn ordenar(doc: &Document, a: Linha, b: Linha) -> (Linha, Linha) {
    if (posicao(doc, a.0), a.1) <= (posicao(doc, b.0), b.1) {
        (a, b)
    } else {
        (b, a)
    }
}

/// O par tem regra própria guardada em algum dos dois membros?
pub fn tem_regra(doc: &Document, a: Linha, b: Linha) -> bool {
    let (early, late) = ordenar(doc, a, b);
    let em = |dono: Linha, mine: usize, other: Linha| match doc.feature(dono.0).map(|f| &f.kind) {
        Some(FeatureKind::StructMember(m)) => m.joints.iter().any(|j| j.mine == mine && j.other == other.0 && j.other_path == other.1),
        _ => false,
    };
    em(late, late.1, early) || em(early, early.1, late)
}

/// Remove do documento qualquer regra do par (nos dois membros).
fn remover_regra(doc: &mut Document, a: Linha, b: Linha) {
    for (dono, outro) in [(a, b), (b, a)] {
        if let Some(f) = doc.feature_mut(dono.0) {
            if let FeatureKind::StructMember(m) = &mut f.kind {
                m.joints.retain(|j| !(j.mine == dono.1 && j.other == outro.0 && j.other_path == outro.1));
            }
        }
    }
}

/// Define a regra do par: `modo` (None volta ao padrão do comando), quem continua (`continua`, uma
/// das duas linhas) e a folga própria (`folga`, `None` usa a do comando). A regra mora no membro
/// posterior, como o recompute espera. Devolve `false` se o membro posterior não é um comando de
/// membro estrutural.
pub fn definir_regra(doc: &mut Document, a: Linha, b: Linha, modo: Option<CornerMode>, continua: Option<Linha>, folga: Option<String>) -> bool {
    let (early, late) = ordenar(doc, a, b);
    remover_regra(doc, a, b);
    let Some(modo) = modo else { return true };
    // padrão: o anterior é o membro 1 (continua)
    let primeiro = continua.map(|c| c == late).unwrap_or(false);
    let Some(f) = doc.feature_mut(late.0) else { return false };
    let FeatureKind::StructMember(m) = &mut f.kind else { return false };
    m.joints.push(StructJoint { mine: late.1, other: early.0, other_path: early.1, mode: modo, first: primeiro, gap: folga.filter(|g| !g.trim().is_empty()) });
    true
}

/// Troca quem continua num canto de dois membros com Aparar ou Encaixe. Sem regra própria, cria uma
/// com o tratamento efetivo e a troca.
pub fn trocar_continua(doc: &mut Document, c: &Canto) -> bool {
    let [a, b] = match c.linhas.as_slice() {
        [a, b] => [*a, *b],
        _ => return false,
    };
    if !matches!(c.modo, CornerMode::Aparar | CornerMode::Encaixe) {
        return false;
    }
    let novo = if c.continua == Some(a) { b } else { a };
    let folga = (c.folga != 0.).then(|| format!("{}", c.folga));
    definir_regra(doc, a, b, Some(c.modo), Some(novo), folga)
}

/// Apaga todas as regras de canto de todos os membros: voltam ao tratamento padrão de cada comando.
/// Devolve quantas regras foram removidas.
pub fn limpar_tudo(doc: &mut Document) -> usize {
    let mut n = 0;
    for f in doc.features.iter_mut() {
        if let FeatureKind::StructMember(m) = &mut f.kind {
            n += m.joints.len();
            m.joints.clear();
        }
    }
    n
}

/// Modo automático: índices dos cantos agrupados por assinatura (perfis, tipo, ângulo, tratamento).
/// Só os cantos de dois membros entram; grupos de um canto só também saem (o painel mostra o total).
pub fn agrupar_iguais(cantos: &[Canto]) -> Vec<Vec<usize>> {
    let mut grupos: Vec<(String, Vec<usize>)> = vec![];
    for (i, c) in cantos.iter().enumerate().filter(|(_, c)| c.linhas.len() == 2) {
        let s = c.assinatura();
        match grupos.iter_mut().find(|(k, _)| *k == s) {
            Some((_, v)) => v.push(i),
            None => grupos.push((s, vec![i])),
        }
    }
    grupos.into_iter().map(|(_, v)| v).collect()
}

/// Aplica o tratamento de um canto a todos os do seu grupo (modo automático). A linha que continua
/// segue a ordem do canto-modelo (a mesma posição no par). Devolve quantos cantos mudaram.
pub fn aplicar_ao_grupo(doc: &mut Document, cantos: &[Canto], grupo: &[usize], modo: Option<CornerMode>) -> usize {
    let mut n = 0;
    for &i in grupo {
        let c = &cantos[i];
        let [a, b] = match c.linhas.as_slice() {
            [a, b] => [*a, *b],
            _ => continue,
        };
        let cont = c.continua.unwrap_or(a);
        let folga = (c.folga != 0.).then(|| format!("{}", c.folga));
        if definir_regra(doc, a, b, modo, Some(cont), folga) {
            n += 1;
        }
    }
    n
}

/// Resumo do painel: (dois membros, três ou mais, tratados, ignorados).
pub fn contagem(cantos: &[Canto]) -> (usize, usize, usize, usize) {
    let n = |c: ClasseCanto| cantos.iter().filter(|x| x.classe == c).count();
    (n(ClasseCanto::DoisMembros), n(ClasseCanto::TresOuMais), n(ClasseCanto::Tratado), n(ClasseCanto::Ignorado))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entrada(a: Linha, b: Linha, p: [f64; 3], modo: CornerMode, explicito: bool) -> EntradaCanto {
        EntradaCanto {
            a,
            b,
            ponto: p,
            dir_a: [1., 0., 0.],
            dir_b: [0., 1., 0.],
            fim_a: [100., 0., 0.],
            fim_b: [0., 100., 0.],
            modo,
            a_continua: true,
            folga: 0.,
            explicito,
            perfil_a: "Tubo quadrado 40x3".into(),
            perfil_b: "Tubo quadrado 40x3".into(),
        }
    }

    #[test]
    fn classifica_dois_tres_tratado_e_ignorado() {
        let (m, n) = (Id(1), Id(2));
        let es = vec![
            // canto simples, padrão
            entrada((m, 0), (m, 1), [0., 0., 0.], CornerMode::MeiaEsquadria, false),
            // três linhas na mesma ponta: três pares
            entrada((m, 1), (m, 2), [100., 0., 0.], CornerMode::MeiaEsquadria, false),
            entrada((m, 1), (n, 0), [100., 0., 0.], CornerMode::MeiaEsquadria, false),
            entrada((m, 2), (n, 0), [100., 0., 0.], CornerMode::MeiaEsquadria, false),
            // regra própria
            entrada((m, 2), (n, 1), [100., 100., 0.], CornerMode::Aparar, true),
            // ignorado
            entrada((n, 1), (n, 2), [0., 100., 0.], CornerMode::Nenhum, true),
        ];
        let cs = montar_cantos(es);
        assert_eq!(cs.len(), 4);
        assert_eq!(cs[0].classe, ClasseCanto::DoisMembros);
        assert_eq!(cs[1].classe, ClasseCanto::TresOuMais);
        assert_eq!(cs[1].linhas.len(), 3);
        assert_eq!(cs[2].classe, ClasseCanto::Tratado);
        assert_eq!(cs[2].continua, Some((m, 2)));
        assert_eq!(cs[3].classe, ClasseCanto::Ignorado);
        assert_eq!(contagem(&cs), (1, 1, 1, 1));
        // ângulo lido da geometria: direções perpendiculares
        assert!((cs[0].angulo.unwrap() - 90.).abs() < 1e-9);
        assert!(cs[1].angulo.is_none());
    }

    #[test]
    fn agrupa_cantos_iguais_e_separa_os_diferentes() {
        let (m, n) = (Id(1), Id(2));
        let mut b = entrada((n, 0), (n, 1), [50., 0., 0.], CornerMode::MeiaEsquadria, false);
        b.perfil_a = "IPE 200".into();
        let es = vec![
            entrada((m, 0), (m, 1), [0., 0., 0.], CornerMode::MeiaEsquadria, false),
            entrada((m, 1), (m, 2), [10., 0., 0.], CornerMode::MeiaEsquadria, false),
            entrada((m, 2), (m, 3), [20., 0., 0.], CornerMode::MeiaEsquadria, false),
            b,
        ];
        let cs = montar_cantos(es);
        let g = agrupar_iguais(&cs);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0], vec![0, 1, 2]);
        assert_eq!(g[1], vec![3]);
    }

    fn doc_com_dois_membros() -> (Document, Id, Id) {
        use fenkra_core::{CurveRef, PointRef, RefKind, SectionKind, StructMember};
        let s = |x: &str| x.to_string();
        let mut d = Document::new("t");
        let mk = |d: &mut Document, a: [f64; 3], b: [f64; 3]| {
            let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") });
            d.add_feature(FeatureKind::StructMember(StructMember {
                paths: vec![CurveRef::Ref(l)],
                family: s("Tubo quadrado"),
                designation: s("40x3"),
                kind: SectionKind::TuboQuadrado,
                dims: vec![s("40"), s("3")],
                anchor: 0,
                angle: s("0"),
                off_x: s("0"),
                off_y: s("0"),
                ext1: s("0"),
                ext2: s("0"),
                corner: CornerMode::MeiaEsquadria,
                gap: s("0"),
                joints: vec![],
                material: String::new(),
                mirror: false,
            }))
        };
        let a = mk(&mut d, [0., 0., 0.], [400., 0., 0.]);
        let b = mk(&mut d, [400., 0., 0.], [400., 300., 0.]);
        (d, a, b)
    }

    #[test]
    fn regra_mora_no_membro_posterior_troca_quem_continua_e_limpa() {
        let (mut d, a, b) = doc_com_dois_membros();
        assert!(!tem_regra(&d, (a, 0), (b, 0)));
        assert!(definir_regra(&mut d, (a, 0), (b, 0), Some(CornerMode::Aparar), Some((a, 0)), None));
        assert!(tem_regra(&d, (b, 0), (a, 0)));
        let Some(FeatureKind::StructMember(m)) = d.feature(b).map(|f| &f.kind) else { unreachable!() };
        assert_eq!(m.joints.len(), 1);
        assert_eq!((m.joints[0].mine, m.joints[0].other, m.joints[0].first), (0, a, false));
        // trocar quem continua: agora o posterior (b) continua
        let c = Canto { ponto: [400., 0., 0.], linhas: vec![(a, 0), (b, 0)], classe: ClasseCanto::Tratado, modo: CornerMode::Aparar, explicito: true, continua: Some((a, 0)), folga: 0., angulo: Some(90.), pares: vec![] };
        assert!(trocar_continua(&mut d, &c));
        let Some(FeatureKind::StructMember(m)) = d.feature(b).map(|f| &f.kind) else { unreachable!() };
        assert_eq!(m.joints.len(), 1, "a regra antiga foi substituída, não duplicada");
        assert!(m.joints[0].first);
        // meia-esquadria não tem quem continue
        let mut c2 = c.clone();
        c2.modo = CornerMode::MeiaEsquadria;
        assert!(!trocar_continua(&mut d, &c2));
        // limpar tudo
        assert_eq!(limpar_tudo(&mut d), 1);
        assert!(!tem_regra(&d, (a, 0), (b, 0)));
        // voltar ao padrão com modo None também limpa
        definir_regra(&mut d, (a, 0), (b, 0), Some(CornerMode::Encaixe), None, Some("1.5".into()));
        assert!(tem_regra(&d, (a, 0), (b, 0)));
        definir_regra(&mut d, (a, 0), (b, 0), None, None, None);
        assert!(!tem_regra(&d, (a, 0), (b, 0)));
    }

    #[test]
    fn cantos_lidos_do_documento_seguem_as_regras_e_mudam_o_solido() {
        let (mut d, a, b) = doc_com_dois_membros();
        let cs = crate::member_corners(&d);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].classe, ClasseCanto::DoisMembros);
        assert!((cs[0].angulo.unwrap() - 90.).abs() < 1e-6, "{:?}", cs[0].angulo);
        assert!(perto(cs[0].ponto, [400., 0., 0.]));
        assert_eq!(cs[0].continua, None, "meia-esquadria: nenhum dos dois continua");
        // o volume total não muda (o membro que continua ganha o que o outro perde); a forma muda:
        // a assinatura de cada membro (comprimento e pontas) é que diferencia os tratamentos
        let chaves = |d: &Document| crate::structural_members(d).iter().map(|m| m.signature.key()).collect::<Vec<_>>();
        let k_meia = if fenkra_kernel::AVAILABLE { chaves(&d) } else { vec![] };
        // aparar: o posterior (b) para na face do anterior (a, que continua)
        definir_regra(&mut d, (a, 0), (b, 0), Some(CornerMode::Aparar), Some((a, 0)), None);
        let cs = crate::member_corners(&d);
        assert_eq!(cs[0].classe, ClasseCanto::Tratado);
        assert_eq!(cs[0].continua, Some((a, 0)));
        // trocar quem continua inverte o lado
        let k_aparar_a = if fenkra_kernel::AVAILABLE { chaves(&d) } else { vec![] };
        trocar_continua(&mut d, &cs[0]);
        assert_eq!(crate::member_corners(&d)[0].continua, Some((b, 0)));
        if fenkra_kernel::AVAILABLE {
            let k_aparar_b = chaves(&d);
            assert_eq!(k_meia.len(), 2);
            assert_ne!(k_meia, k_aparar_a, "aparar muda a forma dos membros");
            assert_ne!(k_aparar_a, k_aparar_b, "trocar quem continua inverte o lado");
        }
        // ignorar o canto
        definir_regra(&mut d, (a, 0), (b, 0), Some(CornerMode::Nenhum), None, None);
        assert_eq!(crate::member_corners(&d)[0].classe, ClasseCanto::Ignorado);
        assert_eq!(limpar_tudo(&mut d), 1);
        assert_eq!(crate::member_corners(&d)[0].classe, ClasseCanto::DoisMembros);
    }
}
