// SPDX-License-Identifier: LGPL-2.1-or-later
//! Alvos de referência (datum targets) do PMI 3D (B189): ponto, linha, círculo, círculo-curva,
//! retângulo, área e curva, com número (A1, A2…), dimensões do alvo, alvo móvel e área restrita.
//!
//! Um alvo de referência substitui a superfície inteira como referência: em vez de apoiar a peça
//! numa face, ela se apoia em pontos, linhas ou pequenas áreas. A regra 3-2-1 diz que a referência
//! primária toca em três pontos (restringe 3 graus de liberdade), a secundária em dois (2) e a
//! terciária em um (1), seis no total. Aqui o alvo guarda só o que o desenho e a troca de dados
//! precisam; o desenho do símbolo sai em primitivas vetoriais, no mesmo formato de `weld_symbol`.

use crate::doc::Id;
use crate::pmi::{Pmi, PmiKind};
use serde::{Deserialize, Serialize};

/// Forma do alvo na superfície.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum FormaAlvo {
    /// um ponto de contato (símbolo ×)
    #[default]
    Ponto,
    /// uma linha de contato (traço com × nas pontas)
    Linha,
    /// área circular de diâmetro dado
    Circulo,
    /// arco de círculo de contato (trecho de circunferência)
    CirculoCurva,
    /// área retangular
    Retangulo,
    /// área de contorno livre (contorno vem da geometria)
    Area,
    /// curva qualquer sobre a superfície
    Curva,
}

impl FormaAlvo {
    pub const ALL: [FormaAlvo; 7] = [FormaAlvo::Ponto, FormaAlvo::Linha, FormaAlvo::Circulo, FormaAlvo::CirculoCurva, FormaAlvo::Retangulo, FormaAlvo::Area, FormaAlvo::Curva];

    pub fn label(self) -> &'static str {
        match self {
            FormaAlvo::Ponto => "ponto",
            FormaAlvo::Linha => "linha",
            FormaAlvo::Circulo => "círculo",
            FormaAlvo::CirculoCurva => "círculo-curva",
            FormaAlvo::Retangulo => "retângulo",
            FormaAlvo::Area => "área",
            FormaAlvo::Curva => "curva",
        }
    }

    /// Quantos valores de dimensão o alvo usa: nenhum (ponto, área, curva), um (comprimento da linha,
    /// diâmetro, raio do arco) ou dois (retângulo).
    pub fn dimensoes(self) -> usize {
        match self {
            FormaAlvo::Ponto | FormaAlvo::Area | FormaAlvo::Curva => 0,
            FormaAlvo::Linha | FormaAlvo::Circulo | FormaAlvo::CirculoCurva => 1,
            FormaAlvo::Retangulo => 2,
        }
    }

    /// Graus de liberdade da regra 3-2-1 que o alvo fixo tira: ponto 1, linha e curva 2, e
    /// o que toca em superfície (círculo, retângulo, área) tira os 3 de um plano.
    pub fn peso_321(self) -> u8 {
        match self {
            FormaAlvo::Ponto => 1,
            FormaAlvo::Linha | FormaAlvo::Curva | FormaAlvo::CirculoCurva => 2,
            FormaAlvo::Circulo | FormaAlvo::Retangulo | FormaAlvo::Area => 3,
        }
    }

    /// Tipo do alvo no modelo de troca de dados do AP242 (`placed_datum_target_feature`).
    pub fn tipo_step(self) -> &'static str {
        match self {
            FormaAlvo::Ponto => "point",
            FormaAlvo::Linha => "line",
            FormaAlvo::Circulo => "circle",
            FormaAlvo::CirculoCurva => "circular curve",
            FormaAlvo::Retangulo => "rectangle",
            FormaAlvo::Area => "area",
            FormaAlvo::Curva => "curve",
        }
    }

    /// Área e curva não têm descrição por dimensões: vão como alvo explícito, com a face (ou aresta)
    /// da própria peça (`advanced_face`).
    pub fn explicito_step(self) -> bool {
        matches!(self, FormaAlvo::Area | FormaAlvo::Curva)
    }
}

/// Alvo móvel: o ponto de apoio desliza na direção dada, por um curso, para tocar a peça.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct AlvoMovel {
    pub direcao: [f64; 3],
    /// curso total do apoio (mm)
    pub curso: f64,
}

/// Dados de um alvo de referência.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AlvoRef {
    pub forma: FormaAlvo,
    /// letra da referência (datum) a que o alvo pertence
    pub letra: String,
    /// número do alvo dentro da referência (A1, A2…)
    pub numero: u32,
    /// dimensões do alvo (mm): ver `FormaAlvo::dimensoes`
    pub dim: [f64; 2],
    /// sentido da dimensão maior (linha e retângulo), no espaço do modelo
    #[serde(default = "eixo_x")]
    pub orientacao: [f64; 3],
    #[serde(default)]
    pub movel: Option<AlvoMovel>,
    /// área restrita: o contorno limita a região da superfície que vale como referência
    #[serde(default)]
    pub restrita: bool,
    /// cotas (básicas) ligadas ao alvo: aparecem enquadradas
    #[serde(default)]
    pub cotas: Vec<Id>,
}

fn eixo_x() -> [f64; 3] {
    [1., 0., 0.]
}

impl Default for AlvoRef {
    fn default() -> Self {
        AlvoRef { forma: FormaAlvo::Ponto, letra: "A".into(), numero: 1, dim: [0.; 2], orientacao: eixo_x(), movel: None, restrita: false, cotas: vec![] }
    }
}

impl AlvoRef {
    /// "A1": letra e número, como no quadro do símbolo.
    pub fn identificador(&self) -> String {
        format!("{}{}", self.letra, self.numero)
    }

    /// Texto da metade de cima do quadro: as dimensões do alvo (vazio no ponto).
    pub fn texto_dimensoes(&self) -> String {
        let n = |v: f64| crate::pmi::num(v);
        match self.forma {
            FormaAlvo::Circulo => format!("Ø{}", n(self.dim[0])),
            FormaAlvo::CirculoCurva => format!("R{}", n(self.dim[0])),
            FormaAlvo::Linha => format!("L{}", n(self.dim[0])),
            FormaAlvo::Retangulo => format!("{}×{}", n(self.dim[0]), n(self.dim[1])),
            _ => String::new(),
        }
    }

    /// Texto completo da chamada, com a dimensão e o sinal de alvo móvel/restrito.
    pub fn rotulo(&self) -> String {
        let mut s = self.identificador();
        let d = self.texto_dimensoes();
        if !d.is_empty() {
            s = format!("{d} {s}");
        }
        if self.movel.is_some() {
            s.push_str(" ↔");
        }
        s
    }

    /// O alvo conta na regra 3-2-1? O móvel não: ele acompanha a peça e só completa o contato.
    pub fn conta_321(&self) -> bool {
        self.movel.is_none()
    }

    /// Dimensões coerentes com a forma (positivas onde a forma usa).
    pub fn valido(&self) -> bool {
        (0..self.forma.dimensoes()).all(|i| self.dim[i] > 0.) && self.movel.is_none_or(|m| m.curso > 0. && norma(m.direcao) > 1e-9)
    }
}

fn norma(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Caminho do símbolo: pontos 2D e se fecha.
pub type Caminho = (Vec<[f64; 2]>, bool);

/// Símbolo vetorial do alvo, centrado na origem, com `h` de tamanho de referência (mm de papel).
/// Ponto: ×. Linha: traço com × nas pontas. Superfícies: contorno com hachura a 45°. Curva: onda.
/// `dim` entra em unidades do mesmo `h` (o chamador já escalou).
pub fn simbolo(forma: FormaAlvo, h: f64, dim: [f64; 2]) -> Vec<Caminho> {
    let cruz = |c: [f64; 2], r: f64| -> Vec<Caminho> { vec![(vec![[c[0] - r, c[1] - r], [c[0] + r, c[1] + r]], false), (vec![[c[0] - r, c[1] + r], [c[0] + r, c[1] - r]], false)] };
    let r = h * 0.5;
    match forma {
        FormaAlvo::Ponto => cruz([0., 0.], r),
        FormaAlvo::Linha => {
            let meio = (dim[0].max(h * 2.)) / 2.;
            let mut v = vec![(vec![[-meio, 0.], [meio, 0.]], false)];
            v.extend(cruz([-meio, 0.], r * 0.7));
            v.extend(cruz([meio, 0.], r * 0.7));
            v
        }
        FormaAlvo::Circulo => {
            let rc = (dim[0].max(h * 1.5)) / 2.;
            let mut v = vec![(circulo(rc, 0., std::f64::consts::TAU, 40), true)];
            v.extend(hachura_circulo(rc, h * 0.35));
            v
        }
        FormaAlvo::CirculoCurva => {
            let rc = (dim[0].max(h * 1.5)) / 2.;
            vec![(circulo(rc, -0.9, 1.8, 24), false), (vec![[rc * (-0.9f64).cos(), rc * (-0.9f64).sin()], [rc * (-0.9f64).cos() * 0.8, rc * (-0.9f64).sin() * 0.8]], false)]
        }
        FormaAlvo::Retangulo => {
            let (w, a) = (dim[0].max(h * 2.) / 2., dim[1].max(h * 1.2) / 2.);
            let mut v = vec![(vec![[-w, -a], [w, -a], [w, a], [-w, a]], true)];
            v.extend(hachura_retangulo(w, a, h * 0.35));
            v
        }
        FormaAlvo::Area => {
            // contorno livre: elipse tracejada na prévia (o contorno real vem da face)
            let (w, a) = (h * 1.6, h * 1.0);
            let pts: Vec<[f64; 2]> = (0..32).map(|i| {
                let t = i as f64 / 32. * std::f64::consts::TAU;
                [w * t.cos(), a * t.sin()]
            }).collect();
            let mut v = vec![(pts, true)];
            v.extend(hachura_retangulo(w * 0.7, a * 0.7, h * 0.35));
            v
        }
        FormaAlvo::Curva => {
            let pts: Vec<[f64; 2]> = (0..=24).map(|i| {
                let t = i as f64 / 24.;
                [(t - 0.5) * h * 3., (t * std::f64::consts::TAU * 1.5).sin() * h * 0.35]
            }).collect();
            vec![(pts, false)]
        }
    }
}

fn circulo(r: f64, a0: f64, varre: f64, n: usize) -> Vec<[f64; 2]> {
    let passos = if (varre - std::f64::consts::TAU).abs() < 1e-9 { n } else { n + 1 };
    (0..passos).map(|i| {
        let t = a0 + varre * i as f64 / n as f64;
        [r * t.cos(), r * t.sin()]
    }).collect()
}

/// Cordas a 45° dentro do círculo, a cada `passo`.
fn hachura_circulo(r: f64, passo: f64) -> Vec<Caminho> {
    let mut v = vec![];
    let mut c = -r * std::f64::consts::SQRT_2;
    while c < r * std::f64::consts::SQRT_2 {
        // reta y = x + c  (distância da origem = |c|/√2)
        let d = c.abs() / std::f64::consts::SQRT_2;
        if d < r - 1e-9 {
            let meio = (r * r - d * d).sqrt();
            // ponto do pé da perpendicular e direção (1,1)/√2
            let pe = [-c / 2., c / 2.];
            let u = [std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2];
            v.push((vec![[pe[0] - u[0] * meio, pe[1] - u[1] * meio], [pe[0] + u[0] * meio, pe[1] + u[1] * meio]], false));
        }
        c += passo * std::f64::consts::SQRT_2;
    }
    v
}

/// Diagonais a 45° recortadas no retângulo [-w,w]×[-a,a].
fn hachura_retangulo(w: f64, a: f64, passo: f64) -> Vec<Caminho> {
    let mut v = vec![];
    let mut c = -(w + a);
    while c < w + a {
        // y = x + c cortada pelo retângulo
        let x0 = (-w).max(-a - c);
        let x1 = w.min(a - c);
        if x1 > x0 + 1e-9 {
            v.push((vec![[x0, x0 + c], [x1, x1 + c]], false));
        }
        c += passo * std::f64::consts::SQRT_2;
    }
    v
}

/// O que a regra 3-2-1 reclama, para a interface traduzir e mostrar.
#[derive(Clone, PartialEq, Debug)]
pub enum Aviso {
    /// o alvo aponta uma letra que não existe como referência na lista
    SemReferencia { letra: String },
    /// a referência de posição `ordem` (1 = primária) tem menos graus de liberdade tirados do que a regra pede
    Falta { letra: String, tem: u8, pede: u8 },
    /// ... ou mais
    Sobra { letra: String, tem: u8, pede: u8 },
    /// a referência primária com três pontos em linha reta não define plano
    Colineares { letra: String },
    /// há mais de três referências com alvo: a regra só tem primária, secundária e terciária
    ReferenciaExtra { letra: String },
    /// dois alvos da mesma referência com o mesmo número
    NumeroRepetido { id: String },
    /// dimensões nulas ou curso/direção do alvo móvel inválidos
    Invalido { id: String },
}

fn alvo_de(p: &Pmi) -> Option<&AlvoRef> {
    match &p.kind {
        PmiKind::DatumTarget { alvo } => Some(alvo),
        _ => None,
    }
}

/// Confere os alvos da lista contra a regra 3-2-1. A ordem das referências é a alfabética (A é a
/// primária, B a secundária, C a terciária), que é a convenção do desenho; alvos móveis não contam.
pub fn verificar_321(lista: &[Pmi]) -> Vec<Aviso> {
    let alvos: Vec<(&Pmi, &AlvoRef)> = lista.iter().filter(|p| p.visible).filter_map(|p| alvo_de(p).map(|a| (p, a))).collect();
    let mut avisos = vec![];
    let mut letras: Vec<&str> = alvos.iter().map(|(_, a)| a.letra.as_str()).collect();
    letras.sort();
    letras.dedup();
    for (i, letra) in letras.iter().enumerate() {
        let existe = lista.iter().any(|p| matches!(&p.kind, PmiKind::Datum { letter } if letter == letra));
        if !existe {
            avisos.push(Aviso::SemReferencia { letra: letra.to_string() });
        }
        let seus: Vec<&(&Pmi, &AlvoRef)> = alvos.iter().filter(|(_, a)| a.letra == *letra).collect();
        let Some(&pede) = [3u8, 2, 1].get(i) else {
            avisos.push(Aviso::ReferenciaExtra { letra: letra.to_string() });
            continue;
        };
        let tem: u8 = seus.iter().filter(|(_, a)| a.conta_321()).map(|(_, a)| a.forma.peso_321()).sum();
        if tem < pede {
            avisos.push(Aviso::Falta { letra: letra.to_string(), tem, pede });
        } else if tem > pede {
            avisos.push(Aviso::Sobra { letra: letra.to_string(), tem, pede });
        }
        // três pontos fixos em linha reta não definem plano
        if i == 0 {
            let pts: Vec<[f64; 3]> = seus.iter().filter(|(_, a)| a.conta_321() && a.forma == FormaAlvo::Ponto).filter_map(|(p, _)| p.anchors.first().copied()).collect();
            if pts.len() >= 3 && colineares(&pts) {
                avisos.push(Aviso::Colineares { letra: letra.to_string() });
            }
        }
        let mut vistos: Vec<u32> = vec![];
        for (_, a) in &seus {
            if vistos.contains(&a.numero) {
                avisos.push(Aviso::NumeroRepetido { id: a.identificador() });
            }
            vistos.push(a.numero);
        }
    }
    for (_, a) in &alvos {
        if !a.valido() {
            avisos.push(Aviso::Invalido { id: a.identificador() });
        }
    }
    avisos
}

fn colineares(p: &[[f64; 3]]) -> bool {
    let a = p[0];
    let Some(b) = p.iter().skip(1).find(|q| (0..3).any(|k| (q[k] - a[k]).abs() > 1e-9)) else { return true };
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let un = norma(u);
    p.iter().all(|q| {
        let w = [q[0] - a[0], q[1] - a[1], q[2] - a[2]];
        let c = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
        norma(c) / un < 1e-6
    })
}

/// Próximo número livre de alvo na referência `letra`.
pub fn proximo_numero(lista: &[Pmi], letra: &str) -> u32 {
    lista.iter().filter_map(alvo_de).filter(|a| a.letra == letra).map(|a| a.numero).max().unwrap_or(0) + 1
}

/// Cotas ligadas a algum alvo: elas aparecem enquadradas (cota básica). Só cotas de medida contam.
pub fn ids_enquadrados(lista: &[Pmi]) -> Vec<Id> {
    let ligadas: Vec<Id> = lista.iter().filter_map(alvo_de).flat_map(|a| a.cotas.iter().copied()).collect();
    lista.iter().filter(|p| ligadas.contains(&p.id) && p.kind.is_dimension()).map(|p| p.id).collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn pmi(id: u64, kind: PmiKind, a: [f64; 3]) -> Pmi {
        Pmi { id: Id(id), name: format!("p{id}"), kind, anchors: vec![a], offset: [0.; 3], text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets: vec![] }
    }

    fn alvo(id: u64, letra: &str, n: u32, forma: FormaAlvo, a: [f64; 3]) -> Pmi {
        let dim = match forma.dimensoes() {
            0 => [0.; 2],
            1 => [10., 0.],
            _ => [20., 10.],
        };
        pmi(id, PmiKind::DatumTarget { alvo: AlvoRef { forma, letra: letra.into(), numero: n, dim, ..Default::default() } }, a)
    }

    fn datum(id: u64, l: &str) -> Pmi {
        pmi(id, PmiKind::Datum { letter: l.into() }, [0.; 3])
    }

    #[test]
    fn identificador_e_dimensoes() {
        let mut a = AlvoRef { forma: FormaAlvo::Circulo, dim: [12., 0.], numero: 2, ..Default::default() };
        assert_eq!(a.identificador(), "A2");
        assert_eq!(a.texto_dimensoes(), "Ø12");
        a.forma = FormaAlvo::Retangulo;
        a.dim = [20., 8.];
        assert_eq!(a.rotulo(), "20×8 A2");
        a.movel = Some(AlvoMovel { direcao: [0., 0., 1.], curso: 3. });
        assert!(a.rotulo().ends_with('↔') && a.valido() && !a.conta_321());
        a.movel = Some(AlvoMovel { direcao: [0.; 3], curso: 3. });
        assert!(!a.valido(), "direção nula");
        assert_eq!(AlvoRef::default().texto_dimensoes(), "");
    }

    #[test]
    fn regra_321_completa_nao_avisa() {
        let l = vec![
            datum(1, "A"),
            datum(2, "B"),
            datum(3, "C"),
            alvo(10, "A", 1, FormaAlvo::Ponto, [0., 0., 0.]),
            alvo(11, "A", 2, FormaAlvo::Ponto, [50., 0., 0.]),
            alvo(12, "A", 3, FormaAlvo::Ponto, [0., 50., 0.]),
            alvo(13, "B", 1, FormaAlvo::Ponto, [0., 0., 10.]),
            alvo(14, "B", 2, FormaAlvo::Ponto, [50., 0., 10.]),
            alvo(15, "C", 1, FormaAlvo::Ponto, [0., 0., 20.]),
        ];
        assert_eq!(verificar_321(&l), vec![]);
    }

    #[test]
    fn regra_321_acusa_falta_sobra_colineares_e_referencia_ausente() {
        // A com dois pontos (falta 1), B sem datum e com três (sobra 1), A... colineares testado à parte
        let l = vec![
            datum(1, "A"),
            alvo(10, "A", 1, FormaAlvo::Ponto, [0., 0., 0.]),
            alvo(11, "A", 2, FormaAlvo::Ponto, [5., 0., 0.]),
            alvo(13, "B", 1, FormaAlvo::Ponto, [0., 0., 10.]),
            alvo(14, "B", 2, FormaAlvo::Ponto, [5., 0., 10.]),
            alvo(15, "B", 3, FormaAlvo::Ponto, [9., 0., 10.]),
        ];
        let v = verificar_321(&l);
        assert!(v.contains(&Aviso::Falta { letra: "A".into(), tem: 2, pede: 3 }));
        assert!(v.contains(&Aviso::Sobra { letra: "B".into(), tem: 3, pede: 2 }));
        assert!(v.contains(&Aviso::SemReferencia { letra: "B".into() }));
        let linha = vec![
            datum(1, "A"),
            alvo(10, "A", 1, FormaAlvo::Ponto, [0., 0., 0.]),
            alvo(11, "A", 2, FormaAlvo::Ponto, [5., 5., 5.]),
            alvo(12, "A", 3, FormaAlvo::Ponto, [10., 10., 10.]),
        ];
        assert_eq!(verificar_321(&linha), vec![Aviso::Colineares { letra: "A".into() }]);
    }

    #[test]
    fn linha_vale_dois_e_movel_nao_conta() {
        let mut l = vec![datum(1, "A"), alvo(10, "A", 1, FormaAlvo::Linha, [0., 0., 0.]), alvo(11, "A", 2, FormaAlvo::Ponto, [0., 20., 0.])];
        assert_eq!(verificar_321(&l), vec![]);
        if let PmiKind::DatumTarget { alvo } = &mut l[2].kind {
            alvo.movel = Some(AlvoMovel { direcao: [0., 0., 1.], curso: 2. });
        }
        assert_eq!(verificar_321(&l), vec![Aviso::Falta { letra: "A".into(), tem: 2, pede: 3 }]);
    }

    #[test]
    fn numero_repetido_e_proximo_numero() {
        let l = vec![datum(1, "A"), alvo(10, "A", 1, FormaAlvo::Area, [0.; 3]), alvo(11, "A", 1, FormaAlvo::Area, [9., 0., 0.])];
        assert!(verificar_321(&l).contains(&Aviso::NumeroRepetido { id: "A1".into() }));
        assert_eq!(proximo_numero(&l, "A"), 2);
        assert_eq!(proximo_numero(&l, "B"), 1);
    }

    #[test]
    fn cotas_ligadas_ficam_enquadradas() {
        let mut a = alvo(10, "A", 1, FormaAlvo::Ponto, [0.; 3]);
        if let PmiKind::DatumTarget { alvo } = &mut a.kind {
            alvo.cotas = vec![Id(20), Id(30)];
        }
        let l = vec![a, pmi(20, PmiKind::Linear, [0.; 3]), pmi(21, PmiKind::Linear, [0.; 3]), pmi(30, PmiKind::Note, [0.; 3])];
        assert_eq!(ids_enquadrados(&l), vec![Id(20)], "a nota ligada não é cota; a cota solta não entra");
    }

    #[test]
    fn simbolos_ficam_dentro_do_contorno_e_tem_tipo_step() {
        for f in FormaAlvo::ALL {
            let s = simbolo(f, 3.5, [10., 6.]);
            assert!(!s.is_empty() && s.iter().all(|(p, _)| p.len() >= 2 && p.iter().all(|q| q[0].is_finite() && q[1].is_finite())), "{f:?}");
        }
        // hachura do círculo fica dentro do raio
        let r = 5.;
        assert!(hachura_circulo(r, 1.).iter().all(|(p, _)| p.iter().all(|q| (q[0] * q[0] + q[1] * q[1]).sqrt() <= r + 1e-9)));
        assert!(hachura_retangulo(4., 2., 1.).iter().all(|(p, _)| p.iter().all(|q| q[0].abs() <= 4. + 1e-9 && q[1].abs() <= 2. + 1e-9)));
        assert_eq!(FormaAlvo::Retangulo.tipo_step(), "rectangle");
        assert!(FormaAlvo::Area.explicito_step() && !FormaAlvo::Ponto.explicito_step());
    }

    #[test]
    fn documento_serializa_e_volta() {
        let k = PmiKind::DatumTarget { alvo: AlvoRef { forma: FormaAlvo::Linha, movel: Some(AlvoMovel { direcao: [0., 1., 0.], curso: 4. }), restrita: true, ..Default::default() } };
        let j = serde_json::to_string(&k).unwrap();
        assert_eq!(serde_json::from_str::<PmiKind>(&j).unwrap(), k);
    }
}
