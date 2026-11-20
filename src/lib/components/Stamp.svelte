<script lang="ts">
  /**
   * Sello de tinta roja (doble filete, rotado −7°) sobre el papel.
   * `variant="critical"` es el sello principal; `variant="cache"` es el
   * sello secundario de CACHÉ, más pequeño y apagado para no competir.
   */
  let {
    label,
    variant = "critical",
    animate = false,
  }: {
    label: string;
    variant?: "critical" | "cache";
    animate?: boolean;
  } = $props();
</script>

<span class="stamp stamp-{variant}" class:animate data-stamp={label}>
  <span class="stamp-inner">{label}</span>
</span>

<style>
  .stamp {
    display: inline-block;
    padding: 3px 12px 3px 8px;
    border: 2px solid var(--stamp);
    outline: 1px solid var(--stamp);
    outline-offset: 1.5px;
    border-radius: 2px;
    transform: rotate(-7deg);
    transform-origin: center;
    line-height: 1.15;
    mix-blend-mode: multiply;
    color: var(--stamp);
    font-family: "IBM Plex Mono", monospace;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.22em;
    white-space: nowrap;
    /* Máscara de grano: la tinta entra irregular en el papel. */
    -webkit-mask-image: url("data:image/svg+xml;utf8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='60'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' seed='7'/%3E%3CfeColorMatrix values='0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0.92 0.25'/%3E%3C/filter%3E%3Crect width='120' height='60' filter='url(%23n)'/%3E%3C/svg%3E");
    mask-image: url("data:image/svg+xml;utf8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='60'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' seed='7'/%3E%3CfeColorMatrix values='0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0.92 0.25'/%3E%3C/filter%3E%3Crect width='120' height='60' filter='url(%23n)'/%3E%3C/svg%3E");
  }

  .stamp-critical {
    font-size: 15px;
  }

  .stamp-cache {
    border-width: 1.5px;
    font-size: 10px;
    letter-spacing: 0.18em;
    transform: rotate(-7deg);
  }

  .stamp-animate {
    animation: stamp-in 160ms ease-out both;
  }

  @keyframes stamp-in {
    from {
      transform: rotate(-9.5deg) scale(1.4);
    }
    to {
      transform: rotate(-7deg) scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .stamp-animate {
      animation: none;
    }
  }
</style>
