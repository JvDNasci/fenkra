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
