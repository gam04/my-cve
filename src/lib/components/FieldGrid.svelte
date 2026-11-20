<script lang="ts">
  /**
   * Rejilla de campos del pick: EPSS, KEV, CWE(s) y hora de publicación.
   * Etiquetas en IBM Plex Mono versalitas; valores en Courier Prime.
   */
  import type { DailyPick } from "../types";
  import { formatEpss } from "../format";

  let { pick }: { pick: DailyPick } = $props();

  const publishedLabel = $derived.by(() => {
    const d = new Date(pick.published);
    if (Number.isNaN(d.getTime())) return "—";
    const hh = String(d.getUTCHours()).padStart(2, "0");
    const mm = String(d.getUTCMinutes()).padStart(2, "0");
    return `${hh}:${mm} UTC`;
  });

  const MESES = ["ENE", "FEB", "MAR", "ABR", "MAY", "JUN", "JUL", "AGO", "SEP", "OCT", "NOV", "DIC"];
  /** "2026-11-21" → "21 NOV" (español abreviado, mayúsculas, sin año). */
  const kevDueLabel = $derived.by(() => {
    if (!pick.kev || !pick.kevDueDate) return null;
    const [y, m, d] = pick.kevDueDate.split("-").map(Number);
    if (!y || !m || !d) return null;
    return `${String(d).padStart(2, "0")} ${MESES[m - 1]}`;
  });
</script>

<dl class="grid">
  <div class="cell">
    <dt>EPSS</dt>
    <dd class="tabular">{formatEpss(pick.epss)}{pick.epssPercentile !== null ? ` · P${Math.round(pick.epssPercentile * 100)}` : ""}</dd>
  </div>
  <div class="cell">
    <dt>KEV</dt>
    <dd class="tabular">
      {pick.kev ? "SÍ" : "NO"}{#if kevDueLabel}<span class="kev-due">· {kevDueLabel}</span>{/if}
    </dd>
  </div>
  <div class="cell">
    <dt>CWE</dt>
    <dd>{pick.cwes.length ? pick.cwes.join(" ") : "—"}</dd>
  </div>
  <div class="cell">
    <dt>PUBLICADO</dt>
    <dd class="tabular">{publishedLabel}</dd>
  </div>
</dl>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1.3fr 0.9fr 0.95fr 1.05fr;
    margin: 0;
    padding: 0;
    border-top: var(--rule);
  }

  .cell {
    padding: 7px 10px 8px 12px;
    border-right: var(--rule);
    min-width: 0;
  }

  .cell:last-child {
    border-right: none;
  }

  dt {
    font-family: "IBM Plex Mono", monospace;
    font-size: 8.5px;
    font-weight: 500;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--ink-62);
  }

  dd {
    margin: 2px 0 0;
    font-size: 12.5px;
    font-weight: 700;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .kev-due {
    margin-left: 4px;
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: var(--ink-62);
  }
</style>
