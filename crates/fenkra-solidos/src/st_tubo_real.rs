// SPDX-License-Identifier: LGPL-2.1-or-later
//! B176: medidas reais do tubo, para a curva de encaixe (B155) e o caminho de corte (B163).
//!
//! O tubo que chega à máquina não é o nominal: o diâmetro e a parede variam dentro da tolerância
//! da norma de fabricação, o redondo sai ovalizado, o retangular tem raio nos cantos e a costura de
//! solda fica numa posição do perímetro. Quem mede o lote anota aqui, por designação, e o Fenkra usa:
//!
//! * diâmetro externo e parede reais entram na seção da curva de encaixe (do membro cortado e do
//!   hospedeiro), então o `λ` de cada ponto já sai com a medida do lote;
//! * a ovalização (D máx. − D mín.) vira uma faixa de incerteza da ponta: num tubo elíptico de
//!   semieixos D/2 ± o/4, o raio varia o/4 para cada lado, e é essa a folga que a curva nominal pode
//!   errar na pele do hospedeiro;
//! * o raio de canto do retangular encurta a pele externa em (8 − 2π)·r, que é o perímetro que a
//!   vista desenvolvida e o comprimento de corte devem usar;
//! * a posição da costura (graus a partir da referência do perímetro, o meio da face +x ou o ponto
//!   s = 0 do redondo) passa a ser o zero da vista desenvolvida e do início do corte, que assim pode
//!   fugir dela.
//!
//! Guarda-se nas propriedades personalizadas da peça (`Tubo real · <designação>`), como texto
//! `d=60.1; t=3.5; ov=0.4; r=4; costura=90`, para viajar com o arquivo sem mudar o formato.

use crate::st_encaixe::Secao;
use fenkra_core::Document;

/// Medidas reais de um lote. Zero quer dizer "a nominal" (ou nenhuma ovalização, canto vivo,
/// costura na referência).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TuboReal {
    /// diâmetro externo real (redondo) ou 0
    pub diametro: f64,
    /// parede real ou 0
    pub parede: f64,
    /// ovalização: diâmetro máximo menos o mínimo
    pub ovalizacao: f64,
    /// raio externo de canto do retangular
    pub raio_canto: f64,
    /// posição da costura, em graus a partir da referência do perímetro
    pub costura: f64,
}

const PREFIXO: &str = "Tubo real · ";

impl TuboReal {
    pub fn vazio(&self) -> bool {
        *self == TuboReal::default()
    }

    /// Texto guardado na propriedade.
    pub fn texto(&self) -> String {
        format!("d={}; t={}; ov={}; r={}; costura={}", self.diametro, self.parede, self.ovalizacao, self.raio_canto, self.costura)
    }

    /// Lê o texto da propriedade; campos desconhecidos ou inválidos ficam em zero.
    pub fn de_texto(t: &str) -> TuboReal {
        let mut r = TuboReal::default();
        for par in t.split(';') {
            let Some((k, v)) = par.split_once('=') else { continue };
            let v: f64 = v.trim().replace(',', ".").parse().unwrap_or(0.);
            let v = if v.is_finite() { v } else { 0. };
            match k.trim() {
                "d" => r.diametro = v.max(0.),
                "t" => r.parede = v.max(0.),
                "ov" => r.ovalizacao = v.max(0.),
                "r" => r.raio_canto = v.max(0.),
                "costura" => r.costura = v.rem_euclid(360.),
                _ => {}
            }
        }
        r
    }

    /// A seção com as medidas reais (o retangular só troca a parede).
    pub fn aplicar(&self, s: Secao) -> Secao {
        match s {
            Secao::Redonda { d, t } => Secao::Redonda { d: if self.diametro > 0. { self.diametro } else { d }, t: if self.parede > 0. && t > 0. { self.parede } else { t } },
            Secao::Retangular { b, h, t } => Secao::Retangular { b, h, t: if self.parede > 0. && t > 0. { self.parede } else { t } },
        }
    }

    /// Perímetro da pele externa com o raio de canto (retangular) e o diâmetro real (redondo).
    pub fn perimetro(&self, s: Secao) -> f64 {
        match self.aplicar(s) {
            Secao::Redonda { d, .. } => std::f64::consts::PI * d,
            Secao::Retangular { b, h, .. } => {
                let r = self.raio_canto.min(b.min(h) / 2.);
                2. * (b + h) - (8. - 2. * std::f64::consts::PI) * r
            }
        }
    }

    /// Quanto a pele de um redondo ovalizado pode estar fora do círculo nominal, para cada lado.
    pub fn desvio_radial(&self) -> f64 {
        self.ovalizacao / 4.
    }
}

/// Medidas reais guardadas para a designação (padrão = nominal).
pub fn ler(doc: &Document, designacao: &str) -> TuboReal {
    let chave = format!("{PREFIXO}{}", designacao.trim());
    doc.props.custom.iter().find(|(k, _)| *k == chave).map(|(_, v)| TuboReal::de_texto(v)).unwrap_or_default()
}

/// Grava (ou apaga, se for tudo nominal) as medidas reais da designação.
pub fn gravar(doc: &mut Document, designacao: &str, r: &TuboReal) {
    let chave = format!("{PREFIXO}{}", designacao.trim());
    doc.props.custom.retain(|(k, _)| *k != chave);
    if !r.vazio() {
        doc.props.custom.push((chave, r.texto()));
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn texto_vai_e_volta_e_aceita_virgula() {
        let r = TuboReal { diametro: 60.1, parede: 3.5, ovalizacao: 0.4, raio_canto: 4., costura: 90. };
        assert_eq!(TuboReal::de_texto(&r.texto()), r);
        assert_eq!(TuboReal::de_texto("d=60,1; costura=450; lixo; t=-2").diametro, 60.1);
        assert_eq!(TuboReal::de_texto("costura=450").costura, 90.);
        assert_eq!(TuboReal::de_texto("t=-2").parede, 0.);
    }

    #[test]
    fn perimetro_com_raio_de_canto_bate_com_a_conta() {
        // 40x40 com raio externo 4: 160 − (8 − 2π)·4 = 160 − 6,8673 = 153,1327
        let r = TuboReal { raio_canto: 4., ..Default::default() };
        let p = r.perimetro(Secao::Retangular { b: 40., h: 40., t: 3. });
        assert!((p - 153.1327).abs() < 1e-4, "{p}");
        // redondo real 60,1: π·60,1 = 188,8097
        let r = TuboReal { diametro: 60.1, ..Default::default() };
        assert!((r.perimetro(Secao::Redonda { d: 60.3, t: 3.6 }) - 188.8097).abs() < 1e-4);
        assert_eq!(r.aplicar(Secao::Redonda { d: 60.3, t: 3.6 }), Secao::Redonda { d: 60.1, t: 3.6 });
        // barra maciça continua maciça
        let r = TuboReal { parede: 3., ..Default::default() };
        assert_eq!(r.aplicar(Secao::Redonda { d: 20., t: 0. }), Secao::Redonda { d: 20., t: 0. });
        assert!((TuboReal { ovalizacao: 0.6, ..Default::default() }.desvio_radial() - 0.15).abs() < 1e-12);
    }

    #[test]
    fn encaixe_usa_o_diametro_real() {
        use fenkra_core::*;
        let s = |x: &str| x.to_string();
        let mut d = Document::new("t");
        let mut l = |a: [f64; 3], b: [f64; 3]| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }));
        let paths = vec![l([0., 0., 0.], [0., 0., 600.]), l([800., 0., 0.], [800., 0., 600.]), l([0., 0., 300.], [800., 0., 300.])];
        d.add_feature(FeatureKind::StructMember(StructMember {
            paths,
            family: s("Tubo redondo"),
            designation: s("60,3x3,6"),
            kind: SectionKind::TuboRedondo,
            dims: vec![s("60.3"), s("3.6")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::Encaixe,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
            mirror: false,
            tube_bend: None, fit: Default::default(),
        }));
        let curva = |d: &Document| {
            let m = crate::st_encaixe_doc::encaixes_da_estrutura(d, &Default::default()).into_iter().find(|m| m.ponta1.is_some() && m.ponta2.is_some()).unwrap();
            m.ponta1.unwrap().corte
        };
        let nominal = curva(&d);
        assert!((nominal.perimetro - std::f64::consts::PI * 60.3).abs() < 1e-9);
        gravar(&mut d, "60,3x3,6", &TuboReal { diametro: 59.7, parede: 3.2, ..Default::default() });
        let real = curva(&d);
        assert!((real.perimetro - std::f64::consts::PI * 59.7).abs() < 1e-9);
        assert_eq!(real.secao, Secao::Redonda { d: 59.7, t: 3.2 });
        // tubo igual em tubo igual, perpendicular: a ponta mais funda chega ao raio do hospedeiro
        assert!((nominal.lambda_max() - 60.3 / 2.).abs() < 0.05, "{}", nominal.lambda_max());
        assert!((real.lambda_max() - 59.7 / 2.).abs() < 0.05, "{}", real.lambda_max());
    }

    #[test]
    fn grava_e_le_no_documento() {
        let mut d = Document::new("t");
        let r = TuboReal { diametro: 60.1, costura: 45., ..Default::default() };
        gravar(&mut d, "60,3x3,6", &r);
        assert_eq!(ler(&d, "60,3x3,6"), r);
        assert_eq!(ler(&d, "48,3x3"), TuboReal::default());
        gravar(&mut d, "60,3x3,6", &TuboReal::default());
        assert!(d.props.custom.is_empty());
    }
}
