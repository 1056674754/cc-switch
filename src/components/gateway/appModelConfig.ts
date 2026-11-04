import type { AppModelConfig, GatewayCatalogEntry } from "@/types/gateway";

/** 槽位定义：只有 Claude Code 有四档，其余 app 没有槽位。 */
export interface SlotDefinition {
  key: string;
  /** 界面上显示的名字。 */
  label: string;
  /** 写进客户端文件的字段说明（提示用）。 */
  hint: string;
}

export function slotDefinitions(appType: string): SlotDefinition[] {
  if (appType !== "claude") return [];
  return [
    { key: "fable", label: "Fable", hint: "ANTHROPIC_DEFAULT_FABLE_MODEL" },
    { key: "opus", label: "Opus", hint: "ANTHROPIC_DEFAULT_OPUS_MODEL" },
    { key: "sonnet", label: "Sonnet", hint: "ANTHROPIC_DEFAULT_SONNET_MODEL" },
    { key: "haiku", label: "Haiku", hint: "ANTHROPIC_DEFAULT_HAIKU_MODEL" },
  ];
}

/** 设置一个槽位；传 undefined 表示清空（不发空串）。 */
export function applySlotChange(
  slots: Record<string, string> | undefined,
  key: string,
  reference: string | undefined,
): Record<string, string> {
  const next = { ...(slots ?? {}) };
  if (reference) next[key] = reference;
  else delete next[key];
  return next;
}

export function addAllowedModel(
  allowed: string[] | undefined,
  reference: string,
): string[] {
  const next = [...(allowed ?? [])];
  if (!next.includes(reference)) next.push(reference);
  return next;
}

export function removeAllowedModel(
  allowed: string[] | undefined,
  reference: string,
): string[] {
  return (allowed ?? []).filter((item) => item !== reference);
}

/**
 * 保存前的本地校验，规则与后端 `validate_app_model_config` 一致：
 * 引用都要在目录里；允许列表非空时默认必填且必须在列表内。
 * 返回人类可读的错误，空数组 = 通过。后端仍会再校验一次。
 */
export function validateDraft(
  config: AppModelConfig,
  catalog: GatewayCatalogEntry[],
): string[] {
  const known = new Set(catalog.map((entry) => entry.reference));
  const errors: string[] = [];
  if (config.default && !known.has(config.default)) {
    errors.push(`默认模型不在目录里：${config.default}`);
  }
  for (const [slot, reference] of Object.entries(config.slots ?? {})) {
    if (!known.has(reference)) {
      errors.push(`槽位 ${slot} 的模型不在目录里：${reference}`);
    }
  }
  for (const reference of config.allowed ?? []) {
    if (!known.has(reference)) {
      errors.push(`允许列表里的模型不在目录里：${reference}`);
    }
  }
  const allowed = config.allowed ?? [];
  if (allowed.length > 0) {
    if (!config.default) {
      errors.push("允许列表非空时必须指定默认模型");
    } else if (!allowed.includes(config.default)) {
      errors.push(`默认模型不在允许列表里：${config.default}`);
    }
  }
  return errors;
}
