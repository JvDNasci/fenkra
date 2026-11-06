// SPDX-License-Identifier: LGPL-2.1-or-later
//! Ponte XDE (B87): modelo de produtos do STEP, com nomes, cores, camadas e estrutura de montagem.
//!
//! Um produto é a definição (peça com forma ou montagem com componentes); uma instância é uma
//! ocorrência dele numa montagem. O modelo é um grafo: a mesma peça usada quatro vezes é um produto
//! e quatro instâncias.

use crate::{KResult, Shape};

/// Matriz 3x4 por linhas de uma instância sem deslocamento.
pub const IDENTIDADE: [f64; 12] = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.];

/// Protocolo de aplicação gravado no cabeçalho do STEP.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EsquemaStep {
    Ap203,
    #[default]
    Ap214,
    Ap242,
}

/// Unidade de comprimento gravada no STEP. A geometria segue em milímetros no Fenkra.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UnidadeStep {
    #[default]
    Milimetro,
    Polegada,
    Metro,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpcoesLerXde {
    pub cores: bool,
    pub nomes: bool,
    pub camadas: bool,
    /// lê o PMI semântico do arquivo; por ora o modelo só conta o que veio
    pub pmi: bool,
}

impl Default for OpcoesLerXde {
    fn default() -> Self {
        OpcoesLerXde { cores: true, nomes: true, camadas: true, pmi: true }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpcoesGravarXde {
    pub esquema: EsquemaStep,
    pub unidade: UnidadeStep,
    pub cores: bool,
    pub nomes: bool,
    pub camadas: bool,
    /// liga a escrita de PMI do gravador; o preenchimento semântico vem no item de AP242
    pub pmi: bool,
}

impl Default for OpcoesGravarXde {
    fn default() -> Self {
        OpcoesGravarXde { esquema: EsquemaStep::default(), unidade: UnidadeStep::default(), cores: true, nomes: true, camadas: true, pmi: false }
    }
}

/// Como gravar um IGES.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpcoesGravarIges {
    /// topologia completa (entidades 186 e 514); desligado, faces aparadas soltas (144), que todo
    /// programa antigo lê, mas chegam como superfícies
    pub brep: bool,
    pub unidade: UnidadeStep,
    pub cores: bool,
    pub nomes: bool,
    pub camadas: bool,
}

impl Default for OpcoesGravarIges {
    fn default() -> Self {
        OpcoesGravarIges { brep: true, unidade: UnidadeStep::default(), cores: true, nomes: true, camadas: true }
    }
}

/// Ocorrência de um produto numa montagem.
#[derive(Clone, Debug, PartialEq)]
pub struct InstanciaXde {
    /// índice em `ModeloXde::produtos`
    pub produto: usize,
    pub nome: String,
    /// matriz 3x4 por linhas, em milímetros
    pub local: [f64; 12],
    pub cor: Option<[f32; 4]>,
}

/// Definição de um produto: `forma` nas peças, `filhos` nas montagens.
#[derive(Clone, Default)]
pub struct ProdutoXde {
    pub nome: String,
    pub forma: Option<Shape>,
    pub cor: Option<[f32; 4]>,
    pub camadas: Vec<String>,
    /// faces com cor própria, pelo índice na ordem do kernel
    pub cores_faces: Vec<(u32, [f32; 4])>,
    pub filhos: Vec<InstanciaXde>,
}

/// Árvore de produtos de um STEP.
#[derive(Clone, Default)]
pub struct ModeloXde {
    pub produtos: Vec<ProdutoXde>,
    /// produtos de topo
    pub raizes: Vec<usize>,
    pub esquema: String,
    pub unidades: String,
    /// PMI semântico no arquivo: dimensões, tolerâncias geométricas e datums
    pub pmi: [u32; 3],
    /// as anotações semânticas, lidas do arquivo ou a gravar (B191); datums antes das tolerâncias
    pub anotacoes: Vec<crate::pmi_xde::AnotacaoXde>,
    /// tipo de cada entidade do arquivo que a leitura não traduziu (só IGES)
    pub ignoradas: Vec<String>,
}

/// O que o reparo achou e fez numa forma. "Abertas" são cascas que não formam sólido.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InfoReparo {
    pub faces_antes: u32,
    pub faces_ruins_antes: u32,
    pub solidos_antes: u32,
    pub abertas_antes: u32,
    pub soltas_antes: u32,
    pub arestas_livres_antes: u32,
    /// maior tolerância da forma, em mm
    pub tolerancia_antes: f64,
    pub faces: u32,
    pub faces_ruins: u32,
    pub solidos: u32,
    pub abertas: u32,
    pub soltas: u32,
    pub arestas_livres: u32,
    pub tolerancia: f64,
    /// faces que o reparo alterou
    pub faces_tocadas: u32,
    /// faces soltas que passaram pela costura
    pub faces_costuradas: u32,
}

impl InfoReparo {
    pub fn tem_defeito(&self) -> bool {
        self.faces_ruins > 0 || self.abertas > 0 || self.soltas > 0
    }
}

/// De que tipo é um defeito que a tela pode realçar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoDefeito {
    /// o `BRepCheck` reprovou a face
    FaceInvalida,
    /// só uma face usa a aresta: é borda de uma casca aberta
    ArestaLivre,
    /// face fora de qualquer casca
    FaceSolta,
}

/// Um defeito e o traço dele, em segmentos no espaço da forma.
#[derive(Clone, Debug, PartialEq)]
pub struct DefeitoForma {
    pub tipo: TipoDefeito,
    /// índice da face na ordem do kernel, ou da aresta na lista de bordas
    pub indice: u32,
    pub segmentos: Vec<[[f64; 3]; 2]>,
}

#[cfg(occt)]
mod ponte {
    use super::*;
    use crate::ffi::bridge as b;
    use crate::KernelError;

    fn err(e: cxx::Exception) -> KernelError {
        KernelError(e.what().to_string())
    }

    fn rgba(v: &[f32]) -> Option<[f32; 4]> {
        v.try_into().ok()
    }

    fn vazio() -> b::XdeProduct {
        b::XdeProduct { name: String::new(), is_assembly: false, rgba: vec![], layers: vec![], face_ids: vec![], face_rgba: vec![], comps: vec![] }
    }

    impl ModeloXde {
        /// Lê um STEP. A geometria sai em milímetros, qualquer que seja a unidade do arquivo.
        pub fn ler(caminho: &str, o: OpcoesLerXde) -> KResult<ModeloXde> {
            let opcoes = b::XdeReadOpts { colors: o.cores, names: o.nomes, layers: o.camadas, gdt: o.pmi };
            Self::do_documento(&b::xde_read(caminho, &opcoes).map_err(err)?, o.pmi)
        }

        /// Lê um IGES, em milímetros como o STEP. Cada raiz do arquivo vira um produto solto.
        pub fn ler_iges(caminho: &str, o: OpcoesLerXde) -> KResult<ModeloXde> {
            let opcoes = b::XdeReadOpts { colors: o.cores, names: o.nomes, layers: o.camadas, gdt: false };
            Self::do_documento(&b::xde_read_iges(caminho, &opcoes).map_err(err)?, false)
        }

        fn do_documento(doc: &cxx::UniquePtr<b::XdeDoc>, o_pmi: bool) -> KResult<ModeloXde> {
            let info = b::xde_info(doc);
            let mut produtos = Vec::new();
            for (i, p) in b::xde_products(doc).map_err(err)?.into_iter().enumerate() {
                let forma = if p.is_assembly { None } else { Some(Shape::da_ponte(b::xde_shape(doc, i as u32).map_err(err)?)) };
                let filhos = p.comps.iter().map(|c| InstanciaXde {
                    produto: c.product as usize,
                    nome: c.name.clone(),
                    local: c.m.as_slice().try_into().unwrap_or(IDENTIDADE),
                    cor: rgba(&c.rgba),
                });
                produtos.push(ProdutoXde {
                    nome: p.name.clone(),
                    filhos: filhos.collect(),
                    forma,
                    cor: rgba(&p.rgba),
                    camadas: p.layers,
                    cores_faces: p.face_ids.iter().zip(p.face_rgba.chunks_exact(4)).filter_map(|(f, c)| Some((*f, rgba(c)?))).collect(),
                });
            }
            Ok(ModeloXde {
                produtos,
                raizes: info.roots.iter().map(|r| *r as usize).collect(),
                esquema: info.schema,
                unidades: info.units,
                pmi: [info.dims, info.tols, info.datums],
                anotacoes: if o_pmi { Self::ler_anotacoes(doc)? } else { vec![] },
                ignoradas: info.skipped,
            })
        }

        /// Monta o documento XCAF. Todos os produtos entram antes dos componentes, que os referenciam.
        fn para_documento(&self) -> KResult<cxx::UniquePtr<b::XdeDoc>> {
            let mut doc = b::xde_new().map_err(err)?;
            let sem_forma = Shape::empty();
            let mut rotulos = Vec::with_capacity(self.produtos.len());
            for p in &self.produtos {
                let ponte = b::XdeProduct {
                    name: p.nome.clone(),
                    is_assembly: p.forma.is_none(),
                    rgba: p.cor.map(|c| c.to_vec()).unwrap_or_default(),
                    layers: p.camadas.clone(),
                    face_ids: p.cores_faces.iter().map(|(f, _)| *f).collect(),
                    face_rgba: p.cores_faces.iter().flat_map(|(_, c)| *c).collect(),
                    ..vazio()
                };
                let forma = p.forma.as_ref().unwrap_or(&sem_forma);
                rotulos.push(b::xde_add(doc.pin_mut(), &ponte, forma.ponte()).map_err(err)?);
            }
            for (p, rotulo) in self.produtos.iter().zip(&rotulos) {
                for f in &p.filhos {
                    let Some(alvo) = rotulos.get(f.produto) else {
                        return Err(KernelError(format!("instância de produto inexistente: {}", f.produto)));
                    };
                    let inst = b::XdeInst { product: *alvo, name: f.nome.clone(), m: f.local.to_vec(), rgba: f.cor.map(|c| c.to_vec()).unwrap_or_default() };
                    b::xde_add_component(doc.pin_mut(), *rotulo, &inst).map_err(err)?;
                }
            }
            self.gravar_anotacoes(&mut doc)?;
            Ok(doc)
        }

        /// Entrega as anotações ao documento XCAF: datums primeiro, para as tolerâncias os referenciarem.
        fn gravar_anotacoes(&self, doc: &mut cxx::UniquePtr<b::XdeDoc>) -> KResult<()> {
            use crate::pmi_xde::DadoXde;
            let em_branco = |a: &crate::pmi_xde::AnotacaoXde, kind: u8| b::XdePmi {
                kind,
                product: a.produto as u32,
                name: a.nome.clone(),
                ty: 0,
                value: 0.,
                lower: 0.,
                upper: 0.,
                has_tol: false,
                faces: a.faces.clone(),
                faces2: vec![],
                zone: 0,
                mat: 0,
                zone_mod: 0,
                zone_val: 0.,
                mods: vec![],
                datums: vec![],
                id: 0,
                position: 0,
                target: false,
                target_type: 0,
                target_number: 0,
                target_length: 0.,
                target_width: 0.,
            };
            // id do datum = ordem de entrada entre os datums
            let mut ids = std::collections::HashMap::new();
            for (i, a) in self.anotacoes.iter().enumerate() {
                if let DadoXde::Datum { letra, posicao, alvo } = &a.dado {
                    let mut m = em_branco(a, 2);
                    m.name = letra.clone();
                    m.id = ids.len() as u32;
                    m.position = *posicao;
                    if let Some(al) = alvo {
                        m.target = true;
                        m.target_type = al.tipo as u32;
                        m.target_number = al.numero;
                        m.target_length = al.comprimento;
                        m.target_width = al.largura;
                    }
                    ids.insert(i, m.id);
                    b::xde_pmi_add(doc.pin_mut(), &m).map_err(err)?;
                }
            }
            for a in &self.anotacoes {
                match &a.dado {
                    DadoXde::Dimensao { tipo, valor, tolerancia, faces2 } => {
                        if !tipo.atravessa() {
                            return Err(KernelError(format!("o tipo de dimensão {tipo:?} não é gravado pelo OpenCASCADE")));
                        }
                        let mut m = em_branco(a, 0);
                        m.ty = *tipo as u32;
                        // em graus na interface; o gravador espera radianos (o leitor devolve graus)
                        m.value = if tipo.e_angulo() { valor.to_radians() } else { *valor };
                        m.faces2 = faces2.clone();
                        if let Some((inf, sup)) = tolerancia {
                            m.has_tol = true;
                            m.lower = *inf;
                            m.upper = *sup;
                        }
                        b::xde_pmi_add(doc.pin_mut(), &m).map_err(err)?;
                    }
                    DadoXde::Tolerancia { tipo, valor, zona, material, mod_zona, valor_zona, modificadores, datums } => {
                        let mut m = em_branco(a, 1);
                        m.ty = *tipo as u32;
                        m.value = *valor;
                        m.zone = *zona as u32;
                        m.mat = *material as u32;
                        m.zone_mod = *mod_zona as u32;
                        m.zone_val = *valor_zona;
                        m.mods = modificadores.iter().map(|k| *k as u32).collect();
                        for d in datums {
                            m.datums.push(*ids.get(d).ok_or_else(|| KernelError(format!("a anotação {d} não é um datum")))?);
                        }
                        b::xde_pmi_add(doc.pin_mut(), &m).map_err(err)?;
                    }
                    DadoXde::Datum { .. } => {}
                }
            }
            Ok(())
        }

        /// Lê as anotações do documento: datums primeiro, como na gravação.
        fn ler_anotacoes(doc: &cxx::UniquePtr<b::XdeDoc>) -> KResult<Vec<crate::pmi_xde::AnotacaoXde>> {
            use crate::pmi_xde::*;
            let lidas = b::xde_pmi_read(doc).map_err(err)?;
            let mut datum_de = std::collections::HashMap::new();
            let mut out = Vec::new();
            // O leitor do OCCT cria um datum novo a cada tolerância que o referencia (e um por alvo): aqui
            // eles voltam a ser um só, pela letra, as faces e o alvo.
            let mut unicos: Vec<(String, usize, Vec<u32>, Option<(u32, u32)>)> = Vec::new();
            for m in lidas.iter().filter(|m| m.kind == 2) {
                let chave = (m.name.clone(), m.product as usize, m.faces.clone(), m.target.then_some((m.target_type, m.target_number)));
                let ja = unicos.iter().position(|c| *c == chave);
                datum_de.insert(m.id, (ja.unwrap_or(out.len()), m.target));
                if ja.is_some() {
                    continue;
                }
                unicos.push(chave);
                let alvo = m.target.then(|| AlvoXde { tipo: AlvoDatumXde::de_numero(m.target_type).unwrap_or(AlvoDatumXde::Ponto), numero: m.target_number, comprimento: m.target_length, largura: m.target_width });
                out.push(AnotacaoXde { produto: m.product as usize, nome: m.name.clone(), faces: m.faces.clone(), dado: DadoXde::Datum { letra: m.name.clone(), posicao: m.position, alvo } });
            }
            for m in lidas.iter().filter(|m| m.kind != 2) {
                let dado = if m.kind == 0 {
                    let Some(tipo) = TipoDimensaoXde::de_numero(m.ty) else { continue };
                    DadoXde::Dimensao { tipo, valor: m.value, tolerancia: m.has_tol.then_some((m.lower, m.upper)), faces2: m.faces2.clone() }
                } else {
                    let Some(tipo) = TipoToleranciaXde::de_numero(m.ty) else { continue };
                    DadoXde::Tolerancia {
                        tipo,
                        valor: m.value,
                        zona: match m.zone {
                            1 => ZonaXde::Diametro,
                            2 => ZonaXde::Esfera,
                            _ => ZonaXde::Largura,
                        },
                        material: match m.mat {
                            1 => MaterialXde::Maximo,
                            2 => MaterialXde::Minimo,
                            _ => MaterialXde::Nenhum,
                        },
                        mod_zona: match m.zone_mod {
                            1 => ModZonaXde::Projetada,
                            2 => ModZonaXde::Batimento,
                            3 => ModZonaXde::NaoUniforme,
                            _ => ModZonaXde::Nenhum,
                        },
                        valor_zona: m.zone_val,
                        modificadores: m.mods.iter().filter_map(|k| ModToleranciaXde::de_numero(*k)).collect(),
                        datums: {
                            // alvos pertencem ao datum da letra; a tolerância cita só o datum
                            let mut v: Vec<usize> = Vec::new();
                            for (i, alvo) in m.datums.iter().filter_map(|d| datum_de.get(d).copied()) {
                                if !alvo && !v.contains(&i) {
                                    v.push(i);
                                }
                            }
                            v
                        },
                    }
                };
                out.push(AnotacaoXde { produto: m.product as usize, nome: m.name.clone(), faces: m.faces.clone(), dado });
            }
            Ok(out)
        }

        /// Grava o modelo num STEP.
        pub fn gravar(&self, caminho: &str, o: OpcoesGravarXde) -> KResult<()> {
            let mut doc = self.para_documento()?;
            let opcoes = b::XdeWriteOpts {
                schema: o.esquema as u8,
                unit: o.unidade as u8,
                colors: o.cores,
                names: o.nomes,
                layers: o.camadas,
                gdt: o.pmi,
            };
            b::xde_write(doc.pin_mut(), caminho, &opcoes).map_err(err)
        }

        /// Grava o modelo num IGES.
        pub fn gravar_iges(&self, caminho: &str, o: OpcoesGravarIges) -> KResult<()> {
            let mut doc = self.para_documento()?;
            let opcoes = b::XdeIgesOpts { brep: o.brep, unit: o.unidade as u8, colors: o.cores, names: o.nomes, layers: o.camadas };
            b::xde_write_iges(doc.pin_mut(), caminho, &opcoes).map_err(err)
        }
    }

    impl Shape {
        /// A forma numa posição (matriz 3x4 por linhas, rígida), sem copiar a geometria.
        pub fn com_matriz(&self, m: &[f64; 12]) -> KResult<Shape> {
            b::xde_moved(self.ponte(), m).map(Shape::da_ponte).map_err(err)
        }

        /// Os sólidos que a forma contém, na ordem do kernel.
        pub fn solidos(&self) -> KResult<Vec<Shape>> {
            (0..b::xde_solid_count(self.ponte())).map(|i| b::xde_solid_at(self.ponte(), i).map(Shape::da_ponte).map_err(err)).collect()
        }

        /// Analisa e, com `nivel` 1 ou 2, repara. Nível 0 só diz o que há; forma sã volta intacta.
        pub fn reparar(&self, nivel: u8, tolerancia: f64) -> KResult<(Shape, InfoReparo)> {
            let mut s = Vec::new();
            let r = b::xde_heal(self.ponte(), nivel, tolerancia, &mut s).map_err(err)?;
            let [faces_antes, faces_ruins_antes, solidos_antes, abertas_antes, soltas_antes, arestas_livres_antes, tolerancia_antes, faces, faces_ruins, solidos, abertas, soltas, arestas_livres, tolerancia, faces_tocadas, faces_costuradas] = s[..] else {
                return Err(KernelError("reparo: resposta incompleta do kernel".into()));
            };
            let n = |v: f64| v as u32;
            let info = InfoReparo {
                faces_antes: n(faces_antes),
                faces_ruins_antes: n(faces_ruins_antes),
                solidos_antes: n(solidos_antes),
                abertas_antes: n(abertas_antes),
                soltas_antes: n(soltas_antes),
                arestas_livres_antes: n(arestas_livres_antes),
                tolerancia_antes,
                faces: n(faces),
                faces_ruins: n(faces_ruins),
                solidos: n(solidos),
                abertas: n(abertas),
                soltas: n(soltas),
                arestas_livres: n(arestas_livres),
                tolerancia,
                faces_tocadas: n(faces_tocadas),
                faces_costuradas: n(faces_costuradas),
            };
            Ok((Shape::da_ponte(r), info))
        }

        /// Defeitos que a tela pode realçar, até `limite`, com o traço de cada um.
        pub fn defeitos(&self, limite: u32) -> KResult<Vec<DefeitoForma>> {
            let lista = b::xde_defects(self.ponte(), limite).map_err(err)?;
            Ok(lista
                .into_iter()
                .map(|d| DefeitoForma {
                    tipo: match d.kind {
                        0 => TipoDefeito::FaceInvalida,
                        1 => TipoDefeito::ArestaLivre,
                        _ => TipoDefeito::FaceSolta,
                    },
                    indice: d.index,
                    segmentos: d.segs.chunks_exact(6).map(|q| [[q[0], q[1], q[2]], [q[3], q[4], q[5]]]).collect(),
                })
                .collect())
        }
    }
}

#[cfg(not(occt))]
mod ponte {
    use super::*;
    use crate::KernelError;

    fn no<T>() -> KResult<T> {
        Err(KernelError("kernel geométrico indisponível (OpenCASCADE não encontrado na compilação)".into()))
    }

    impl ModeloXde {
        pub fn ler(_: &str, _: OpcoesLerXde) -> KResult<ModeloXde> {
            no()
        }
        pub fn gravar(&self, _: &str, _: OpcoesGravarXde) -> KResult<()> {
            no()
        }
        pub fn ler_iges(_: &str, _: OpcoesLerXde) -> KResult<ModeloXde> {
            no()
        }
        pub fn gravar_iges(&self, _: &str, _: OpcoesGravarIges) -> KResult<()> {
            no()
        }
    }

    impl Shape {
        pub fn com_matriz(&self, _: &[f64; 12]) -> KResult<Shape> {
            no()
        }
        pub fn solidos(&self) -> KResult<Vec<Shape>> {
            no()
        }
        pub fn reparar(&self, _: u8, _: f64) -> KResult<(Shape, InfoReparo)> {
            no()
        }
        pub fn defeitos(&self, _: u32) -> KResult<Vec<DefeitoForma>> {
            no()
        }
    }
}

#[cfg(all(test, occt))]
mod testes {
    use super::*;
    use crate::{Frame, MassProps};

    fn pasta(teste: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("Fenkra xde açaí {} {teste}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn caixa(l: [f64; 3]) -> Shape {
        Shape::box_tool([0., 0., 0.], [1., 0., 0.], [0., 0., 1.], l).unwrap()
    }

    fn translacao(x: f64, y: f64, z: f64) -> [f64; 12] {
        [1., 0., 0., x, 0., 1., 0., y, 0., 0., 1., z]
    }

    fn peca(nome: &str, forma: Shape) -> ProdutoXde {
        ProdutoXde { nome: nome.into(), forma: Some(forma), ..Default::default() }
    }

    /// Montagem com subconjunto: a mesma peça aparece duas vezes, uma delas dentro do subconjunto.
    fn modelo() -> ModeloXde {
        let mut base = peca("Base de aço", caixa([100., 60., 10.]));
        base.cor = Some([0.8, 0.2, 0.1, 1.]);
        base.camadas = vec!["Estrutura".into()];
        let mut pino = peca("Pino", caixa([10., 10., 40.]));
        pino.cor = Some([0.1, 0.3, 0.9, 1.]);
        let sub = ProdutoXde {
            nome: "Subconjunto".into(),
            filhos: vec![InstanciaXde { produto: 1, nome: "Pino interno".into(), local: translacao(5., 5., 10.), cor: None }],
            ..Default::default()
        };
        let topo = ProdutoXde {
            nome: "Conjunto".into(),
            filhos: vec![
                InstanciaXde { produto: 0, nome: "Base".into(), local: IDENTIDADE, cor: None },
                InstanciaXde { produto: 1, nome: "Pino solto".into(), local: translacao(80., 40., 10.), cor: None },
                InstanciaXde { produto: 2, nome: "Sub".into(), local: translacao(0., 20., 0.), cor: None },
            ],
            ..Default::default()
        };
        ModeloXde { produtos: vec![base, pino, sub, topo], raizes: vec![3], ..Default::default() }
    }

    fn produto<'a>(m: &'a ModeloXde, nome: &str) -> &'a ProdutoXde {
        m.produtos.iter().find(|p| p.nome == nome).unwrap_or_else(|| panic!("sem produto {nome}: {:?}", m.produtos.iter().map(|p| &p.nome).collect::<Vec<_>>()))
    }

    fn perto(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.)
    }

    fn confere_massa(a: MassProps, b: MassProps) {
        assert!(perto(a.volume, b.volume), "volume {} x {}", a.volume, b.volume);
        assert!(perto(a.area, b.area), "área {} x {}", a.area, b.area);
        for i in 0..3 {
            assert!(perto(a.center[i], b.center[i]), "centroide {:?} x {:?}", a.center, b.center);
        }
    }

    #[test]
    fn ida_e_volta_preserva_estrutura_nomes_cores_camadas_e_massa() {
        let dir = pasta("ida");
        let alvo = dir.join("conjunto com espaço.step");
        let caminho = alvo.to_str().unwrap();
        for esquema in [EsquemaStep::Ap214, EsquemaStep::Ap242] {
            let original = modelo();
            original.gravar(caminho, OpcoesGravarXde { esquema, ..Default::default() }).unwrap();
            let lido = ModeloXde::ler(caminho, OpcoesLerXde::default()).unwrap();

            assert_eq!(lido.raizes.len(), 1);
            assert_eq!(lido.produtos.len(), 4, "a peça repetida continua sendo um produto");
            let topo = &lido.produtos[lido.raizes[0]];
            assert_eq!(topo.nome, "Conjunto");
            assert!(topo.forma.is_none());
            assert_eq!(topo.filhos.len(), 3);

            let base = produto(&lido, "Base de aço");
            assert_eq!(base.camadas, vec!["Estrutura".to_string()]);
            let cor = base.cor.expect("cor da base");
            for (a, b) in cor.iter().zip([0.8, 0.2, 0.1, 1.]) {
                assert!((a - b).abs() < 2e-3, "{cor:?}");
            }
            confere_massa(base.forma.as_ref().unwrap().mass(), original.produtos[0].forma.as_ref().unwrap().mass());

            let pino = lido.produtos.iter().position(|p| p.nome == "Pino").unwrap();
            let usos: Vec<_> = lido.produtos.iter().flat_map(|p| &p.filhos).filter(|f| f.produto == pino).collect();
            assert_eq!(usos.len(), 2, "duas ocorrências da mesma definição");
            assert!(usos.iter().any(|u| u.nome == "Pino solto"), "{:?}", usos.iter().map(|u| &u.nome).collect::<Vec<_>>());
            let solto = usos.iter().find(|u| u.nome == "Pino solto").unwrap();
            for (a, b) in solto.local.iter().zip(translacao(80., 40., 10.)) {
                assert!((a - b).abs() < 1e-9, "{:?}", solto.local);
            }
        }
    }

    #[test]
    fn cor_de_face_atravessa_o_step() {
        let dir = pasta("face");
        let alvo = dir.join("face.step");
        let caminho = alvo.to_str().unwrap();
        let mut p = peca("Cubo", caixa([10., 10., 10.]));
        p.cores_faces = vec![(2, [0.0, 1.0, 0.0, 1.0])];
        let m = ModeloXde { produtos: vec![p], raizes: vec![0], ..Default::default() };
        m.gravar(caminho, OpcoesGravarXde::default()).unwrap();
        let lido = ModeloXde::ler(caminho, OpcoesLerXde::default()).unwrap();
        let c = &lido.produtos[0].cores_faces;
        assert_eq!(c.len(), 1, "{c:?}");
        assert!((c[0].1[1] - 1.).abs() < 2e-3 && c[0].1[0] < 2e-3, "{c:?}");
    }

    #[test]
    fn esquema_vai_para_o_cabecalho_e_opcao_de_cor_e_respeitada() {
        let dir = pasta("esquema");
        for (esquema, marca) in [(EsquemaStep::Ap203, "CONFIG_CONTROL_DESIGN"), (EsquemaStep::Ap214, "AUTOMOTIVE_DESIGN"), (EsquemaStep::Ap242, "AP242")] {
            let alvo = dir.join(format!("{marca}.step"));
            let caminho = alvo.to_str().unwrap();
            modelo().gravar(caminho, OpcoesGravarXde { esquema, pmi: esquema == EsquemaStep::Ap242, ..Default::default() }).unwrap();
            let lido = ModeloXde::ler(caminho, OpcoesLerXde::default()).unwrap();
            assert!(lido.esquema.to_uppercase().contains(marca), "{} em {marca}", lido.esquema);
            assert_eq!(lido.pmi, [0, 0, 0]);
            let sem_cor = ModeloXde::ler(caminho, OpcoesLerXde { cores: false, ..Default::default() }).unwrap();
            assert!(produto(&sem_cor, "Base de aço").cor.is_none());
        }
    }

    #[test]
    fn step_em_polegadas_volta_em_milimetros() {
        let dir = pasta("pol");
        let alvo = dir.join("pol.step");
        let caminho = alvo.to_str().unwrap();
        let m = ModeloXde { produtos: vec![peca("Bloco", caixa([25.4, 50.8, 12.7]))], raizes: vec![0], ..Default::default() };
        m.gravar(caminho, OpcoesGravarXde { unidade: UnidadeStep::Polegada, ..Default::default() }).unwrap();
        let lido = ModeloXde::ler(caminho, OpcoesLerXde::default()).unwrap();
        assert!(lido.unidades.to_uppercase().contains("INCH"), "{}", lido.unidades);
        let (a, b) = lido.produtos[0].forma.as_ref().unwrap().bbox().unwrap();
        for (i, l) in [25.4, 50.8, 12.7].into_iter().enumerate() {
            assert!(perto(b[i] - a[i], l), "{:?} {:?}", a, b);
        }
    }

    #[test]
    fn arquivo_que_nao_e_step_devolve_erro() {
        let dir = pasta("lixo");
        let alvo = dir.join("lixo.step");
        std::fs::write(&alvo, "isto não é um STEP").unwrap();
        assert!(ModeloXde::ler(alvo.to_str().unwrap(), OpcoesLerXde::default()).is_err());
        assert!(ModeloXde::ler(dir.join("nao existe.step").to_str().unwrap(), OpcoesLerXde::default()).is_err());
    }

    /// As seis faces de um cubo, soltas: o caso do corpo que chega sem casca.
    fn faces_soltas(tirar: usize) -> Shape {
        let quad = |o: [f64; 3], x: [f64; 3], n: [f64; 3]| {
            let f = Frame { origin: o, x, normal: n };
            let cantos = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
            let segs: Vec<_> = (0..4).map(|i| crate::Seg2::Line(cantos[i], cantos[(i + 1) % 4])).collect();
            Shape::face(&f, &[segs]).unwrap()
        };
        let lados = [
            quad([0., 0., 0.], [0., 1., 0.], [0., 0., -1.]),
            quad([0., 0., 10.], [1., 0., 0.], [0., 0., 1.]),
            quad([0., 0., 0.], [1., 0., 0.], [0., -1., 0.]),
            quad([0., 10., 0.], [0., 0., 1.], [0., 1., 0.]),
            quad([0., 0., 0.], [0., 0., 1.], [-1., 0., 0.]),
            quad([10., 0., 0.], [0., 1., 0.], [1., 0., 0.]),
        ];
        let mut lados = lados.into_iter().skip(tirar);
        let primeiro = lados.next().unwrap();
        lados.fold(primeiro, |a, b| a.compound(&b))
    }

    #[test]
    fn reparo_costura_faces_soltas_em_solido() {
        let solta = faces_soltas(0);
        let (igual, antes) = solta.reparar(0, 1e-3).unwrap();
        assert_eq!((antes.solidos, antes.soltas, antes.faces), (0, 6, 6));
        assert!(!igual.is_solid(), "nível 0 só analisa");

        let (r, info) = solta.reparar(1, 1e-3).unwrap();
        assert_eq!((info.solidos, info.abertas, info.soltas), (1, 0, 0), "{info:?}");
        assert_eq!(info.faces_costuradas, 6);
        assert!(!info.tem_defeito());
        assert!(perto(r.mass().volume, 1000.), "{}", r.mass().volume);
    }

    #[test]
    fn solidos_de_uma_peca_multicorpo_e_posicao_por_matriz() {
        let duas = caixa([1., 1., 1.]).compound(&caixa([2., 2., 2.]).com_matriz(&translacao(10., 0., 0.)).unwrap());
        let s = duas.solidos().unwrap();
        assert_eq!(s.len(), 2);
        assert!(perto(s[1].mass().volume, 8.));
        let (a, _) = s[1].bbox().unwrap();
        assert!(perto(a[0], 10.), "{a:?}");
    }

    #[test]
    fn defeitos_apontam_face_solta_e_borda_de_casca_com_traco_no_lugar() {
        let soltas = faces_soltas(1).defeitos(100).unwrap();
        assert_eq!(soltas.len(), 5);
        assert!(soltas.iter().all(|d| d.tipo == TipoDefeito::FaceSolta && !d.segmentos.is_empty()));

        // as mesmas cinco faces costuradas: a casca abre numa borda de quatro arestas, em z = 0
        let (casca, info) = faces_soltas(1).reparar(1, 1e-3).unwrap();
        assert_eq!((info.soltas, info.arestas_livres, info.arestas_livres_antes), (0, 4, 0), "{info:?}");
        let bordas = casca.defeitos(100).unwrap();
        assert_eq!(bordas.len(), 4, "{bordas:?}");
        for d in &bordas {
            assert_eq!(d.tipo, TipoDefeito::ArestaLivre);
            assert!(d.segmentos.iter().flatten().all(|p| p[2].abs() < 1e-9), "{d:?}");
        }
        assert_eq!(casca.defeitos(3).unwrap().len(), 3, "o limite vale");
        assert!(caixa([1., 2., 3.]).defeitos(10).unwrap().is_empty(), "forma sã não tem defeito");
    }

    #[test]
    fn reparo_guarda_as_curvas_soltas_ao_costurar() {
        let curva = Shape::curve_shape(&[[0., 0., 50.], [10., 0., 50.]]).unwrap();
        let (r, info) = faces_soltas(0).compound(&curva).reparar(1, 1e-3).unwrap();
        assert_eq!(info.solidos, 1, "{info:?}");
        let (_, alto) = r.bbox().unwrap();
        assert!(alto[2] > 49., "a curva sumiu na costura: {alto:?}");
    }

    #[test]
    fn iges_ida_e_volta_nos_dois_modos_e_unidade() {
        let dir = pasta("iges ç");
        let forma = caixa([10., 20., 30.]);
        let m = ModeloXde { produtos: vec![peca("Bloco", forma)], raizes: vec![0], ..Default::default() };

        let brep = dir.join("brep.igs");
        m.gravar_iges(brep.to_str().unwrap(), OpcoesGravarIges::default()).unwrap();
        let lido = ModeloXde::ler_iges(brep.to_str().unwrap(), OpcoesLerXde::default()).unwrap();
        assert!(lido.esquema.starts_with("IGES"), "{}", lido.esquema);
        assert_eq!(lido.unidades.to_uppercase(), "MM");
        let s = lido.produtos[0].forma.as_ref().unwrap();
        assert!(s.is_solid() || s.solidos().unwrap().len() == 1, "o modo BRep leva o sólido");
        assert!(perto(s.mass().volume, 6000.), "{}", s.mass().volume);

        let faces = dir.join("faces.igs");
        m.gravar_iges(faces.to_str().unwrap(), OpcoesGravarIges { brep: false, ..Default::default() }).unwrap();
        let texto_faces = std::fs::read_to_string(&faces).unwrap();
        let texto_brep = std::fs::read_to_string(&brep).unwrap();
        let tem = |t: &str, tipo: &str| t.lines().any(|l| l.ends_with(|c: char| c.is_ascii_digit()) && l.len() >= 73 && &l[72..73] == "D" && l[..8].trim() == tipo);
        assert!(tem(&texto_brep, "186") && !tem(&texto_faces, "186"), "só o modo BRep grava sólido (186)");
        assert!(tem(&texto_faces, "144"), "o modo Faces grava superfície aparada (144)");
        let lido = ModeloXde::ler_iges(faces.to_str().unwrap(), OpcoesLerXde::default()).unwrap();
        let area: f64 = lido.produtos.iter().filter_map(|p| p.forma.as_ref()).map(|f| f.mass().area).sum();
        assert!(perto(area, 2. * (200. + 300. + 600.)), "{area}");
        assert!(lido.produtos.iter().filter_map(|p| p.forma.as_ref()).all(|f| !f.is_solid()), "o modo Faces chega como superfícies");

        let pol = dir.join("pol.igs");
        m.gravar_iges(pol.to_str().unwrap(), OpcoesGravarIges { unidade: UnidadeStep::Polegada, ..Default::default() }).unwrap();
        let lido = ModeloXde::ler_iges(pol.to_str().unwrap(), OpcoesLerXde::default()).unwrap();
        assert!(lido.unidades.to_uppercase().starts_with("IN"), "{}", lido.unidades);
        let (a, b) = lido.produtos[0].forma.as_ref().unwrap().bbox().unwrap();
        assert!((b[1] - a[1] - 20.).abs() < 1e-3, "polegada volta em milímetros: {a:?} {b:?}");
    }

    #[test]
    fn iges_em_paralelo_nao_corrompe() {
        let dir = pasta("iges paralelo");
        let fios: Vec<_> = (0..4)
            .map(|i| {
                let dir = dir.clone();
                std::thread::spawn(move || {
                    let l = 10. + i as f64;
                    let m = ModeloXde { produtos: vec![peca("Bloco", caixa([l, l, l]))], raizes: vec![0], ..Default::default() };
                    let alvo = dir.join(format!("{i}.igs"));
                    for rodada in 0..3 {
                        // cada gravação liga um modo diferente: o estado global não pode vazar entre fios
                        let brep = (i + rodada) % 2 == 0;
                        m.gravar_iges(alvo.to_str().unwrap(), OpcoesGravarIges { brep, ..Default::default() }).unwrap();
                        let lido = ModeloXde::ler_iges(alvo.to_str().unwrap(), OpcoesLerXde::default()).unwrap();
                        let area: f64 = lido.produtos.iter().filter_map(|p| p.forma.as_ref()).map(|f| f.mass().area).sum();
                        assert!(perto(area, 6. * l * l), "{area} em {l}");
                        let solidos: usize = lido.produtos.iter().filter_map(|p| p.forma.as_ref()).map(|f| f.solidos().unwrap().len()).sum();
                        assert_eq!(solidos == 1, brep, "modo trocado em outro fio");
                    }
                })
            })
            .collect();
        for f in fios {
            f.join().unwrap();
        }
    }

    #[test]
    fn reparo_conta_o_que_nao_fechou_e_nao_mexe_em_forma_sa() {
        let (_, info) = faces_soltas(1).reparar(1, 1e-3).unwrap();
        assert_eq!(info.solidos, 0);
        assert_eq!(info.abertas, 1, "{info:?}");
        assert!(info.tem_defeito());

        let boa = caixa([10., 20., 30.]);
        let (r, info) = boa.reparar(2, 1e-3).unwrap();
        assert!(!info.tem_defeito());
        assert_eq!((info.faces_tocadas, info.faces_costuradas), (0, 0));
        assert_eq!(r.face_count(), 6);
    }
}
