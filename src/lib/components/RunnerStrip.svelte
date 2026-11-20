<script lang="ts">
  /**
   * Tira de microficha: hasta 3 runners como marcos numerados
   * (02 03 04) con ID y score en dos líneas, separados por reglas
   * verticales de 1 px. El ID se muestra completo.
   */
  import type { DailyPick } from "../types";
  import { formatScore } from "../format";

  let { runners }: { runners: DailyPick[] } = $props();
</script>

<footer class="strip" aria-label="Siguientes vulnerabilidades del día">
  {#if runners.length === 0}
    <span class="strip-empty">— SIN MÁS REGISTROS HOY —</span>
  {:else}
    {#each runners as r, i (r.id)}
      <span class="frame" title={`${r.id} · ${r.severity}`}>
        <span class="frame-no">{String(i + 2).padStart(2, "0")}</span>
        <span class="frame-col">
          <span class="frame-id">{r.id}</span>
          <span class="frame-score tabular">{formatScore(r.score)}</span>
        </span>
      </span>
    {/each}
  {/if}
</footer>

<style>
  .strip {
    display: flex;
    align-items: stretch;
    height: 40px;
    border-top: var(--rule);
  }

  .strip-empty {
    align-self: center;
    width: 100%;
    text-align: center;
    font-family: "IBM Plex Mono", monospace;
    font-size: 9px;
    letter-spacing: 0.18em;
    color: var(--ink-62);
  }

  .frame {
    display: flex;
    align-items: center;
    gap: 7px;
    flex: 1 1 0;
    min-width: 0;
    padding: 4px 8px 4px 10px;
    border-right: var(--rule);
  }

  .frame:first-child {
    padding-left: 12px;
  }

  .frame:last-child {
    border-right: none;
  }

  .frame-no {
    flex: 0 0 auto;
    font-family: "IBM Plex Mono", monospace;
    font-size: 8.5px;
    font-weight: 500;
    letter-spacing: 0.08em;
    color: var(--ink-62); /* AC-011: texto visible ≥ 4.5:1; jerarquía por tamaño, no por contraste */
  }

  .frame-col {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .frame-id {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 10px;
    font-weight: 700;
    line-height: 1.1;
    color: var(--ink);
  }

  .frame-score {
    flex: 0 0 auto;
    font-family: "IBM Plex Mono", monospace;
    font-size: 9.5px;
    font-weight: 500;
    line-height: 1.1;
    color: var(--ink-62);
  }
</style>
