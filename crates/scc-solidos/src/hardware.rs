//! Tabela do furo inteligente (RF-SOL-008) para roscas métricas ISO grossas.
//!
//! **Dados preliminares** para o protótipo: valores usuais de engenharia (passo, broca de rosca,
//! furos de passagem fino/normal/folgado, rebaixo para parafuso Allen, escareado 90°).
//! Conferir e migrar para a biblioteca pública (RF-BIB-014) antes de qualquer uso real.

#[derive(Clone, Copy, Debug)]
pub struct MetricSize {
    pub name: &'static str,
    pub pitch: f64,
    pub tap_drill: f64,
    pub clearance: [f64; 3],
    pub cbore_diameter: f64,
    pub cbore_depth: f64,
    pub csink_diameter: f64,
}

pub const METRIC: &[MetricSize] = &[
    MetricSize { name: "M3", pitch: 0.5, tap_drill: 2.5, clearance: [3.2, 3.4, 3.6], cbore_diameter: 6.5, cbore_depth: 3.4, csink_diameter: 6.9 },
    MetricSize { name: "M4", pitch: 0.7, tap_drill: 3.3, clearance: [4.3, 4.5, 4.8], cbore_diameter: 8.0, cbore_depth: 4.4, csink_diameter: 9.2 },
    MetricSize { name: "M5", pitch: 0.8, tap_drill: 4.2, clearance: [5.3, 5.5, 5.8], cbore_diameter: 10.0, cbore_depth: 5.4, csink_diameter: 11.5 },
    MetricSize { name: "M6", pitch: 1.0, tap_drill: 5.0, clearance: [6.4, 6.6, 7.0], cbore_diameter: 11.0, cbore_depth: 6.4, csink_diameter: 13.7 },
    MetricSize { name: "M8", pitch: 1.25, tap_drill: 6.8, clearance: [8.4, 9.0, 10.0], cbore_diameter: 15.0, cbore_depth: 8.6, csink_diameter: 18.3 },
    MetricSize { name: "M10", pitch: 1.5, tap_drill: 8.5, clearance: [10.5, 11.0, 12.0], cbore_diameter: 18.0, cbore_depth: 10.6, csink_diameter: 22.7 },
    MetricSize { name: "M12", pitch: 1.75, tap_drill: 10.2, clearance: [13.0, 13.5, 14.5], cbore_diameter: 20.0, cbore_depth: 12.6, csink_diameter: 27.2 },
    MetricSize { name: "M16", pitch: 2.0, tap_drill: 14.0, clearance: [17.0, 17.5, 18.5], cbore_diameter: 26.0, cbore_depth: 16.6, csink_diameter: 36.0 },
    MetricSize { name: "M20", pitch: 2.5, tap_drill: 17.5, clearance: [21.0, 22.0, 24.0], cbore_diameter: 33.0, cbore_depth: 20.6, csink_diameter: 44.0 },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoleUse {
    /// Furo roscado (broca de rosca + rosca cosmética).
    Roscado,
    /// Furo de passagem: 0 = fino, 1 = normal, 2 = folgado.
    Passagem(u8),
}

pub fn metric(name: &str) -> Option<&'static MetricSize> {
    METRIC.iter().find(|m| m.name == name)
}

// ---------------------------------------------------------------- roscas por norma

/// Rosca: norma (grupo), designação, diâmetro maior (mm), passo (mm), broca de rosca (mm), ângulo do filete (°).
///
/// **Dados preliminares** (valores usuais de tabelas de engenharia) — conferir antes de uso real
/// e migrar para a biblioteca pública (RF-BIB-014).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThreadSpec {
    pub standard: &'static str,
    pub name: &'static str,
    pub major: f64,
    pub pitch: f64,
    pub tap_drill: f64,
    pub angle: f64,
}

const fn t(standard: &'static str, name: &'static str, major: f64, pitch: f64, tap_drill: f64, angle: f64) -> ThreadSpec {
    ThreadSpec { standard, name, major, pitch, tap_drill, angle }
}

pub const STANDARDS: [&str; 7] = [
    "ISO métrica grossa",
    "ISO métrica fina",
    "UNC (unificada grossa)",
    "UNF (unificada fina)",
    "BSP paralela (G)",
    "NPT (cônica)",
    "Trapezoidal (Tr)",
];

pub const THREADS: &[ThreadSpec] = &[
    t("ISO métrica grossa", "M2x0.4", 2.0, 0.4, 1.6, 60.),
    t("ISO métrica grossa", "M2.5x0.45", 2.5, 0.45, 2.05, 60.),
    t("ISO métrica grossa", "M3x0.5", 3.0, 0.5, 2.5, 60.),
    t("ISO métrica grossa", "M4x0.7", 4.0, 0.7, 3.3, 60.),
    t("ISO métrica grossa", "M5x0.8", 5.0, 0.8, 4.2, 60.),
    t("ISO métrica grossa", "M6x1", 6.0, 1.0, 5.0, 60.),
    t("ISO métrica grossa", "M8x1.25", 8.0, 1.25, 6.8, 60.),
    t("ISO métrica grossa", "M10x1.5", 10.0, 1.5, 8.5, 60.),
    t("ISO métrica grossa", "M12x1.75", 12.0, 1.75, 10.2, 60.),
    t("ISO métrica grossa", "M14x2", 14.0, 2.0, 12.0, 60.),
    t("ISO métrica grossa", "M16x2", 16.0, 2.0, 14.0, 60.),
    t("ISO métrica grossa", "M20x2.5", 20.0, 2.5, 17.5, 60.),
    t("ISO métrica grossa", "M24x3", 24.0, 3.0, 21.0, 60.),
    t("ISO métrica grossa", "M30x3.5", 30.0, 3.5, 26.5, 60.),
    t("ISO métrica fina", "M8x1", 8.0, 1.0, 7.0, 60.),
    t("ISO métrica fina", "M10x1", 10.0, 1.0, 9.0, 60.),
    t("ISO métrica fina", "M10x1.25", 10.0, 1.25, 8.8, 60.),
    t("ISO métrica fina", "M12x1.25", 12.0, 1.25, 10.8, 60.),
    t("ISO métrica fina", "M12x1.5", 12.0, 1.5, 10.5, 60.),
    t("ISO métrica fina", "M16x1.5", 16.0, 1.5, 14.5, 60.),
    t("ISO métrica fina", "M20x1.5", 20.0, 1.5, 18.5, 60.),
    t("UNC (unificada grossa)", "#4-40 UNC", 2.845, 0.635, 2.26, 60.),
    t("UNC (unificada grossa)", "#6-32 UNC", 3.505, 0.794, 2.69, 60.),
    t("UNC (unificada grossa)", "#8-32 UNC", 4.166, 0.794, 3.45, 60.),
    t("UNC (unificada grossa)", "#10-24 UNC", 4.826, 1.058, 3.80, 60.),
    t("UNC (unificada grossa)", "1/4-20 UNC", 6.35, 1.27, 5.1, 60.),
    t("UNC (unificada grossa)", "5/16-18 UNC", 7.938, 1.411, 6.6, 60.),
    t("UNC (unificada grossa)", "3/8-16 UNC", 9.525, 1.5875, 8.0, 60.),
    t("UNC (unificada grossa)", "1/2-13 UNC", 12.7, 1.954, 10.8, 60.),
    t("UNC (unificada grossa)", "5/8-11 UNC", 15.875, 2.309, 13.5, 60.),
    t("UNC (unificada grossa)", "3/4-10 UNC", 19.05, 2.54, 16.5, 60.),
    t("UNF (unificada fina)", "#10-32 UNF", 4.826, 0.794, 4.1, 60.),
    t("UNF (unificada fina)", "1/4-28 UNF", 6.35, 0.907, 5.5, 60.),
    t("UNF (unificada fina)", "5/16-24 UNF", 7.938, 1.058, 6.9, 60.),
    t("UNF (unificada fina)", "3/8-24 UNF", 9.525, 1.058, 8.5, 60.),
    t("UNF (unificada fina)", "1/2-20 UNF", 12.7, 1.27, 11.5, 60.),
    t("BSP paralela (G)", "G1/8", 9.728, 0.907, 8.8, 55.),
    t("BSP paralela (G)", "G1/4", 13.157, 1.337, 11.8, 55.),
    t("BSP paralela (G)", "G3/8", 16.662, 1.337, 15.25, 55.),
    t("BSP paralela (G)", "G1/2", 20.955, 1.814, 19.0, 55.),
    t("BSP paralela (G)", "G3/4", 26.441, 1.814, 24.5, 55.),
    t("BSP paralela (G)", "G1", 33.249, 2.309, 30.75, 55.),
    t("NPT (cônica)", "1/8 NPT", 10.287, 0.941, 8.7, 60.),
    t("NPT (cônica)", "1/4 NPT", 13.716, 1.411, 11.1, 60.),
    t("NPT (cônica)", "3/8 NPT", 17.145, 1.411, 14.3, 60.),
    t("NPT (cônica)", "1/2 NPT", 21.336, 1.814, 17.9, 60.),
    t("NPT (cônica)", "3/4 NPT", 26.67, 1.814, 23.0, 60.),
    t("Trapezoidal (Tr)", "Tr10x2", 10.0, 2.0, 8.0, 30.),
    t("Trapezoidal (Tr)", "Tr12x3", 12.0, 3.0, 9.0, 30.),
    t("Trapezoidal (Tr)", "Tr16x4", 16.0, 4.0, 12.0, 30.),
    t("Trapezoidal (Tr)", "Tr20x4", 20.0, 4.0, 16.0, 30.),
    t("Trapezoidal (Tr)", "Tr24x5", 24.0, 5.0, 19.0, 30.),
    t("Trapezoidal (Tr)", "Tr30x6", 30.0, 6.0, 24.0, 30.),
];

/// Rosca pela designação (aceita também "M6x1.0" e similares).
pub fn thread(name: &str) -> Option<&'static ThreadSpec> {
    let n = name.trim();
    THREADS.iter().find(|t| t.name.eq_ignore_ascii_case(n)).or_else(|| {
        // "M6x1.0" → "M6x1"
        let norm = n.trim_end_matches('0').trim_end_matches('.');
        THREADS.iter().find(|t| t.name.eq_ignore_ascii_case(norm))
    }).or_else(|| {
        // só o tamanho ("M6", "1/4-20" sem passo): primeira da tabela com esse prefixo (série grossa)
        THREADS.iter().find(|t| t.name.to_ascii_lowercase().starts_with(&format!("{}x", n.to_ascii_lowercase())))
    })
}

pub fn threads_of(standard: &str) -> impl Iterator<Item = &'static ThreadSpec> + '_ {
    THREADS.iter().filter(move |t| t.standard == standard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabela_de_roscas() {
        for s in STANDARDS {
            assert!(threads_of(s).count() >= 5, "{s}");
        }
        assert_eq!(thread("M6x1").unwrap().tap_drill, 5.0);
        assert_eq!(thread("1/4-20 UNC").unwrap().pitch, 1.27);
        for t in THREADS {
            assert!(t.tap_drill < t.major && t.pitch > 0.);
        }
    }
}
