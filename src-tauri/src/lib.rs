//! Cableado de la app: orden de plugins (single-instance primero), setup de
//! ventana/tray/bucle de refresco (§4.7, REQ-005, REQ-010, REQ-012).

pub mod cache;
pub mod commands;
pub mod enrich;
pub mod model;
pub mod nvd;
pub mod rank;
pub mod tray;
pub use tauri_plugin_http::reqwest;

use chrono::Utc;
use commands::AppState;
use std::time::{Duration, Instant};
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

/// Guarda la posición de ventana como mucho una vez por segundo mientras se
/// arrastra, para no martillar el disco en cada evento `Moved`.
const POS_SAVE_THROTTLE_MS: u128 = 1000;
/// Sondeo del bucle de refresco: comprueba vencimiento y cambio de día UTC.
const LOOP_TICK_SECS: u64 = 30;

pub fn run() {
    tauri::Builder::default()
        // single-instance PRIMERO (§4.7): una segunda instancia revela la ventana.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        // SEC-006: en producción solo warn+.
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Info
                } else {
                    log::LevelFilter::Warn
                })
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_http::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::board_today,
            commands::refresh_now,
            commands::settings_get,
            commands::settings_set,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            tray::build(&handle)?;
            restore_window_position(&handle);
            persist_window_position(&handle);
            // REQ-010: la ventana queda `visible: false`; solo la muestra el
            // frontend tras el primer pintado.
            start_refresh_loop(handle.clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Restaura la última posición guardada; si cae fuera de cualquier monitor
/// visible, recentra (AC-007, §9.3 monitor desconectado).
fn restore_window_position(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Some(pos) = commands::load_settings(app).window_pos else {
        // Primer arranque (decisión por defecto): esquina inferior derecha.
        place_bottom_right(&window);
        return;
    };
    let target = tauri::PhysicalPosition::new(pos.x as i32, pos.y as i32);
    let visible = window
        .available_monitors()
        .map(|monitors| {
            monitors.iter().any(|monitor| {
                let origin = monitor.position();
                let size = monitor.size();
                target.x >= origin.x
                    && target.x < origin.x + size.width as i32
                    && target.y >= origin.y
                    && target.y < origin.y + size.height as i32
            })
        })
        .unwrap_or(false);
    if visible {
        let _ = window.set_position(target);
    } else {
        let _ = window.center();
    }
}

/// Primer arranque: esquina inferior derecha del monitor principal, dentro del
/// área de trabajo (excluye la barra de tareas). Los arranques siguientes
/// respetan la posición guardada por el usuario.
fn place_bottom_right(window: &tauri::WebviewWindow) {
    let Ok(Some(monitor)) = window.primary_monitor() else {
        let _ = window.center();
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let size = window.outer_size().unwrap_or(tauri::PhysicalSize::new(
        (440.0 * scale) as u32,
        (310.0 * scale) as u32,
    ));
    let margin = (20.0 * scale) as i32;
    let area = monitor.work_area();
    let x = area.position.x + area.size.width as i32 - size.width as i32 - margin;
    let y = area.position.y + area.size.height as i32 - size.height as i32 - margin;
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

/// Persiste `windowPos` (con throttle) cada vez que la ventana se mueve.
fn persist_window_position(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let handle = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Moved(pos) = event {
            let should_save = {
                let state = handle.state::<AppState>();
                let Ok(mut last) = state.last_pos_save.lock() else {
                    return;
                };
                let due = match *last {
                    Some(t) => t.elapsed().as_millis() >= POS_SAVE_THROTTLE_MS,
                    None => true,
                };
                if due {
                    *last = Some(Instant::now());
                }
                due
            };
            if should_save {
                let mut settings = commands::load_settings(&handle);
                settings.window_pos = Some(model::WindowPos {
                    x: pos.x as f64,
                    y: pos.y as f64,
                });
                if let Err(e) = commands::save_settings(&handle, &settings) {
                    log::warn!("no se pudo guardar windowPos: {e}");
                }
            }
        }
    });
}

/// Bucle de refresco (REQ-005, REQ-012): refresca al arrancar, cada
/// `refreshMinutes` y al cambiar el día UTC; emite `board:updated` y notifica
/// si cambió el pick (lo hace `refresh_and_emit`).
fn start_refresh_loop(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let minutes = u64::from(commands::load_settings(&app).refresh_minutes.clamp(15, 60));
        let period = Duration::from_secs(minutes * 60);
        let mut interval = tokio::time::interval(Duration::from_secs(LOOP_TICK_SECS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // Refresco inmediato al arrancar (REQ-005).
        let mut last_refresh = Instant::now() - period;
        let mut last_date = Utc::now().date_naive();
        let mut last_tick = Instant::now();
        loop {
            interval.tick().await;
            // REQ-012: si el sistema estuvo suspendido, el intervalo pierde
            // ticks y `last_tick` envejece mucho más que un tick normal.
            let resumed = last_tick.elapsed() > Duration::from_secs(LOOP_TICK_SECS * 2 + 5);
            last_tick = Instant::now();
            let due = last_refresh.elapsed() >= period;
            let day_changed = Utc::now().date_naive() != last_date;
            if due || day_changed || resumed {
                let board = commands::refresh_and_emit(&app).await;
                last_date = Utc::now().date_naive();
                last_refresh = Instant::now();
                log::debug!(
                    "tablero refrescado: {} ({} registros)",
                    board.board_date,
                    board.total_critical_today
                );
            }
        }
    });
}
