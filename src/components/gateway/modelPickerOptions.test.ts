import { describe, expect, it } from "vitest";
import type { GatewayCatalogEntry } from "@/types/gateway";
import {
  buildModelPickerGroups,
  filterModelGroups,
  selectedLabel,
} from "./modelPickerOptions";

function entry(
  reference: string,
  upstreamName: string,
  modelId: string,
  displayName?: string,
  window?: number,
): GatewayCatalogEntry {
  return {
    reference,
    upstreamId: `up-${upstreamName}`,
    upstreamName,
    modelId,
    displayName,
    window,
    endpointIds: ["ep-1"],
    sources: [`claude:${upstreamName}`],
  };
}

const entries = [
  entry("CTYun/deepseek-v4.1-flash-1017", "CTYun", "deepseek-v4.1-flash-1017", "DeepSeek V4.1", 200000),
  entry("CTYun/qwen3.8-max", "CTYun", "qwen3.8-max", undefined, 1_000_000),
  entry("Kimi/kimi-k3", "Kimi", "kimi-k3", "Kimi K3"),
];

describe("modelPickerOptions", () => {
  it("groups by supplier and keeps catalog order", () => {
    const groups = buildModelPickerGroups(entries);
    expect(groups.map((g) => g.upstreamName)).toEqual(["CTYun", "Kimi"]);
    expect(groups[0].options.map((o) => o.value)).toEqual([
      "CTYun/deepseek-v4.1-flash-1017",
      "CTYun/qwen3.8-max",
    ]);
  });

  it("uses display name, falls back to model id, and labels the window", () => {
    const [ctyun] = buildModelPickerGroups(entries);
    expect(ctyun.options[0].label).toBe("DeepSeek V4.1");
    expect(ctyun.options[0].description).toBe("CTYun · 200K");
    expect(ctyun.options[1].label).toBe("qwen3.8-max");
    expect(ctyun.options[1].description).toBe("CTYun · 1M");
  });

  it("filters by label, model id, reference or supplier, case-insensitively", () => {
    const groups = buildModelPickerGroups(entries);
    expect(filterModelGroups(groups, "kimi")[0].upstreamName).toBe("Kimi");
    expect(filterModelGroups(groups, "QWEN")[0].options).toHaveLength(1);
    expect(filterModelGroups(groups, "ctyun")).toHaveLength(1);
    expect(filterModelGroups(groups, "nothing")).toHaveLength(0);
    expect(filterModelGroups(groups, "   ")).toHaveLength(2);
  });

  it("marks a value that is not in the catalog as stale", () => {
    const groups = buildModelPickerGroups(entries);
    expect(selectedLabel(groups, "Kimi/kimi-k3")).toEqual({
      label: "Kimi K3",
      stale: false,
    });
    expect(selectedLabel(groups, "Old/removed-model")).toEqual({
      label: "Old/removed-model",
      stale: true,
    });
    expect(selectedLabel(groups, undefined)).toEqual({ label: "", stale: false });
  });
});
