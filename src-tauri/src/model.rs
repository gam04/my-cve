//! Modelos de dominio (§4.1, §4.2). Espejo exacto de `src/lib/types.ts`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Un CVE normalizado, listo para pintar y rankear.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyPick {
    pub id: String,
    /// ISO-8601 UTC con sufijo `Z`, p. ej. "2026-11-18T16:17:47.560Z".
    pub published: DateTime<Utc>,
    /// 0.0 – 10.0
    pub score: f32,
    /// "3.1" | "4.0" | "3.0" | "2.0"
    pub score_version: String,
    /// "CRITICAL" | "HIGH" | "MEDIUM" | "LOW" | "NONE"
    pub severity: String,
    pub vector: String,
    /// "nvd" | "cna"
    pub score_source: String,
    pub epss: Option<f32>,
    pub epss_percentile: Option<f32>,
    pub kev: bool,
    /// "YYYY-MM-DD", si el catálogo KEV declara fecha límite.
    pub kev_due_date: Option<String>,
    pub cwes: Vec<String>,
    pub vendor: Option<String>,
    pub product: Option<String>,
    pub summary: String,
    /// 1-based tras el ranking; 0 hasta que se rankea.
    pub rank: u32,
}

impl Default for DailyPick {
    fn default() -> Self {
        Self {
            id: String::new(),
            published: DateTime::<Utc>::UNIX_EPOCH,
            score: 0.0,
            score_version: "3.1".into(),
            severity: "NONE".into(),
            vector: String::new(),
            score_source: "cna".into(),
            epss: None,
            epss_percentile: None,
            kev: false,
            kev_due_date: None,
            cwes: Vec::new(),
            vendor: None,
            product: None,
            summary: String::new(),
            rank: 0,
        }
    }
}

/// El tablero del día.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    /// "YYYY-MM-DD" en UTC.
    pub board_date: String,
    /// Instante del último refresco exitoso.
    pub generated_at: DateTime<Utc>,
    pub total_critical_today: u32,
    pub pick: Option<DailyPick>,
    /// Máximo 3, en orden de ranking tras el pick.
    pub runners_up: Vec<DailyPick>,
    pub stale: bool,
    pub fetch_error: Option<String>,
}

/// Posición física de la ventana persistida entre sesiones.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPos {
    pub x: f64,
    pub y: f64,
}

/// Ajustes de la app (§4.2). Persistidos en `<appDataDir>/settings.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub auto_start: bool,
    pub notify_on_top_change: bool,
    /// 15 | 30 | 60; valores fuera de rango se recortan al usarlos.
    pub refresh_minutes: u32,
    pub window_pos: Option<WindowPos>,
    pub pinned: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_start: false,
            notify_on_top_change: true,
            refresh_minutes: 30,
            window_pos: None,
            pinned: true,
        }
    }
}
