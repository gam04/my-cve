/** Funciones puras de formato para la ficha. Sin estado, sin I/O. */

/** Score 0–10 con un decimal fijo: 9.8 → "9.8", 10 → "10.0". */
export function formatScore(n: number): string {
  if (!Number.isFinite(n)) return "—";
  const clamped = Math.min(10, Math.max(0, n));
  return clamped.toFixed(1);
}

/** EPSS 0–1 como porcentaje con un decimal; null → "—". */
export function formatEpss(n: number | null): string {
  if (n === null || !Number.isFinite(n)) return "—";
  const pct = Math.min(100, Math.max(0, n * 100));
  return `${pct.toFixed(1)} %`;
}

/**
 * Vector CVSS como línea de parámetros legible:
 * "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:C/C:H/I:H/A:H"
 *   → "AV:N AC:L PR:N UI:N · C:H I:H A:H"
 */
export function formatVector(v: string): string {
  if (!v) return "—";
  const tokens = v
    .replace(/^CVSS:[0-9.]+\//, "")
    .split("/")
    .filter(Boolean);
  const base: string[] = [];
  const impact: string[] = [];
  for (const t of tokens) {
    const key = t.slice(0, t.indexOf(":"));
    if (["AV", "AC", "PR", "UI"].includes(key)) base.push(t);
    else if (["C", "I", "A"].includes(key)) impact.push(t);
    // S, E, RL, RC… se omiten para mantener la línea corta.
  }
  const parts: string[] = [];
  if (base.length) parts.push(base.join(" "));
  if (impact.length) parts.push(impact.join(" "));
  return parts.join(" · ") || "—";
}

/** Normaliza ISO sin zona (p. ej. "2026-11-18T19:17:53.607") a UTC. */
function toUtcDate(iso: string): Date {
  const needsZ = /\d$/.test(iso.trim()) && !/(Z|[+-]\d{2}:?\d{2})$/.test(iso.trim());
  return new Date(needsZ ? `${iso.trim()}Z` : iso);
}

/** Antigüedad relativa en español: "hace 42 min", "hace 3 h", "hace 2 d". */
export function relativeTime(iso: string, now: number = Date.now()): string {
  const t = toUtcDate(iso).getTime();
  if (!Number.isFinite(t)) return "—";
  const min = Math.max(0, Math.floor((now - t) / 60_000));
  if (min < 1) return "hace menos de 1 min";
  if (min < 60) return `hace ${min} min`;
  const h = Math.floor(min / 60);
  if (h < 24) return `hace ${h} h`;
  return `hace ${Math.floor(h / 24)} d`;
}

/** Recorta un resumen por palabras completas, nunca a mitad de palabra. */
export function clampSummary(s: string, max: number): string {
  if (max <= 0) return "";
  if (s.length <= max) return s;
  const cut = s.lastIndexOf(" ", max);
  return `${s.slice(0, cut > 0 ? cut : max).trimEnd()}…`;
}
