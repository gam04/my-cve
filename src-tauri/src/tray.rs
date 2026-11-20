//! Bandeja del sistema (REQ-007): mostrar/ocultar, refrescar, copiar ID,
//! abrir en NVD, arranque con el sistema y salir.

use crate::commands::{self, AppState};
use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_clipboard_manager::ClipboardExt as _;
use tauri_plugin_opener::OpenerExt as _;

const NVD_DETAIL: &str = "https://nvd.nist.gov/vuln/detail/";
const TRAY_ID: &str = "ficha-tray";

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let autostart_on = app.autolaunch().is_enabled().unwrap_or(false);

    let show_hide = MenuItem::with_id(app, "show_hide", "Mostrar / Ocultar", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "Refrescar ahora", true, None::<&str>)?;
    let copy_id = MenuItem::with_id(app, "copy_id", "Copiar ID", true, None::<&str>)?;
    let open_nvd = MenuItem::with_id(app, "open_nvd", "Abrir en NVD", true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "Arrancar con el sistema",
        true,
        autostart_on,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&show_hide, &refresh, &copy_id, &open_nvd, &autostart, &quit],
    )?;

    // Icono del widget: el de la ventana por defecto; si no hubiera (build
    // sin bundle), un icono transparente 1x1 para no fallar el arranque.
    let icon = app.default_window_icon().cloned().unwrap_or_else(|| {
        log::warn!("sin icono por defecto; usando icono transparente de reserva");
        tauri::image::Image::new_owned(vec![0, 0, 0, 0], 1, 1)
    });

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Ficha del día")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| handle_menu(app, event.id().as_ref()))
        .build(app)?;
    Ok(())
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "show_hide" => {
            if let Some(window) = app.get_webview_window("main") {
                if window.is_visible().unwrap_or(false) {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        }
        "refresh" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                commands::refresh_and_emit(&app).await;
            });
        }
        "copy_id" => {
            if let Some(id) = current_pick_id(app) {
                if let Err(e) = app.clipboard().write_text(id) {
                    log::warn!("no se pudo copiar el ID: {e}");
                }
            }
        }
        "open_nvd" => {
            if let Some(id) = current_pick_id(app) {
                let url = format!("{NVD_DETAIL}{id}");
                if let Err(e) = app.opener().open_url(url, None::<&str>) {
                    log::warn!("no se pudo abrir NVD: {e}");
                }
            }
        }
        "autostart" => {
            let manager = app.autolaunch();
            let result = if manager.is_enabled().unwrap_or(false) {
                manager.disable()
            } else {
                manager.enable()
            };
            if let Err(e) = result {
                log::warn!("autostart no se pudo cambiar: {e}");
            }
        }
        "quit" => app.exit(0),
        _ => {}
    }
}

fn current_pick_id(app: &AppHandle) -> Option<String> {
    let state = app.state::<AppState>();
    let board = state.board.lock().ok()?;
    board.as_ref()?.pick.as_ref().map(|p| p.id.clone())
}
