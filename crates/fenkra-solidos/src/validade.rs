// SPDX-License-Identifier: LGPL-2.1-or-later
//! B413: validade do sólido depois de cada comando.
//!
//! Uma booleana ou uma extrusão pode devolver um sólido que o OpenCASCADE aceita sem erro mas que o
//! `BRepCheck` reprova (casca aberta, face degenerada, volume que não bate com o contorno). O status
//! "ok" não vê isso, e os comandos de depois (volume comum, interferência, massa) passam a mentir
//! (K-10 do kart). Aqui o corpo de destino é conferido logo depois do comando que o mexeu, e o
//! defeito vira aviso no próprio comando: aparece na árvore e vai no `Status` que a API devolve.

use crate::Status;
use fenkra_kernel::Shape;

/// Texto do aviso. Fixo, para a interface e os testes reconhecerem.
pub const AVISO: &str = "sólido inválido depois deste comando: o resultado pode dar volume, massa e interferência errados";

/// O corpo é inválido? Corpo ausente não é problema deste módulo.
pub fn invalido(corpo: Option<&Shape>) -> bool {
    #[cfg(test)]
    if testes::FORCAR.with(|f| f.get()) && corpo.is_some() {
        return true;
    }
    corpo.is_some_and(|s| !s.is_valid())
}

/// Junta o aviso de validade ao status do comando. Erro fica como está; aviso já existente é mantido.
pub fn com_aviso(status: Status, algum_invalido: bool) -> Status {
    if !algum_invalido {
        return status;
    }
    match status {
        Status::Ok => Status::Warning(AVISO.into()),
        Status::Warning(w) if !w.contains(AVISO) => Status::Warning(format!("{w}; {AVISO}")),
        outro => outro,
    }
}

#[cfg(test)]
pub(crate) mod testes {
    use std::cell::Cell;
    thread_local! {
        /// Os testes fingem que todo corpo é inválido, porque os comandos de hoje já barram as causas conhecidas.
        pub static FORCAR: Cell<bool> = const { Cell::new(false) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::*;

    fn caixa() -> Document {
        let mut d = Document::new("v");
        let mut data = SketchData::default();
        let mut al = || d.alloc();
        data.add_rect(&mut al, [0., 0.], [10., 10.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("5".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        d
    }

    #[test]
    fn corpo_invalido_vira_aviso_no_comando_que_o_gerou_e_sai_pela_api() {
        let d = caixa();
        let ext = d.features.last().unwrap().id;
        let sk = d.features[0].id;
        let r = crate::recompute(&d);
        assert_eq!(r.status[&ext], Status::Ok, "corpo bom não ganha aviso");
        testes::FORCAR.with(|f| f.set(true));
        let r = crate::recompute(&d);
        testes::FORCAR.with(|f| f.set(false));
        assert_eq!(r.status[&ext], Status::Warning(AVISO.into()));
        assert_eq!(r.status[&ext].message(), Some(AVISO));
        assert_eq!(r.status[&sk], Status::Ok, "o sketch não mexe em corpo");
        // o corpo continua na saída: o aviso não esconde a peça
        assert_eq!(r.bodies.len(), 1);
    }

    /// A prévia de um comando aberto não paga a conferência; o comando é conferido quando o cálculo
    /// seguinte o reaproveita, sem refazê-lo.
    #[test]
    fn conferencia_adiada_com_comando_aberto_e_feita_ao_reaproveitar() {
        let d = caixa();
        let ext = d.features.last().unwrap().id;
        let mut cache = crate::Cache::default();
        testes::FORCAR.with(|f| f.set(true));
        cache.leve = Some(ext);
        let r = crate::recompute_cached(&d, &mut cache);
        assert_eq!(r.status[&ext], Status::Ok, "com o comando aberto não confere");
        cache.leve = None;
        let r = crate::recompute_cached(&d, &mut cache);
        assert!(cache.reused >= 1, "o comando foi reaproveitado, não refeito");
        assert_eq!(r.status[&ext], Status::Warning(AVISO.into()));
        // e a conferência não se repete a cada cálculo
        let r = crate::recompute_cached(&d, &mut cache);
        testes::FORCAR.with(|f| f.set(false));
        assert_eq!(r.status[&ext], Status::Warning(AVISO.into()), "o aviso fica no cache");
    }

    #[test]
    fn aviso_so_quando_ha_corpo_invalido_e_nao_apaga_erro() {
        assert_eq!(com_aviso(Status::Ok, false), Status::Ok);
        assert_eq!(com_aviso(Status::Ok, true), Status::Warning(AVISO.into()));
        assert_eq!(com_aviso(Status::Error("x".into()), true), Status::Error("x".into()));
        let w = com_aviso(Status::Warning("rosca".into()), true);
        assert!(matches!(&w, Status::Warning(m) if m.starts_with("rosca; ")));
        assert_eq!(com_aviso(w.clone(), true), w);
    }
}
