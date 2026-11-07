import { describe, expect, it } from "vitest";

import {
  searchSupplierPresets,
  supplierPresets,
} from "@/components/suppliers/supplierPresets";

describe("supplierPresets", () => {
  it("extracts supplier-level fields from app presets", () => {
    expect(supplierPresets.length).toBeGreaterThan(20);
    for (const preset of supplierPresets) {
      expect(preset.name.trim()).not.toBe("");
      expect(preset.baseUrl).toMatch(/^https?:\/\//);
      expect(preset.baseUrl.endsWith("/")).toBe(false);
      expect([
        "anthropic",
        "openai_chat",
        "openai_responses",
        "gemini_native",
      ]).toContain(preset.protocol);
    }
  });

  it("keeps one entry per protocol + address", () => {
    const ids = supplierPresets.map((preset) => preset.id.toLowerCase());
    expect(new Set(ids).size).toBe(ids.length);
    // 同一地址出现在多个应用的预设里时只留一条
    const urls = supplierPresets.map(
      (preset) => `${preset.protocol}|${preset.baseUrl.toLowerCase()}`,
    );
    expect(new Set(urls).size).toBe(urls.length);
  });

  it("covers the usual anthropic relay presets", () => {
    const moonshot = searchSupplierPresets("moonshot").find((preset) =>
      preset.baseUrl.includes("api.moonshot.cn"),
    );
    expect(moonshot?.protocol).toBe("anthropic");
    expect(moonshot?.name.toLowerCase()).toContain("kimi");
  });

  it("searches by name, address and protocol", () => {
    expect(searchSupplierPresets("deepseek").length).toBeGreaterThan(0);
    expect(
      searchSupplierPresets("api.deepseek.com").some((preset) =>
        preset.baseUrl.includes("api.deepseek.com"),
      ),
    ).toBe(true);
    expect(
      searchSupplierPresets("gemini_native").every(
        (preset) => preset.protocol === "gemini_native",
      ),
    ).toBe(true);
    expect(searchSupplierPresets("   ")).toHaveLength(supplierPresets.length);
    expect(searchSupplierPresets("no-such-supplier-xyz")).toHaveLength(0);
  });
});
