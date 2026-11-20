//! Cliente NVD CVE API 2.0 (§4.4) + normalización a `DailyPick`.
//!
//! Reglas de normalización: métrica v3.1 → v4.0 → v3.0 → v2.0 prefiriendo
//! `type == "Primary"`; vendor/product desde `affected[]` (nunca CPE, porque
//! `configurations` suele venir `null`); saneo de mojibake; dedupe por `id`.

use crate::model::DailyPick;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashSet;
use tauri_plugin_http::reqwest::header::ACCEPT;
use tauri_plugin_http::reqwest::Client;

pub const NVD_URL: &str = "https://services.nvd.nist.gov/rest/json/cves/2.0";
/// SEC-003: techo de 8 MB por respuesta.
pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const PAGE_SIZE: u32 = 2000;
const MAX_PAGES: u32 = 100;
const TIMEOUT_SECS: u64 = 15;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP {0}")]
    HttpStatus(u16),
    #[error("respuesta excede el techo de 8 MB")]
    TooLarge,
    #[error("error de red: {0}")]
    Http(#[from] tauri_plugin_http::reqwest::Error),
    #[error("JSON inválido: {0}")]
    Json(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Tipos crudos de NVD (solo lectura, deserialización estricta y tolerante)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NvdResponse {
    #[serde(default)]
    pub total_results: u32,
    #[serde(default)]
    pub vulnerabilities: Vec<Vulnerability>,
}

#[derive(Debug, Deserialize)]
pub struct Vulnerability {
    pub cve: Cve,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cve {
    pub id: String,
    #[serde(default)]
    pub published: Option<String>,
    #[serde(default)]
    pub descriptions: Vec<Description>,
    #[serde(default)]
    pub metrics: Metrics,
    #[serde(default)]
    pub weaknesses: Vec<Weakness>,
    #[serde(default)]
    pub affected: Vec<Affected>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    #[serde(default)]
    pub cvss_metric_v31: Vec<Metric>,
    #[serde(default)]
    pub cvss_metric_v40: Vec<Metric>,
    #[serde(default)]
    pub cvss_metric_v30: Vec<Metric>,
    #[serde(default)]
    pub cvss_metric_v2: Vec<Metric>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metric {
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub cvss_data: Option<CvssData>,
    /// CVSS v2 trae `baseSeverity` fuera de `cvssData`.
    #[serde(default)]
    pub base_severity: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CvssData {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub vector_string: Option<String>,
    #[serde(default)]
    pub base_score: Option<f32>,
    #[serde(default)]
    pub base_severity: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Description {
    #[serde(default)]
    pub lang: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Weakness {
    #[serde(default)]
    pub description: Vec<Description>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Affected {
    #[serde(default)]
    pub affected_data: Vec<AffectedData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedData {
    #[serde(default)]
    pub vendor: Option<String>,
    #[serde(default)]
    pub product: Option<String>,
}

// ---------------------------------------------------------------------------
// Cliente HTTP
// ---------------------------------------------------------------------------

/// Cliente con timeout de 15 s y User-Agent fijo (§4.4, CON-007).
pub fn build_client() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .user_agent(concat!("ficha-del-dia/", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap_or_default()
}

/// GET con `Accept: application/json`, `apiKey` opcional (solo si existe
/// `NVD_API_KEY` en el entorno; nunca se loguea) y techo de 8 MB.
pub async fn get_capped(
    client: &Client,
    url: &str,
    api_key: Option<&str>,
) -> Result<Vec<u8>, Error> {
    let mut req = client.get(url).header(ACCEPT, "application/json");
    if let Some(key) = api_key {
        req = req.header("apiKey", key);
    }
    let mut resp = req.send().await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(Error::HttpStatus(status.as_u16()));
    }
    if let Some(len) = resp.content_length() {
        if len as usize > MAX_RESPONSE_BYTES {
            return Err(Error::TooLarge);
        }
    }
    // Lectura por chunks: respeta el techo aunque la respuesta sea chunked.
    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await? {
        if buf.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(Error::TooLarge);
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf)
}

/// Descarga y normaliza la ventana completa del día UTC (§4.6), paginando
/// hasta agotar `totalResults`. Sin filtro de severidad: se filtra en local.
pub async fn fetch_day(
    client: &Client,
    base: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<DailyPick>, Error> {
    let start_s = start
        .date_naive()
        .format("%Y-%m-%dT00:00:00.000Z")
        .to_string();
    let end_s = format!("{}Z", end.format("%Y-%m-%dT%H:%M:%S%.3f"));
    let api_key = std::env::var("NVD_API_KEY").ok();
    let mut all: Vec<Vulnerability> = Vec::new();
    let mut start_index: u32 = 0;
    let mut pages = 0;
    loop {
        pages += 1;
        let url = format!(
            "{base}?pubStartDate={start_s}&pubEndDate={end_s}&resultsPerPage={PAGE_SIZE}&startIndex={start_index}"
        );
        let body = get_capped(client, &url, api_key.as_deref()).await?;
        let page: NvdResponse = serde_json::from_slice(&body)?;
        let count = page.vulnerabilities.len() as u32;
        let total = page.total_results;
        all.extend(page.vulnerabilities);
        start_index += count;
        if count == 0 || start_index >= total || pages >= MAX_PAGES {
            break;
        }
    }
    Ok(normalize_all(&all))
}

// ---------------------------------------------------------------------------
// Normalización
// ---------------------------------------------------------------------------

/// Normaliza y deduplica por `id` (§9.3: IDs duplicados entre páginas).
pub fn normalize(resp: &NvdResponse) -> Vec<DailyPick> {
    normalize_all(&resp.vulnerabilities)
}

pub fn normalize_all(vulns: &[Vulnerability]) -> Vec<DailyPick> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for v in vulns {
        if seen.insert(v.cve.id.clone()) {
            if let Some(pick) = normalize_cve(&v.cve) {
                out.push(pick);
            }
        }
    }
    out
}

/// Normaliza un registro NVD. Devuelve `None` si no hay métrica utilizable.
pub fn normalize_cve(cve: &Cve) -> Option<DailyPick> {
    let (score_version, metric) = choose_metric(cve)?;
    let data = metric.cvss_data.as_ref()?;
    let score = data.base_score?;
    let vector = data.vector_string.clone().unwrap_or_default();
    let severity = severity_of(
        score,
        data.base_severity
            .as_ref()
            .or(metric.base_severity.as_ref()),
    );
    let published = cve
        .published
        .as_deref()
        .map(parse_published)
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH);

    let mut cwes: Vec<String> = cve
        .weaknesses
        .iter()
        .flat_map(|w| w.description.iter())
        .filter(|d| d.lang == "en")
        .map(|d| d.value.clone())
        .filter(|v| !v.is_empty())
        .collect();
    cwes.sort();
    cwes.dedup();

    let (vendor, product) = vendor_product(cve);

    let summary = cve
        .descriptions
        .iter()
        .find(|d| d.lang == "en")
        .map(|d| sanitize(&d.value))
        .unwrap_or_default();

    Some(DailyPick {
        id: cve.id.clone(),
        published,
        score,
        score_version: score_version.to_string(),
        severity,
        vector,
        score_source: score_source(metric),
        epss: None,
        epss_percentile: None,
        kev: false,
        kev_due_date: None,
        cwes,
        vendor,
        product,
        summary,
        rank: 0,
    })
}

/// Orden de preferencia: v3.1 → v4.0 → v3.0 → v2.0, prefiriendo "Primary".
fn choose_metric(cve: &Cve) -> Option<(&'static str, &Metric)> {
    let m = &cve.metrics;
    let order: [(&'static str, &Vec<Metric>); 4] = [
        ("3.1", &m.cvss_metric_v31),
        ("4.0", &m.cvss_metric_v40),
        ("3.0", &m.cvss_metric_v30),
        ("2.0", &m.cvss_metric_v2),
    ];
    for (version, list) in order {
        if let Some(metric) = prefer_primary(list) {
            return Some((version, metric));
        }
    }
    None
}

fn prefer_primary(list: &[Metric]) -> Option<&Metric> {
    list.iter()
        .find(|m| m.kind.as_deref() == Some("Primary"))
        .or_else(|| list.first())
}

/// "nvd" si la fuente es NIST, "cna" en cualquier otro caso.
fn score_source(metric: &Metric) -> String {
    if metric.source.as_deref() == Some("nvd@nist.gov") {
        "nvd".into()
    } else {
        "cna".into()
    }
}

/// `baseSeverity` declarada (mayúsculas) o derivada del score.
fn severity_of(score: f32, explicit: Option<&String>) -> String {
    if let Some(s) = explicit {
        let s = s.trim();
        if !s.is_empty() {
            return s.to_uppercase();
        }
    }
    if score >= 9.0 {
        "CRITICAL".into()
    } else if score >= 7.0 {
        "HIGH".into()
    } else if score >= 4.0 {
        "MEDIUM".into()
    } else if score > 0.0 {
        "LOW".into()
    } else {
        "NONE".into()
    }
}

/// Primer `affected[].affectedData[]` con vendor/product; nunca CPE.
fn vendor_product(cve: &Cve) -> (Option<String>, Option<String>) {
    let mut vendor = None;
    let mut product = None;
    'outer: for affected in &cve.affected {
        for entry in &affected.affected_data {
            if vendor.is_none() {
                vendor = non_empty(entry.vendor.as_deref()).map(Into::into);
            }
            if product.is_none() {
                product = non_empty(entry.product.as_deref()).map(Into::into);
            }
            if vendor.is_some() && product.is_some() {
                break 'outer;
            }
        }
    }
    (vendor, product)
}

fn non_empty(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|s| !s.is_empty())
}

/// Acepta RFC-3339 con o sin `Z` y el formato sin zona de NVD
/// ("2026-11-18T16:17:47.560"). Si nada cuadra, época Unix (nunca pánico).
pub fn parse_published(raw: &str) -> DateTime<Utc> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return dt.with_timezone(&Utc);
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S%.f") {
        return naive.and_utc();
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        if let Some(t) = date.and_hms_opt(0, 0, 0) {
            return t.and_utc();
        }
    }
    DateTime::<Utc>::UNIX_EPOCH
}

/// Mojibake conocido de CNA sobre UTF-8 (§9.3). Se aplica de forma genérica
/// sobre el string completo, entradas más largas primero.
pub fn sanitize(s: &str) -> String {
    const MAP: &[(&str, &str)] = &[
        ("â€œ", "\u{201C}"),      // “
        ("â€\u{9D}", "\u{201D}"), // ”
        ("â€™", "\u{2019}"),      // ’
        ("â€“", "\u{2013}"),      // –
        ("Ã©", "é"),
        ("Ã³", "ó"),
        ("Ã¡", "á"),
        ("Ã±", "ñ"),
    ];
    let mut out = s.to_string();
    for (from, to) in MAP {
        out = out.replace(from, to);
    }
    out
}
