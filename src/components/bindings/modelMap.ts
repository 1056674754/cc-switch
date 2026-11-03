import type {
  SupplierEndpoint,
  SupplierEndpointBinding,
} from "@/lib/api/suppliers";

/**
 * 模型映射的六个固定槽位。
 *
 * 键名与后端写配置时用的 env 键一一对应（ANTHROPIC_DEFAULT_*_MODEL 一族 +
 * CLAUDE_CODE_SUBAGENT_MODEL + ANTHROPIC_MODEL），所以这里不能改名、不能增删：
 * 界面写进去的槽位，下次对账要能原样读回来。
 */
export const MODEL_MAP_SLOTS = [
  "haiku",
  "sonnet",
  "opus",
  "fable",
  "subagent",
  "default",
] as const;

export type ModelMapSlot = (typeof MODEL_MAP_SLOTS)[number];

/** 编辑器里的草稿：六个槽位都在，值可以是空串。 */
export type ModelMapDraft = Record<ModelMapSlot, string>;

/** 只有 claude 系（配置里有 `env` 对象）能写模型字段，其余 app 后端会拒绝。 */
export function supportsModelMap(app: string): boolean {
  return app === "claude" || app === "claude-desktop";
}

/** 空草稿：六个槽位都空，保存后等于清掉全部映射。 */
export function emptyModelMapDraft(): ModelMapDraft {
  return {
    haiku: "",
    sonnet: "",
    opus: "",
    fable: "",
    subagent: "",
    default: "",
  };
}

/**
 * 后端返回的映射补齐成六槽位草稿。
 *
 * 后端只返回配过的槽位（没配时甚至是空对象），所以缺的槽位按空串处理；
 * 表里多出来的未知键直接忽略——界面不认识，写回去也没意义。
 */
export function toModelMapDraft(
  modelMap: Record<string, string> | null | undefined,
): ModelMapDraft {
  const draft = emptyModelMapDraft();
  if (!modelMap) return draft;
  for (const slot of MODEL_MAP_SLOTS) {
    const value = modelMap[slot];
    if (typeof value === "string" && value.trim()) {
      draft[slot] = value.trim();
    }
  }
  return draft;
}

/**
 * 草稿 → 提交给后端的 map。
 *
 * 只带非空槽位：空槽位在后端同样是「删掉这个 env 键」，少传几个不会改变语义，
 * 但请求体里不会出现一堆空串。
 */
export function toModelMapPayload(
  draft: ModelMapDraft,
): Record<string, string> {
  const payload: Record<string, string> = {};
  for (const slot of MODEL_MAP_SLOTS) {
    const value = draft[slot]?.trim();
    if (value) {
      payload[slot] = value;
    }
  }
  return payload;
}

/**
 * 一行摘要：`haiku → kimi-k2 · sonnet → kimi-k2-pro`。
 *
 * 没有配置时返回空串，由调用方显示「未配置」的本地化文案。
 */
export function summarizeModelMap(
  modelMap: Record<string, string> | null | undefined,
): string {
  if (!modelMap) return "";
  const parts: string[] = [];
  for (const slot of MODEL_MAP_SLOTS) {
    const value = modelMap[slot]?.trim();
    if (value) {
      parts.push(`${slot} → ${value}`);
    }
  }
  return parts.join(" · ");
}

/**
 * 解析 `"<app>:<providerId>"`。
 *
 * app 类型里不会有冒号，providerId 可能带，所以按第一个冒号切。
 */
export function parseConsumer(
  consumer: string,
): { appType: string; providerId: string } | null {
  const separator = consumer.indexOf(":");
  if (separator <= 0) return null;
  const providerId = consumer.slice(separator + 1);
  if (!providerId) return null;
  return { appType: consumer.slice(0, separator), providerId };
}

/**
 * 找某个 app 在这个端点上的接入明细。
 *
 * 同一个 app 在同一个端点上正常只有一条；给了 providerId 就先精确匹配，
 * 对不上时退回第一条同 app 的明细（例如库刚对账、绑定还没刷新）。
 */
export function findBinding(
  endpoint: SupplierEndpoint,
  appType: string,
  providerId?: string,
): SupplierEndpointBinding | undefined {
  const bindings = endpoint.bindings ?? [];
  const byApp = bindings.filter((binding) => binding.appType === appType);
  if (providerId) {
    const exact = byApp.find((binding) => binding.providerId === providerId);
    if (exact) return exact;
  }
  return byApp[0];
}
