//! Ranking determinista (§4.3). Comparador total: score desc → KEV primero
//! (solo desempate a igual score) → EPSS desc (None = -1) → published desc →
//! id asc. Incluye asignación de `rank` y construcción del `Board`.

use crate::model::{Board, DailyPick};
use chrono::{DateTime, Utc};
use std::cmp::Ordering;

/// Comparador total: `Less` = `a` va antes que `b`.
pub fn compare(a: &DailyPick, b: &DailyPick) -> Ordering {
    // 1. score descendente (NaN se trata como empate y cae a los siguientes
    //    criterios, lo que mantiene la comparación total).
    b.score
        .partial_cmp(&a.score)
        .unwrap_or(Ordering::Equal)
        // 2. KEV primero, solo como desempate a igual score.
        .then_with(|| b.kev.cmp(&a.kev))
        // 3. EPSS descendente, None cuenta como -1.
        .then_with(|| {
            epss_key(b)
                .partial_cmp(&epss_key(a))
                .unwrap_or(Ordering::Equal)
        })
        // 4. published descendente.
        .then_with(|| b.published.cmp(&a.published))
        // 5. id ascendente: garantiza un orden total ante cualquier empate.
        .then_with(|| a.id.cmp(&b.id))
}

fn epss_key(p: &DailyPick) -> f32 {
    p.epss.unwrap_or(-1.0)
}

/// Ordena los picks y asigna `rank` 1..n de forma estable.
pub fn sorted(mut picks: Vec<DailyPick>) -> Vec<DailyPick> {
    picks.sort_by(compare);
    for (i, pick) in picks.iter_mut().enumerate() {
        pick.rank = (i + 1) as u32;
    }
    picks
}

/// Construye el tablero: pick + hasta 3 runners-up + total de críticas (§4.1).
pub fn build_board(
    board_date: String,
    picks: Vec<DailyPick>,
    generated_at: DateTime<Utc>,
) -> Board {
    let mut picks = sorted(picks);
    let total_critical_today = picks
        .iter()
        .filter(|p| p.score >= 9.0)
        .count()
        .min(u32::MAX as usize) as u32;
    let pick = if picks.is_empty() {
        None
    } else {
        Some(picks.remove(0))
    };
    let runners_up: Vec<DailyPick> = picks.into_iter().take(3).collect();
    Board {
        board_date,
        generated_at,
        total_critical_today,
        pick,
        runners_up,
        stale: false,
        fetch_error: None,
    }
}
