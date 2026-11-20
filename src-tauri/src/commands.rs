//! Comandos Tauri (§4.2) + el bucle de refresco y la emisión de eventos.
//!
//! `board_today` nunca toca la red. `refresh_now` fuerza red y, en error,
//! devuelve el último válido con `stale: true` y emite `board:error`.

use crate::cache;
use crate::enrich;
use crate::model::{Board, DailyPick, Settings};
use crate::nvd;
use crate::rank;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_http::reqwest::Client;
use tauri_plugin_notification::NotificationExt;

/// Contexto de refresco: inyectable para pruebas (bases de URL intercambiables).
pub struct RefreshCtx {
    pub client: Client,
    pub nvd_base: String,
    pub epss_base: String,
    pub kev_base: String,
    pub app_data: PathBuf,
}

/// Estado compartido de la app.
#[derive(Default)]
pub struct AppState {
    pub board: Mutex<Option<Board>>,
    /// Último pick visto; `None` hasta el primer tablero (no notifica entonces).
    pub last_pick_id: Mutex<Option<String>>,
    pub last_pos_save: Mutex<Option<Instant>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct BoardErrorPayload {
    message: String,
    last_good_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Refresco (núcleo probado sin AppHandle)
// ---------------------------------------------------------------------------

/// Refresca contra la red, enriquece, rankea, persiste y devuelve el tablero.
/// En error de red devuelve `Err(mensaje)`; el fallback a caché es del
/// llamador (así se puede probar con wiremock sin AppHandle).
pub async fn refresh_board(ctx: &RefreshCtx, now: DateTime<Utc>) -> Result<Board, String> {
    let start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .ok_or("fecha inválida")?
        .and_utc();
    let picks = nvd::fetch_day(&ctx.client, &ctx.nvd_base, start, now)
        .await
        .map_err(|e| e.to_string())?;
    let mut picks = picks;
    // El enriquecimiento nunca tumba el tablero: degrada a None/false.
    enrich::enrich(&ctx.client, &ctx.app_data, &mut picks).await;
    let board = rank::build_board(now.date_naive().to_string(), picks, now);
    if let Ok(raw) = serde_json::to_string(&board) {
        let _ = cache::write_atomic(&cache::board_path(&ctx.app_data, &board.board_date), &raw);
        let _ = cache::write_atomic(&cache::last_good_path(&ctx.app_data), &raw);
    }
    Ok(board)
}

/// Carga `last-good.json`, lo marca `stale` y le pega el mensaje de error.
pub fn fallback_board(app_data: &std::path::Path, message: &str) -> Option<Board> {
    let mut board: Board = cache::read_json(&cache::last_good_path(app_data))?;
    board.stale = true;
    board.fetch_error = Some(message.to_string());
    Some(board)
}

pub fn empty_board(now: DateTime<Utc>, message: &str) -> Board {
    Board {
        board_date: now.date_naive().to_string(),
        generated_at: now,
        total_critical_today: 0,
        pick: None,
        runners_up: Vec::new(),
        stale: false,
        fetch_error: Some(message.to_string()),
    }
}

fn ctx_for(app: &AppHandle) -> RefreshCtx {
    let app_data = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("ficha-del-dia"));
    RefreshCtx {
        client: nvd::build_client(),
        nvd_base: nvd::NVD_URL.to_string(),
        epss_base: enrich::EPSS_URL.to_string(),
        kev_base: enrich::KEV_URL.to_string(),
        app_data,
    }
}

/// Refresco completo desde la app: red, fallback, notificación y eventos.
/// Devuelve el tablero resultante (fresco o degradado).
pub async fn refresh_and_emit(app: &AppHandle) -> Board {
    let ctx = ctx_for(app);
    let board = match refresh_board(&ctx, Utc::now()).await {
        Ok(board) => board,
        Err(message) => {
            log::warn!("refresco falló: {message}");
            let last_good_at =
                fallback_board(&ctx.app_data, &message).map(|b| b.generated_at.to_rfc3339());
            let _ = app.emit(
                "board:error",
                BoardErrorPayload {
                    message: message.clone(),
                    last_good_at,
                },
            );
            fallback_board(&ctx.app_data, &message)
                .unwrap_or_else(|| empty_board(Utc::now(), &message))
        }
    };

    maybe_notify_pick_change(app, &board);

    let _ = app.emit("board:updated", &board);
    if let Ok(mut slot) = app.state::<AppState>().board.lock() {
        *slot = Some(board.clone());
    }
    board
}

/// REQ-009: notifica solo si el pick cambió respecto al tablero anterior
/// (el primer tablero no notifica) y `notifyOnTopChange` está activo.
fn maybe_notify_pick_change(app: &AppHandle, board: &Board) {
    let new_id = board.pick.as_ref().map(|p| p.id.clone());
    let should_notify = {
        let state = app.state::<AppState>();
        let mut prev = match state.last_pick_id.lock() {
            Ok(prev) => prev,
            Err(p) => p.into_inner(),
        };
        let changed = prev.is_some() && *prev != new_id;
        *prev = new_id;
        changed && load_settings(app).notify_on_top_change
    };
    if should_notify {
        if let Some(pick) = &board.pick {
            notify_pick(app, pick);
        }
    }
}

fn notify_pick(app: &AppHandle, pick: &DailyPick) {
    let _ = app
        .notification()
        .builder()
        .title("Nuevo CVE más crítico")
        .body(format!("{} · CVSS {:.1}", pick.id, pick.score))
        .show();
}

// ---------------------------------------------------------------------------
// Settings (persistidos en <appDataDir>/settings.json)
// ---------------------------------------------------------------------------

pub fn load_settings(app: &AppHandle) -> Settings {
    let path = app
        .path()
        .app_data_dir()
        .map(|dir| cache::settings_path(&dir));
    match path {
        Ok(path) => cache::read_json::<Settings>(&path).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save_settings(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("settings.json");
    let raw = serde_json::to_string(settings).map_err(|e| e.to_string())?;
    cache::write_atomic(&path, &raw).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Comandos Tauri
// ---------------------------------------------------------------------------

/// Tablero del día, solo desde memoria o disco. Nunca red.
#[tauri::command]
pub async fn board_today(app: AppHandle) -> Result<Board, String> {
    let today = Utc::now().date_naive().to_string();
    let state = app.state::<AppState>();
    if let Some(board) = state.board.lock().map_err(|p| p.to_string())?.clone() {
        return Ok(board);
    }
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    // Snapshot del día; si no hay, último tablero válido (puede ser de otro día).
    for path in [
        cache::board_path(&app_data, &today),
        cache::last_good_path(&app_data),
    ] {
        if let Some(mut board) = cache::read_json::<Board>(&path) {
            board.stale = true;
            if let Ok(mut slot) = state.board.lock() {
                *slot = Some(board.clone());
            }
            return Ok(board);
        }
    }
    Ok(empty_board(Utc::now(), "sin datos: sin caché y sin red"))
}

/// Fuerza refresco contra la red y emite eventos.
#[tauri::command]
pub async fn refresh_now(app: AppHandle) -> Result<Board, String> {
    Ok(refresh_and_emit(&app).await)
}

#[tauri::command]
pub fn settings_get(app: AppHandle) -> Result<Settings, String> {
    Ok(load_settings(&app))
}

#[tauri::command]
pub fn settings_set(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    save_settings(&app, &settings)?;
    Ok(settings)
}
