// SPDX-License-Identifier: LGPL-2.1-or-later
//! Soltar um item arrastado na árvore (B446).
//!
//! A árvore desenha o que o documento guarda: a pasta de comandos aparece dentro do corpo dos seus
//! membros, na posição do primeiro deles, e o corpo lista os comandos cujo `target` é ele. Por isso
//! cada soltura é uma operação do documento, validada aqui, e a interface só pergunta "aceita?" e
//! mostra o motivo quando não aceita:
//!
//! - **pasta de geometria**: elementos de referência, sketches livres e subpastas
//!   ([`Document::move_to_folder`]);
//! - **pasta de comandos**: um comando do mesmo corpo entra na pasta e fica encostado nos membros,
//!   do lado em que já estava, para a linha do tempo não mudar sem necessidade; se a dependência
//!   impedir, tenta o outro lado; senão recusa com o nome de quem impede;
//! - **nó do corpo**: o comando passa a construir esse corpo, se não usar o resultado de outro corpo
//!   nem for usado por um comando que fica no corpo de origem;
//! - **parâmetro sobre parâmetro**: reordena a lista (a avaliação não depende da ordem).

use crate::*;

/// Por que uma soltura foi recusada. A interface traduz e mostra; nada muda no documento.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecusaSoltura {
    /// O alvo só aceita comandos da linha do tempo.
    NaoEComando,
    /// Comando da linha do tempo não vai para pasta de geometria.
    SoGeometriaLivre,
    /// A pasta iria para dentro dela mesma.
    PastaDentroDelaMesma,
    /// A pasta de comandos é de outro corpo (o nome).
    PastaDeOutroCorpo(String),
    /// O comando já está neste corpo (o nome).
    JaNoCorpo(String),
    /// Geometria de referência não entra na linha do tempo de um corpo.
    Referencia,
    /// O comando usa o resultado de outro corpo (o nome do comando usado).
    UsaOutroCorpo(String),
    /// Um comando que fica no corpo de origem usa este (o nome).
    UsadoNoCorpo(String),
    /// A ordem violaria a dependência deste comando (o nome).
    Ordem(String),
    /// O comando modifica um sólido e o corpo de destino não tem sólido antes dele (o nome do corpo).
    SemSolidoNoCorpo(String),
    /// O alvo não guarda este tipo de item.
    NaoAceita,
    /// Item ou alvo não existe mais.
    NaoEncontrado,
}

impl std::fmt::Display for RecusaSoltura {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NaoEComando => write!(f, "só comandos da linha do tempo entram aqui"),
            Self::SoGeometriaLivre => write!(f, "comando da linha do tempo não vai para pasta de geometria"),
            Self::PastaDentroDelaMesma => write!(f, "a pasta não pode ir para dentro dela mesma"),
            Self::PastaDeOutroCorpo(c) => write!(f, "a pasta é do corpo {c}"),
            Self::JaNoCorpo(c) => write!(f, "já está no corpo {c}"),
            Self::Referencia => write!(f, "geometria de referência não entra num corpo; fica numa pasta de geometria"),
            Self::UsaOutroCorpo(n) => write!(f, "usa o resultado de {n}, de outro corpo"),
            Self::UsadoNoCorpo(n) => write!(f, "{n} usa este comando e fica no corpo atual"),
            Self::Ordem(n) => write!(f, "a ordem violaria a dependência de {n}"),
            Self::SemSolidoNoCorpo(c) => write!(f, "o corpo {c} não tem sólido antes deste comando para ele modificar"),
            Self::NaoAceita => write!(f, "este grupo não guarda este item"),
            Self::NaoEncontrado => write!(f, "objeto não encontrado"),
        }
    }
}

impl Document {
    /// Soltar `item` sobre a pasta `pasta`, de geometria ou de comandos.
    pub fn soltar_em_pasta(&mut self, item: Id, pasta: Id) -> Result<(), RecusaSoltura> {
        let Some(fo) = self.folder(pasta) else { return Err(RecusaSoltura::NaoEncontrado) };
        if item == pasta {
            return Err(RecusaSoltura::PastaDentroDelaMesma);
        }
        match fo.kind {
            FolderKind::Comandos => self.soltar_em_pasta_de_comandos(item, pasta),
            FolderKind::Geometria => {
                if self.folder(item).is_some() && self.folder_within(pasta, item) {
                    return Err(RecusaSoltura::PastaDentroDelaMesma);
                }
                if self.params.iter().any(|p| p.id == item) {
                    return Err(RecusaSoltura::NaoAceita);
                }
                self.move_to_folder(item, pasta).map_err(|_| RecusaSoltura::SoGeometriaLivre)
            }
        }
    }

    /// Corpo dos comandos de uma pasta de comandos (`None` quando vazia).
    pub fn corpo_da_pasta(&self, pasta: Id) -> Option<Id> {
        self.features.iter().find(|f| f.folder == Some(pasta)).map(|f| f.target)
    }

    /// Comando para a pasta de comandos `pasta`, encostado nos membros (ver o módulo).
    fn soltar_em_pasta_de_comandos(&mut self, item: Id, pasta: Id) -> Result<(), RecusaSoltura> {
        let Some(f) = self.feature(item) else { return Err(RecusaSoltura::NaoEComando) };
        let alvo = f.target;
        if let Some(corpo) = self.corpo_da_pasta(pasta) {
            if corpo != alvo {
                return Err(RecusaSoltura::PastaDeOutroCorpo(self.name_of(corpo)));
            }
        }
        let antes = (self.features.clone(), self.rollback);
        // posições dos outros membros, já na ordem da linha do tempo
        let membros: Vec<usize> = self.features.iter().enumerate().filter(|(_, g)| g.folder == Some(pasta) && g.id != item).map(|(i, _)| i).collect();
        if let (Some(&prim), Some(&ult)) = (membros.first(), membros.last()) {
            let de = self.feature_index(item).ok_or(RecusaSoltura::NaoEncontrado)?;
            // destinos pós-remoção: logo antes do primeiro membro e logo depois do último
            let (antes_do_primeiro, depois_do_ultimo) = if de < prim { (prim - 1, ult) } else if de > ult { (prim, ult + 1) } else { (de, de) };
            let tentativas = if de > ult { [depois_do_ultimo, antes_do_primeiro] } else { [antes_do_primeiro, depois_do_ultimo] };
            let mut erro = None;
            let mut ok = false;
            for para in tentativas {
                if para == de {
                    ok = true;
                    break;
                }
                match self.move_feature(item, para) {
                    Ok(()) => {
                        ok = true;
                        break;
                    }
                    Err(DocError::Order(n)) => erro = erro.or(Some(n)),
                    Err(_) => {}
                }
            }
            if !ok {
                return Err(RecusaSoltura::Ordem(erro.unwrap_or_default()));
            }
        }
        match self.mover_comando_para_conteiner(item, pasta) {
            Ok(()) => Ok(()),
            Err(e) => {
                (self.features, self.rollback) = antes;
                Err(match e {
                    DocError::Order(n) => RecusaSoltura::Ordem(n),
                    _ => RecusaSoltura::NaoEComando,
                })
            }
        }
    }

    /// Soltar `item` sobre o nó do corpo `corpo`: o comando passa a construir esse corpo.
    pub fn soltar_no_corpo(&mut self, item: Id, corpo: Id) -> Result<(), RecusaSoltura> {
        let Some(c) = self.collectors.iter().find(|c| c.id == corpo) else { return Err(RecusaSoltura::NaoEncontrado) };
        let nome_corpo = c.name.clone();
        if self.reference(item).is_some() {
            return Err(RecusaSoltura::Referencia);
        }
        let Some(f) = self.feature(item) else { return Err(RecusaSoltura::NaoEComando) };
        let livre = self.is_free(f);
        if f.target == corpo && !livre {
            return Err(RecusaSoltura::JaNoCorpo(nome_corpo));
        }
        let sketch = matches!(f.kind, FeatureKind::Sketch(_));
        let origem = f.target;
        // o sketch é entrada: serve a qualquer corpo. Um comando sólido não pode levar consigo o
        // resultado de outro corpo, nem deixar para trás quem usa o resultado dele.
        if !sketch {
            let usa = Self::refs_of(&f.kind);
            if let Some(g) = self.features.iter().find(|g| usa.contains(&g.id) && !matches!(g.kind, FeatureKind::Sketch(_)) && g.target != corpo) {
                return Err(RecusaSoltura::UsaOutroCorpo(g.name.clone()));
            }
            if let Some(g) = self.features.iter().find(|g| g.id != item && g.target == origem && !matches!(g.kind, FeatureKind::Sketch(_)) && Self::refs_of(&g.kind).contains(&item)) {
                return Err(RecusaSoltura::UsadoNoCorpo(g.name.clone()));
            }
            // o corpo se constrói em sequência: o primeiro sólido é a base dos que vêm depois, mesmo
            // sem referência explícita; e um comando que só modifica precisa de sólido antes dele
            let idx = self.feature_index(item).ok_or(RecusaSoltura::NaoEncontrado)?;
            let solido = |g: &Feature| !matches!(g.kind, FeatureKind::Sketch(_));
            if !self.features[..idx].iter().any(|g| g.target == origem && solido(g)) {
                if let Some(g) = self.features[idx + 1..].iter().find(|g| g.target == origem && solido(g)) {
                    return Err(RecusaSoltura::UsadoNoCorpo(g.name.clone()));
                }
            }
            if !Self::cria_solido(&f.kind) && !self.features[..idx].iter().any(|g| g.target == corpo && solido(g)) {
                return Err(RecusaSoltura::SemSolidoNoCorpo(nome_corpo));
            }
        }
        // sai da pasta de geometria (entra na linha do tempo) ou de uma pasta de comandos de outro corpo
        let pasta = f.folder;
        let sai_da_pasta = pasta.is_some_and(|p| match self.folder(p).map(|fo| fo.kind) {
            Some(FolderKind::Geometria) => true,
            Some(FolderKind::Comandos) => self.features.iter().any(|g| g.folder == Some(p) && g.id != item && g.target != corpo),
            None => true,
        });
        if let Some(f) = self.feature_mut(item) {
            f.target = corpo;
            if sai_da_pasta {
                f.folder = None;
            }
        }
        Ok(())
    }

    /// O comando cria sólido sozinho (pode abrir um corpo vazio)? Os demais modificam o que já existe.
    fn cria_solido(k: &FeatureKind) -> bool {
        match k {
            FeatureKind::Extrude(e) => e.op == BoolOp::Add,
            FeatureKind::Revolve(r) => r.op == BoolOp::Add,
            FeatureKind::Primitive(_) | FeatureKind::Sweep(_) | FeatureKind::Loft(_) | FeatureKind::ThickSurface(_) | FeatureKind::Stiffener(_) => true,
            _ => false,
        }
    }

    /// Parâmetro `item` solto sobre o parâmetro `sobre`: vai para o lugar dele (quem desce fica
    /// depois, quem sobe fica antes). A avaliação resolve por nome, então a ordem é só organização.
    pub fn mover_parametro(&mut self, item: Id, sobre: Id) -> Result<(), RecusaSoltura> {
        let de = self.params.iter().position(|p| p.id == item).ok_or(RecusaSoltura::NaoAceita)?;
        let para = self.params.iter().position(|p| p.id == sobre).ok_or(RecusaSoltura::NaoAceita)?;
        if de != para {
            let p = self.params.remove(de);
            self.params.insert(para, p);
        }
        Ok(())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn sketch(d: &mut Document) -> Id {
        d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: SketchData::default() }))
    }

    fn extrusao(d: &mut Document, sk: Id, op: BoolOp) -> Id {
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op, extent: Extent::Blind("5".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }))
    }

    fn ordem(d: &Document) -> Vec<Id> {
        d.features.iter().map(|f| f.id).collect()
    }

    #[test]
    fn pasta_de_comandos_vazia_aceita_e_cheia_encosta_no_lado_de_onde_veio() {
        let mut d = Document::new("p");
        let a = sketch(&mut d);
        let ea = extrusao(&mut d, a, BoolOp::Add);
        let b = sketch(&mut d);
        let eb = extrusao(&mut d, b, BoolOp::Add);
        let vazia = d.add_folder("Vazia");
        d.soltar_em_pasta(eb, vazia).unwrap();
        assert_eq!(d.feature(eb).unwrap().folder, Some(vazia));
        assert_eq!(ordem(&d), vec![a, ea, b, eb], "pasta vazia não mexe na linha do tempo");
        // ea, que vem antes, entra encostado no primeiro membro: b passa para antes da pasta
        d.soltar_em_pasta(ea, vazia).unwrap();
        assert_eq!(ordem(&d), vec![a, b, ea, eb]);
        assert_eq!(d.corpo_da_pasta(vazia), Some(d.feature(ea).unwrap().target));
        // o sketch a, usado por ea, encosta antes do primeiro membro: continua antes de ea
        d.soltar_em_pasta(a, vazia).unwrap();
        assert_eq!(ordem(&d), vec![b, a, ea, eb]);
        // a pasta só com eb e uma extrusão depois que usa um sketch entre os dois: recusa com o nome
        let mut d = Document::new("q");
        let a = sketch(&mut d);
        let ea = extrusao(&mut d, a, BoolOp::Add);
        let b = sketch(&mut d);
        let eb = extrusao(&mut d, b, BoolOp::Add);
        let p = d.add_folder("P");
        d.soltar_em_pasta(ea, p).unwrap();
        let r = d.soltar_em_pasta(eb, p);
        assert_eq!(r, Err(RecusaSoltura::Ordem(d.name_of(b))), "eb não pode vir antes do seu sketch");
        assert_eq!(ordem(&d), vec![a, ea, b, eb], "recusa não mexe em nada");
        assert_eq!(d.feature(eb).unwrap().folder, None);
    }

    #[test]
    fn pasta_de_comandos_recusa_outro_corpo_referencia_e_parametro() {
        let mut d = Document::new("p");
        let a = sketch(&mut d);
        let ea = extrusao(&mut d, a, BoolOp::Add);
        let p = d.add_folder("P");
        d.soltar_em_pasta(ea, p).unwrap();
        let outro = d.add_collector("Outro");
        let b = sketch(&mut d);
        let eb = extrusao(&mut d, b, BoolOp::Add);
        d.feature_mut(eb).unwrap().target = outro;
        assert!(matches!(d.soltar_em_pasta(eb, p), Err(RecusaSoltura::PastaDeOutroCorpo(_))));
        let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "3");
        assert_eq!(d.soltar_em_pasta(pl, p), Err(RecusaSoltura::NaoEComando));
        let par = d.add_param("x", "1", Unit::Comprimento).unwrap();
        assert_eq!(d.soltar_em_pasta(par, p), Err(RecusaSoltura::NaoEComando));
        // move_to_folder aceita a pasta de comandos agora, com as mesmas regras
        let q = d.add_folder("Q");
        d.move_to_folder(ea, q).unwrap();
        assert_eq!(d.feature(ea).unwrap().folder, Some(q));
    }

    #[test]
    fn corpo_recusa_quem_ja_esta_nele_referencia_e_dependencia_cruzada() {
        let mut d = Document::new("p");
        let corpo1 = d.collectors[0].id;
        let a = sketch(&mut d);
        let ea = extrusao(&mut d, a, BoolOp::Add);
        let corpo2 = d.add_collector("Corpo2");
        assert!(matches!(d.soltar_no_corpo(ea, corpo1), Err(RecusaSoltura::JaNoCorpo(_))));
        let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "3");
        assert_eq!(d.soltar_no_corpo(pl, corpo2), Err(RecusaSoltura::Referencia));
        // o sketch é entrada: muda de corpo sem levar a extrusão
        d.soltar_no_corpo(a, corpo2).unwrap();
        assert_eq!(d.feature(a).unwrap().target, corpo2);
        assert_eq!(d.feature(ea).unwrap().target, corpo1);
        d.soltar_no_corpo(ea, corpo2).unwrap();
        assert_eq!(d.feature(ea).unwrap().target, corpo2);
        assert_eq!(ordem(&d), vec![a, ea], "mudar de corpo não reordena");
        // um corte depois: a base não sai deixando o corte sem sólido, e o corte não vai para um
        // corpo vazio
        let corte = extrusao(&mut d, a, BoolOp::Cut);
        d.feature_mut(corte).unwrap().target = corpo2;
        assert!(matches!(d.soltar_no_corpo(ea, corpo1), Err(RecusaSoltura::UsadoNoCorpo(_))));
        assert!(matches!(d.soltar_no_corpo(corte, corpo1), Err(RecusaSoltura::SemSolidoNoCorpo(_))));
    }

    #[test]
    fn parametro_reordena_pelos_dois_lados() {
        let mut d = Document::new("p");
        let a = d.add_param("a", "1", Unit::Comprimento).unwrap();
        let b = d.add_param("b", "a*2", Unit::Comprimento).unwrap();
        let c = d.add_param("c", "3", Unit::Comprimento).unwrap();
        d.mover_parametro(a, c).unwrap();
        assert_eq!(d.params.iter().map(|p| p.id).collect::<Vec<_>>(), vec![b, c, a]);
        d.mover_parametro(a, b).unwrap();
        assert_eq!(d.params.iter().map(|p| p.id).collect::<Vec<_>>(), vec![a, b, c]);
        assert_eq!(d.eval_params().unwrap()["b"], 2.);
        let sk = sketch(&mut d);
        assert_eq!(d.mover_parametro(sk, a), Err(RecusaSoltura::NaoAceita));
    }
}
