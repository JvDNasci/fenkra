// SPDX-License-Identifier: LGPL-2.1-or-later
//! Perfil de qualidade do projeto e exceções no diário (B305).
//!
//! O perfil fica em `.qualidade.json`, na raiz, gravado de forma atômica (ver `seguro`). Quem pode
//! mudá-lo decide [`Perfil::validar_edicao`] a partir do papel `administrador` do PLM. As exceções
//! não têm arquivo próprio: são eventos `Excecao` do diário, que só acrescenta, e por isso a trilha
//! não pode ser reescrita por quem recebeu a exceção.

use crate::diario::{self, Evento, TipoEvento};
use crate::qualidade_modelo::{avaliar, Acesso, Excecao, Gravidade, Medidas, Momento, Perfil, Regra, Relatorio};
use crate::seguro::gravar_atomico;
use crate::{now, PResult, Project, ProjectError};

pub const ARQUIVO_PERFIL: &str = ".qualidade.json";

impl Project {
    /// Acesso de quem opera, pelos papéis do PLM.
    pub fn acesso_qualidade(&self) -> Acesso {
        Acesso::dos_papeis(&self.papeis)
    }

    /// Perfil em vigor. Sem arquivo (ou arquivo ilegível) vale o padrão: nunca fica sem regra.
    pub fn perfil_qualidade(&self) -> Perfil {
        std::fs::read_to_string(self.root.join(ARQUIVO_PERFIL)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(Perfil::padrao)
    }

    /// Grava um perfil novo se o acesso de quem opera permite a mudança em relação ao atual.
    pub fn definir_perfil_qualidade(&mut self, novo: Perfil) -> PResult<()> {
        self.pode_gravar()?;
        self.perfil_qualidade().validar_edicao(&novo, self.acesso_qualidade()).map_err(ProjectError::Recusado)?;
        let json = serde_json::to_string_pretty(&novo)?;
        gravar_atomico(&self.root.join(ARQUIVO_PERFIL), json.as_bytes())?;
        Ok(())
    }

    /// Exceções em vigor para a versão atual do documento (as de versões antigas não valem).
    pub fn excecoes_qualidade(&self, id: u64) -> PResult<Vec<Excecao>> {
        let versao = self.entry(id)?.versions.last().map(|v| v.hash.clone()).unwrap_or_default();
        let (eventos, _) = diario::ler(&self.root);
        Ok(eventos
            .into_iter()
            .filter(|e| e.tipo == TipoEvento::Excecao && e.doc_id == id && e.para == versao)
            .filter_map(|e| Some(Excecao { regra: Regra::de_chave(&e.de)?, versao: e.para, nome: e.autor, motivo: e.motivo, tempo: e.tempo }))
            .collect())
    }

    /// Aceita a violação de uma regra para a versão atual. Exige nome e motivo; num perfil travado,
    /// regra que bloqueia só o administrador libera.
    pub fn conceder_excecao_qualidade(&mut self, id: u64, regra: Regra, nome: &str, motivo: &str) -> PResult<()> {
        self.pode_gravar()?;
        if nome.trim().is_empty() || motivo.trim().is_empty() {
            return Err(ProjectError::Recusado("a exceção precisa de nome e motivo".into()));
        }
        let perfil = self.perfil_qualidade();
        let bloqueia = perfil.limite(regra).is_some_and(|l| l.gravidade == Gravidade::Bloqueia);
        if perfil.travado && bloqueia && self.acesso_qualidade() != Acesso::Administrador {
            return Err(ProjectError::Recusado("perfil travado: só o administrador concede exceção a uma regra que bloqueia".into()));
        }
        let e = self.entry(id)?;
        let versao = e.versions.last().map(|v| v.hash.clone()).ok_or(ProjectError::NotFound)?;
        let uid = e.uid.clone();
        diario::acrescentar(
            &self.root,
            &Evento { tempo: now(), autor: nome.trim().into(), tipo: TipoEvento::Excecao, doc_id: id, uid, de: regra.chave().into(), para: versao, motivo: motivo.trim().into() },
        )?;
        Ok(())
    }

    /// Mede o documento contra o perfil do projeto e as exceções da versão atual.
    pub fn avaliar_qualidade(&self, id: u64, medidas: &Medidas, momento: Momento) -> PResult<Relatorio> {
        let excecoes = self.excecoes_qualidade(id)?;
        Ok(avaliar(&self.perfil_qualidade(), medidas, momento, &excecoes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qualidade_modelo::{Gravidade, PAPEL_ADMINISTRADOR};
    use fenkra_core::Document;
    use std::path::PathBuf;

    fn tmp(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fenkra-qual-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    fn pesada() -> Medidas {
        Medidas { faces: 4000, ..Default::default() }
    }

    #[test]
    fn admin_trava_o_perfil_e_a_peca_pesada_e_barrada() {
        let raiz = tmp("trava");
        let mut p = Project::create(&raiz, "T").unwrap();
        let id = p.add_document(Document::new("Peça")).unwrap();
        p.papeis = vec![PAPEL_ADMINISTRADOR.into()];
        let mut rig = Perfil::rigoroso();
        rig.travado = true;
        p.definir_perfil_qualidade(rig.clone()).unwrap();
        assert_eq!(p.perfil_qualidade(), rig, "o perfil volta do disco igual");
        assert!(p.avaliar_qualidade(id, &pesada(), Momento::SubirAoServidor).unwrap().bloqueia());

        // o usuário local não afrouxa o perfil travado
        p.papeis = vec!["projetista".into()];
        let mut frouxo = rig.clone();
        frouxo.regras.iter_mut().find(|l| l.regra == Regra::Faces).unwrap().limite = 9999.;
        assert!(matches!(p.definir_perfil_qualidade(frouxo), Err(ProjectError::Recusado(_))));
        assert_eq!(p.perfil_qualidade(), rig, "a recusa não mexe no arquivo");
        // mas pode endurecer
        let mut duro = rig;
        duro.regras.iter_mut().find(|l| l.regra == Regra::Faces).unwrap().limite = 500.;
        p.definir_perfil_qualidade(duro).unwrap();
        let _ = std::fs::remove_dir_all(raiz);
    }

    #[test]
    fn excecao_entra_na_auditoria_com_nome_e_motivo() {
        let raiz = tmp("excecao");
        let mut p = Project::create(&raiz, "T").unwrap();
        let mut d = Document::new("Peça");
        let id = p.add_document(d.clone()).unwrap();
        p.papeis = vec![PAPEL_ADMINISTRADOR.into()];
        let mut rig = Perfil::rigoroso();
        rig.travado = true;
        p.definir_perfil_qualidade(rig).unwrap();

        // sem nome ou sem motivo, nada é registrado
        assert!(p.conceder_excecao_qualidade(id, Regra::Faces, "", "x").is_err());
        assert!(p.conceder_excecao_qualidade(id, Regra::Faces, "Ana", "  ").is_err());
        assert!(diario::ler(&raiz).0.iter().all(|e| e.tipo != TipoEvento::Excecao));

        // usuário local não libera regra que bloqueia em perfil travado
        p.papeis = vec!["projetista".into()];
        assert!(p.conceder_excecao_qualidade(id, Regra::Faces, "Beto", "urgente").is_err());

        p.papeis = vec![PAPEL_ADMINISTRADOR.into()];
        p.conceder_excecao_qualidade(id, Regra::Faces, "Ana", "peça de fundição").unwrap();
        let ev: Vec<_> = diario::ler(&raiz).0.into_iter().filter(|e| e.tipo == TipoEvento::Excecao).collect();
        assert_eq!(ev.len(), 1);
        assert_eq!((ev[0].autor.as_str(), ev[0].motivo.as_str(), ev[0].de.as_str()), ("Ana", "peça de fundição", "faces"));

        let r = p.avaliar_qualidade(id, &pesada(), Momento::SubirAoServidor).unwrap();
        assert!(!r.bloqueia(), "a exceção libera a peça");
        assert!(r.violacoes.iter().any(|v| v.regra == Regra::Faces && v.excecao.as_ref().is_some_and(|e| e.nome == "Ana")));
        assert!(r.avisos().any(|v| v.regra == Regra::Faces), "segue listada como aviso");

        // editar a peça cria versão nova: a exceção antiga deixa de valer
        d.props.description = "mudou".into();
        p.save(id, &d, "edição").unwrap();
        assert!(p.excecoes_qualidade(id).unwrap().is_empty());
        assert!(p.avaliar_qualidade(id, &pesada(), Momento::SubirAoServidor).unwrap().bloqueia());
        let _ = std::fs::remove_dir_all(raiz);
    }

    #[test]
    fn sem_arquivo_vale_o_padrao() {
        let raiz = tmp("padrao");
        let p = Project::create(&raiz, "T").unwrap();
        assert_eq!(p.perfil_qualidade(), Perfil::padrao());
        assert!(p.perfil_qualidade().regras.iter().any(|l| l.gravidade == Gravidade::Bloqueia));
        let _ = std::fs::remove_dir_all(raiz);
    }
}
