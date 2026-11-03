import { describe, expect, it } from "vitest";
import type {
  SupplierEndpoint,
  SupplierEndpointBinding,
} from "@/lib/api/suppliers";
import {
  emptyModelMapDraft,
  findBinding,
  MODEL_MAP_SLOTS,
  parseConsumer,
  summarizeModelMap,
  supportsModelMap,
  toModelMapDraft,
  toModelMapPayload,
} from "@/components/bindings/modelMap";

const endpoint = (
  overrides: Partial<SupplierEndpoint> = {},
): SupplierEndpoint => ({
  id: "ep-1",
  protocol: "anthropic",
  baseUrl: "https://api.acme.dev",
  apps: ["claude"],
  consumers: ["claude:p1"],
  credentialCount: 1,
  bindings: [],
  ...overrides,
});

describe("modelMap", () => {
  it("keeps the six slots in backend order", () => {
    expect([...MODEL_MAP_SLOTS]).toEqual([
      "haiku",
      "sonnet",
      "opus",
      "fable",
      "subagent",
      "default",
    ]);
  });

  it("fills a partial stored map into all six slots", () => {
    expect(
      toModelMapDraft({ sonnet: " kimi-k2 ", haiku: "kimi-k2-small" }),
    ).toEqual({
      haiku: "kimi-k2-small",
      sonnet: "kimi-k2",
      opus: "",
      fable: "",
      subagent: "",
      default: "",
    });
    // 后端没配过时给空对象，也要能补齐
    expect(toModelMapDraft({})).toEqual(emptyModelMapDraft());
    expect(toModelMapDraft(undefined)).toEqual(emptyModelMapDraft());
    // 不认识的键忽略：界面不认识，写回去也没意义
    expect(toModelMapDraft({ sonnet: "kimi-k2", unknown: "x" })).toEqual({
      ...emptyModelMapDraft(),
      sonnet: "kimi-k2",
    });
  });

  it("submits only the filled slots so empty ones clear the model field", () => {
    expect(
      toModelMapPayload({
        haiku: "",
        sonnet: " kimi-k3 ",
        opus: "   ",
        fable: "",
        subagent: "claude-code",
        default: "kimi-k2",
      }),
    ).toEqual({
      sonnet: "kimi-k3",
      subagent: "claude-code",
      default: "kimi-k2",
    });
    expect(toModelMapPayload(emptyModelMapDraft())).toEqual({});
  });

  it("summarizes the configured slots in slot order", () => {
    expect(
      summarizeModelMap({ sonnet: "kimi-k2", haiku: "kimi-k2-small" }),
    ).toBe("haiku → kimi-k2-small · sonnet → kimi-k2");
    expect(summarizeModelMap({})).toBe("");
    expect(summarizeModelMap(undefined)).toBe("");
    expect(summarizeModelMap({ opus: "   " })).toBe("");
  });

  it("parses `<app>:<providerId>` consumers", () => {
    expect(parseConsumer("claude:p1")).toEqual({
      appType: "claude",
      providerId: "p1",
    });
    expect(parseConsumer("claude-desktop:sup:ep-1")).toEqual({
      appType: "claude-desktop",
      providerId: "sup:ep-1",
    });
    expect(parseConsumer("claude:")).toBeNull();
    expect(parseConsumer("claude")).toBeNull();
  });

  it("finds the binding by app and provider, falling back to the app", () => {
    const bindings: SupplierEndpointBinding[] = [
      { appType: "claude", providerId: "p1", modelMap: { sonnet: "a" } },
      { appType: "claude", providerId: "p2", modelMap: { sonnet: "b" } },
      { appType: "codex", providerId: "p3", modelMap: {} },
    ];

    expect(
      findBinding(endpoint({ bindings }), "claude", "p2")?.modelMap,
    ).toEqual({ sonnet: "b" });
    // providerId 对不上（例如绑定刚对账完还没刷新）时退回同 app 的第一条
    expect(
      findBinding(endpoint({ bindings }), "claude", "gone")?.providerId,
    ).toBe("p1");
    expect(findBinding(endpoint({ bindings }), "gemini")).toBeUndefined();
    expect(findBinding(endpoint(), "claude")).toBeUndefined();
    // 老版本后端没有 bindings 字段时也不能炸
    const legacy = { ...endpoint(), bindings: undefined };
    expect(
      findBinding(legacy as unknown as SupplierEndpoint, "claude"),
    ).toBeUndefined();
  });

  it("only claude-family apps can write model fields", () => {
    expect(supportsModelMap("claude")).toBe(true);
    expect(supportsModelMap("claude-desktop")).toBe(true);
    expect(supportsModelMap("codex")).toBe(false);
    expect(supportsModelMap("gemini")).toBe(false);
    expect(supportsModelMap("hermes")).toBe(false);
  });
});
