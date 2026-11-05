// SPDX-License-Identifier: LGPL-2.1-or-later
//! Comando de restrição ou junta antes de existir (B244). Os dois elementos, o tipo e os parâmetros
//! ficam aqui, fora da montagem. `Comando::resolver` faz tudo numa cópia: acrescenta o item, resolve
//! e devolve a cópia com as peças que se moveram. A montagem de verdade só muda quando a pessoa
//! aceita e a cópia toma o lugar dela, então cancelar não tem nada a desfazer e o que aparece como
//! fantasma é exatamente o que será gravado.

use crate::kin::{JFrame, JRef, JointKind};
use crate::solve::{self, SolveReport};
use crate::*;

/// O que foi clicado, como a pessoa reconhece.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Classe {
    Face,
    ArestaReta,
    ArestaCurva { circular: bool },
    Vertice,
}

/// Um elemento escolhido no 3D, com o que a junta pode precisar além da geometria.
#[derive(Clone, PartialEq, Debug)]
pub struct Elemento {
    pub r: MateRef,
    pub classe: Classe,
    /// polilinha da aresta, no sistema da peça (trajetória do ponto em curva)
    pub curva: Vec<V3>,
    /// contorno da seção da peça pelo plano da aresta, no sistema da peça (perfil do came)
    pub perfil: Option<Vec<V3>>,
    /// raio da aresta circular (rolete do seguidor)
    pub raio: Option<f64>,
}

impl Elemento {
    pub fn novo(r: MateRef, classe: Classe) -> Self {
        Elemento { r, classe, curva: vec![], perfil: None, raio: None }
    }
}

/// O modo do comando. Trocar o modo não mexe nos elementos já escolhidos.
#[derive(Clone, PartialEq, Debug)]
pub enum Tipo {
    /// o tipo mais lógico para o par
    Automatica,
    Restricao(MateKind),
    /// posiciona pelo par e não guarda a restrição
    Alinhar,
    Junta(JointKind),
}

impl Tipo {
    pub fn e_junta(&self) -> bool {
        matches!(self, Tipo::Junta(_))
    }
    pub fn label(&self) -> &'static str {
        match self {
            Tipo::Automatica => "Automática",
            Tipo::Alinhar => "Alinhar",
            Tipo::Restricao(k) => k.label(),
            Tipo::Junta(k) => k.label(),
        }
    }
}

/// Uma peça que a proposta tira do lugar.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Movido {
    pub comp: Id,
    pub antes: Placement,
    pub depois: Placement,
}

impl Movido {
    /// Transformação que leva o que está na posição de antes para a posição final.
    pub fn delta(&self) -> Placement {
        self.depois.then(&self.antes.inverse())
    }
}

/// Resultado de resolver a proposta numa cópia da montagem.
#[derive(Clone, Debug)]
pub struct Previa {
    pub copia: Assembly,
    /// a restrição ou junta dentro da cópia (o Alinhar não deixa item nenhum)
    pub id: Option<Id>,
    pub movidos: Vec<Movido>,
    pub report: SolveReport,
    /// restrições que passaram a falhar por causa da proposta (as que já falhavam não contam)
    pub novas_falhas: Vec<Id>,
    /// valor medido (ou digitado) de Distância e Ângulo, para o campo do painel
    pub valor: Option<String>,
    pub aceita_alinhamento: bool,
    pub aceita_lado: bool,
}

impl Previa {
    /// A própria proposta ficou sem solução, ou derrubou outra restrição.
    pub fn falhou(&self) -> bool {
        !self.novas_falhas.is_empty()
    }
}

/// Comando pendente.
#[derive(Clone, PartialEq, Debug)]
pub struct Comando {
    pub tipo: Tipo,
    pub caixas: [Option<Elemento>; 2],
    /// caixa que recebe o próximo clique
    pub ativa: usize,
    /// valor digitado; sem ele vale a medida de agora
    pub valor: Option<String>,
    pub inv_alinhamento: bool,
    pub inv_lado: bool,
    /// nome pedido no avançado (vazio: nome automático)
    pub nome: String,
    pub suprimida: bool,
}

impl Comando {
    pub fn novo(tipo: Tipo) -> Self {
        Comando { tipo, caixas: [None, None], ativa: 0, valor: None, inv_alinhamento: false, inv_lado: false, nome: String::new(), suprimida: false }
    }

    /// Põe o elemento na caixa ativa e avança para a caixa vazia.
    pub fn receber(&mut self, e: Elemento) {
        self.caixas[self.ativa] = Some(e);
        if let Some(k) = self.caixas.iter().position(Option::is_none) {
            self.ativa = k;
        }
    }

    /// Esvazia uma caixa e a deixa pronta para o próximo clique.
    pub fn limpar_caixa(&mut self, k: usize) {
        self.caixas[k] = None;
        self.ativa = k;
    }

    /// Etapa em que o comando está: 0 e 1 pedem um elemento; 2 tem tudo.
    pub fn etapa(&self) -> usize {
        self.caixas.iter().filter(|c| c.is_some()).count()
    }

    pub fn completo(&self) -> bool {
        self.etapa() == 2
    }

    /// Muda o modo sem repetir a seleção. O valor digitado era do modo anterior e não vale mais.
    pub fn trocar_tipo(&mut self, tipo: Tipo) {
        if tipo != self.tipo {
            self.tipo = tipo;
            self.valor = None;
            self.inv_alinhamento = false;
            self.inv_lado = false;
        }
    }

    /// Volta às caixas vazias para o próximo par, mantendo o modo (Enter aceita e continua).
    pub fn continuar(&mut self) {
        *self = Comando::novo(self.tipo.clone());
    }

    /// Inverte pela tecla: o alinhamento, se o par tiver; se não, o lado. Devolve se mudou algo.
    pub fn inverter(&mut self, previa: Option<&Previa>) -> bool {
        match previa {
            Some(p) if p.aceita_alinhamento => self.inv_alinhamento = !self.inv_alinhamento,
            Some(p) if p.aceita_lado => self.inv_lado = !self.inv_lado,
            _ => return false,
        }
        true
    }

    /// Resolve a proposta numa cópia de `base`, que fica intacta.
    pub fn resolver(&self, base: &Assembly) -> Result<Previa, String> {
        let [Some(a), Some(b)] = &self.caixas else { return Err("faltam elementos".into()) };
        if a.r.comp == b.r.comp {
            return Err("Os dois elementos são do mesmo componente".into());
        }
        let mut copia = base.clone();
        let (id, valor, aceita_alinhamento, aceita_lado) = match &self.tipo {
            Tipo::Junta(k) => (montar_junta(&mut copia, k, a, b)?, None, false, false),
            _ => self.montar_restricao(&mut copia, a, b)?,
        };
        if !self.nome.trim().is_empty() {
            if let Some(m) = copia.mates.iter_mut().find(|m| m.id == id) {
                m.name = self.nome.trim().to_string();
            }
            if let Some(j) = copia.joints.iter_mut().find(|j| j.id == id) {
                j.name = self.nome.trim().to_string();
            }
        }
        if self.suprimida {
            if let Some(m) = copia.mates.iter_mut().find(|m| m.id == id) {
                m.suppressed = true;
            }
            if let Some(j) = copia.joints.iter_mut().find(|j| j.id == id) {
                j.suppressed = true;
            }
        }
        let report = solve::solve(&mut copia);
        let so_posicionar = self.tipo == Tipo::Alinhar;
        if so_posicionar {
            copia.mates.retain(|m| m.id != id);
        }
        let ja_falhavam = solve::solve(&mut base.clone()).failed;
        let novas_falhas = report.failed.iter().copied().filter(|f| !ja_falhavam.contains(f)).collect();
        let movidos = movidos(base, &copia);
        Ok(Previa { copia, id: (!so_posicionar).then_some(id), movidos, report, novas_falhas, valor, aceita_alinhamento, aceita_lado })
    }

    fn montar_restricao(&self, c: &mut Assembly, a: &Elemento, b: &Elemento) -> Result<(Id, Option<String>, bool, bool), String> {
        let lugar = |id: Id| c.node(id).map(|n| n.placement).unwrap_or_default();
        let kind = match &self.tipo {
            Tipo::Restricao(k) => k.clone(),
            _ => tipo_automatico(&a.r.geom, &b.r.geom, &lugar(a.r.comp), &lugar(b.r.comp)),
        };
        if !par_valido(&kind, &a.r.geom, &b.r.geom) {
            return Err("este par de elementos não define a restrição (escolha planos ou eixos)".into());
        }
        let id = c.add_mate(kind, a.r, b.r);
        // engrenagem e cremalheira: o giro de agora é o zero
        c.init_motion(id);
        // distância e ângulo: a medida de agora, para a peça não saltar
        c.iniciar_medida(id);
        if let (Some(v), Some(m)) = (&self.valor, c.mates.iter_mut().find(|m| m.id == id)) {
            match &mut m.kind {
                MateKind::Distance(e) | MateKind::Angle(e) => *e = v.clone(),
                MateKind::Gear { ratio: e, .. } | MateKind::RackPinion { mm_per_rev: e, .. } => *e = v.clone(),
                _ => {}
            }
        }
        if self.inv_alinhamento {
            c.inverter_alinhamento(id);
        }
        if self.inv_lado {
            c.inverter_lado(id);
        }
        let m = c.mates.iter().find(|m| m.id == id).ok_or("restrição sumiu")?;
        let valor = match &m.kind {
            MateKind::Distance(e) | MateKind::Angle(e) => Some(e.clone()),
            MateKind::Gear { ratio: e, .. } | MateKind::RackPinion { mm_per_rev: e, .. } => Some(e.clone()),
            _ => None,
        };
        Ok((id, valor, m.aceita_alinhamento(), m.aceita_lado()))
    }
}

/// Peças que mudaram de lugar da montagem `base` para `copia`.
pub fn movidos(base: &Assembly, copia: &Assembly) -> Vec<Movido> {
    let longe = |p: &Placement, q: &Placement| norm(sub(p.origin, q.origin)) > 1e-7 || (0..3).any(|k| norm(sub(p.rot[k], q.rot[k])) > 1e-7);
    copia
        .nodes
        .iter()
        .filter_map(|n| {
            let antes = base.node(n.id)?.placement;
            longe(&antes, &n.placement).then_some(Movido { comp: n.id, antes, depois: n.placement })
        })
        .collect()
}

/// Restrição automática: o tipo mais lógico para o par escolhido.
pub fn tipo_automatico(a: &Geom, b: &Geom, wa: &Placement, wb: &Placement) -> MateKind {
    match (a, b) {
        // faces de frente uma para a outra: contato; mesmo sentido: coincidência
        (Geom::Plane { n: na, .. }, Geom::Plane { n: nb, .. }) if dot(wa.dir(*na), wb.dir(*nb)) < 0. => MateKind::Contact,
        _ => MateKind::Coincident { flip: false },
    }
}

/// Sistema da junta a partir da geometria: o eixo ou a normal vira o z.
pub fn sistema(g: &Geom) -> JFrame {
    match *g {
        Geom::Axis { p, d } => JFrame::from_axis(p, d),
        Geom::Plane { p, n } => JFrame::from_axis(p, n),
        Geom::Point(p) => JFrame::from_axis(p, [0., 0., 1.]),
    }
}

/// Acrescenta a junta na cópia. O x de B começa como o x de A projetado no plano de B, para o zero
/// do ângulo ser comum às duas peças.
fn montar_junta(c: &mut Assembly, k: &JointKind, a: &Elemento, b: &Elemento) -> Result<Id, String> {
    let lugar = |id: Id| c.node(id).map(|n| n.placement).unwrap_or_default();
    let (pa, pb) = (lugar(a.r.comp), lugar(b.r.comp));
    let fa = sistema(&a.r.geom);
    let mut fb = sistema(&b.r.geom);
    let xa = pb.dir_to_local(pa.dir(fa.x));
    let k2 = dot(xa, fb.z);
    let px = [xa[0] - fb.z[0] * k2, xa[1] - fb.z[1] * k2, xa[2] - fb.z[2] * k2];
    if norm(px) > 1e-6 {
        fb.x = unit(px);
    }
    let kind = match k {
        JointKind::PointCurve { .. } if a.curva.len() < 2 => return Err("Ponto em curva: a trajetória é uma aresta".into()),
        JointKind::PointCurve { .. } => JointKind::PointCurve { curve: a.curva.clone() },
        JointKind::Cam { radius, .. } => {
            let Some(perfil) = a.perfil.clone() else { return Err("Came: o perfil é uma aresta do came".into()) };
            let raio = b.raio.map(|r| format!("{r:.3}")).unwrap_or_else(|| radius.clone());
            JointKind::Cam { profile: perfil, radius: raio }
        }
        k => k.clone(),
    };
    Ok(c.add_joint(kind, JRef { comp: a.r.comp, frame: fa }, JRef { comp: b.r.comp, frame: fb }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Base fixa na origem e peça solta 30 acima, com faces de frente uma para a outra.
    fn par() -> (Assembly, Id, Id) {
        let mut a = Assembly::new("par");
        let f = a.add_component("base", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        a.node_mut(f).unwrap().fixed = true;
        let m = a.add_component("tampa", NodeKind::Part { path: "b.fkp".into() }, Placement { origin: [3., 4., 30.], ..Default::default() }, None);
        (a, f, m)
    }

    fn face(comp: Id, p: V3, n: V3) -> Elemento {
        Elemento::novo(MateRef { comp, geom: Geom::Plane { p, n }, pick: None }, Classe::Face)
    }

    fn eixo(comp: Id, p: V3, d: V3) -> Elemento {
        Elemento::novo(MateRef { comp, geom: Geom::Axis { p, d }, pick: None }, Classe::ArestaCurva { circular: true })
    }

    fn contato(f: Id, m: Id) -> Comando {
        let mut c = Comando::novo(Tipo::Restricao(MateKind::Contact));
        c.receber(face(f, [0., 0., 10.], [0., 0., 1.]));
        c.receber(face(m, [0., 0., 0.], [0., 0., -1.]));
        c
    }

    #[test]
    fn resolver_nao_toca_a_montagem() {
        let (a, f, m) = par();
        let antes = a.to_json();
        let p = contato(f, m).resolver(&a).unwrap();
        assert_eq!(a.to_json(), antes, "a prévia mexeu na montagem");
        assert!(a.mates.is_empty());
        assert_eq!(p.copia.mates.len(), 1);
        assert_eq!(p.movidos.len(), 1);
        assert_eq!(p.movidos[0].comp, m);
        assert!((p.movidos[0].depois.origin[2] - 10.).abs() < 1e-6, "encosta na face de cima");
        assert!(!p.falhou());
    }

    #[test]
    fn aceitar_grava_o_que_o_fantasma_mostrou() {
        let (mut a, f, m) = par();
        let p = contato(f, m).resolver(&a).unwrap();
        let final_previa = p.movidos[0].depois;
        a = p.copia;
        solve::solve(&mut a);
        assert!(norm(sub(a.node(m).unwrap().placement.origin, final_previa.origin)) < 1e-6);
    }

    #[test]
    fn trocar_o_tipo_nao_repete_a_selecao() {
        let (a, f, m) = par();
        let mut c = contato(f, m);
        let caixas = c.caixas.clone();
        c.trocar_tipo(Tipo::Restricao(MateKind::Distance("0".into())));
        assert_eq!(c.caixas, caixas);
        assert!(c.completo());
        let p = c.resolver(&a).unwrap();
        assert!(matches!(&p.copia.mates[0].kind, MateKind::Distance(v) if v == "20"), "nasce com a medida de agora");
        assert!(p.movidos.is_empty(), "distância medida não salta");
        c.trocar_tipo(Tipo::Junta(JointKind::Planar));
        assert!(c.resolver(&a).unwrap().copia.joints.len() == 1);
    }

    #[test]
    fn valor_digitado_move_a_previa() {
        let (a, f, m) = par();
        let mut c = contato(f, m);
        c.trocar_tipo(Tipo::Restricao(MateKind::Distance("0".into())));
        c.valor = Some("5".into());
        let p = c.resolver(&a).unwrap();
        assert!((p.movidos[0].depois.origin[2] - 15.).abs() < 1e-6);
        assert_eq!(p.valor.as_deref(), Some("5"));
    }

    #[test]
    fn tab_inverte_e_a_previa_acompanha() {
        let (a, f, m) = par();
        let mut c = contato(f, m);
        c.trocar_tipo(Tipo::Restricao(MateKind::Coincident { flip: true }));
        let p0 = c.resolver(&a).unwrap();
        let z0 = p0.copia.node(m).unwrap().placement.dir([0., 0., 1.])[2];
        assert!(c.inverter(Some(&p0)));
        let p1 = c.resolver(&a).unwrap();
        let z1 = p1.copia.node(m).unwrap().placement.dir([0., 0., 1.])[2];
        assert!((z0 - z1).abs() > 1.5, "a peça vira: {z0} {z1}");
    }

    #[test]
    fn tab_sem_alinhamento_passa_para_o_lado() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Restricao(MateKind::Distance("0".into())));
        c.receber(face(f, [0., 0., 10.], [0., 0., 1.]));
        c.receber(Elemento::novo(MateRef { comp: m, geom: Geom::Point([0., 0., 0.]), pick: None }, Classe::Vertice));
        c.valor = Some("20".into());
        let p0 = c.resolver(&a).unwrap();
        assert!(!p0.aceita_alinhamento && p0.aceita_lado);
        assert!(c.inverter(Some(&p0)));
        let p1 = c.resolver(&a).unwrap();
        assert!((p1.copia.node(m).unwrap().placement.origin[2] + 10.).abs() < 1e-6, "passa para baixo da face");
    }

    #[test]
    fn enter_continua_com_as_caixas_vazias_no_mesmo_modo() {
        let (_, f, m) = par();
        let mut c = contato(f, m);
        c.valor = Some("1".into());
        c.continuar();
        assert_eq!(c.etapa(), 0);
        assert_eq!(c.ativa, 0);
        assert_eq!(c.tipo, Tipo::Restricao(MateKind::Contact));
        assert_eq!(c.valor, None);
    }

    #[test]
    fn clique_vai_para_a_caixa_ativa_e_troca_so_ela() {
        let (_, f, m) = par();
        let mut c = contato(f, m);
        c.ativa = 0;
        let nova = face(f, [1., 1., 1.], [1., 0., 0.]);
        c.receber(nova.clone());
        assert_eq!(c.caixas[0], Some(nova));
        assert!(c.caixas[1].is_some());
        c.limpar_caixa(1);
        assert_eq!((c.ativa, c.etapa()), (1, 1));
    }

    #[test]
    fn conflito_fica_na_previa() {
        let (mut a, f, m) = par();
        a.node_mut(m).unwrap().fixed = true;
        let p = contato(f, m).resolver(&a).unwrap();
        assert!(p.falhou(), "as duas peças presas: não dá para encostar");
        assert!(p.movidos.is_empty());
        assert!(a.mates.is_empty());
    }

    #[test]
    fn par_sem_sentido_e_mesma_peca_recusam() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Restricao(MateKind::Parallel));
        c.receber(Elemento::novo(MateRef { comp: f, geom: Geom::Point([0.; 3]), pick: None }, Classe::Vertice));
        c.receber(face(m, [0.; 3], [0., 0., 1.]));
        assert!(c.resolver(&a).is_err());
        let mut c = Comando::novo(Tipo::Automatica);
        c.receber(face(f, [0.; 3], [0., 0., 1.]));
        c.receber(face(f, [1., 0., 0.], [0., 0., 1.]));
        assert!(c.resolver(&a).is_err());
    }

    #[test]
    fn alinhar_posiciona_e_nao_guarda() {
        let (a, f, m) = par();
        let mut c = contato(f, m);
        c.trocar_tipo(Tipo::Alinhar);
        let p = c.resolver(&a).unwrap();
        assert!(p.copia.mates.is_empty() && p.id.is_none());
        assert_eq!(p.movidos.len(), 1);
    }

    #[test]
    fn automatica_escolhe_pelas_normais() {
        let (a, f, m) = par();
        let mut c = contato(f, m);
        c.trocar_tipo(Tipo::Automatica);
        assert_eq!(c.resolver(&a).unwrap().copia.mates[0].kind, MateKind::Contact);
    }

    #[test]
    fn junta_entra_na_copia_e_prende_o_eixo() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Junta(JointKind::Revolute));
        c.receber(eixo(f, [10., 0., 0.], [0., 0., 1.]));
        c.receber(eixo(m, [0., 0., 0.], [0., 0., 1.]));
        let p = c.resolver(&a).unwrap();
        assert!(a.joints.is_empty());
        assert_eq!(p.copia.joints.len(), 1);
        let o = p.copia.node(m).unwrap().placement.origin;
        assert!((o[0] - 10.).abs() < 1e-5 && o[1].abs() < 1e-5, "eixos colineares: {o:?}");
        assert!((o[2] - 30.).abs() < 1e-5, "mantém a altura de agora");
        assert_eq!(solve::dof(&p.copia), 1);
    }

    #[test]
    fn junta_de_trajetoria_exige_aresta() {
        let (a, f, m) = par();
        let mut c = Comando::novo(Tipo::Junta(JointKind::PointCurve { curve: vec![] }));
        c.receber(eixo(f, [0.; 3], [0., 0., 1.]));
        c.receber(Elemento::novo(MateRef { comp: m, geom: Geom::Point([0.; 3]), pick: None }, Classe::Vertice));
        assert!(c.resolver(&a).is_err());
    }

    #[test]
    fn nome_e_suprimir_do_avancado() {
        let (a, f, m) = par();
        let mut c = contato(f, m);
        c.nome = " Tampa fechada ".into();
        c.suprimida = true;
        let p = c.resolver(&a).unwrap();
        assert_eq!(p.copia.mates[0].name, "Tampa fechada");
        assert!(p.copia.mates[0].suppressed);
        assert!(p.movidos.is_empty(), "suprimida não move");
    }
}
