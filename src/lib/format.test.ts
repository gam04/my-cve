import { describe, expect, it } from "vitest";
import { clampSummary, formatEpss, formatScore, formatVector, relativeTime } from "./format";

describe("formatScore", () => {
  it("formatea con un decimal", () => {
    expect(formatScore(9.8)).toBe("9.8");
    expect(formatScore(10)).toBe("10.0");
    expect(formatScore(0)).toBe("0.0");
    expect(formatScore(8.14)).toBe("8.1");
  });

  it("satura en 0–10", () => {
    expect(formatScore(11)).toBe("10.0");
    expect(formatScore(-1)).toBe("0.0");
  });

  it("no numérico → guion", () => {
    expect(formatScore(Number.NaN)).toBe("—");
  });
});

describe("formatEpss", () => {
  it("convierte 0–1 a porcentaje con un decimal", () => {
    expect(formatEpss(0.00526)).toBe("0.5 %");
    expect(formatEpss(0.0)).toBe("0.0 %");
    expect(formatEpss(0.25)).toBe("25.0 %");
  });

  it("null → guion", () => {
    expect(formatEpss(null)).toBe("—");
  });
});

describe("formatVector", () => {
  it("separa base e impacto con ·", () => {
    expect(formatVector("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H")).toBe(
      "AV:N AC:L PR:N UI:N · C:H I:H A:H",
    );
  });

  it("funciona con v4.0 y sin S legible", () => {
    expect(formatVector("CVSS:4.0/AV:N/AC:L/AT:N/PR:N/UI:N/C:H/I:H/A:H")).toBe(
      "AV:N AC:L PR:N UI:N · C:H I:H A:H",
    );
  });

  it("vacío → guion", () => {
    expect(formatVector("")).toBe("—");
  });
});

describe("relativeTime", () => {
  const now = Date.parse("2026-11-19T16:02:11Z");

  it("minutos", () => {
    expect(relativeTime("2026-11-19T15:20:11Z", now)).toBe("hace 42 min");
    expect(relativeTime("2026-11-19T16:02:00Z", now)).toBe("hace menos de 1 min");
  });

  it("horas y días", () => {
    expect(relativeTime("2026-11-19T13:02:11Z", now)).toBe("hace 3 h");
    expect(relativeTime("2026-11-17T16:02:11Z", now)).toBe("hace 2 d");
  });

  it("tolera ISO sin Z (lo trata como UTC)", () => {
    expect(relativeTime("2026-11-19T15:20:11.000", now)).toBe("hace 42 min");
  });

  it("inválido → guion", () => {
    expect(relativeTime("no-es-fecha", now)).toBe("—");
  });
});

describe("clampSummary", () => {
  it("no recorta si cabe", () => {
    expect(clampSummary("Hola mundo", 20)).toBe("Hola mundo");
  });

  it("recorta por palabra completa", () => {
    expect(clampSummary("palabra1 palabra2 palabra3", 20)).toBe("palabra1 palabra2…");
  });

  it("respeta máximos pequeños", () => {
    expect(clampSummary("aa bb cc", 4)).toBe("aa…");
    expect(clampSummary("", 10)).toBe("");
  });
});
