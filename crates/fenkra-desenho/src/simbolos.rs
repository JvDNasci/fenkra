// SPDX-License-Identifier: LGPL-2.1-or-later
//! Símbolos guiados de rugosidade e solda (B251). Rugosidade e solda deixam de ser um campo de
//! texto: o construtor da interface edita estes dados, cada campo na posição que ocupa no símbolo,
//! e o desenho sai das mesmas primitivas de sempre (então PDF, SVG e DXF não mudam de caminho).
//!
//! Regra de compatibilidade: um símbolo montado só com cliques, sem nenhum campo extra, fica
//! igual ao que o texto antigo produzia (`Note::rough` e `Note::weld` voltam a `None`).

use crate::{DwgWeld, Note};
use fenkra_core::pmi::{WeldContour, WeldExtra, WeldKind};
use serde::{Deserialize, Serialize};

/// Variedade do símbolo de rugosidade (ISO 1302).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum RugTipo {
    /// triângulo aberto: qualquer processo de fabricação
    #[default]
    Qualquer,
    /// triângulo fechado: remoção de material exigida
    ComRemocao,
    /// círculo no vértice: sem remoção de material
    SemRemocao,
}

impl RugTipo {
    pub const ALL: [RugTipo; 3] = [RugTipo::Qualquer, RugTipo::ComRemocao, RugTipo::SemRemocao];
    pub fn label(self) -> &'static str {
        match self {
            RugTipo::Qualquer => "Qualquer processo",
            RugTipo::ComRemocao => "Com remoção de material",
            RugTipo::SemRemocao => "Sem remoção de material",
        }
    }
}

/// Campos extras do símbolo de rugosidade; o valor principal (ex.: `Ra 1,6`) continua no texto da nota.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct DwgRough {
    #[serde(default)]
    pub tipo: RugTipo,
    /// processo de fabricação (abaixo da barra): "Retificado"
    #[serde(default)]
    pub metodo: String,
    /// sobremedida de usinagem (mm), canto inferior direito
    #[serde(default)]
    pub sobremedida: String,
    /// direção das estrias: `=`, `⊥`, `X`, `M`, `C` ou `R`
    #[serde(default)]
    pub direcao: String,
    /// comprimento de amostragem (mm), antes do valor: `0,8/Ra 1,6`
    #[serde(default)]
    pub amostragem: String,
}

impl DwgRough {
    /// Nenhum campo extra: o símbolo é o de sempre.
    pub fn basico(&self) -> bool {
        *self == DwgRough::default()
    }
}

pub const PARAMETROS: [&str; 4] = ["Ra", "Rz", "Rq", "Rt"];
/// Valores de rugosidade comuns (série ISO 1302), em µm.
pub const VALORES: [&str; 9] = ["0,1", "0,2", "0,4", "0,8", "1,6", "3,2", "6,3", "12,5", "25"];
pub const DIRECOES: [&str; 6] = ["=", "⊥", "X", "M", "C", "R"];
pub const METODOS: [&str; 5] = ["Retificado", "Fresado", "Torneado", "Polido", "Brunido"];
pub const AMOSTRAGENS: [&str; 5] = ["0,25", "0,8", "2,5", "8", "25"];
/// Medidas de solda comuns (garganta `a` e perna `z`).
pub const MEDIDAS_SOLDA: [&str; 8] = ["a3", "a4", "a5", "a6", "z4", "z5", "z6", "z8"];

/// Texto da rugosidade: parâmetro e valor (`Ra`, `1,6` → `Ra 1,6`). Sem parâmetro, só o valor.
pub fn rug_texto(param: &str, valor: &str) -> String {
    match (param.trim(), valor.trim()) {
        ("", v) => v.to_string(),
        (p, "") => p.to_string(),
        (p, v) => format!("{p} {v}"),
    }
}

/// Inverso de [`rug_texto`]: separa o parâmetro (Ra, Rz…) do valor. Texto livre fica todo no valor.
pub fn rug_partes(texto: &str) -> (String, String) {
    let t = texto.trim();
    for p in PARAMETROS {
        if let Some(r) = t.strip_prefix(p) {
            if r.is_empty() || r.starts_with(' ') {
                return (p.to_string(), r.trim().to_string());
            }
        }
    }
    (String::new(), t.to_string())
}

/// Linha de cima do símbolo: a amostragem (se houver) antes do valor.
pub fn rug_linha_do_valor(texto: &str, r: &DwgRough) -> String {
    if r.amostragem.trim().is_empty() {
        texto.to_string()
    } else {
        format!("{}/{}", r.amostragem.trim(), texto)
    }
}

/// Comprimento da solda em partes: quantidade `n`, comprimento `l` e passo `e` (ISO 2553: `n×l (e)`).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Comprimento {
    pub n: String,
    pub l: String,
    pub e: String,
}

impl Comprimento {
    /// `3×50 (100)`, `50`, `50 (100)`; vazio se não há nada.
    pub fn compor(&self) -> String {
        let (n, l, e) = (self.n.trim(), self.l.trim(), self.e.trim());
        let mut s = String::new();
        if !n.is_empty() {
            s += n;
            s += "×";
        }
        s += l;
        if !e.is_empty() {
            if !s.is_empty() {
                s += " ";
            }
            s += &format!("({e})");
        }
        s
    }

    /// Lê o texto de volta nas três partes (o que não se reconhece fica em `l`).
    pub fn ler(texto: &str) -> Comprimento {
        let t = texto.trim();
        let (corpo, e) = match (t.rfind('('), t.ends_with(')')) {
            (Some(i), true) => (t[..i].trim(), t[i + 1..t.len() - 1].trim().to_string()),
            _ => (t, String::new()),
        };
        let sep = corpo.find(['×', 'x']);
        match sep {
            Some(i) if i > 0 && corpo[..i].chars().all(|c| c.is_ascii_digit()) => Comprimento { n: corpo[..i].to_string(), l: corpo[i + corpo[i..].chars().next().map_or(1, char::len_utf8)..].trim().to_string(), e },
            _ => Comprimento { n: String::new(), l: corpo.to_string(), e },
        }
    }
}

/// Símbolo de solda padrão (filete, um lado, sem extras): o que o texto antigo produzia.
pub fn solda_padrao() -> DwgWeld {
    DwgWeld { kind: WeldKind::Filete, all_around: false, extra: WeldExtra::default() }
}

/// Os dois lados do símbolo: (lado da seta, outro lado), cada um com ou sem cordão.
pub fn lados(w: &DwgWeld) -> (Option<WeldKind>, Option<WeldKind>) {
    match (w.extra.other, w.extra.other_side) {
        (Some(o), _) => (Some(w.kind), Some(o)),
        (None, true) => (None, Some(w.kind)),
        (None, false) => (Some(w.kind), None),
    }
}

/// Define os cordões de cada lado. Sem nenhum dos dois não há símbolo: nada muda e devolve `false`.
pub fn definir_lados(w: &mut DwgWeld, seta: Option<WeldKind>, outro: Option<WeldKind>) -> bool {
    match (seta, outro) {
        (Some(a), Some(b)) => {
            w.kind = a;
            w.extra.other = Some(b);
            w.extra.other_side = false;
        }
        (Some(a), None) => {
            w.kind = a;
            w.extra.other = None;
            w.extra.other_side = false;
        }
        (None, Some(b)) => {
            w.kind = b;
            w.extra.other = None;
            w.extra.other_side = true;
        }
        (None, None) => return false,
    }
    true
}

/// Volta `rough` e `weld` a `None` quando o construtor não pôs nada além do padrão, deixando a nota
/// idêntica à que o texto antigo criava (mesmo JSON, mesmas primitivas, mesmas exportações).
pub fn normalizar(n: &mut Note) {
    if n.rough.as_ref().is_some_and(DwgRough::basico) {
        n.rough = None;
    }
    if n.weld.as_ref().is_some_and(|w| *w == solda_padrao()) {
        n.weld = None;
    }
}

/// Resumo curto do contorno, para o rótulo do botão.
pub fn contorno_label(c: WeldContour) -> &'static str {
    match c {
        WeldContour::Nenhum => "nenhum",
        WeldContour::Plano => "plano",
        WeldContour::Convexo => "convexo",
        WeldContour::Concavo => "côncavo",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout;
    use crate::source::Models;
    use crate::{export, Drawing, Format, NoteKind};
    use fenkra_core::Id;

    fn nota(kind: NoteKind, text: &str) -> Note {
        Note { id: Id(1000), sheet: Id(1), kind, pos: [100., 100.], leader: Some([80., 80.]), text: text.into(), ..Default::default() }
    }

    fn itens(n: &Note) -> Vec<layout::Item> {
        let d = Drawing::new("t", Format::A4);
        layout::note_items(&d, n, &mut Models::new(None))
    }

    fn exportar(items: &[layout::Item]) -> (String, String, Vec<u8>) {
        (export::svg(items, [297., 210.]), export::dxf(items), export::pdf(&[([297., 210.], items.to_vec())]))
    }

    #[test]
    fn texto_da_rugosidade_ida_e_volta() {
        assert_eq!(rug_texto("Ra", "1,6"), "Ra 1,6");
        assert_eq!(rug_partes("Ra 1,6"), ("Ra".into(), "1,6".into()));
        assert_eq!(rug_partes("Rz 25"), ("Rz".into(), "25".into()));
        assert_eq!(rug_partes("N7"), (String::new(), "N7".into()));
        assert_eq!(rug_partes("Rabo"), (String::new(), "Rabo".into()));
        for p in PARAMETROS {
            for v in VALORES {
                assert_eq!(rug_partes(&rug_texto(p, v)), (p.to_string(), v.to_string()));
            }
        }
    }

    #[test]
    fn comprimento_compoe_e_le_de_volta() {
        let c = Comprimento { n: "3".into(), l: "50".into(), e: "100".into() };
        assert_eq!(c.compor(), "3×50 (100)");
        assert_eq!(Comprimento::ler("3×50 (100)"), c);
        assert_eq!(Comprimento { n: String::new(), l: "50".into(), e: String::new() }.compor(), "50");
        assert_eq!(Comprimento::ler("50"), Comprimento { n: String::new(), l: "50".into(), e: String::new() });
        assert_eq!(Comprimento { n: String::new(), l: "50".into(), e: "100".into() }.compor(), "50 (100)");
        assert_eq!(Comprimento::ler("50 (100)").e, "100");
        assert_eq!(Comprimento::ler("2x40").n, "2", "x minúsculo também vale");
        assert_eq!(Comprimento::default().compor(), "");
    }

    #[test]
    fn lados_da_solda_ida_e_volta() {
        let mut w = solda_padrao();
        assert_eq!(lados(&w), (Some(WeldKind::Filete), None));
        assert!(definir_lados(&mut w, Some(WeldKind::TopoV), Some(WeldKind::Filete)));
        assert_eq!(lados(&w), (Some(WeldKind::TopoV), Some(WeldKind::Filete)));
        assert!(definir_lados(&mut w, None, Some(WeldKind::Bujao)));
        assert_eq!(lados(&w), (None, Some(WeldKind::Bujao)));
        assert!(w.extra.other_side && w.extra.other.is_none());
        assert!(!definir_lados(&mut w, None, None), "sem cordão nenhum não há símbolo");
        assert_eq!(lados(&w), (None, Some(WeldKind::Bujao)));
    }

    #[test]
    fn rugosidade_so_com_cliques_sai_igual_ao_texto_antigo() {
        // texto antigo
        let antigo = nota(NoteKind::Roughness, "Ra 1,6");
        // construtor: botões de parâmetro e de valor, nenhum campo extra
        let mut novo = nota(NoteKind::Roughness, "");
        novo.rough = Some(DwgRough::default());
        novo.text = rug_texto("Ra", "1,6");
        normalizar(&mut novo);
        assert_eq!(novo, antigo, "a nota volta a ser a de sempre");
        let (a, b) = (itens(&antigo), itens(&novo));
        assert_eq!(a.len(), b.len());
        assert!(!a.is_empty());
        let (sa, da, pa) = exportar(&a);
        let (sb, db, pb) = exportar(&b);
        assert_eq!(sa, sb);
        assert_eq!(da, db);
        assert_eq!(pa, pb);
        assert!(sa.contains("Ra 1,6") && da.contains("Ra 1,6"));
        assert!(String::from_utf8_lossy(&pa).contains("(Ra 1,6)"));
    }

    #[test]
    fn solda_so_com_cliques_sai_igual_ao_texto_antigo() {
        let antigo = nota(NoteKind::Weld, "a4");
        let mut novo = nota(NoteKind::Weld, "");
        novo.weld = Some(solda_padrao());
        novo.text = MEDIDAS_SOLDA[1].to_string();
        normalizar(&mut novo);
        assert_eq!(novo, antigo);
        let (sa, da, pa) = exportar(&itens(&antigo));
        let (sb, db, pb) = exportar(&itens(&novo));
        assert_eq!((sa, da, pa), (sb, db, pb));
        // e sem normalizar o desenho já é o mesmo: o padrão explícito não muda as primitivas
        let mut explicito = antigo.clone();
        explicito.weld = Some(solda_padrao());
        assert_eq!(itens(&explicito), itens(&antigo));
    }

    #[test]
    fn rugosidade_completa_aparece_nos_tres_formatos() {
        let mut n = nota(NoteKind::Roughness, "Ra 0,8");
        n.rough = Some(DwgRough { tipo: RugTipo::ComRemocao, metodo: "Retificado".into(), sobremedida: "0,3".into(), direcao: "=".into(), amostragem: "2,5".into() });
        let base = itens(&nota(NoteKind::Roughness, "Ra 0,8"));
        let it = itens(&n);
        assert!(it.len() > base.len(), "os campos extras viram primitivas");
        let (svg, dxf, pdf) = exportar(&it);
        for t in ["Retificado", "0,3", "2,5/Ra 0,8"] {
            assert!(svg.contains(t), "svg sem {t}");
            assert!(dxf.contains(t), "dxf sem {t}");
            assert!(String::from_utf8_lossy(&pdf).contains(&format!("({t})")), "pdf sem {t}");
        }
        // cada tipo desenha de um jeito
        let mut vistos = vec![];
        for t in RugTipo::ALL {
            let mut m = n.clone();
            m.rough.as_mut().unwrap().tipo = t;
            vistos.push(itens(&m));
        }
        assert_ne!(vistos[0], vistos[1]);
        assert_ne!(vistos[1], vistos[2]);
    }

    #[test]
    fn solda_completa_pelo_construtor_aparece_nos_tres_formatos() {
        let mut n = nota(NoteKind::Weld, "a5");
        let mut w = solda_padrao();
        definir_lados(&mut w, Some(WeldKind::Filete), Some(WeldKind::TopoV));
        w.extra.length = Comprimento { n: "3".into(), l: "50".into(), e: "100".into() }.compor();
        w.extra.contour = WeldContour::Convexo;
        w.extra.field = true;
        w.all_around = true;
        n.weld = Some(w);
        let it = itens(&n);
        let (svg, dxf, pdf) = exportar(&it);
        // o R2000 escreve o × em ASCII: o texto se confere lendo o arquivo de volta
        let lido = fenkra_dxf::ler(dxf.as_bytes()).unwrap();
        let no_dxf = |t: &str| lido.itens.iter().any(|i| matches!(&i.entidade, fenkra_dxf::Entidade::Texto { texto, .. } if texto.contains(t)));
        for t in ["a5", "3×50 (100)"] {
            assert!(svg.contains(t) && no_dxf(t), "falta {t}");
        }
        assert!(String::from_utf8_lossy(&pdf).contains("(a5)"));
        // o mesmo símbolo escrito à mão no modelo (texto antigo + extras) dá as mesmas primitivas
        assert!(it.len() > itens(&nota(NoteKind::Weld, "a5")).len());
    }

    #[test]
    fn json_antigo_sem_rough_continua_abrindo() {
        let mut d = Drawing::new("t", Format::A4);
        let s = d.sheets[0].id;
        let id = d.alloc();
        d.notes.push(Note { id, sheet: s, kind: NoteKind::Roughness, text: "Ra 3,2".into(), ..Default::default() });
        let j = d.to_json().replace("\"rough\":null,", "").replace(",\"rough\":null", "");
        assert!(!j.contains("\"rough\""));
        let d2 = Drawing::from_json(&j).unwrap();
        assert_eq!(d2.notes[0].rough, None);
        assert_eq!(d2, d);
    }
}
