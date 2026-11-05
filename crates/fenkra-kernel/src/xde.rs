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
}

/// O que o reparo achou e fez numa forma. "Abertas" são cascas que não formam sólido.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InfoReparo {
    pub faces_antes: u32,
    pub faces_ruins_antes: u32,
    pub solidos_antes: u32,
    pub abertas_antes: u32,
    pub soltas_antes: u32,
    pub faces: u32,
    pub faces_ruins: u32,
    pub solidos: u32,
    pub abertas: u32,
    pub soltas: u32,
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
            let doc = b::xde_read(caminho, &opcoes).map_err(err)?;
            let info = b::xde_info(&doc);
            let mut produtos = Vec::new();
            for (i, p) in b::xde_products(&doc).map_err(err)?.into_iter().enumerate() {
                let forma = if p.is_assembly { None } else { Some(Shape::da_ponte(b::xde_shape(&doc, i as u32).map_err(err)?)) };
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
            })
        }

        /// Grava o modelo num STEP. Todos os produtos entram antes dos componentes, que os referenciam.
        pub fn gravar(&self, caminho: &str, o: OpcoesGravarXde) -> KResult<()> {
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
            let n: Vec<u32> = s.iter().map(|v| *v as u32).collect();
            let [faces_antes, faces_ruins_antes, solidos_antes, abertas_antes, soltas_antes, faces, faces_ruins, solidos, abertas, soltas, faces_tocadas, faces_costuradas] = n[..] else {
                return Err(KernelError("reparo: resposta incompleta do kernel".into()));
            };
            let info = InfoReparo { faces_antes, faces_ruins_antes, solidos_antes, abertas_antes, soltas_antes, faces, faces_ruins, solidos, abertas, soltas, faces_tocadas, faces_costuradas };
            Ok((Shape::da_ponte(r), info))
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
