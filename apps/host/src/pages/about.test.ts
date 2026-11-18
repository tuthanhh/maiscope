import { describe, expect, it } from "vitest";
import page from "./about.vue?raw";

// The About page is what most people read about maiscope's licence. It must
// match LICENSE and ADR-0027 (visualizer-majdata, final review).

describe("About page", () => {
  it("states maiscope's licence, GPL-3.0-or-later", () => {
    expect(page).toContain("GPL-3.0-or-later");
    expect(page).not.toMatch(/MIT License/);
  });

  it("credits MajdataPlay and points at the third-party notices and the source", () => {
    expect(page).toContain("https://github.com/TeamMajdata/MajdataPlay");
    expect(page).toContain("/THIRD_PARTY_NOTICES.txt");
    expect(page).toContain("config.sourceCodeUrl");
  });
});
