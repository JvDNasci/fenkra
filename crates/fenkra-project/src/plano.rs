//! Plano de salvamento: o que o Salvar tudo vai gravar e por quê (B56).
//!
//! A interface junta os documentos abertos (peça, montagem, desenho) em [`Candidato`]s, o plano os
//! ordena e valida, e só depois de confirmado grava. Regras:
//! - cada arquivo aparece com **um** motivo ([`Motivo`]); quem não tem motivo não entra no plano;
//! - só leitura e conflito nunca são gravados (aparecem para o usuário saber por que ficaram de fora);
//! - arquivo novo exige nome e pasta antes de gravar;
//! - dependências gravam primeiro, derivados desatualizados por último, para que cada arquivo já
//!   encontre no disco o que ele referencia;
//! - se qualquer validação falha, **nada** é gravado; cancelar é simplesmente não executar o plano.

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motivo {
    /// nunca foi salvo
    Novo,
    /// alterado desde que foi aberto ou salvo
    Editado,
    /// não foi editado, mas outro arquivo a salvar depende dele
    Dependencia,
    /// gerado de outro arquivo (desenho, planificação) que mudou
    DerivadoDesatualizado,
    /// aberto sem permissão de edição (revisão congelada, formato futuro, arquivo do sistema)
    SoLeitura,
    /// alterado por fora desde que foi aberto
    Conflito,
}

impl Motivo {
    /// Texto para a interface (passe por `tr` ao exibir).
    pub fn rotulo(self) -> &'static str {
        match self {
            Motivo::Novo => "Novo",
            Motivo::Editado => "Editado",
            Motivo::Dependencia => "Dependência",
            Motivo::DerivadoDesatualizado => "Derivado desatualizado",
            Motivo::SoLeitura => "Só leitura",
            Motivo::Conflito => "Conflito",
        }
    }
    /// Este motivo impede a gravação.
    pub fn bloqueia(self) -> bool {
        matches!(self, Motivo::SoLeitura | Motivo::Conflito)
    }
    fn ordem(self) -> u8 {
        match self {
            Motivo::Dependencia => 0,
            Motivo::Novo | Motivo::Editado => 1,
            Motivo::DerivadoDesatualizado => 2,
            Motivo::SoLeitura | Motivo::Conflito => 3,
        }
    }
}

/// Situação de um documento aberto, como a interface a conhece.
#[derive(Clone, Debug, Default)]
pub struct Candidato {
    /// chave do chamador (índice do documento aberto)
    pub id: u64,
    pub nome: String,
    /// pasta de destino (obrigatória para os novos)
    pub pasta: Option<PathBuf>,
    pub novo: bool,
    pub editado: bool,
    pub dependencia: bool,
    pub derivado_desatualizado: bool,
    pub so_leitura: bool,
    pub conflito: bool,
}

impl Candidato {
    /// O motivo único do arquivo, ou `None` se não há o que salvar. Conflito e só leitura vencem
    /// os demais: o que não pode ser gravado precisa ser dito, não escondido.
    pub fn motivo(&self) -> Option<Motivo> {
        if self.conflito {
            Some(Motivo::Conflito)
        } else if self.so_leitura && (self.novo || self.editado) {
            Some(Motivo::SoLeitura)
        } else if self.novo {
            Some(Motivo::Novo)
        } else if self.editado {
            Some(Motivo::Editado)
        } else if self.derivado_desatualizado {
            Some(Motivo::DerivadoDesatualizado)
        } else if self.dependencia {
            Some(Motivo::Dependencia)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct ItemPlano {
    pub id: u64,
    pub nome: String,
    pub pasta: Option<PathBuf>,
    pub motivo: Motivo,
    /// o usuário pode desmarcar; itens bloqueados nunca vão para a gravação
    pub marcado: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Plano {
    pub itens: Vec<ItemPlano>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProblemaPlano {
    pub id: u64,
    pub mensagem: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Resultado {
    pub gravados: Vec<u64>,
    /// (id, mensagem do erro); a gravação segue nos demais
    pub falhas: Vec<(u64, String)>,
}

impl Plano {
    /// Monta o plano ordenado. O documento `ativo` (Ctrl+S) fica na frente dentro do seu grupo.
    pub fn montar(candidatos: &[Candidato], ativo: Option<u64>) -> Plano {
        let mut itens: Vec<ItemPlano> = candidatos
            .iter()
            .filter_map(|c| {
                c.motivo().map(|m| ItemPlano { id: c.id, nome: c.nome.clone(), pasta: c.pasta.clone(), motivo: m, marcado: !m.bloqueia() })
            })
            .collect();
        itens.sort_by_key(|i| (i.motivo.ordem(), Some(i.id) != ativo, i.id));
        Plano { itens }
    }

    pub fn marcar(&mut self, id: u64, marcado: bool) {
        if let Some(i) = self.itens.iter_mut().find(|i| i.id == id && !i.motivo.bloqueia()) {
            i.marcado = marcado;
        }
    }

    /// Itens que serão gravados, na ordem.
    pub fn a_gravar(&self) -> impl Iterator<Item = &ItemPlano> {
        self.itens.iter().filter(|i| i.marcado && !i.motivo.bloqueia())
    }

    /// Problemas que impedem confirmar: arquivo novo sem nome ou sem pasta.
    pub fn problemas(&self) -> Vec<ProblemaPlano> {
        let mut v = vec![];
        for i in self.a_gravar().filter(|i| i.motivo == Motivo::Novo) {
            if i.nome.trim().is_empty() {
                v.push(ProblemaPlano { id: i.id, mensagem: "Dê um nome ao arquivo novo" });
            }
            if i.pasta.is_none() {
                v.push(ProblemaPlano { id: i.id, mensagem: "Escolha a pasta do arquivo novo" });
            }
        }
        v
    }

    /// Grava com `gravar`. Com qualquer problema de validação não grava nada. Uma falha de gravação
    /// não interrompe os demais itens; vai para [`Resultado::falhas`].
    pub fn executar(&self, mut gravar: impl FnMut(&ItemPlano) -> Result<(), String>) -> Result<Resultado, Vec<ProblemaPlano>> {
        let p = self.problemas();
        if !p.is_empty() {
            return Err(p);
        }
        let mut r = Resultado::default();
        for i in self.a_gravar() {
            match gravar(i) {
                Ok(()) => r.gravados.push(i.id),
                Err(e) => r.falhas.push((i.id, e)),
            }
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(id: u64, nome: &str) -> Candidato {
        Candidato { id, nome: nome.into(), pasta: Some(PathBuf::from("/p")), ..Default::default() }
    }

    #[test]
    fn cada_arquivo_tem_um_motivo_e_sem_motivo_fica_de_fora() {
        let limpo = c(1, "limpo");
        assert_eq!(limpo.motivo(), None);
        let mut x = c(2, "x");
        x.novo = true;
        x.editado = true;
        assert_eq!(x.motivo(), Some(Motivo::Novo));
        x.conflito = true;
        assert_eq!(x.motivo(), Some(Motivo::Conflito), "conflito vence");
        let mut ro = c(3, "ro");
        ro.editado = true;
        ro.so_leitura = true;
        assert_eq!(ro.motivo(), Some(Motivo::SoLeitura));
        let mut ro_limpo = c(4, "ro2");
        ro_limpo.so_leitura = true;
        assert_eq!(ro_limpo.motivo(), None, "só leitura sem alteração não polui o plano");
    }

    #[test]
    fn ordem_dependencias_primeiro_derivados_por_ultimo_ativo_na_frente() {
        let mut dep = c(1, "dep");
        dep.dependencia = true;
        let mut der = c(2, "desenho");
        der.derivado_desatualizado = true;
        let mut a = c(3, "a");
        a.editado = true;
        let mut b = c(4, "b");
        b.editado = true;
        let p = Plano::montar(&[der, b, a, dep], Some(4));
        let ids: Vec<u64> = p.itens.iter().map(|i| i.id).collect();
        assert_eq!(ids, vec![1, 4, 3, 2]);
    }

    #[test]
    fn bloqueados_aparecem_mas_nunca_sao_gravados() {
        let mut ro = c(1, "ro");
        ro.editado = true;
        ro.so_leitura = true;
        let mut cf = c(2, "cf");
        cf.conflito = true;
        let mut ok = c(3, "ok");
        ok.editado = true;
        let mut p = Plano::montar(&[ro, cf, ok], None);
        assert_eq!(p.itens.len(), 3);
        p.marcar(1, true);
        p.marcar(2, true);
        let mut gravados = vec![];
        p.executar(|i| {
            gravados.push(i.id);
            Ok(())
        })
        .unwrap();
        assert_eq!(gravados, vec![3]);
    }

    #[test]
    fn novo_sem_nome_ou_pasta_nao_grava_nada() {
        let mut n = c(1, "  ");
        n.novo = true;
        n.pasta = None;
        let mut e = c(2, "editado");
        e.editado = true;
        let p = Plano::montar(&[n, e], None);
        let mut chamadas = 0;
        let r = p.executar(|_| {
            chamadas += 1;
            Ok(())
        });
        let erros = r.unwrap_err();
        assert_eq!(erros.len(), 2);
        assert_eq!(chamadas, 0, "validação falhou: nem o item válido pode ser gravado");
    }

    #[test]
    fn desmarcar_tira_do_plano_e_falha_de_um_nao_para_os_outros() {
        let mut a = c(1, "a");
        a.editado = true;
        let mut b = c(2, "b");
        b.editado = true;
        let mut d = c(3, "d");
        d.editado = true;
        let mut p = Plano::montar(&[a, b, d], None);
        p.marcar(3, false);
        let r = p
            .executar(|i| if i.id == 1 { Err("disco cheio".into()) } else { Ok(()) })
            .unwrap();
        assert_eq!(r.gravados, vec![2]);
        assert_eq!(r.falhas, vec![(1, "disco cheio".to_string())]);
    }
}
