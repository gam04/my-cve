import raw from "./fixtures-data.json";
import type { Board, FixtureName, Settings } from "./types";
import { DEFAULT_SETTINGS } from "./types";

const base = raw as unknown as Board;

function withPick(board: Board, score: number, severity: "HIGH"): Board {
  if (!board.pick) return board;
  return {
    ...board,
    pick: { ...board.pick, score, severity },
    totalCriticalToday: 0,
  };
}

export function fixtureBoard(name: FixtureName): Board {
  switch (name) {
    case "empty":
      return { ...base, pick: null, runnersUp: [], totalCriticalToday: 0 };
    case "nocritical":
      return withPick(base, 8.1, "HIGH");
    case "offline":
      return { ...base, stale: true, fetchError: "NVD no respondió (503)" };
    case "kev":
      // Pick explotado activamente: ejercita la fecha límite KEV en la rejilla.
      return base.pick
        ? {
            ...base,
            pick: {
              ...base.pick,
              kev: true,
              kevDueDate: "2026-11-21",
              epss: 0.91234,
              epssPercentile: 0.9987,
            },
          }
        : base;
    case "loading":
    case "normal":
    default:
      return base;
  }
}

export function fixtureSettings(): Settings {
  return { ...DEFAULT_SETTINGS };
}
