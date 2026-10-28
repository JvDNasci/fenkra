//! Leitura de malhas STL (texto ou binário) e OBJ como lista de triângulos (9 números cada).

/// Lê o arquivo e devolve os triângulos achatados (x0 y0 z0 x1 … z2 por triângulo).
pub fn read(path: &str) -> Result<Vec<f64>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("não foi possível ler '{path}': {e}"))?;
    let low = path.to_lowercase();
    let tris = if low.ends_with(".obj") { obj(&String::from_utf8_lossy(&bytes))? } else { stl(&bytes)? };
    if tris.is_empty() {
        return Err("a malha não tem triângulos".into());
    }
    if tris.len() / 9 > 200_000 {
        return Err(format!("malha grande demais ({} triângulos; máximo 200 mil) — simplifique antes", tris.len() / 9));
    }
    Ok(tris)
}

fn stl(b: &[u8]) -> Result<Vec<f64>, String> {
    // binário: 80 de cabeçalho + contagem; confere o tamanho para não confundir com texto "solid"
    if b.len() >= 84 {
        let n = u32::from_le_bytes([b[80], b[81], b[82], b[83]]) as usize;
        if b.len() == 84 + n * 50 {
            let mut out = Vec::with_capacity(n * 9);
            for i in 0..n {
                let base = 84 + i * 50 + 12;
                for k in 0..9 {
                    let o = base + k * 4;
                    out.push(f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) as f64);
                }
            }
            return Ok(out);
        }
    }
    let t = String::from_utf8_lossy(b);
    let mut out = vec![];
    for line in t.lines() {
        let mut w = line.split_whitespace();
        if w.next() == Some("vertex") {
            for _ in 0..3 {
                out.push(w.next().and_then(|x| x.parse().ok()).ok_or("STL: vértice inválido")?);
            }
        }
    }
    if out.len() % 9 != 0 {
        return Err("STL: número de vértices não é múltiplo de 3".into());
    }
    Ok(out)
}

fn obj(t: &str) -> Result<Vec<f64>, String> {
    let mut v: Vec<[f64; 3]> = vec![];
    let mut out = vec![];
    for line in t.lines() {
        let mut w = line.split_whitespace();
        match w.next() {
            Some("v") => {
                let c: Vec<f64> = w.take(3).filter_map(|x| x.parse().ok()).collect();
                if c.len() == 3 {
                    v.push([c[0], c[1], c[2]]);
                }
            }
            Some("f") => {
                // índices 1-based, negativos contam do fim; polígonos viram leque de triângulos
                let idx: Vec<usize> = w
                    .filter_map(|x| x.split('/').next()?.parse::<i64>().ok())
                    .map(|i| if i < 0 { (v.len() as i64 + i) as usize } else { (i - 1) as usize })
                    .collect();
                for k in 1..idx.len().saturating_sub(1) {
                    for &j in &[idx[0], idx[k], idx[k + 1]] {
                        let p = v.get(j).ok_or("OBJ: face com vértice inexistente")?;
                        out.extend_from_slice(p);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn obj_e_stl_texto() {
        let o = super::obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n").unwrap();
        assert_eq!(o.len(), 18);
        let s = super::stl(b"solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid").unwrap();
        assert_eq!(s.len(), 9);
    }
}
