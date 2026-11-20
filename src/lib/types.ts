export type Severity = "CRITICAL" | "HIGH" | "MEDIUM" | "LOW" | "NONE";

/** Un CVE normalizado, listo para pintar. Espejo exacto de `model::DailyPick`. */
export interface DailyPick {
  id: string;
  /** ISO-8601 UTC, p. ej. "2026-11-18T16:17:47.560Z" */
  published: string;
  /** 0.0 – 10.0 */
  score: number;
  /** "3.1" | "4.0" | "3.0" | "2.0" */
  scoreVersion: string;
  severity: Severity;
  vector: string;
  /** "nvd" | "cna" */
  scoreSource: string;
  epss: number | null;
  epssPercentile: number | null;
  kev: boolean;
  kevDueDate: string | null;
  cwes: string[];
  vendor: string | null;
  product: string | null;
  summary: string;
  rank: number;
}

/** El tablero del día. Espejo exacto de `model::Board`. */
export interface Board {
  /** "YYYY-MM-DD" en UTC */
  boardDate: string;
  /** ISO-8601 UTC del último refresco exitoso */
  generatedAt: string;
  totalCriticalToday: number;
  pick: DailyPick | null;
  runnersUp: DailyPick[];
  stale: boolean;
  fetchError: string | null;
}

export interface Settings {
  autoStart: boolean;
  notifyOnTopChange: boolean;
  refreshMinutes: 15 | 30 | 60;
  windowPos: { x: number; y: number } | null;
  pinned: boolean;
}

export const DEFAULT_SETTINGS: Settings = {
  autoStart: false,
  notifyOnTopChange: true,
  refreshMinutes: 30,
  windowPos: null,
  pinned: true,
};

export type FixtureName =
  | "normal"
  | "kev"
  | "empty"
  | "nocritical"
  | "offline"
  | "loading";
