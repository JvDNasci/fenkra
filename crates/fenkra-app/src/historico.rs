//! Histórico de desfazer/refazer com nome por entrada e teto de memória (B68).
//!
//! Cada entrada guarda o documento de *antes* da edição e um nome curto ("Criar Extrusão 1",
//! "Suprimir Furo 2"). Ao desfazer, o documento atual vai para a pilha de refazer com o mesmo
//! nome. O teto em MB descarta as entradas mais antigas: o clone de documento é barato (0,23 ms
//! com 300 comandos), mas a pilha acumulada não tinha limite.

use fenkra_core::doc::Document;

pub struct Entrada {
    pub nome: String,
    pub doc: Document,
    bytes: usize,
}

pub struct Historico {
    desfazer: Vec<Entrada>,
    refazer: Vec<Entrada>,
    /// teto somado das duas pilhas, em bytes (0 = sem teto)
    limite: usize,
}

/// Tamanho aproximado do documento na memória (usa o tamanho serializado, que acompanha o real).
fn bytes_de(d: &Document) -> usize {
    serde_json::to_vec(d).map(|v| v.len()).unwrap_or(0)
}

impl Historico {
    pub fn new(limite_mb: u32) -> Self {
        Historico { desfazer: vec![], refazer: vec![], limite: limite_mb as usize * 1024 * 1024 }
    }

    pub fn definir_limite_mb(&mut self, mb: u32) {
        let novo = mb as usize * 1024 * 1024;
        if novo != self.limite {
            self.limite = novo;
            self.aparar();
        }
    }

    /// Edição dentro da mesma rajada (agrupada na entrada anterior): só invalida o refazer.
    pub fn invalidar_refazer(&mut self) {
        self.refazer.clear();
    }

    pub fn limpar(&mut self) {
        self.desfazer.clear();
        self.refazer.clear();
    }

    /// Registra o estado anterior a uma edição; invalida o refazer.
    pub fn registrar(&mut self, antes: Document, depois: &Document) {
        let nome = nome_da_edicao(&antes, depois);
        let bytes = bytes_de(&antes);
        self.desfazer.push(Entrada { nome, doc: antes, bytes });
        self.refazer.clear();
        self.aparar();
    }

    pub fn pode_desfazer(&self) -> bool {
        !self.desfazer.is_empty()
    }
    pub fn pode_refazer(&self) -> bool {
        !self.refazer.is_empty()
    }
    pub fn nome_desfazer(&self) -> Option<&str> {
        self.desfazer.last().map(|e| e.nome.as_str())
    }
    pub fn nome_refazer(&self) -> Option<&str> {
        self.refazer.last().map(|e| e.nome.as_str())
    }
    #[cfg(test)]
    pub fn quantos(&self) -> usize {
        self.desfazer.len()
    }
    pub fn bytes(&self) -> usize {
        self.desfazer.iter().chain(&self.refazer).map(|e| e.bytes).sum()
    }

    /// Desfaz: devolve o documento a restaurar e o nome da edição desfeita.
    pub fn desfazer(&mut self, atual: Document) -> Option<(Document, String)> {
        let e = self.desfazer.pop()?;
        let bytes = bytes_de(&atual);
        self.refazer.push(Entrada { nome: e.nome.clone(), doc: atual, bytes });
        Some((e.doc, e.nome))
    }

    pub fn refazer(&mut self, atual: Document) -> Option<(Document, String)> {
        let e = self.refazer.pop()?;
        let bytes = bytes_de(&atual);
        self.desfazer.push(Entrada { nome: e.nome.clone(), doc: atual, bytes });
        Some((e.doc, e.nome))
    }

    /// Descarta as mais antigas até caber no teto (sempre mantém ao menos a mais recente).
    fn aparar(&mut self) {
        while self.limite > 0 && self.desfazer.len() > 1 && self.bytes() > self.limite {
            self.desfazer.remove(0);
        }
    }
}

/// Nome curto da edição, comparando o documento antes e depois.
pub fn nome_da_edicao(antes: &Document, depois: &Document) -> String {
    use crate::i18n::tr;
    let (a, d) = (&antes.features, &depois.features);
    if d.len() > a.len() {
        let novo = d.iter().find(|f| !a.iter().any(|x| x.id == f.id)).or(d.last());
        return format!("{} {}", tr("Criar"), novo.map(|f| f.name.as_str()).unwrap_or(""));
    }
    if d.len() < a.len() {
        let ido = a.iter().find(|f| !d.iter().any(|x| x.id == f.id));
        return format!("{} {}", tr("Apagar"), ido.map(|f| f.name.as_str()).unwrap_or(""));
    }
    if a.iter().map(|f| f.id).ne(d.iter().map(|f| f.id)) {
        return tr("Reordenar comandos").to_string();
    }
    for (x, y) in a.iter().zip(d) {
        if x == y {
            continue;
        }
        let verbo = if x.suppressed != y.suppressed {
            if y.suppressed {
                "Suprimir"
            } else {
                "Reativar"
            }
        } else if x.hidden != y.hidden {
            if y.hidden {
                "Ocultar"
            } else {
                "Exibir"
            }
        } else if x.name != y.name {
            "Renomear"
        } else {
            "Editar"
        };
        return format!("{} {}", tr(verbo), if verbo == "Renomear" { &x.name } else { &y.name });
    }
    if antes.rollback != depois.rollback {
        return tr("Mover o marcador de retrocesso").to_string();
    }
    if antes.params != depois.params {
        return tr("Editar parâmetros").to_string();
    }
    tr("Editar o documento").to_string()
}

#[cfg(test)]
mod testes {
    use super::*;
    use fenkra_core::doc::FeatureKind;

    fn doc_com(n: usize) -> Document {
        let mut d = Document::new("t");
        for i in 0..n {
            let id = d.add_feature(FeatureKind::Sketch(Default::default()));
            d.feature_mut(id).unwrap().name = format!("Sketch {}", i + 1);
        }
        d
    }

    #[test]
    fn nomeia_criar_apagar_suprimir() {
        let a = doc_com(1);
        let mut b = a.clone();
        let id = b.add_feature(FeatureKind::Sketch(Default::default()));
        b.feature_mut(id).unwrap().name = "Sketch 2".into();
        assert_eq!(nome_da_edicao(&a, &b), "Criar Sketch 2");
        assert_eq!(nome_da_edicao(&b, &a), "Apagar Sketch 2");
        let mut c = a.clone();
        c.features[0].suppressed = true;
        assert_eq!(nome_da_edicao(&a, &c), "Suprimir Sketch 1");
    }

    #[test]
    fn desfazer_e_refazer_levam_o_nome() {
        let a = doc_com(1);
        let mut b = a.clone();
        b.features[0].hidden = true;
        let mut h = Historico::new(64);
        h.registrar(a.clone(), &b);
        assert_eq!(h.nome_desfazer(), Some("Ocultar Sketch 1"));
        let (volta, nome) = h.desfazer(b.clone()).unwrap();
        assert_eq!(nome, "Ocultar Sketch 1");
        assert!(!volta.features[0].hidden);
        assert_eq!(h.nome_refazer(), Some("Ocultar Sketch 1"));
        let (de_novo, _) = h.refazer(volta).unwrap();
        assert!(de_novo.features[0].hidden);
        assert!(!h.pode_refazer());
    }

    #[test]
    fn teto_descarta_as_mais_antigas() {
        let grande = doc_com(40);
        let mut h = Historico::new(0);
        for _ in 0..20 {
            h.registrar(grande.clone(), &grande);
        }
        let um = h.bytes() / 20;
        h.limite = um * 5;
        h.aparar();
        assert!(h.quantos() <= 5 && h.quantos() >= 1, "ficou com {}", h.quantos());
        assert!(h.bytes() <= um * 5);
    }
}
