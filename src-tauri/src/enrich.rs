//! Enriquecimiento no fatal (EXT-002, EXT-003): EPSS por lotes y catálogo
//! KEV cacheado 24 h. Cualquier fallo se degrada a `None`/`false` y deja
//! el tablero intacto.

use crate::cache;
use crate::model::DailyPick;
use crate::nvd;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tauri_plugin_http::reqwest::Client;

pub const EPSS_URL: &str = "https://api.first.org/data/v1/epss";
pub const KEV_URL: &str =
    "https://www.cisa.gov/sites/default/files/feeds/known_exploited_vulnerabilities.json";

const EPSS_BATCH: usize = 100;
const KEV_TTL_HOURS: i64 = 24;

// ---------------------------------------------------------------------------
// EPSS
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct EpssResponse {
    #[serde(default)]
    pub data: Vec<EpssEntry>,
}

#[derive(Debug, Deserialize)]
pub struct EpssEntry {
    #[serde(default)]
    pub cve: String,
    #[serde(default)]
    pub epss: Option<String>,
    #[serde(default)]
    pub percentile: Option<String>,
}

/// Consulta EPSS para un lote de IDs. Los CVE sin score no aparecen.
pub async fn fetch_epss(
    client: &Client,
    base: &str,
    ids: &[String],
) -> Result<HashMap<String, (f32, f32)>, nvd::Error> {
    let url = format!("{base}?cve={}", ids.join(","));
    let body = nvd::get_capped(client, &url, None).await?;
    let resp: EpssResponse = serde_json::from_slice(&body)?;
    Ok(resp
        .data
        .into_iter()
        .filter_map(|e| {
            let score = e.epss.as_deref()?.trim().parse::<f32>().ok()?;
            let percentile = e
                .percentile
                .as_deref()
                .and_then(|s| s.trim().parse::<f32>().ok())
                .unwrap_or(0.0);
            Some((e.cve, (score, percentile)))
        })
        .collect())
}

// ---------------------------------------------------------------------------
// KEV
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KevFile {
    /// Momento en que se descargó; `None` si el archivo viene tal cual de CISA.
    #[serde(default)]
    pub fetched_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub vulnerabilities: Vec<KevEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KevEntry {
    /// La feed de CISA usa `cveID` (D mayúscula); `camelCase` sola daría
    /// `cveId` y el catálogo entero fallaría al deserializar.
    #[serde(rename = "cveID")]
    pub cve_id: String,
    #[serde(default)]
    pub due_date: Option<String>,
}

/// Mapa cveID → fecha límite (ausente = sigue siendo KEV, sin `dueDate`).
pub fn kev_index(file: &KevFile) -> HashMap<String, Option<String>> {
    file.vulnerabilities
        .iter()
        .map(|e| (e.cve_id.clone(), e.due_date.clone()))
        .collect()
}

pub async fn fetch_kev(client: &Client, base: &str) -> Result<KevFile, nvd::Error> {
    let body = nvd::get_capped(client, base, None).await?;
    Ok(serde_json::from_slice(&body)?)
}

/// Devuelve el índice KEV usando la caché de disco si es vigente (< 24 h);
/// si no, descarga y persiste; si la red falla, usa la caché vencida y si
/// no hay, un índice vacío. Nunca falla.
pub async fn load_kev(
    client: &Client,
    base: &str,
    app_data: &Path,
    now: DateTime<Utc>,
) -> HashMap<String, Option<String>> {
    let path = cache::kev_path(app_data);
    let cached = cache::read_json::<KevFile>(&path);
    if let Some(file) = &cached {
        let fresh = file
            .fetched_at
            .is_some_and(|t| now - t < Duration::hours(KEV_TTL_HOURS));
        if fresh {
            return kev_index(file);
        }
    }
    match fetch_kev(client, base).await {
        Ok(mut file) => {
            file.fetched_at = Some(now);
            if let Ok(raw) = serde_json::to_string(&file) {
                if let Err(e) = cache::write_atomic(&path, &raw) {
                    log::warn!("no se pudo guardar kev.json: {e}");
                }
            }
            kev_index(&file)
        }
        Err(e) => {
            // Catálogo de 1.9 MB corrupto o inaccesible: se ignora con warn.
            log::warn!("KEV no disponible: {e}");
            cached.map(|file| kev_index(&file)).unwrap_or_default()
        }
    }
}

// ---------------------------------------------------------------------------
// Enriquecimiento sobre los picks ya normalizados
// ---------------------------------------------------------------------------

/// Rellena `epss`, `epss_percentile`, `kev` y `kev_due_date` en sitio.
/// Los fallos de red solo producen `warn` y campos degradados.
pub async fn enrich(client: &Client, app_data: &Path, picks: &mut [DailyPick]) {
    if picks.is_empty() {
        return;
    }
    let kev = load_kev(client, KEV_URL, app_data, Utc::now()).await;

    let ids: Vec<String> = picks.iter().map(|p| p.id.clone()).collect();
    let mut scores: HashMap<String, (f32, f32)> = HashMap::new();
    for batch in ids.chunks(EPSS_BATCH) {
        match fetch_epss(client, EPSS_URL, batch).await {
            Ok(map) => scores.extend(map),
            Err(e) => log::warn!("EPSS lote de {} CVEs falló: {e}", batch.len()),
        }
    }

    for pick in picks.iter_mut() {
        if let Some((epss, percentile)) = scores.get(&pick.id) {
            pick.epss = Some(*epss);
            pick.epss_percentile = Some(*percentile);
        }
        if let Some(due) = kev.get(&pick.id) {
            pick.kev = true;
            pick.kev_due_date = due.clone();
        }
    }
}
