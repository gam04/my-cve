import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";

const BASE = process.env.BASE ?? "http://localhost:1420";
const states = ["normal", "kev", "nocritical", "empty", "offline", "loading"];

await mkdir("shots", { recursive: true });

const browser = await chromium.launch();
let failures = 0;

for (const state of states) {
  for (const dpr of [1, 2]) {
    const context = await browser.newContext({
      viewport: { width: 440, height: 310 },
      deviceScaleFactor: dpr,
    });
    const page = await context.newPage();
    const errors = [];
    page.on("console", (m) => {
      if (m.type() === "error") errors.push(m.text());
    });
    page.on("pageerror", (e) => errors.push(String(e)));

    await page.goto(`${BASE}/?fixture=${state}`, { waitUntil: "networkidle" });
    await page.waitForTimeout(450);

    const box = await page.evaluate(() => {
      const el = document.querySelector("main");
      if (!el) return null;
      const r = el.getBoundingClientRect();
      return {
        w: r.width,
        h: r.height,
        scrollW: document.documentElement.scrollWidth,
        scrollH: document.documentElement.scrollHeight,
      };
    });

    await page.screenshot({ path: `shots/${state}@${dpr}x.png` });

    const overflow =
      !box || box.scrollH > 310 || box.scrollW > 440 || box.h > 310.5;
    if (overflow) failures += 1;

    console.log(
      `${state}@${dpr}x  card=${box?.w}x${box?.h}  scroll=${box?.scrollW}x${box?.scrollH}` +
        `${overflow ? "  <-- OVERFLOW" : ""}` +
        `${errors.length ? `  console: ${errors.join(" | ")}` : ""}`,
    );

    await context.close();
  }
}

await browser.close();
console.log(failures === 0 ? "OK: sin desbordes" : `FALLO: ${failures} capturas con desborde`);
process.exit(failures === 0 ? 0 : 1);
