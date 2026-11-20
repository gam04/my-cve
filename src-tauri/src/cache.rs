//! Cache en disco dentro de `app_data_dir` (§4.5): escritura atómica (tmp + rename)
//! y lectura tolerante (JSON corrupto se ignora, nunca truena la app).

use serde::de::DeserializeOwned;
use std::fs;
use std::path::{Path, PathBuf};

pub fn dir(app_data: &Path) -> PathBuf {
    app_data.join("cache")
}

pub fn board_path(app_data: &Path, date: &str) -> PathBuf {
    dir(app_data).join(format!("board-{date}.json"))
}

pub fn last_good_path(app_data: &Path) -> PathBuf {
    dir(app_data).join("last-good.json")
}

pub fn kev_path(app_data: &Path) -> PathBuf {
    dir(app_data).join("kev.json")
}

pub fn settings_path(app_data: &Path) -> PathBuf {
    app_data.join("settings.json")
}

/// Escribe `contents` de forma atómica: primero a un `.tmp` del mismo
/// directorio y después `rename` sobre el destino.
pub fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("ruta sin directorio padre"))?;
    fs::create_dir_all(parent)?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path)
}

/// Lee y deserializa un JSON. Si el archivo no existe o el contenido está
/// corrupto devuelve `None` (y registra `warn`), nunca entra en pánico.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            log::warn!("cache ilegible en {}: {e}", path.display());
            return None;
        }
    };
    match serde_json::from_str(&raw) {
        Ok(value) => Some(value),
        Err(e) => {
            log::warn!("JSON corrupto en {}: {e}", path.display());
            None
        }
    }
}
