// SPDX-License-Identifier: LGPL-2.1-or-later
//! Registro de pedidos e avisos de mudança de engenharia em arquivo (B141).
//!
//! - **Pedido de mudança** (ECR): alguém pede para mudar um ou mais documentos, com o motivo. O
//!   pedido nasce aberto e é aprovado ou rejeitado, sempre com quem decidiu e por quê.
//! - **Aviso de mudança** (ECN): depois de aprovado, o aviso registra o que de fato saiu: para cada
//!   documento do pedido, a revisão que valia quando o pedido foi aberto e a revisão liberada agora.
//!   Só sai aviso quando todos os documentos do pedido ganharam revisão nova: o aviso é a prova de que
//!   a mudança foi liberada, não uma intenção.
//!
//! Tudo fica em `mudancas.fenkra.json` na raiz do projeto, regravado de forma atômica, e cada passo
//! também vai para o diário só-acréscimo. Números são sequenciais por projeto (`ECR-0001`,
//! `ECN-0001`) e nunca são reaproveitados.

use crate::seguro::gravar_atomico;
use crate::{PResult, Project, ProjectError};
use serde::{Deserialize, Serialize};

pub const ARQUIVO_MUDANCAS: &str = "mudancas.fenkra.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstadoPedido {
    Aberto,
    Aprovado,
    Rejeitado,
    /// o aviso de mudança já foi emitido
    Implementado,
}

impl EstadoPedido {
    pub fn rotulo(self) -> &'static str {
        match self {
            EstadoPedido::Aberto => "Aberto",
            EstadoPedido::Aprovado => "Aprovado",
            EstadoPedido::Rejeitado => "Rejeitado",
            EstadoPedido::Implementado => "Implementado",
        }
    }
}

/// Documento citado num pedido, com a revisão que valia ao abrir.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocPedido {
    pub id: u64,
    pub pn: String,
    pub nome: String,
    /// revisão liberada quando o pedido foi aberto ("" se ainda não tinha)
    pub revisao_antes: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PedidoMudanca {
    pub numero: String,
    pub titulo: String,
    pub motivo: String,
    pub autor: String,
    pub data: u64,
    pub documentos: Vec<DocPedido>,
    pub estado: EstadoPedido,
    #[serde(default)]
    pub decidido_por: String,
    #[serde(default)]
    pub decisao: String,
    #[serde(default)]
    pub aviso: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemAviso {
    pub id: u64,
    pub pn: String,
    pub nome: String,
    pub revisao_antes: String,
    pub revisao_depois: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvisoMudanca {
    pub numero: String,
    pub pedido: String,
    pub descricao: String,
    pub autor: String,
    pub data: u64,
    pub itens: Vec<ItemAviso>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Registro {
    #[serde(default)]
    pub pedidos: Vec<PedidoMudanca>,
    #[serde(default)]
    pub avisos: Vec<AvisoMudanca>,
    #[serde(default)]
    proximo_pedido: u32,
    #[serde(default)]
    proximo_aviso: u32,
}

impl Registro {
    pub fn pedido(&self, numero: &str) -> Option<&PedidoMudanca> {
        self.pedidos.iter().find(|p| p.numero == numero)
    }
    pub fn aviso(&self, numero: &str) -> Option<&AvisoMudanca> {
        self.avisos.iter().find(|a| a.numero == numero)
    }
    /// Pedidos abertos ou aprovados que citam o documento `id`.
    pub fn pendentes_de(&self, id: u64) -> Vec<&PedidoMudanca> {
        self.pedidos.iter().filter(|p| matches!(p.estado, EstadoPedido::Aberto | EstadoPedido::Aprovado) && p.documentos.iter().any(|d| d.id == id)).collect()
    }
}

fn agora() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl Project {
    /// O registro de mudanças. Arquivo ausente é registro vazio; arquivo ilegível é erro, porque
    /// regravar por cima apagaria o histórico.
    pub fn mudancas(&self) -> PResult<Registro> {
        match std::fs::read_to_string(self.root.join(ARQUIVO_MUDANCAS)) {
            Ok(s) => Ok(serde_json::from_str(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Registro::default()),
            Err(e) => Err(e.into()),
        }
    }

    fn gravar_mudancas(&self, r: &Registro) -> PResult<()> {
        self.pode_gravar()?;
        gravar_atomico(&self.root.join(ARQUIVO_MUDANCAS), serde_json::to_string_pretty(r)?.as_bytes())?;
        Ok(())
    }

    /// Abre um pedido de mudança para os documentos `ids`. Devolve o número.
    pub fn abrir_pedido_mudanca(&mut self, titulo: &str, motivo: &str, ids: &[u64]) -> PResult<String> {
        if titulo.trim().is_empty() || motivo.trim().is_empty() {
            return Err(ProjectError::Recusado("o pedido precisa de título e motivo".into()));
        }
        if ids.is_empty() {
            return Err(ProjectError::Recusado("o pedido precisa citar ao menos um documento".into()));
        }
        let mut documentos = vec![];
        for &id in ids {
            let e = self.entry(id)?;
            if !documentos.iter().any(|d: &DocPedido| d.id == id) {
                documentos.push(DocPedido { id, pn: e.pn.clone(), nome: e.name.clone(), revisao_antes: e.current_revision().map(|r| r.label.clone()).unwrap_or_default() });
            }
        }
        let mut r = self.mudancas()?;
        r.proximo_pedido += 1;
        let numero = format!("ECR-{:04}", r.proximo_pedido);
        r.pedidos.push(PedidoMudanca {
            numero: numero.clone(),
            titulo: titulo.trim().into(),
            motivo: motivo.trim().into(),
            autor: self.autor.clone(),
            data: agora(),
            documentos,
            estado: EstadoPedido::Aberto,
            decidido_por: String::new(),
            decisao: String::new(),
            aviso: None,
        });
        self.gravar_mudancas(&r)?;
        for &id in ids {
            self.registrar(crate::diario::TipoEvento::Mudanca, id, "", &numero, motivo)?;
        }
        Ok(numero)
    }

    /// Aprova ou rejeita um pedido aberto. A decisão precisa de justificativa.
    pub fn decidir_pedido_mudanca(&mut self, numero: &str, aprovar: bool, justificativa: &str) -> PResult<()> {
        if justificativa.trim().is_empty() {
            return Err(ProjectError::Recusado("a decisão precisa de justificativa".into()));
        }
        let mut r = self.mudancas()?;
        let autor = self.autor.clone();
        let p = r.pedidos.iter_mut().find(|p| p.numero == numero).ok_or(ProjectError::NotFound)?;
        if p.estado != EstadoPedido::Aberto {
            return Err(ProjectError::Recusado(format!("{numero} já está {}", p.estado.rotulo().to_lowercase())));
        }
        p.estado = if aprovar { EstadoPedido::Aprovado } else { EstadoPedido::Rejeitado };
        p.decidido_por = autor;
        p.decisao = justificativa.trim().into();
        let ids: Vec<u64> = p.documentos.iter().map(|d| d.id).collect();
        let para = p.estado.rotulo();
        self.gravar_mudancas(&r)?;
        for id in ids {
            self.registrar(crate::diario::TipoEvento::Mudanca, id, numero, para, justificativa)?;
        }
        Ok(())
    }

    /// Emite o aviso de mudança de um pedido aprovado. Recusa se algum documento ainda não ganhou
    /// revisão nova desde a abertura do pedido.
    pub fn emitir_aviso_mudanca(&mut self, pedido: &str, descricao: &str) -> PResult<String> {
        let mut r = self.mudancas()?;
        let p = r.pedido(pedido).ok_or(ProjectError::NotFound)?.clone();
        if p.estado != EstadoPedido::Aprovado {
            return Err(ProjectError::Recusado(format!("{pedido} precisa estar aprovado (está {})", p.estado.rotulo().to_lowercase())));
        }
        let mut itens = vec![];
        let mut faltam = vec![];
        for d in &p.documentos {
            let e = self.entry(d.id)?;
            let depois = e.current_revision().map(|x| x.label.clone()).unwrap_or_default();
            if depois.is_empty() || depois == d.revisao_antes {
                faltam.push(e.pn.clone());
            }
            itens.push(ItemAviso { id: d.id, pn: e.pn.clone(), nome: e.name.clone(), revisao_antes: d.revisao_antes.clone(), revisao_depois: depois });
        }
        if !faltam.is_empty() {
            return Err(ProjectError::Recusado(format!("sem revisão nova liberada: {}", faltam.join(", "))));
        }
        r.proximo_aviso += 1;
        let numero = format!("ECN-{:04}", r.proximo_aviso);
        r.avisos.push(AvisoMudanca { numero: numero.clone(), pedido: pedido.into(), descricao: descricao.trim().into(), autor: self.autor.clone(), data: agora(), itens: itens.clone() });
        if let Some(x) = r.pedidos.iter_mut().find(|x| x.numero == pedido) {
            x.estado = EstadoPedido::Implementado;
            x.aviso = Some(numero.clone());
        }
        self.gravar_mudancas(&r)?;
        for i in &itens {
            self.registrar(crate::diario::TipoEvento::Mudanca, i.id, pedido, &numero, descricao)?;
        }
        Ok(numero)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::Document;

    fn pasta(nome: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("fenkra_mudancas_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn pedido_aprovado_vira_aviso_so_com_revisao_nova() {
        let d = pasta("fluxo");
        let mut p = Project::create(&d, "P").unwrap();
        let a = p.add_document(Document::new("Base")).unwrap();
        let b = p.add_document(Document::new("Tampa")).unwrap();
        p.release(a, "primeira").unwrap();
        // pedido sem motivo é recusado; com motivo, numerado
        assert!(p.abrir_pedido_mudanca("Furo maior", "", &[a]).is_err());
        let ecr = p.abrir_pedido_mudanca("Furo maior", "parafuso M8 não passa", &[a, b, a]).unwrap();
        assert_eq!(ecr, "ECR-0001");
        let reg = p.mudancas().unwrap();
        let ped = reg.pedido(&ecr).unwrap();
        assert_eq!(ped.documentos.len(), 2, "documento repetido entra uma vez");
        assert_eq!(ped.documentos[0].revisao_antes, "A");
        assert_eq!(reg.pendentes_de(b).len(), 1);
        // aviso antes da aprovação: recusado
        assert!(p.emitir_aviso_mudanca(&ecr, "x").is_err());
        assert!(p.decidir_pedido_mudanca(&ecr, true, "").is_err(), "decisão sem justificativa");
        p.decidir_pedido_mudanca(&ecr, true, "custo baixo").unwrap();
        assert!(p.decidir_pedido_mudanca(&ecr, false, "mudei de ideia").is_err(), "decisão não volta");
        // só a base ganhou revisão: o aviso aponta a tampa
        let mut doc = p.load(a).unwrap();
        doc.name = "Base furo 9".into();
        p.save(a, &doc, "furo").unwrap();
        p.release(a, "furo 9").unwrap();
        let erro = p.emitir_aviso_mudanca(&ecr, "furo aumentado").unwrap_err().to_string();
        assert!(erro.contains(&p.entry(b).unwrap().pn), "{erro}");
        p.release(b, "primeira").unwrap();
        let ecn = p.emitir_aviso_mudanca(&ecr, "furo aumentado").unwrap();
        assert_eq!(ecn, "ECN-0001");
        // o registro reaberto do disco tem tudo, com as revisões de antes e depois
        let reg = Project::open(&d).unwrap().mudancas().unwrap();
        let av = reg.aviso(&ecn).unwrap();
        assert_eq!(av.itens.iter().map(|i| (i.revisao_antes.as_str(), i.revisao_depois.as_str())).collect::<Vec<_>>(), vec![("A", "B"), ("", "A")]);
        assert_eq!(reg.pedido(&ecr).unwrap().estado, EstadoPedido::Implementado);
        assert!(reg.pendentes_de(b).is_empty());
        // número novo nunca reaproveita
        assert_eq!(p.abrir_pedido_mudanca("Outro", "motivo", &[b]).unwrap(), "ECR-0002");
        // arquivo ilegível é erro, não registro vazio (regravar apagaria o histórico)
        std::fs::write(d.join(ARQUIVO_MUDANCAS), "{ lixo").unwrap();
        assert!(p.mudancas().is_err());
        assert!(p.abrir_pedido_mudanca("X", "y", &[a]).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
