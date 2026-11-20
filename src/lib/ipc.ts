import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Board, FixtureName, Settings } from "./types";

const params = new URLSearchParams(location.search);
const fixtureParam = params.get("fixture") as FixtureName | null;

/** Fijado por `?fixture=`; el shell lo usa para quedarse en un estado concreto. */
export const fixtureName = fixtureParam;

export const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** En navegador (o con `?fixture=`) se sirve data local y se desactiva la red. */
export const useFixtures = fixtureParam !== null || !isTauri;

async function fixture(name: FixtureName): Promise<Board> {
  const mod = await import("./fixtures");
  return mod.fixtureBoard(name);
}

export async function boardToday(): Promise<Board> {
  if (useFixtures) {
    // `?fixture=loading` deja la ficha en el esqueleto, para diseñar ese estado.
    if (fixtureParam === "loading") return new Promise<Board>(() => {});
    return fixture(fixtureParam ?? "normal");
  }
  return invoke<Board>("board_today");
}

export async function refreshNow(): Promise<Board> {
  if (useFixtures) return fixture(fixtureParam ?? "normal");
  return invoke<Board>("refresh_now");
}

export async function settingsGet(): Promise<Settings> {
  if (useFixtures) return (await import("./fixtures")).fixtureSettings();
  return invoke<Settings>("settings_get");
}

export async function settingsSet(settings: Settings): Promise<Settings> {
  if (useFixtures) return settings;
  return invoke<Settings>("settings_set", { settings });
}

export async function openCve(id: string): Promise<void> {
  const url = `https://nvd.nist.gov/vuln/detail/${encodeURIComponent(id)}`;
  if (useFixtures) {
    window.open(url, "_blank", "noopener");
    return;
  }
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}

export async function onBoardUpdated(
  handler: (board: Board) => void,
): Promise<UnlistenFn> {
  if (useFixtures) return () => {};
  return listen<Board>("board:updated", (event) => handler(event.payload));
}

export async function onBoardError(
  handler: (payload: { message: string; lastGoodAt: string | null }) => void,
): Promise<UnlistenFn> {
  if (useFixtures) return () => {};
  return listen<{ message: string; lastGoodAt: string | null }>(
    "board:error",
    (event) => handler(event.payload),
  );
}

/** Muestra la ventana una vez que el primer frame ya está pintado (REQ-010). */
export async function showWindowWhenReady(): Promise<void> {
  if (!isTauri) return;
  requestAnimationFrame(() => {
    requestAnimationFrame(async () => {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().show();
    });
  });
}
