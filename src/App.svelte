<script lang="ts">
  import { onMount } from "svelte";
  import type { Board } from "./lib/types";
  import {
    boardToday,
    fixtureName,
    openCve,
    onBoardError,
    onBoardUpdated,
    refreshNow,
    showWindowWhenReady,
  } from "./lib/ipc";
  import { clampSummary, formatScore, formatVector, relativeTime } from "./lib/format";
  import Header from "./lib/components/Header.svelte";
  import Stamp from "./lib/components/Stamp.svelte";
  import FieldGrid from "./lib/components/FieldGrid.svelte";
  import RunnerStrip from "./lib/components/RunnerStrip.svelte";

  let board = $state<Board | null>(null);
  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state<string | null>(null);
  let sweepKey = $state(0);

  const pick = $derived(board?.pick ?? null);
  const hasCritical = $derived(
    pick !== null && pick.score >= 9 && pick.severity === "CRITICAL",
  );
  const isEmpty = $derived(board !== null && pick === null);
  const cacheAge = $derived(
    board?.stale && board.generatedAt ? relativeTime(board.generatedAt) : null,
  );

  const headerTitle = $derived.by(() => {
    if (isEmpty) return "ARCHIVO VACÍO · SIN REGISTROS HOY";
    if (pick && !hasCritical) return `SIN CRÍTICAS · MÁXIMO ${formatScore(pick.score)}`;
    return "FICHA DEL DÍA";
  });

  const MESES = ["ENE", "FEB", "MAR", "ABR", "MAY", "JUN", "JUL", "AGO", "SEP", "OCT", "NOV", "DIC"];
  const headerMeta = $derived.by(() => {
    if (!board) return "";
    const [y, m, d] = board.boardDate.split("-").map(Number);
    if (!y || !m || !d) return `${board.boardDate} UTC`;
    const short = `${String(d).padStart(2, "0")} ${MESES[m - 1]} · UTC`;
    if (isEmpty) return short; // el título largo de estado vacío tiene prioridad
    return `NVD · ${String(d).padStart(2, "0")} ${MESES[m - 1]} ${y} UTC`;
  });

  /** Tres líneas de Courier Prime ≈ 104 caracteres. Texto íntegro en title. */
  const summary = $derived(pick ? clampSummary(pick.summary, 104) : "");

  let cleanedUp = false;
  const cleanups: Array<() => void> = [];

  /** `?fixture=loading`: el esqueleto permanece en pantalla (modo captura). */
  const stuckLoading = fixtureName === "loading";

  async function load() {
    loading = true;
    error = null;
    try {
      board = await boardToday();
      if (stuckLoading) board = null; // el esqueleto manda
    } catch (e) {
      error = shortError(e);
    } finally {
      if (!stuckLoading) {
        loading = false;
        await showWindowWhenReady();
      }
    }
  }

  async function manualRefresh() {
    if (refreshing) return;
    refreshing = true;
    sweepKey += 1; // barrido de tinta de 90 ms
    try {
      board = await refreshNow();
      error = null;
    } catch (e) {
      error = shortError(e);
    } finally {
      refreshing = false;
    }
  }

  function shortError(e: unknown): string {
    const raw = typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
    return raw.length > 90 ? `${raw.slice(0, 90)}…` : raw;
  }

  onMount(() => {
    void load();
    (async () => {
      const un1 = await onBoardUpdated((b) => {
        board = b;
        error = null;
      });
      const un2 = await onBoardError((p) => {
        error = p.message;
      });
      if (cleanedUp) {
        un1();
        un2();
        return;
      }
      cleanups.push(un1, un2);
    })();
    return () => {
      cleanedUp = true;
      cleanups.forEach((fn) => fn());
    };
  });
</script>

<main class="card">
  {#if loading && !board}
    <!-- Esqueleto: mismas métricas y composición que la ficha real -->
    <div class="skel-head" data-tauri-drag-region>
      <span class="skel-bar w-title"></span>
      <span class="skel-bar w-meta"></span>
    </div>
    <div class="skel-hero" aria-hidden="true">
      <div class="skel-left">
        <div class="skel-id-row">
          <span class="skel-cursor" aria-hidden="true">▌</span>
          <span class="skel-bar w-id"></span>
        </div>
        <div class="skel-bar w-product"></div>
        <div class="skel-bar w-vector"></div>
        <div class="skel-bar w-sum"></div>
        <div class="skel-bar w-sum"></div>
        <div class="skel-bar w-sum short"></div>
      </div>
      <div class="skel-right">
        <div class="skel-score"></div>
        <div class="skel-bar w-meta2"></div>
      </div>
    </div>
    <div class="skel-grid" aria-hidden="true">
      {#each [0, 1, 2, 3] as c (c)}
        <div class="skel-cell">
          <span class="skel-bar w-label"></span>
          <span class="skel-bar w-value"></span>
        </div>
      {/each}
    </div>
    <div class="skel-strip" aria-hidden="true">
      {#each [0, 1, 2] as f (f)}
        <div class="skel-frame">
          <span class="skel-bar w-no"></span>
          <div class="skel-frame-col">
            <span class="skel-bar w-fid"></span>
            <span class="skel-bar w-fscore"></span>
          </div>
        </div>
      {/each}
    </div>
    <span class="sr-only" role="status">Cargando ficha del día…</span>
  {:else if !board}
    <!-- Error sin caché -->
    <div class="err" role="alert">
      <p class="err-label">SIN CONEXIÓN · SIN CACHÉ</p>
      <p class="err-msg">{error ?? "No se pudo consultar NVD."}</p>
      <button class="retry" onclick={manualRefresh}>REINTENTAR</button>
    </div>
  {:else if isEmpty}
    <Header title={headerTitle} meta={headerMeta} onRefresh={manualRefresh} {refreshing} />
    <section class="empty" data-tauri-drag-region>
      <p class="empty-date tabular">{board.boardDate}</p>
      <p class="empty-sub">CORTE 00:00 UTC · CONSULTE MAÑANA</p>
      <p class="empty-note">{error ?? ""}</p>
    </section>
  {:else if pick}
    <Header title={headerTitle} meta={headerMeta} onRefresh={manualRefresh} {refreshing} />

    <section class="hero" aria-live="polite">
      {#key pick.id}
        <div class="hero-inner">
          <div class="hero-left">
            <button class="cve-id" onclick={() => openCve(pick.id)} title="Abrir en NVD">
              {pick.id}
            </button>
            <p class="product">
              {pick.product ?? "—"}{pick.vendor ? ` / ${pick.vendor}` : ""}
            </p>
            <p class="vector tabular">{formatVector(pick.vector)}</p>
            <p class="summary" title={pick.summary}>{summary}</p>
            {#if cacheAge}
              <p class="cache-row">
                <Stamp label="CACHÉ" variant="cache" animate={false} />
                <span class="cache-age">DATO {cacheAge.toUpperCase()}</span>
              </p>
            {/if}
          </div>
          <div class="hero-right">
            <div class="score-block">
              <span class="score tabular">{formatScore(pick.score)}</span>
              <span class="score-of">/ 10</span>
            </div>
            <p class="score-meta">CVSS {pick.scoreVersion} · {pick.severity}</p>
            {#if hasCritical}
              <div class="stamp-pos">
                <Stamp label="CRITICAL" variant="critical" animate={true} />
              </div>
            {/if}
          </div>
        </div>
      {/key}
      {#key sweepKey}
        {#if sweepKey > 0}
          <div class="sweep" aria-hidden="true"></div>
        {/if}
      {/key}
    </section>

    <FieldGrid {pick} />
    <RunnerStrip runners={board.runnersUp} />

    <span class="sr-only" role="status">
      {pick.id}, {pick.severity}, {formatScore(pick.score)} de 10
    </span>
  {/if}
</main>

<style>
  .card {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: 30px 1fr auto 40px;
    width: var(--w);
    height: var(--h);
    overflow: hidden;
    background: var(--paper);
    border: 1px solid var(--ink); /* marco cerrado: la ficha está enmarcada */
  }

  /* Grano de papel (SVG feTurbulence inline, ~4 %) */
  .card::after {
    content: "";
    position: absolute;
    inset: 0;
    z-index: 5;
    pointer-events: none;
    opacity: 0.04;
    background-image: url("data:image/svg+xml;utf8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='g'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' seed='11' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='160' height='160' filter='url(%23g)'/%3E%3C/svg%3E");
  }

  /* Muesca semicircular de 6 px en el borde izquierdo */
  .card::before {
    content: "";
    position: absolute;
    left: -7px;
    top: calc(100% - 88px);
    transform: translateY(-50%);
    z-index: 6;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 1px solid var(--ink-38);
    background: var(--paper-lift, #f4efe2);
  }

  /* ---------- Hero ---------- */
  .hero {
    position: relative;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
    display: flex;
    align-items: center;
  }

  .hero-inner {
    display: flex;
    gap: 8px;
    width: 100%;
    padding: 14px 12px;
    align-items: flex-start;
  }

  .hero-left {
    flex: 1 1 auto;
    min-width: 0;
  }

  .cve-id {
    display: block;
    padding: 2px 0;
    min-height: 24px;
    border: none;
    background: none;
    font-family: "Courier Prime", monospace;
    font-size: 24px;
    font-weight: 700;
    letter-spacing: 0.01em;
    color: var(--ink);
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .cve-id:hover {
    text-decoration: underline;
  }

  .cve-id:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: 1px;
  }

  .product {
    margin: 2px 0 0;
    font-size: 13.5px;
    font-weight: 700;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .vector {
    margin: 8px 0 0;
    font-family: "IBM Plex Mono", monospace;
    font-size: 10px;
    letter-spacing: 0.02em;
    color: var(--ink-62);
    white-space: nowrap;
    overflow: hidden;
  }

  .summary {
    margin: 8px 0 0;
    font-size: 12px;
    line-height: 1.42;
    color: var(--ink-62);
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .cache-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 0 0;
  }

  .cache-age {
    font-family: "IBM Plex Mono", monospace;
    font-size: 9px;
    letter-spacing: 0.12em;
    color: var(--ink-62);
  }

  .hero-right {
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    padding-top: 2px;
  }

  .score-block {
    display: flex;
    align-items: baseline;
    gap: 4px;
  }

  .score {
    font-family: "Handjet", monospace;
    font-weight: 700;
    font-size: 64px;
    line-height: 0.9;
    color: var(--ink);
  }

  .score-of {
    font-family: "IBM Plex Mono", monospace;
    font-size: 10px;
    color: var(--ink-62);
  }

  .score-meta {
    margin: 2px 0 0;
    font-family: "IBM Plex Mono", monospace;
    font-size: 8.5px;
    font-weight: 500;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--ink-62);
  }

  .stamp-pos {
    margin: 12px 2px 0 0;
    padding: 0 2px;
    display: flex;
    justify-content: flex-end;
  }

  /* Barrido de tinta al refrescar: 90 ms, bloque sólido, sin degradado */
  .sweep {
    position: absolute;
    inset: 0;
    z-index: 7;
    pointer-events: none;
    overflow: hidden;
  }

  .sweep::after {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 26%;
    background: var(--ink);
    opacity: 0.12;
    animation: sweep-across 90ms linear both;
  }

  @keyframes sweep-across {
    from {
      transform: translateX(-110%) skewX(-12deg);
    }
    to {
      transform: translateX(420%) skewX(-12deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .sweep::after {
      animation: none;
      opacity: 0;
    }
  }

  /* ---------- Estado vacío ---------- */
  .empty {
    grid-row: 2 / 5;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }

  .empty-date {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.08em;
  }

  .empty-sub {
    margin: 0;
    font-family: "IBM Plex Mono", monospace;
    font-size: 9px;
    letter-spacing: 0.2em;
    color: var(--ink-62);
  }

  .empty-note {
    margin: 0;
    padding: 0 24px;
    font-size: 10.5px;
    color: var(--ink-62);
    text-align: center;
  }

  /* ---------- Error sin caché ---------- */
  .err {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 0 32px;
    text-align: center;
  }

  .err-label {
    margin: 0;
    font-family: "IBM Plex Mono", monospace;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.18em;
  }

  .err-msg {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--ink-62);
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
  }

  .retry {
    min-width: 120px;
    min-height: 28px;
    border: var(--rule);
    background: transparent;
    font-family: "IBM Plex Mono", monospace;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.18em;
    color: var(--ink);
    cursor: pointer;
  }

  .retry:hover {
    background: var(--paper-deep);
  }

  .retry:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: 2px;
  }

  /* ---------- Esqueleto ---------- */
  .skel-head {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 30px;
    padding: 0 12px;
    border-bottom: var(--rule);
  }

  .skel-head .w-title {
    width: 96px;
  }

  .skel-head .w-meta {
    width: 150px;
    margin-left: auto;
  }

  .skel-bar {
    display: inline-block;
    height: 12px;
    background: var(--paper-deep);
    border-radius: 1px;
  }

  .skel-hero {
    display: flex;
    align-items: center;
    min-height: 0;
    min-width: 0;
    padding: 14px 12px;
  }

  .skel-left {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }

  .skel-id-row {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 28px;
    margin-bottom: 8px;
  }

  .skel-cursor {
    flex: 0 0 auto;
    color: var(--ink);
    font-family: "Courier Prime", monospace;
    font-size: 24px;
    font-weight: 700;
    line-height: 1;
    animation: blink 1s steps(1) infinite;
  }

  .skel-left .w-id {
    width: 262px;
    height: 20px;
  }

  @keyframes blink {
    50% {
      opacity: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .skel-cursor {
      animation: none;
    }
  }

  .skel-left .w-product {
    width: 120px;
    height: 13px;
    margin-bottom: 9px;
  }

  .skel-left .w-vector {
    width: 210px;
    height: 10px;
    margin-bottom: 10px;
  }

  .skel-left .w-sum {
    width: 92%;
    height: 13px;
    margin-bottom: 6px;
  }

  .skel-left .w-sum.short {
    width: 64%;
    margin-bottom: 0;
  }

  .skel-right {
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    padding-left: 8px;
  }

  .skel-score {
    width: 119px;
    height: 60px;
    background: var(--paper-deep);
  }

  .skel-right .w-meta2 {
    width: 112px;
    height: 9px;
    margin-top: 8px;
  }

  .skel-grid {
    display: grid;
    grid-template-columns: 1.35fr 0.75fr 1fr 1.1fr;
    border-top: var(--rule);
  }

  .skel-cell {
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 7px 10px 8px 12px;
    border-right: var(--rule);
  }

  .skel-cell:last-child {
    border-right: none;
  }

  .skel-cell .w-label {
    width: 60%;
    height: 8px;
  }

  .skel-cell .w-value {
    width: 80%;
    height: 12px;
  }

  .skel-strip {
    display: flex;
    align-items: stretch;
    height: 40px;
    border-top: var(--rule);
  }

  .skel-frame {
    display: flex;
    align-items: center;
    gap: 7px;
    flex: 1 1 0;
    padding: 4px 8px 4px 12px;
    border-right: var(--rule);
  }

  .skel-frame:last-child {
    border-right: none;
  }

  .skel-frame .w-no {
    width: 14px;
    height: 9px;
  }

  .skel-frame-col {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .skel-frame .w-fid {
    width: 88px;
    height: 10px;
  }

  .skel-frame .w-fscore {
    width: 26px;
    height: 9px;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
</style>
