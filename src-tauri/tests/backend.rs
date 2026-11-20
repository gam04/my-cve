//! Pruebas del backend: normalización NVD sobre el fixture real (AC-010),
//! ranking total y estable (AC-002), y fallback de caché ante NVD 503 (AC-005).

use chrono::{DateTime, Utc};
use ficha_del_dia_lib::commands::{self, RefreshCtx};
use ficha_del_dia_lib::model::{Board, DailyPick};
use ficha_del_dia_lib::nvd::{self, NvdResponse};
use ficha_del_dia_lib::rank;

fn load_fixture(name: &str) -> NvdResponse {
    let raw = std::fs::read_to_string(format!("tests/fixtures/{name}")).expect("fixture existe");
    serde_json::from_str(&raw).expect("fixture es NVD JSON válido")
}

fn fixture_board(name: &str, at: DateTime<Utc>) -> Board {
    let resp = load_fixture(name);
    rank::build_board(at.date_naive().to_string(), nvd::normalize(&resp), at)
}

fn published_at(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .expect("fecha de prueba RFC-3339")
        .with_timezone(&Utc)
}

fn mk(id: &str, score: f32, kev: bool, epss: Option<f32>, published: &str) -> DailyPick {
    DailyPick {
        id: id.to_string(),
        score,
        kev,
        epss,
        published: published_at(published),
        ..DailyPick::default()
    }
}

// ---------------------------------------------------------------------------
// Normalización sobre el fixture real (AC-010)
// ---------------------------------------------------------------------------

#[test]
fn normalize_fixture() {
    let board = fixture_board("nvd-2026-11-19.json", Utc::now());
    let pick = board.pick.expect("el fixture debe producir un pick");
    assert_eq!(pick.id, "CVE-2026-130778");
    assert_eq!(pick.score, 10.0);
    assert_eq!(pick.score_version, "3.1");
    assert_eq!(pick.severity, "CRITICAL");
    assert_eq!(pick.score_source, "cna");
    assert_eq!(board.total_critical_today, 13);
    let ids: Vec<&str> = board.runners_up.iter().map(|p| p.id.as_str()).collect();
    // Empate en 9.9: publicados más recientes primero (§4.3 regla 4).
    assert_eq!(
        ids,
        vec!["CVE-2026-132143", "CVE-2026-132141", "CVE-2026-130786"]
    );
}

#[test]
fn normalize_fixture_dedupes_and_assigns_ranks() {
    let board = fixture_board("nvd-2026-11-19.json", Utc::now());
    let mut ids: Vec<&str> = board
        .pick
        .iter()
        .chain(board.runners_up.iter())
        .map(|p| p.id.as_str())
        .collect();
    ids.sort();
    let unique = ids.iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), unique.len(), "no debe haber IDs duplicados");
    assert_eq!(board.pick.expect("pick").rank, 1);
    for (i, runner) in board.runners_up.iter().enumerate() {
        assert_eq!(runner.rank, i as u32 + 2);
    }
}

// ---------------------------------------------------------------------------
// Métrica v4.0 única (AC-004) y vendor/product con configurations: null (AC-003)
// ---------------------------------------------------------------------------

#[test]
fn v40_only() {
    let board = fixture_board("nvd-v40-only.json", Utc::now());
    let pick = board.pick.expect("pick");
    assert_eq!(pick.score_version, "4.0");
    assert_eq!(pick.score, 8.7);
    assert_eq!(pick.severity, "HIGH");
    // La preferencia Primary evita coger la métrica Secondary de nvd@nist.gov.
    assert_eq!(pick.score_source, "cna");
    // Saneo de mojibake en la descripción (§9.3).
    assert!(
        pick.summary.contains('–'),
        "â€“ debe sanearse a –: {}",
        pick.summary
    );
    // CWEs únicos, ordenados y solo lang == "en".
    assert_eq!(pick.cwes, vec!["CWE-79"]);
}

#[test]
fn configurations_null_uses_affected() {
    let board = fixture_board("nvd-2026-11-19.json", Utc::now());
    let pick = board.pick.expect("pick");
    assert_eq!(pick.vendor.as_deref(), Some("lxc"));
    assert_eq!(pick.product.as_deref(), Some("incus"));
}

// ---------------------------------------------------------------------------
// Ranking: totalidad y estabilidad bajo permutación (AC-002)
// ---------------------------------------------------------------------------

fn reference_order(picks: &[DailyPick]) -> Vec<String> {
    rank::sorted(picks.to_vec())
        .iter()
        .map(|p| p.id.clone())
        .collect()
}

#[test]
fn ranking_is_stable_under_permutation() {
    let base = vec![
        mk("CVE-A", 9.8, false, Some(0.5), "2026-11-19T10:00:00Z"),
        mk("CVE-B", 9.8, true, Some(0.1), "2026-11-19T09:00:00Z"),
        mk("CVE-C", 7.5, false, None, "2026-11-19T08:00:00Z"),
        mk("CVE-D", 9.8, false, Some(0.5), "2026-11-19T11:00:00Z"),
        mk("CVE-E", 9.8, false, Some(0.5), "2026-11-19T11:00:00Z"), // empate total con D salvo id
    ];
    let expected = reference_order(&base);
    // KEV gana el empate a 9.8; después EPSS 0.5 y published desc; D/E
    // empatan del todo y el id ascendente los ordena.
    assert_eq!(expected, vec!["CVE-B", "CVE-D", "CVE-E", "CVE-A", "CVE-C"]);

    // Todas las permutaciones de 4 elementos sobre los 4 primeros, más 200
    // permutaciones aleatorias con semilla fija sobre los 5.
    for a in 0..4usize {
        for b in 0..4usize {
            for c in 0..4usize {
                for d in 0..4usize {
                    let idx = [a, b, c, d];
                    if [a, b, c, d]
                        .iter()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        != 4
                    {
                        continue;
                    }
                    let input: Vec<DailyPick> = idx.iter().map(|&i| base[i].clone()).collect();
                    // La referencia restringida al subconjunto presente.
                    let sub: Vec<String> = expected
                        .iter()
                        .filter(|id| input.iter().any(|p| &p.id == *id))
                        .cloned()
                        .collect();
                    assert_eq!(reference_order(&input), sub, "permutación {idx:?}");
                }
            }
        }
    }

    let mut seed: u64 = 0x2026_1119;
    for _ in 0..200 {
        let mut input = base.clone();
        for i in (1..input.len()).rev() {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let j = (seed >> 33) as usize % (i + 1);
            input.swap(i, j);
        }
        assert_eq!(reference_order(&input), expected);
    }
}

#[test]
fn kev_only_breaks_score_ties() {
    // Un KEV de 8.8 no supera a un 9.8 sin explotación conocida (§7).
    let board = rank::build_board(
        "2026-11-19".into(),
        vec![
            mk("CVE-NO-KEV", 9.8, false, None, "2026-11-19T08:00:00Z"),
            mk("CVE-KEV", 8.8, true, None, "2026-11-19T09:00:00Z"),
        ],
        Utc::now(),
    );
    assert_eq!(board.pick.expect("pick").id, "CVE-NO-KEV");

    // Entre dos 9.8, gana el KEV aunque su id sea mayor.
    let board = rank::build_board(
        "2026-11-19".into(),
        vec![
            mk("CVE-A", 9.8, false, None, "2026-11-19T08:00:00Z"),
            mk("CVE-B", 9.8, true, None, "2026-11-19T09:00:00Z"),
        ],
        Utc::now(),
    );
    assert_eq!(board.pick.expect("pick").id, "CVE-B");

    // EPSS más alto desempata a igual score y KEV.
    let board = rank::build_board(
        "2026-11-19".into(),
        vec![
            mk("CVE-A", 9.8, true, Some(0.01), "2026-11-19T09:00:00Z"),
            mk("CVE-B", 9.8, true, Some(0.02), "2026-11-19T08:00:00Z"),
        ],
        Utc::now(),
    );
    assert_eq!(board.pick.expect("pick").id, "CVE-B");

    // None cuenta como -1: pierde frente a cualquier EPSS.
    let board = rank::build_board(
        "2026-11-19".into(),
        vec![
            mk("CVE-A", 9.8, false, None, "2026-11-19T11:00:00Z"),
            mk("CVE-B", 9.8, false, Some(0.0), "2026-11-19T08:00:00Z"),
        ],
        Utc::now(),
    );
    assert_eq!(board.pick.expect("pick").id, "CVE-B");
}

// ---------------------------------------------------------------------------
// NVD 503 → último válido con stale: true (AC-005)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn nvd_503_falls_back_to_cache() {
    let mock = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(wiremock::ResponseTemplate::new(503))
        .mount(&mock)
        .await;

    let dir = std::env::temp_dir().join(format!(
        "ficha-del-dia-test-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    std::fs::create_dir_all(&dir).expect("crear dir temporal");
    let last_good = fixture_board("nvd-2026-11-19.json", Utc::now());
    let last_good_path = ficha_del_dia_lib::cache::last_good_path(&dir);
    std::fs::create_dir_all(last_good_path.parent().expect("ruta con padre"))
        .expect("crear dir de caché");
    std::fs::write(
        &last_good_path,
        serde_json::to_string(&last_good).expect("serializar board"),
    )
    .expect("escribir last-good.json");

    let ctx = RefreshCtx {
        client: nvd::build_client(),
        nvd_base: mock.uri(),
        epss_base: "http://127.0.0.1:9/epss".into(),
        kev_base: "http://127.0.0.1:9/kev".into(),
        app_data: dir.clone(),
    };

    let result = commands::refresh_board(&ctx, Utc::now()).await;
    assert!(result.is_err(), "NVD 503 debe propagar el error");

    let message = result.unwrap_err();
    let board = commands::fallback_board(&dir, &message).expect("debe haber caché previa");
    assert!(board.stale);
    assert!(board.fetch_error.is_some());
    assert_eq!(board.pick.expect("pick conservado").id, "CVE-2026-130778");

    std::fs::remove_dir_all(&dir).ok();
}

/// Regresión: la feed real de CISA usa `cveID` (D mayúscula). Con `camelCase`
/// a secas el catálogo entero fallaba y todos los `kev` quedaban en `false`.
#[test]
fn kev_real_feed_parses() {
    let raw = std::fs::read_to_string("tests/fixtures/kev-sample.json").expect("fixture KEV");
    let file: ficha_del_dia_lib::enrich::KevFile =
        serde_json::from_str(&raw).expect("la feed real de CISA debe parsear");
    assert_eq!(file.vulnerabilities.len(), 1786);
    let index = ficha_del_dia_lib::enrich::kev_index(&file);
    let log4shell = index
        .get("CVE-2021-44228")
        .expect("Log4Shell está en el catálogo");
    assert!(log4shell.is_some(), "Log4Shell tiene fecha límite");
}

#[tokio::test]
async fn nvd_empty_day_produces_empty_board() {
    let mock = wiremock::MockServer::start().await;
    let raw = std::fs::read_to_string("tests/fixtures/nvd-empty.json").expect("fixture");
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(raw))
        .mount(&mock)
        .await;

    let dir = std::env::temp_dir().join(format!(
        "ficha-del-dia-test-empty-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    std::fs::create_dir_all(&dir).expect("crear dir temporal");

    let ctx = RefreshCtx {
        client: nvd::build_client(),
        nvd_base: mock.uri(),
        epss_base: "http://127.0.0.1:9/epss".into(),
        kev_base: "http://127.0.0.1:9/kev".into(),
        app_data: dir.clone(),
    };
    let now = Utc::now();
    let board = commands::refresh_board(&ctx, now)
        .await
        .expect("tablero vacío");
    assert_eq!(board.board_date, now.date_naive().to_string());
    assert!(board.pick.is_none());
    assert!(board.runners_up.is_empty());
    assert_eq!(board.total_critical_today, 0);
    assert!(!board.stale);
    assert!(board.fetch_error.is_none());
    // Persistió el snapshot y last-good.
    assert!(dir
        .join("cache")
        .join(format!("board-{}.json", board.board_date))
        .exists());
    assert!(ficha_del_dia_lib::cache::last_good_path(&dir).exists());

    std::fs::remove_dir_all(&dir).ok();
}
