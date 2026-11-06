// SPDX-License-Identifier: LGPL-2.1-or-later
//! Gravação em R2000 (`AC1015`). Diferente do R12, esse esquema exige estrutura: cada objeto tem um
//! identificador, diz quem é o dono e declara as classes em marcadores `100`, e as tabelas, os dois
//! blocos de espaço e o dicionário raiz precisam existir. Sem isso, leitores estritos recusam o arquivo.

use crate::{Anexo, Camada, Desenho, Entidade, Item, TipoLinha, Vertice};
use std::fmt::Write as _;

struct Gravador {
    s: String,
    proximo: u32,
}

impl Gravador {
    fn par(&mut self, c: i32, v: &str) {
        let _ = write!(self.s, "{c}\n{v}\n");
    }

    /// Medida em mm ou grau: um décimo de mícron basta.
    fn num(&mut self, c: i32, v: f64) {
        let _ = write!(self.s, "{c}\n{v:.4}\n");
    }

    /// Número sem unidade (bojo, razão, parâmetro, nó, direção), em que o erro cresce com a peça.
    fn fino(&mut self, c: i32, v: f64) {
        let _ = write!(self.s, "{c}\n{v:.9}\n");
    }

    fn int(&mut self, c: i32, v: i64) {
        let _ = write!(self.s, "{c}\n{v}\n");
    }

    fn ponto(&mut self, c: i32, p: [f64; 2]) {
        self.num(c, p[0]);
        self.num(c + 10, p[1]);
        self.num(c + 20, 0.);
    }

    fn id(&mut self) -> String {
        self.proximo += 1;
        format!("{:X}", self.proximo)
    }

    /// Identificador já reservado, para quem precisa ser citado antes de existir.
    fn identificado(&mut self, h: &str) {
        self.par(5, h);
    }

    fn secao(&mut self, nome: &str) {
        self.par(0, "SECTION");
        self.par(2, nome);
    }

    fn fim(&mut self) {
        self.par(0, "ENDSEC");
    }

    fn tabela(&mut self, nome: &str, n: usize) -> String {
        let h = self.id();
        self.par(0, "TABLE");
        self.par(2, nome);
        self.identificado(&h);
        self.par(330, "0");
        self.par(100, "AcDbSymbolTable");
        self.int(70, n as i64);
        h
    }

    fn registro(&mut self, tipo: &str, dono: &str, classe: &str) -> String {
        let h = self.id();
        self.par(0, tipo);
        self.identificado(&h);
        self.par(330, dono);
        self.par(100, "AcDbSymbolTableRecord");
        self.par(100, classe);
        h
    }

    fn fim_tabela(&mut self) {
        self.par(0, "ENDTAB");
    }
}

/// Texto de campo do DXF. O R2000 grava em página de código, e o Fenkra não sabe a de quem vai ler:
/// tudo fora do ASCII sai como `\U+XXXX`, que todo leitor dessa versão entende.
fn ascii(t: &str) -> String {
    let mut s = String::new();
    for c in t.chars() {
        match c {
            ' '..='~' | '\n' => s.push(c),
            c => {
                let mut b = [0u16; 2];
                for u in c.encode_utf16(&mut b) {
                    let _ = write!(s, "\\U+{:04X}", *u);
                }
            }
        }
    }
    s
}

/// Conteúdo de texto: além do ASCII, a contrabarra e as chaves são comandos e a quebra é `\P`.
fn texto_mtext(t: &str) -> String {
    ascii(&t.replace('\\', "\\\\").replace('{', "\\{").replace('}', "\\}")).replace('\n', "\\P")
}

fn cabecalho(g: &mut Gravador, d: &Desenho) {
    g.secao("HEADER");
    g.par(9, "$ACADVER");
    g.par(1, "AC1015");
    g.par(9, "$DWGCODEPAGE");
    g.par(3, "ANSI_1252");
    g.par(9, "$INSUNITS");
    g.int(70, d.unidade.codigo() as i64);
    g.par(9, "$MEASUREMENT");
    g.int(70, 1);
    g.par(9, "$HANDSEED");
    // reserva o espaço: o valor só se conhece no fim, e por isso é trocado por `gravar`
    g.par(5, "SEMENTE");
    g.fim();
    g.secao("CLASSES");
    g.fim();
}

const TIPOS_FIXOS: [&str; 3] = ["ByBlock", "ByLayer", "Continuous"];

fn tipos_linha(g: &mut Gravador, d: &Desenho) {
    let h = g.tabela("LTYPE", TIPOS_FIXOS.len() + d.tipos_linha.len());
    for nome in TIPOS_FIXOS {
        g.registro("LTYPE", &h, "AcDbLinetypeTableRecord");
        g.par(2, nome);
        g.int(70, 0);
        g.par(3, "");
        g.int(72, 65);
        g.int(73, 0);
        g.num(40, 0.);
    }
    for TipoLinha { nome, padrao } in &d.tipos_linha {
        g.registro("LTYPE", &h, "AcDbLinetypeTableRecord");
        g.par(2, &ascii(nome));
        g.int(70, 0);
        g.par(3, &ascii(nome));
        g.int(72, 65);
        g.int(73, padrao.len() as i64);
        g.num(40, padrao.iter().map(|x| x.abs()).sum());
        for x in padrao {
            g.num(49, *x);
            g.int(74, 0);
        }
    }
    g.fim_tabela();
}

fn camadas(g: &mut Gravador, d: &Desenho, estilo_de_plotagem: &str) {
    let tem_zero = d.camadas.iter().any(|c| c.nome == "0");
    let zero = Camada::nova("0", 7, "Continuous");
    let lista: Vec<&Camada> = (!tem_zero).then_some(&zero).into_iter().chain(&d.camadas).collect();
    let h = g.tabela("LAYER", lista.len());
    for c in lista {
        g.registro("LAYER", &h, "AcDbLayerTableRecord");
        g.par(2, &ascii(&c.nome));
        g.int(70, 0);
        g.int(62, c.cor as i64);
        g.par(6, &ascii(&c.tipo_linha));
        g.int(370, c.espessura.unwrap_or(-3) as i64);
        g.par(390, estilo_de_plotagem);
    }
    g.fim_tabela();
}

/// Tabelas. Devolve os identificadores dos registros de bloco dos dois espaços.
fn tabelas(g: &mut Gravador, d: &Desenho, estilo_de_plotagem: &str) -> (String, String) {
    g.secao("TABLES");
    let h = g.tabela("VPORT", 1);
    g.registro("VPORT", &h, "AcDbViewportTableRecord");
    g.par(2, "*Active");
    g.int(70, 0);
    for (c, v) in [(10, 0.), (20, 0.), (11, 1.), (21, 1.), (12, 0.), (22, 0.), (13, 0.), (23, 0.), (14, 10.), (24, 10.), (15, 10.), (25, 10.), (16, 0.), (26, 0.), (36, 1.), (17, 0.), (27, 0.), (37, 0.), (40, 300.), (41, 1.5), (42, 50.), (43, 0.), (44, 0.), (50, 0.), (51, 0.)] {
        g.num(c, v);
    }
    g.fim_tabela();
    tipos_linha(g, d);
    camadas(g, d, estilo_de_plotagem);
    let h = g.tabela("STYLE", 1);
    g.registro("STYLE", &h, "AcDbTextStyleTableRecord");
    g.par(2, "Standard");
    g.int(70, 0);
    g.num(40, 0.);
    g.num(41, 1.);
    g.num(50, 0.);
    g.int(71, 0);
    g.num(42, 2.5);
    g.par(3, "txt");
    g.par(4, "");
    g.fim_tabela();
    g.tabela("VIEW", 0);
    g.fim_tabela();
    g.tabela("UCS", 0);
    g.fim_tabela();
    let h = g.tabela("APPID", 1);
    g.registro("APPID", &h, "AcDbRegAppTableRecord");
    g.par(2, "ACAD");
    g.int(70, 0);
    g.fim_tabela();
    let h = g.tabela("DIMSTYLE", 1);
    g.par(100, "AcDbDimStyleTable");
    g.int(71, 1);
    let r = g.id();
    g.par(0, "DIMSTYLE");
    g.par(105, &r);
    g.par(330, &h);
    g.par(100, "AcDbSymbolTableRecord");
    g.par(100, "AcDbDimStyleTableRecord");
    g.par(2, "Standard");
    g.int(70, 0);
    g.fim_tabela();
    let h = g.tabela("BLOCK_RECORD", 2);
    let modelo = g.registro("BLOCK_RECORD", &h, "AcDbBlockTableRecord");
    g.par(2, "*Model_Space");
    let papel = g.registro("BLOCK_RECORD", &h, "AcDbBlockTableRecord");
    g.par(2, "*Paper_Space");
    g.fim_tabela();
    g.fim();
    (modelo, papel)
}

fn blocos(g: &mut Gravador, modelo: &str, papel: &str) {
    g.secao("BLOCKS");
    for (nome, dono) in [("*Model_Space", modelo), ("*Paper_Space", papel)] {
        let h = g.id();
        g.par(0, "BLOCK");
        g.identificado(&h);
        g.par(330, dono);
        g.par(100, "AcDbEntity");
        g.par(8, "0");
        g.par(100, "AcDbBlockBegin");
        g.par(2, nome);
        g.int(70, 0);
        g.ponto(10, [0., 0.]);
        g.par(3, nome);
        g.par(1, "");
        let h = g.id();
        g.par(0, "ENDBLK");
        g.identificado(&h);
        g.par(330, dono);
        g.par(100, "AcDbEntity");
        g.par(8, "0");
        g.par(100, "AcDbBlockEnd");
    }
    g.fim();
}

fn comum(g: &mut Gravador, tipo: &str, dono: &str, item: &Item) {
    let h = g.id();
    g.par(0, tipo);
    g.identificado(&h);
    g.par(330, dono);
    g.par(100, "AcDbEntity");
    g.par(8, &ascii(&item.camada));
    if let Some(c) = item.cor {
        g.int(62, c as i64);
    }
    if let Some(e) = item.espessura {
        g.int(370, e as i64);
    }
}

fn normal(g: &mut Gravador) {
    g.num(210, 0.);
    g.num(220, 0.);
    g.num(230, 1.);
}

fn entidade(g: &mut Gravador, dono: &str, item: &Item) {
    match &item.entidade {
        Entidade::Linha(a, b) => {
            comum(g, "LINE", dono, item);
            g.par(100, "AcDbLine");
            g.ponto(10, *a);
            g.ponto(11, *b);
        }
        Entidade::Circulo { centro, raio } => {
            comum(g, "CIRCLE", dono, item);
            g.par(100, "AcDbCircle");
            g.ponto(10, *centro);
            g.num(40, *raio);
        }
        Entidade::Arco { centro, raio, ini, fim } => {
            comum(g, "ARC", dono, item);
            g.par(100, "AcDbCircle");
            g.ponto(10, *centro);
            g.num(40, *raio);
            g.par(100, "AcDbArc");
            g.num(50, *ini);
            g.num(51, *fim);
        }
        Entidade::Polilinha { vertices, fechada } => {
            comum(g, "LWPOLYLINE", dono, item);
            g.par(100, "AcDbPolyline");
            g.int(90, vertices.len() as i64);
            g.int(70, *fechada as i64);
            for Vertice { p, bojo } in vertices {
                g.num(10, p[0]);
                g.num(20, p[1]);
                if *bojo != 0. {
                    g.fino(42, *bojo);
                }
            }
        }
        Entidade::Elipse { centro, maior, razao, t0, t1 } => {
            comum(g, "ELLIPSE", dono, item);
            g.par(100, "AcDbEllipse");
            g.ponto(10, *centro);
            g.ponto(11, *maior);
            normal(g);
            g.fino(40, *razao);
            g.fino(41, *t0);
            g.fino(42, *t1);
        }
        Entidade::Spline { grau, controle, nos, pesos, ajuste, fechada } => {
            comum(g, "SPLINE", dono, item);
            g.par(100, "AcDbSpline");
            normal(g);
            g.int(70, 8 + *fechada as i64 + if pesos.is_empty() { 0 } else { 4 });
            g.int(71, *grau as i64);
            g.int(72, nos.len() as i64);
            g.int(73, controle.len() as i64);
            g.int(74, ajuste.len() as i64);
            for k in nos {
                g.fino(40, *k);
            }
            for w in pesos {
                g.fino(41, *w);
            }
            for p in controle {
                g.ponto(10, *p);
            }
            for p in ajuste {
                g.ponto(11, *p);
            }
        }
        Entidade::Ponto(p) => {
            comum(g, "POINT", dono, item);
            g.par(100, "AcDbPoint");
            g.ponto(10, *p);
        }
        Entidade::Texto { pos, altura, angulo, anexo, texto } => {
            comum(g, "MTEXT", dono, item);
            g.par(100, "AcDbMText");
            g.ponto(10, *pos);
            g.num(40, *altura);
            g.num(41, 0.);
            g.int(71, match anexo {
                Anexo::Esquerda => 7,
                Anexo::Centro => 8,
                Anexo::Direita => 9,
            });
            g.int(72, 1);
            // o conteúdo vai em pedaços: nenhuma linha do arquivo pode passar de 255 caracteres
            let t = texto_mtext(texto);
            let letras: Vec<char> = t.chars().collect();
            let mut pedacos = letras.chunks(240).map(|c| c.iter().collect::<String>()).collect::<Vec<_>>();
            let ultimo = pedacos.pop().unwrap_or_default();
            for p in pedacos {
                g.par(3, &p);
            }
            g.par(1, &ultimo);
            g.par(7, "Standard");
            let (seno, cosseno) = angulo.to_radians().sin_cos();
            g.fino(11, cosseno);
            g.fino(21, seno);
            g.num(31, 0.);
        }
        Entidade::Hachura { lacos } => {
            comum(g, "HATCH", dono, item);
            g.par(100, "AcDbHatch");
            g.ponto(10, [0., 0.]);
            normal(g);
            g.par(2, "SOLID");
            g.int(70, 1);
            g.int(71, 0);
            g.int(91, lacos.len() as i64);
            for laco in lacos {
                // 1 externo + 2 polilinha
                g.int(92, 3);
                g.int(72, 0);
                g.int(73, 1);
                g.int(93, laco.len() as i64);
                for p in laco {
                    g.num(10, p[0]);
                    g.num(20, p[1]);
                }
                g.int(97, 0);
            }
            g.int(75, 0);
            g.int(76, 1);
            g.int(98, 1);
            g.num(10, 0.);
            g.num(20, 0.);
        }
    }
}

/// Dicionário raiz com os grupos e o estilo de plotagem, que toda camada do R2000 aponta.
fn objetos(g: &mut Gravador, raiz: &str, estilo_de_plotagem: &str) {
    g.secao("OBJECTS");
    let (grupos, estilos) = (g.id(), g.id());
    g.par(0, "DICTIONARY");
    g.identificado(raiz);
    g.par(330, "0");
    g.par(100, "AcDbDictionary");
    g.int(281, 1);
    g.par(3, "ACAD_GROUP");
    g.par(350, &grupos);
    g.par(3, "ACAD_PLOTSTYLENAME");
    g.par(350, &estilos);
    g.par(0, "DICTIONARY");
    g.identificado(&grupos);
    g.par(330, raiz);
    g.par(100, "AcDbDictionary");
    g.int(281, 1);
    g.par(0, "ACDBDICTIONARYWDFLT");
    g.identificado(&estilos);
    g.par(330, raiz);
    g.par(100, "AcDbDictionary");
    g.int(281, 1);
    g.par(3, "Normal");
    g.par(350, estilo_de_plotagem);
    g.par(100, "AcDbDictionaryWithDefault");
    g.par(340, estilo_de_plotagem);
    g.par(0, "ACDBPLACEHOLDER");
    g.identificado(estilo_de_plotagem);
    g.par(330, &estilos);
    g.fim();
}

/// Texto do arquivo. Uma camada citada por item e fora da lista entra sozinha, com a cor padrão.
pub fn gravar(d: &Desenho) -> String {
    let mut d = d.clone();
    let faltam: Vec<String> = d.itens.iter().map(|i| i.camada.clone()).filter(|n| n != "0").collect();
    for nome in faltam {
        if !d.camadas.iter().any(|c| c.nome == nome) {
            d.camadas.push(Camada::nova(&nome, 7, "Continuous"));
        }
    }
    let mut g = Gravador { s: String::new(), proximo: 0x10 };
    let (raiz, estilo) = (g.id(), g.id());
    cabecalho(&mut g, &d);
    let (modelo, papel) = tabelas(&mut g, &d, &estilo);
    blocos(&mut g, &modelo, &papel);
    g.secao("ENTITIES");
    for item in &d.itens {
        entidade(&mut g, &modelo, item);
    }
    g.fim();
    objetos(&mut g, &raiz, &estilo);
    g.par(0, "EOF");
    let semente = format!("{:X}", g.proximo + 1);
    g.s.replacen("SEMENTE", &semente, 1)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::{Unidade, P2};

    fn desenho() -> Desenho {
        let quadrado: Vec<P2> = vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        let i = |e| Item::novo("Contorno", e);
        Desenho {
            unidade: Unidade::Milimetro,
            tipos_linha: vec![TipoLinha { nome: "TRACO".into(), padrao: vec![6., -3.] }],
            camadas: vec![Camada::nova("Contorno", 3, "TRACO")],
            itens: vec![
                i(Entidade::Linha([0., 0.], [10., 0.])),
                i(Entidade::Arco { centro: [0., 0.], raio: 5., ini: 0., fim: 90. }),
                i(Entidade::Circulo { centro: [1., 2.], raio: 3. }),
                i(Entidade::Polilinha { vertices: quadrado.iter().map(|p| Vertice { p: *p, bojo: 0. }).collect(), fechada: true }),
                i(Entidade::Hachura { lacos: vec![quadrado] }),
                i(Entidade::Texto { pos: [1., 1.], altura: 2.5, angulo: 0., anexo: Anexo::Esquerda, texto: "Ø10 {a}\\b".into() }),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn identificadores_nao_repetem_e_a_semente_passa_de_todos() {
        let t = gravar(&desenho());
        let linhas: Vec<&str> = t.lines().collect();
        let i = linhas.iter().position(|l| *l == "$HANDSEED").unwrap();
        let semente = u32::from_str_radix(linhas[i + 2], 16).unwrap();
        let mut vistos = std::collections::HashSet::new();
        for (n, par) in linhas.chunks(2).enumerate() {
            if matches!(par[0], "5" | "105") && n * 2 != i + 1 {
                let h = u32::from_str_radix(par[1], 16).unwrap();
                assert!(vistos.insert(h), "identificador repetido {h:X}");
                assert!(h < semente, "{h:X} passa da semente {semente:X}");
            }
        }
        assert!(vistos.len() > 20);
    }

    #[test]
    fn texto_escapa_comandos_e_o_que_nao_e_ascii() {
        let t = gravar(&desenho());
        assert!(t.contains("\\U+00D810 \\{a\\}\\\\b"), "{t}");
        assert!(t.is_ascii(), "o arquivo não depende de página de código");
        assert_eq!(ascii("𝄞"), "\\U+D834\\U+DD1E");
    }

    #[test]
    fn texto_longo_vai_em_pedacos_e_nenhuma_linha_passa_do_limite() {
        let mut d = desenho();
        d.itens = vec![Item::novo("0", Entidade::Texto { pos: [0., 0.], altura: 2., angulo: 0., anexo: Anexo::Centro, texto: "á".repeat(400) })];
        let t = gravar(&d);
        assert!(t.lines().all(|l| l.len() <= 255));
        let de_volta = crate::ler(t.as_bytes()).unwrap();
        let Entidade::Texto { texto, anexo, .. } = &de_volta.itens[0].entidade else { panic!() };
        assert_eq!((texto.as_str(), *anexo), ("á".repeat(400).as_str(), Anexo::Centro));
    }

    #[test]
    fn camada_sem_cadastro_entra_na_tabela() {
        let mut d = desenho();
        d.itens.push(Item::novo("Solta", Entidade::Ponto([0., 0.])));
        let lido = crate::ler(gravar(&d).as_bytes()).unwrap();
        assert!(lido.camadas.iter().any(|c| c.nome == "Solta"));
        assert_eq!(lido.camadas.iter().find(|c| c.nome == "0").map(|c| c.cor), Some(7));
    }
}
