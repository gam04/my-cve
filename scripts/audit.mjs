import { chromium } from "playwright";

const BASE = process.env.BASE ?? "http://localhost:1420";
const states = ["normal", "kev", "nocritical", "empty", "offline", "loading"];

const browser = await chromium.launch();

for (const state of states) {
  const context = await browser.newContext({
    viewport: { width: 440, height: 310 },
    deviceScaleFactor: 2,
  });
  const page = await context.newPage();
  await page.goto(`${BASE}/?fixture=${state}`, { waitUntil: "networkidle" });
  await page.waitForTimeout(400);

  const report = await page.evaluate(() => {
    const card = document.querySelector("main")?.getBoundingClientRect();
    if (!card) return { error: "no main" };
    const out = { clipped: [], truncated: [], overlaps: [] };

    for (const el of document.querySelectorAll("main *")) {
      const r = el.getBoundingClientRect();
      if (r.width === 0 || r.height === 0) continue;
      const name =
        (el.className && typeof el.className === "string"
          ? "." + el.className.trim().split(/\s+/).join(".")
          : el.tagName.toLowerCase()) + ` "${(el.textContent ?? "").trim().slice(0, 22)}"`;

      if (r.right > card.right + 0.5 || r.bottom > card.bottom + 0.5 || r.left < card.left - 0.5) {
        out.clipped.push(
          `${name} → x:${Math.round(r.left)}..${Math.round(r.right)} y:${Math.round(r.top)}..${Math.round(r.bottom)}`,
        );
      }
      if (
        (el instanceof HTMLElement && el.scrollWidth > el.clientWidth + 1 && el.clientWidth > 0) ||
        (el instanceof HTMLElement && el.scrollHeight > el.clientHeight + 1 && el.clientHeight > 0 && getComputedStyle(el).overflow === "hidden")
      ) {
        out.truncated.push(`${name} (${el.clientWidth}x${el.clientHeight} vs ${el.scrollWidth}x${el.scrollHeight})`);
      }
    }

    const stamp = document.querySelector(".stamp-pos, [class*='stamp']");
    const score = document.querySelector(".score");
    if (stamp && score) {
      const a = stamp.getBoundingClientRect();
      const b = score.getBoundingClientRect();
      const ox = Math.max(0, Math.min(a.right, b.right) - Math.max(a.left, b.left));
      const oy = Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top));
      if (ox > 1 && oy > 1) out.overlaps.push(`sello x score: ${Math.round(ox)}x${Math.round(oy)} px`);
    }
    return out;
  });

  console.log(`\n=== ${state} ===`);
  if (report.error) console.log(report.error);
  for (const kind of ["clipped", "truncated", "overlaps"]) {
    const list = report[kind] ?? [];
    console.log(`${kind}: ${list.length}`);
    for (const line of list.slice(0, 12)) console.log("   " + line);
  }
  await context.close();
}

await browser.close();
