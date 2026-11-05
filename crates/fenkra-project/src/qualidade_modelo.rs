// SPDX-License-Identifier: LGPL-2.1-or-later
//! Checador de qualidade do modelo com perfis por acesso (B305).
//!
//! Mede o peso e a saúde de uma peça (faces, triângulos, tamanho do arquivo, tempo de recálculo,
//! faces e arestas minúsculas, geometria inválida, rosca modelada, malha importada e referências em
//! erro) e compara cada medida com os limites de um **perfil**. Cada regra do perfil tem uma
//! gravidade: informa, avisa ou bloqueia.
//!
//! Por que o perfil carrega um cadeado: numa equipe, o administrador define o que uma peça pode
//! pesar antes de subir ao servidor. Se qualquer pessoa pudesse afrouxar o perfil, a regra não valeria
//! nada. Por isso, com o perfil **travado**, o usuário local só consegue deixá-lo mais rígido (limite
//! menor, gravidade maior, mais regras, mais momentos). Perfil destravado o usuário local edita à
//! vontade, mas só o administrador trava e destrava.
//!
//! Exceção é a válvula de escape com rastro: aceitar uma peça acima do limite exige **nome e motivo**,
//! vale só para aquela versão (editou, precisa justificar de novo) e fica no diário
//! (`.diario.jsonl`) como evento `Excecao`. Num perfil travado só o administrador concede exceção a
//! uma regra que bloqueia.
//!
//! Este módulo não conhece o kernel: as [`Medidas`] chegam prontas de quem calculou a peça.

use serde::{Deserialize, Serialize};

/// O que se mede. A ordem é a da tela.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Regra {
    Faces,
    Triangulos,
    TamanhoArquivo,
    TempoRecalculo,
    FacesMinusculas,
    ArestasMinusculas,
    GeometriaInvalida,
    RoscaModelada,
    MalhaImportada,
    ReferenciasEmErro,
}

impl Regra {
    pub const TODAS: [Regra; 10] = [
        Regra::Faces,
        Regra::Triangulos,
        Regra::TamanhoArquivo,
        Regra::TempoRecalculo,
        Regra::FacesMinusculas,
        Regra::ArestasMinusculas,
        Regra::GeometriaInvalida,
        Regra::RoscaModelada,
        Regra::MalhaImportada,
        Regra::ReferenciasEmErro,
    ];

    /// Chave estável, usada no arquivo do perfil e no diário.
    pub fn chave(self) -> &'static str {
        match self {
            Regra::Faces => "faces",
            Regra::Triangulos => "triangulos",
            Regra::TamanhoArquivo => "tamanho_arquivo",
            Regra::TempoRecalculo => "tempo_recalculo",
            Regra::FacesMinusculas => "faces_minusculas",
            Regra::ArestasMinusculas => "arestas_minusculas",
            Regra::GeometriaInvalida => "geometria_invalida",
            Regra::RoscaModelada => "rosca_modelada",
            Regra::MalhaImportada => "malha_importada",
            Regra::ReferenciasEmErro => "referencias_em_erro",
        }
    }

    pub fn de_chave(c: &str) -> Option<Regra> {
        Regra::TODAS.into_iter().find(|r| r.chave() == c)
    }

    /// Texto de interface (passa por `tr`).
    pub fn rotulo(self) -> &'static str {
        match self {
            Regra::Faces => "Faces",
            Regra::Triangulos => "Triângulos da malha",
            Regra::TamanhoArquivo => "Tamanho do arquivo (KB)",
            Regra::TempoRecalculo => "Tempo de recálculo (ms)",
            Regra::FacesMinusculas => "Faces minúsculas",
            Regra::ArestasMinusculas => "Arestas minúsculas",
            Regra::GeometriaInvalida => "Geometria inválida",
            Regra::RoscaModelada => "Roscas modeladas",
            Regra::MalhaImportada => "Malhas importadas",
            Regra::ReferenciasEmErro => "Referências em erro",
        }
    }
}

/// O que acontece quando a regra é violada. A ordem importa: `Bloqueia` é a mais rígida.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gravidade {
    Informa,
    Avisa,
    Bloqueia,
}

impl Gravidade {
    pub const TODAS: [Gravidade; 3] = [Gravidade::Informa, Gravidade::Avisa, Gravidade::Bloqueia];

    pub fn rotulo(self) -> &'static str {
        match self {
            Gravidade::Informa => "Informa",
            Gravidade::Avisa => "Avisa",
            Gravidade::Bloqueia => "Bloqueia",
        }
    }
}

/// Quando o checador roda.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Momento {
    InserirNaMontagem,
    MudarEstado,
    Revisar,
    SubirAoServidor,
}

impl Momento {
    pub const TODOS: [Momento; 4] = [Momento::InserirNaMontagem, Momento::MudarEstado, Momento::Revisar, Momento::SubirAoServidor];

    pub fn chave(self) -> &'static str {
        match self {
            Momento::InserirNaMontagem => "inserir_na_montagem",
            Momento::MudarEstado => "mudar_estado",
            Momento::Revisar => "revisar",
            Momento::SubirAoServidor => "subir_ao_servidor",
        }
    }

    pub fn rotulo(self) -> &'static str {
        match self {
            Momento::InserirNaMontagem => "Ao inserir na montagem",
            Momento::MudarEstado => "Ao mudar de estado",
            Momento::Revisar => "Ao criar revisão",
            Momento::SubirAoServidor => "Antes de subir ao servidor",
        }
    }
}

/// Medidas de uma peça, como o recálculo as apurou. Zero = nada a relatar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Medidas {
    pub faces: u64,
    pub triangulos: u64,
    pub tamanho_arquivo_kb: f64,
    pub tempo_recalculo_ms: f64,
    pub faces_minusculas: u64,
    pub arestas_minusculas: u64,
    pub geometria_invalida: u64,
    pub roscas_modeladas: u64,
    pub malhas_importadas: u64,
    pub referencias_em_erro: u64,
}

impl Medidas {
    pub fn valor(&self, r: Regra) -> f64 {
        match r {
            Regra::Faces => self.faces as f64,
            Regra::Triangulos => self.triangulos as f64,
            Regra::TamanhoArquivo => self.tamanho_arquivo_kb,
            Regra::TempoRecalculo => self.tempo_recalculo_ms,
            Regra::FacesMinusculas => self.faces_minusculas as f64,
            Regra::ArestasMinusculas => self.arestas_minusculas as f64,
            Regra::GeometriaInvalida => self.geometria_invalida as f64,
            Regra::RoscaModelada => self.roscas_modeladas as f64,
            Regra::MalhaImportada => self.malhas_importadas as f64,
            Regra::ReferenciasEmErro => self.referencias_em_erro as f64,
        }
    }
}

/// Uma regra do perfil: violada quando a medida passa de `limite`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Limite {
    pub regra: Regra,
    pub limite: f64,
    pub gravidade: Gravidade,
}

/// Conjunto de limites com a política de quem pode mexer nele.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Perfil {
    pub nome: String,
    /// travado pelo administrador: o usuário local só deixa mais rígido
    pub travado: bool,
    pub regras: Vec<Limite>,
    /// momentos em que o checador roda
    pub momentos: Vec<Momento>,
    /// teto da soma de triângulos da montagem; 0 = sem teto
    pub montagem_triangulos: f64,
    pub montagem_gravidade: Gravidade,
}

impl Perfil {
    fn com(nome: &str, regras: &[(Regra, f64, Gravidade)], montagem: f64) -> Perfil {
        Perfil {
            nome: nome.into(),
            travado: false,
            regras: regras.iter().map(|&(regra, limite, gravidade)| Limite { regra, limite, gravidade }).collect(),
            momentos: Momento::TODOS.to_vec(),
            montagem_triangulos: montagem,
            montagem_gravidade: Gravidade::Avisa,
        }
    }

    /// Só informa: serve para conhecer o peso das peças sem atrapalhar ninguém.
    pub fn informativo() -> Perfil {
        use Gravidade::Informa as I;
        Perfil::com("Informativo", &[(Regra::Faces, 5000., I), (Regra::Triangulos, 500_000., I), (Regra::TamanhoArquivo, 20_000., I), (Regra::TempoRecalculo, 5000., I)], 0.)
    }

    /// Padrão: avisa do peso e bloqueia o que quebra o modelo (geometria inválida, referência em erro).
    pub fn padrao() -> Perfil {
        use Gravidade::{Avisa as A, Bloqueia as B};
        Perfil::com(
            "Padrão",
            &[
                (Regra::Faces, 3000., A),
                (Regra::Triangulos, 300_000., A),
                (Regra::TamanhoArquivo, 10_000., A),
                (Regra::TempoRecalculo, 3000., A),
                (Regra::FacesMinusculas, 0., A),
                (Regra::ArestasMinusculas, 0., A),
                (Regra::GeometriaInvalida, 0., B),
                (Regra::RoscaModelada, 0., A),
                (Regra::MalhaImportada, 0., A),
                (Regra::ReferenciasEmErro, 0., B),
            ],
            2_000_000.,
        )
    }

    /// Rigoroso: o que sobe ao servidor precisa estar leve e limpo.
    pub fn rigoroso() -> Perfil {
        use Gravidade::{Avisa as A, Bloqueia as B};
        let mut p = Perfil::com(
            "Rigoroso",
            &[
                (Regra::Faces, 1500., B),
                (Regra::Triangulos, 150_000., B),
                (Regra::TamanhoArquivo, 5000., B),
                (Regra::TempoRecalculo, 1500., A),
                (Regra::FacesMinusculas, 0., B),
                (Regra::ArestasMinusculas, 0., A),
                (Regra::GeometriaInvalida, 0., B),
                (Regra::RoscaModelada, 0., B),
                (Regra::MalhaImportada, 0., B),
                (Regra::ReferenciasEmErro, 0., B),
            ],
            1_000_000.,
        );
        p.montagem_gravidade = Gravidade::Bloqueia;
        p
    }

    pub fn limite(&self, r: Regra) -> Option<&Limite> {
        self.regras.iter().find(|l| l.regra == r)
    }

    /// Confere a edição de `self` para `novo` segundo o acesso de quem edita.
    pub fn validar_edicao(&self, novo: &Perfil, acesso: Acesso) -> Result<(), String> {
        if acesso == Acesso::Administrador {
            return Ok(());
        }
        if novo.travado != self.travado {
            return Err("só o administrador trava ou destrava um perfil".into());
        }
        if !self.travado {
            return Ok(());
        }
        // perfil travado: só pode ficar igual ou mais rígido
        for antigo in &self.regras {
            let Some(n) = novo.limite(antigo.regra) else {
                return Err(format!("perfil travado: a regra \"{}\" não pode ser removida", antigo.regra.rotulo()));
            };
            if n.limite > antigo.limite {
                return Err(format!("perfil travado: o limite de \"{}\" não pode subir de {} para {}", antigo.regra.rotulo(), antigo.limite, n.limite));
            }
            if n.gravidade < antigo.gravidade {
                return Err(format!("perfil travado: a regra \"{}\" não pode ficar menos rígida", antigo.regra.rotulo()));
            }
        }
        if let Some(m) = self.momentos.iter().find(|m| !novo.momentos.contains(m)) {
            return Err(format!("perfil travado: \"{}\" não pode ser desligado", m.rotulo()));
        }
        // teto de montagem: 0 significa "sem teto" e é o mais frouxo de todos
        let frouxo = |antigo: f64, novo: f64| antigo > 0. && (novo <= 0. || novo > antigo);
        if frouxo(self.montagem_triangulos, novo.montagem_triangulos) {
            return Err("perfil travado: o teto de triângulos da montagem não pode subir".into());
        }
        if novo.montagem_gravidade < self.montagem_gravidade {
            return Err("perfil travado: o teto da montagem não pode ficar menos rígido".into());
        }
        Ok(())
    }
}

/// Quem opera, para decidir o que pode editar no perfil. Vem do papel `administrador` do PLM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acesso {
    Administrador,
    UsuarioLocal,
}

pub const PAPEL_ADMINISTRADOR: &str = "administrador";

impl Acesso {
    pub fn dos_papeis(papeis: &[String]) -> Acesso {
        if papeis.iter().any(|p| p == PAPEL_ADMINISTRADOR) {
            Acesso::Administrador
        } else {
            Acesso::UsuarioLocal
        }
    }
}

/// Aceite registrado de uma violação: quem, por quê, e para qual versão.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Excecao {
    pub regra: Regra,
    /// hash da versão do documento em que a exceção vale
    pub versao: String,
    pub nome: String,
    pub motivo: String,
    pub tempo: u64,
}

/// Uma regra violada no relatório.
#[derive(Clone, Debug, PartialEq)]
pub struct Violacao {
    pub regra: Regra,
    pub medido: f64,
    pub limite: f64,
    pub gravidade: Gravidade,
    /// exceção em vigor: a violação segue listada, mas não bloqueia
    pub excecao: Option<Excecao>,
}

impl Violacao {
    /// Gravidade que vale agora, já com a exceção.
    pub fn efetiva(&self) -> Gravidade {
        if self.excecao.is_some() && self.gravidade == Gravidade::Bloqueia {
            Gravidade::Avisa
        } else {
            self.gravidade
        }
    }

    pub fn frase(&self) -> String {
        format!("{}: {} (limite {})", self.regra.rotulo(), self.medido, self.limite)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Relatorio {
    pub violacoes: Vec<Violacao>,
    /// o momento não está ligado no perfil: nada foi medido
    pub dispensado: bool,
}

impl Relatorio {
    pub fn bloqueia(&self) -> bool {
        self.violacoes.iter().any(|v| v.efetiva() == Gravidade::Bloqueia)
    }

    pub fn avisos(&self) -> impl Iterator<Item = &Violacao> {
        self.violacoes.iter().filter(|v| v.efetiva() == Gravidade::Avisa)
    }

    pub fn bloqueios(&self) -> impl Iterator<Item = &Violacao> {
        self.violacoes.iter().filter(|v| v.efetiva() == Gravidade::Bloqueia)
    }

    pub fn resumo_bloqueios(&self) -> String {
        self.bloqueios().map(Violacao::frase).collect::<Vec<_>>().join("; ")
    }
}

/// Mede `m` contra o perfil, no `momento`. `excecoes` são as da versão em exame.
pub fn avaliar(perfil: &Perfil, m: &Medidas, momento: Momento, excecoes: &[Excecao]) -> Relatorio {
    if !perfil.momentos.contains(&momento) {
        return Relatorio { violacoes: vec![], dispensado: true };
    }
    let violacoes = perfil
        .regras
        .iter()
        .filter_map(|l| {
            let medido = m.valor(l.regra);
            // NaN nunca passa batido: medida que não é número conta como violação
            (medido > l.limite || medido.is_nan()).then(|| Violacao {
                regra: l.regra,
                medido,
                limite: l.limite,
                gravidade: l.gravidade,
                excecao: excecoes.iter().find(|e| e.regra == l.regra).cloned(),
            })
        })
        .collect();
    Relatorio { violacoes, dispensado: false }
}

/// O que o aviso de inserção mostra antes de a peça entrar na montagem.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviaInsercao {
    pub soma_triangulos: f64,
    pub limite: f64,
    pub passa_do_limite: bool,
    pub gravidade: Gravidade,
    /// oferece inserir a versão simplificada (B125) em vez da peça completa
    pub oferecer_simplificada: bool,
}

/// Quanto a montagem soma ao inserir `quantidade` cópias da peça, e se passa do teto do perfil.
pub fn previa_insercao(perfil: &Perfil, triangulos_atuais: u64, peca: &Medidas, quantidade: u32) -> PreviaInsercao {
    let soma = triangulos_atuais as f64 + peca.triangulos as f64 * quantidade as f64;
    let passa = perfil.montagem_triangulos > 0. && soma > perfil.montagem_triangulos;
    PreviaInsercao { soma_triangulos: soma, limite: perfil.montagem_triangulos, passa_do_limite: passa, gravidade: perfil.montagem_gravidade, oferecer_simplificada: passa }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pesada() -> Medidas {
        Medidas { faces: 4000, triangulos: 100_000, tamanho_arquivo_kb: 800., ..Default::default() }
    }

    fn ajustar(p: &mut Perfil, r: Regra, f: impl FnOnce(&mut Limite)) {
        f(p.regras.iter_mut().find(|l| l.regra == r).unwrap());
    }

    #[test]
    fn peca_acima_do_limite_e_barrada_no_perfil_travado() {
        let mut p = Perfil::rigoroso();
        p.travado = true;
        let r = avaliar(&p, &pesada(), Momento::SubirAoServidor, &[]);
        assert!(r.bloqueia(), "4000 faces passam do limite de 1500 do perfil rigoroso");
        assert_eq!(r.bloqueios().map(|v| v.regra).collect::<Vec<_>>(), vec![Regra::Faces]);
        // o usuário local não consegue afrouxar o limite para a peça passar
        let mut frouxo = p.clone();
        ajustar(&mut frouxo, Regra::Faces, |l| l.limite = 10_000.);
        let erro = p.validar_edicao(&frouxo, Acesso::UsuarioLocal).unwrap_err();
        assert!(erro.contains("Faces"), "{erro}");
        // nem trocar bloqueio por aviso, nem destravar
        let mut aviso = p.clone();
        ajustar(&mut aviso, Regra::Faces, |l| l.gravidade = Gravidade::Avisa);
        assert!(p.validar_edicao(&aviso, Acesso::UsuarioLocal).is_err());
        let mut destravado = p.clone();
        destravado.travado = false;
        assert!(p.validar_edicao(&destravado, Acesso::UsuarioLocal).is_err());
        // o administrador pode tudo
        assert!(p.validar_edicao(&frouxo, Acesso::Administrador).is_ok());
    }

    #[test]
    fn usuario_local_pode_endurecer_o_perfil_travado() {
        let mut p = Perfil::padrao();
        p.travado = true;
        let mut mais = p.clone();
        ajustar(&mut mais, Regra::Faces, |l| {
            l.limite = 1000.;
            l.gravidade = Gravidade::Bloqueia;
        });
        mais.montagem_triangulos = 1_000_000.;
        assert!(p.validar_edicao(&mais, Acesso::UsuarioLocal).is_ok());
        // tirar o teto da montagem (0 = sem teto) é afrouxar
        let mut sem_teto = p.clone();
        sem_teto.montagem_triangulos = 0.;
        assert!(p.validar_edicao(&sem_teto, Acesso::UsuarioLocal).is_err());
        // remover regra e desligar momento também
        let mut menos = p.clone();
        menos.regras.pop();
        assert!(p.validar_edicao(&menos, Acesso::UsuarioLocal).is_err());
        let mut sem_momento = p.clone();
        sem_momento.momentos.pop();
        assert!(p.validar_edicao(&sem_momento, Acesso::UsuarioLocal).is_err());
    }

    #[test]
    fn perfil_destravado_o_usuario_local_edita_mas_nao_trava() {
        let p = Perfil::padrao();
        let mut novo = p.clone();
        ajustar(&mut novo, Regra::Faces, |l| l.limite = 99_999.);
        assert!(p.validar_edicao(&novo, Acesso::UsuarioLocal).is_ok());
        novo.travado = true;
        assert!(p.validar_edicao(&novo, Acesso::UsuarioLocal).is_err());
    }

    #[test]
    fn excecao_rebaixa_o_bloqueio_para_aviso_e_continua_listada() {
        let p = Perfil::rigoroso();
        let ex = Excecao { regra: Regra::Faces, versao: "h1".into(), nome: "Ana".into(), motivo: "peça de fundição".into(), tempo: 1 };
        let r = avaliar(&p, &pesada(), Momento::Revisar, std::slice::from_ref(&ex));
        assert!(!r.bloqueia());
        let v = r.violacoes.iter().find(|v| v.regra == Regra::Faces).unwrap();
        assert_eq!(v.excecao.as_ref(), Some(&ex));
        assert!(r.avisos().any(|a| a.regra == Regra::Faces));
    }

    #[test]
    fn momento_desligado_dispensa_e_medida_dentro_do_limite_passa() {
        let mut p = Perfil::padrao();
        p.momentos = vec![Momento::SubirAoServidor];
        let r = avaliar(&p, &pesada(), Momento::InserirNaMontagem, &[]);
        assert!(r.dispensado && r.violacoes.is_empty());
        let leve = Medidas { faces: 10, triangulos: 200, ..Default::default() };
        assert!(avaliar(&p, &leve, Momento::SubirAoServidor, &[]).violacoes.is_empty());
        // geometria inválida bloqueia no perfil padrão
        let ruim = Medidas { geometria_invalida: 1, ..leve };
        assert!(avaliar(&p, &ruim, Momento::SubirAoServidor, &[]).bloqueia());
        // medida que não é número não passa batido
        let nan = Medidas { tempo_recalculo_ms: f64::NAN, ..leve };
        assert!(!avaliar(&p, &nan, Momento::SubirAoServidor, &[]).violacoes.is_empty());
    }

    #[test]
    fn previa_de_insercao_soma_e_oferece_a_simplificada() {
        let p = Perfil::padrao();
        let peca = Medidas { triangulos: 600_000, ..Default::default() };
        let pr = previa_insercao(&p, 1_000_000, &peca, 1);
        assert_eq!(pr.soma_triangulos, 1_600_000.);
        assert!(!pr.passa_do_limite && !pr.oferecer_simplificada);
        let pr = previa_insercao(&p, 1_000_000, &peca, 2);
        assert_eq!(pr.soma_triangulos, 2_200_000.);
        assert!(pr.passa_do_limite && pr.oferecer_simplificada);
        let mut sem_teto = p;
        sem_teto.montagem_triangulos = 0.;
        assert!(!previa_insercao(&sem_teto, u64::MAX / 2, &peca, 9).passa_do_limite);
    }

    #[test]
    fn perfil_vai_e_volta_pelo_json_e_as_chaves_sao_unicas() {
        let p = Perfil::rigoroso();
        let j = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<Perfil>(&j).unwrap(), p);
        let mut chaves: Vec<_> = Regra::TODAS.iter().map(|r| r.chave()).collect();
        chaves.sort();
        chaves.dedup();
        assert_eq!(chaves.len(), Regra::TODAS.len());
        assert!(Regra::TODAS.iter().all(|r| Regra::de_chave(r.chave()) == Some(*r)));
        assert_eq!(Acesso::dos_papeis(&["projetista".into()]), Acesso::UsuarioLocal);
        assert_eq!(Acesso::dos_papeis(&["administrador".into()]), Acesso::Administrador);
    }
}
