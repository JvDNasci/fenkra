// SPDX-License-Identifier: LGPL-2.1-or-later
//! B89: término de cada lado da extrusão, deslocamento do término e início deslocado.
//!
//! Como funciona no kernel: a extrusão vira uma ferramenta (prisma do perfil) que entra no corpo por
//! união, corte ou interseção. Cada lado tem a sua ferramenta: o lado 1 anda na direção da extrusão e o
//! lado 2 na contrária, cada um com o seu término (distância, até plano, até superfície, até o
//! próximo, até o último, passante). O início deslocado move o perfil ao longo da direção antes de
//! extrudar; o deslocamento do término empurra (positivo) ou recua (negativo) a parada em relação
//! ao alvo, na direção da extrusão.

use super::*;

/// Dados de um lado da extrusão.
pub(crate) struct LadoExtrusao<'a> {
    /// perfil já deslocado para o início
    pub face: &'a Shape,
    /// um ponto do perfil já deslocado (onde o lado começa)
    pub origin: V3,
    /// sentido em que este lado anda
    pub dir: V3,
    /// tamanho do perfil (para dimensionar ferramentas longas)
    pub size: f64,
    /// ângulo de saída em graus (só no término cego)
    pub taper: f64,
    pub target: Id,
    pub op: BoolOp,
}

/// Quais campos do painel valem para cada término (B89: o painel mostra só o que serve).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CamposTermino {
    /// comprimento: só em término por dimensão
    pub comprimento: bool,
    /// segundo comprimento (dois sentidos)
    pub comprimento2: bool,
    /// deslocamento do término: passante, até o próximo, até o último, até plano e até face
    pub deslocamento: bool,
    /// precisa de um alvo (plano ou superfície)
    pub alvo: bool,
}

/// Campos que valem para o término `e`.
pub fn campos_do_termino(e: &Extent) -> CamposTermino {
    match e {
        Extent::Blind(_) | Extent::Symmetric(_) => CamposTermino { comprimento: true, comprimento2: false, deslocamento: false, alvo: false },
        Extent::TwoSides(..) => CamposTermino { comprimento: true, comprimento2: true, deslocamento: false, alvo: false },
        Extent::ThroughAll | Extent::UpToNext | Extent::UpToLast => CamposTermino { comprimento: false, comprimento2: false, deslocamento: true, alvo: false },
        Extent::UpTo(_) | Extent::UpToSurface(_) => CamposTermino { comprimento: false, comprimento2: false, deslocamento: true, alvo: true },
    }
}

fn neg(v: V3) -> V3 {
    v.map(|x| -x)
}

impl Ctx<'_> {
    /// Valor (mm) de um campo opcional da extrusão: vazio vale 0.
    pub(crate) fn mm_opcional(&self, texto: &str, nome: &str) -> Result<f64, String> {
        if texto.trim().is_empty() {
            Ok(0.)
        } else {
            ev(texto, &self.vars, nome)
        }
    }

    /// Ferramenta de um lado da extrusão e o papel de cada face (só nas extrusões simples).
    pub(crate) fn tool_lado(&self, l: &LadoExtrusao, extent: &Extent, offset: f64) -> Result<(Shape, Option<Vec<i32>>), String> {
        if offset.abs() > 1e-12 && !campos_do_termino(extent).deslocamento {
            return Err(format!("o deslocamento do término não vale com o término '{}': só em passante, até o próximo, até o último, até plano e até superfície", extent_name(extent)));
        }
        let mut roles = None;
        let tool = self.tool_lado_sem_papel(l, extent, offset, &mut roles)?;
        // B15: término que o kernel não rotula (até plano, até superfície, até o próximo, saída):
        // papéis pela geometria; na dúvida, sem papéis
        if roles.is_none() {
            roles = crate::papeis::por_perfil(&tool, l.face, crate::papeis::Fim::Resto).filter(|r| r.len() == tool.face_count() as usize);
        }
        Ok((tool, roles))
    }

    fn tool_lado_sem_papel(&self, l: &LadoExtrusao, extent: &Extent, offset: f64, roles: &mut Option<Vec<i32>>) -> Result<Shape, String> {
        let k = |e: KernelError| e.0;
        let LadoExtrusao { face, origin, dir, size, taper, target, op } = *l;
        let tool = match extent {
            Extent::UpTo(p) => {
                let alvo = self.plane(p, 0)?;
                let n = alvo.normal;
                let den = dot(dir, n);
                if den.abs() < 1e-9 {
                    return Err("o plano de término é paralelo à direção da extrusão".into());
                }
                // o deslocamento anda na direção da extrusão: o plano de parada sobe ou desce com ele
                let t0 = dot(sub(alvo.origin, origin), n) / den;
                if t0 <= 1e-9 {
                    return Err("o plano de término está do outro lado — use 'inverter sentido'".into());
                }
                let t = t0 + offset;
                if t <= 1e-9 {
                    return Err("o deslocamento recua o término para antes do início da extrusão".into());
                }
                let parada = [0, 1, 2].map(|i| origin[i] + dir[i] * t);
                face.extrude_until(dir, t * 4. + size * 10. + 100., parada, n).map_err(k)?
            }
            Extent::UpToLast => {
                let len = self.last_dist(target, origin, dir)? + offset;
                if len <= 1e-9 {
                    return Err("o deslocamento recua o término para antes do início da extrusão".into());
                }
                let (t, r) = face.extrude_roles(dir, len, 0.).map_err(k)?;
                *roles = Some(r);
                t
            }
            Extent::UpToNext if offset.abs() > 1e-12 => {
                let body = self.body(target).ok_or("'até o próximo' precisa de um corpo no coletor")?;
                return self.ate_o_proximo_deslocado(l, body, offset);
            }
            Extent::UpToNext if op == BoolOp::Add => {
                let body = self.body(target).ok_or("'até o próximo' precisa de um corpo no coletor")?;
                face.extrude_to_next(dir, self.through(target)?, body).map_err(k)?
            }
            Extent::ThroughAll => {
                let len = self.through(target)? + offset;
                if len <= 1e-9 {
                    return Err("o deslocamento recua o término para antes do início da extrusão".into());
                }
                let (t, r) = face.extrude_roles(dir, len, 0.).map_err(k)?;
                *roles = Some(r);
                t
            }
            _ => {
                if let Some((cut, ext)) = self.end_cutter(extent, target)? {
                    let cut = if offset.abs() > 1e-12 { self.deslocar_corte(cut, dir, offset)? } else { cut };
                    let start = face.surface_near(origin).map_err(k)?.0;
                    let len = self.reach(&cut, origin, target) + size;
                    face.extrude(dir, len, 0.).map_err(k)?.clip_until(&cut, start, ext).map_err(k)?
                } else if taper.abs() > 1e-9 {
                    let Extent::Blind(d) = extent else {
                        return Err("o ângulo de saída vale só com término 'cego'".into());
                    };
                    face.extrude_taper(dir, ev(d, &self.vars, "distância")?, taper).map_err(k)?
                } else {
                    let (front, back) = self.extent(extent, target)?;
                    let (t, r) = face.extrude_roles(dir, front, back).map_err(k)?;
                    *roles = Some(r);
                    t
                }
            }
        };
        Ok(tool)
    }

    /// Superfície de parada deslocada `offset` mm na direção da extrusão (positivo avança além do alvo).
    fn deslocar_corte(&self, cut: Shape, dir: V3, offset: f64) -> Result<Shape, String> {
        let k = |e: KernelError| e.0;
        let centro = |s: &Shape| s.bbox().map(|(a, b)| geo3d::lerp(a, b, 0.5));
        let antes = centro(&cut).ok_or("a superfície de término não tem dimensões")?;
        let mais = cut.offset_surface(offset.abs()).map_err(k)?;
        let depois = centro(&mais).ok_or("a superfície deslocada não tem dimensões")?;
        let avancou = dot(sub(depois, antes), dir);
        // a normal da superfície pode apontar para qualquer lado: confere para onde o deslocamento foi
        if (avancou >= 0.) == (offset >= 0.) {
            Ok(mais)
        } else {
            cut.offset_surface(-offset.abs()).map_err(k)
        }
    }

    /// "Até o próximo" com deslocamento: só quando a próxima parede é plana e perpendicular à direção,
    /// onde a parada é um plano exato; parede inclinada ou curva pede "até superfície".
    fn ate_o_proximo_deslocado(&self, l: &LadoExtrusao, body: &Shape, offset: f64) -> Result<Shape, String> {
        let k = |e: KernelError| e.0;
        let (lo, hi) = l.face.bbox().ok_or("o perfil não tem dimensões")?;
        let centro = geo3d::lerp(lo, hi, 0.5);
        let Some((hit, t)) = body.ray_hit(centro, l.dir).map_err(k)? else {
            return Err("não há parede à frente do perfil na direção da extrusão".into());
        };
        let (_, n) = body.surface_near(hit).map_err(k)?;
        if dot(n, l.dir).abs() < 1.0 - 1e-6 {
            return Err("a próxima parede é inclinada ou curva: o deslocamento do término vale só com parede perpendicular — use 'até superfície'".into());
        }
        let ate = t + offset - dot(sub(centro, l.origin), l.dir);
        if ate <= 1e-9 {
            return Err("o deslocamento recua o término para antes do início da extrusão".into());
        }
        let parada = [0, 1, 2].map(|i| l.origin[i] + l.dir[i] * ate);
        l.face.extrude_until(l.dir, ate * 4. + l.size * 10. + 100., parada, l.dir).map_err(k)
    }

    /// Extrusão completa: início deslocado, lado 1 e, se houver, lado 2 com término próprio.
    /// Devolve a ferramenta e o papel das faces (só quando é uma extrusão simples de um lado).
    pub(crate) fn tool_extrusao(&self, e: &Extrude, face: &Shape, frame: &Frame, dir: V3, size: f64, taper: f64, target: Id) -> Result<(Shape, Option<Vec<i32>>), String> {
        let k = |e: KernelError| e.0;
        let start = self.mm_opcional(&e.ends.start, "início deslocado")?;
        let off1 = self.mm_opcional(&e.ends.offset, "deslocamento do término")?;
        let off2 = self.mm_opcional(&e.ends.offset2, "deslocamento do término 2")?;
        let face_i;
        let (face, origin) = if start.abs() > 1e-12 {
            let mv = geo3d::mul(dir, start);
            face_i = face.transformed(Xform::Translate(mv)).map_err(k)?;
            (&face_i, geo3d::add(frame.origin, mv))
        } else {
            (face, frame.origin)
        };
        let lado1 = LadoExtrusao { face, origin, dir, size, taper, target, op: e.op };
        let (mut tool, mut roles) = self.tool_lado(&lado1, &e.extent, off1)?;
        if let Some(e2) = &e.ends.extent2 {
            if matches!(e.extent, Extent::Symmetric(_) | Extent::TwoSides(..)) || matches!(e2, Extent::Symmetric(_) | Extent::TwoSides(..)) {
                return Err("o término do lado 2 vale com um término de um só lado em cada lado: troque 'plano médio' e 'dois sentidos' por 'cego', 'até plano' etc.".into());
            }
            if taper.abs() > 1e-9 {
                return Err("o ângulo de saída vale só com um único término cego".into());
            }
            let lado2 = LadoExtrusao { face, origin, dir: neg(dir), size, taper: 0., target, op: e.op };
            let (t2, r2) = self.tool_lado(&lado2, e2, off2)?;
            // B15: o composto lista as faces do lado 1 e depois as do lado 2
            let n1 = tool.face_count() as usize;
            tool = tool.compound(&t2);
            roles = match (roles, r2) {
                (Some(mut a), Some(b)) if a.len() == n1 && a.len() + b.len() == tool.face_count() as usize => {
                    a.extend(b);
                    Some(a)
                }
                _ => None,
            };
        }
        Ok((tool, roles))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn painel_mostra_so_os_campos_do_termino() {
        let c = campos_do_termino(&Extent::Blind("5".into()));
        assert!(c.comprimento && !c.deslocamento && !c.alvo);
        let c = campos_do_termino(&Extent::TwoSides("1".into(), "2".into()));
        assert!(c.comprimento && c.comprimento2 && !c.deslocamento);
        for e in [Extent::ThroughAll, Extent::UpToNext, Extent::UpToLast] {
            let c = campos_do_termino(&e);
            assert!(!c.comprimento && c.deslocamento && !c.alvo, "{e:?}");
        }
        let c = campos_do_termino(&Extent::UpTo(PlaneRef::Origin(OriginPlane::XY)));
        assert!(!c.comprimento && c.deslocamento && c.alvo);
    }
}
