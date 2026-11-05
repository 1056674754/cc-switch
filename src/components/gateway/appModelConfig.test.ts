import { describe, expect, it } from "vitest";
import type { AppModelConfig, GatewayCatalogEntry } from "@/types/gateway";
import {
  addAllowedModel,
  applySlotChange,
  removeAllowedModel,
  slotDefinitions,
  validateDraft,
} from "./appModelConfig";

const catalog: GatewayCatalogEntry[] = [
  {
    reference: "CTYun/qwen3.8-max",
    upstreamId: "up-1",
    upstreamName: "CTYun",
    modelId: "qwen3.8-max",
    endpointIds: ["ep-1"],
    sources: ["claude:CTYun"],
  },
  {
    reference: "Kimi/kimi-k3",
    upstreamId: "up-2",
    upstreamName: "Kimi",
    modelId: "kimi-k3",
    endpointIds: ["ep-2"],
    sources: ["claude:Kimi"],
  },
];

describe("appModelConfig", () => {
  it("claude and claude-desktop have the fixed four slots", () => {
    const keys = ["fable", "opus", "sonnet", "haiku"];
    expect(slotDefinitions("claude").map((slot) => slot.key)).toEqual(keys);
    expect(slotDefinitions("claude-desktop").map((slot) => slot.key)).toEqual(keys);
    // Desktop 的提示写的是它模型菜单里的角色 route id
    expect(slotDefinitions("claude-desktop")[1].hint).toBe(
      "Desktop 菜单的 claude-opus-5",
    );
    expect(slotDefinitions("codex")).toEqual([]);
    expect(slotDefinitions("opencode")).toEqual([]);
  });

  it("sets and clears a slot without leaving empty strings", () => {
    const withSlot = applySlotChange(undefined, "haiku", "Kimi/kimi-k3");
    expect(withSlot).toEqual({ haiku: "Kimi/kimi-k3" });
    expect(applySlotChange(withSlot, "haiku", undefined)).toEqual({});
  });

  it("adds unique allowed models and removes them", () => {
    const once = addAllowedModel(undefined, "CTYun/qwen3.8-max");
    const twice = addAllowedModel(once, "CTYun/qwen3.8-max");
    expect(twice).toEqual(["CTYun/qwen3.8-max"]);
    expect(removeAllowedModel(twice, "CTYun/qwen3.8-max")).toEqual([]);
  });

  it("validates references and the allowed/default rule", () => {
    const ok: AppModelConfig = {
      default: "CTYun/qwen3.8-max",
      slots: { haiku: "Kimi/kimi-k3" },
      allowed: ["CTYun/qwen3.8-max", "Kimi/kimi-k3"],
    };
    expect(validateDraft(ok, catalog)).toEqual([]);

    expect(
      validateDraft({ default: "CTYun/not-there" }, catalog),
    ).toEqual(["默认模型不在目录里：CTYun/not-there"]);

    expect(
      validateDraft({ allowed: ["CTYun/qwen3.8-max"] }, catalog),
    ).toEqual(["允许列表非空时必须指定默认模型"]);

    expect(
      validateDraft(
        { default: "Kimi/kimi-k3", allowed: ["CTYun/qwen3.8-max"] },
        catalog,
      ),
    ).toEqual(["默认模型不在允许列表里：Kimi/kimi-k3"]);

    expect(
      validateDraft({ slots: { opus: "Gone/x" } }, catalog),
    ).toEqual(["槽位 opus 的模型不在目录里：Gone/x"]);
  });
});
