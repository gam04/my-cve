<script lang="ts">
  /**
   * Encabezado de la ficha: título del estado, fecha UTC y control de refresco.
   * Es la única franja con `data-tauri-drag-region` (junto a la franja vacía).
   */
  let {
    title,
    meta,
    onRefresh,
    refreshing = false,
  }: {
    title: string;
    meta: string;
    onRefresh: () => void;
    refreshing?: boolean;
  } = $props();
</script>

<header class="head" data-tauri-drag-region>
  <span class="head-title" data-tauri-drag-region>{title}</span>
  <span class="head-meta tabular" data-tauri-drag-region>{meta}</span>
  <button
    class="refresh"
    onclick={onRefresh}
    disabled={refreshing}
    aria-label="Refrescar ahora"
    title="Refrescar ahora"
  >
    ↻
  </button>
</header>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 30px;
    padding: 0 0 0 12px;
    border-bottom: var(--rule);
    font-family: "IBM Plex Mono", monospace;
    font-size: 10px;
    letter-spacing: 0.09em;
    text-transform: uppercase;
  }

  .head-title {
    font-weight: 700;
    color: var(--ink);
    white-space: nowrap;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .head-meta {
    margin-left: auto;
    color: var(--ink-62);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .refresh {
    flex: 0 0 auto;
    width: 30px;
    height: 29px;
    border: none;
    border-left: var(--rule);
    background: transparent;
    color: var(--ink);
    font-family: "Courier Prime", monospace;
    font-size: 15px;
    line-height: 1;
    cursor: pointer;
  }

  .refresh:hover:not(:disabled) {
    background: var(--paper-deep);
  }

  .refresh:disabled {
    color: var(--ink-38);
    cursor: default;
  }

  .refresh:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: -2px;
  }
</style>
