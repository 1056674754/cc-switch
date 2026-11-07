import { providerPresets as claudePresets } from "@/config/claudeProviderPresets";
import { codexProviderPresets } from "@/config/codexProviderPresets";
import { geminiProviderPresets } from "@/config/geminiProviderPresets";

/**
 * 供应商级预设：从各应用的「预设供应商」里抽出来的**上游**信息。
 *
 * 供应商 = 服务根 + 上游协议，和 app 无关；应用页的预设里夹着 app 专属配置（env 形状、
 * 模型槽位、认证流程），这里只取三样能跨 app 复用的东西：名字、服务根地址、上游协议。
 * 同一家服务在多个应用预设里各写一份，按「协议 + 地址」去重。
 */
export type SupplierPresetProtocol =
  | "anthropic"
  | "openai_chat"
  | "openai_responses"
  | "gemini_native";

export interface SupplierPreset {
  /** `protocol|url`，稳定 id */
  id: string;
  name: string;
  baseUrl: string;
  protocol: SupplierPresetProtocol;
  /** 预设来自哪个应用的预设表，用于展示来源 */
  source: "claude" | "codex" | "gemini";
  websiteUrl?: string;
  apiKeyUrl?: string;
}

function normalizeUrl(raw: string): string {
  return raw.trim().replace(/\/+$/, "");
}

function isUsableUrl(raw: string): boolean {
  return /^https?:\/\//i.test(raw.trim());
}

function presetNameKey(name: string): string {
  return name.trim().toLowerCase();
}

function codexBaseUrl(preset: {
  config?: string;
  endpointCandidates?: string[];
}): string {
  const candidates = preset.endpointCandidates ?? [];
  const direct = candidates.find((url) => isUsableUrl(url));
  if (direct) return direct;
  const match = /base_url\s*=\s*"([^"]+)"/.exec(preset.config ?? "");
  return match?.[1] ?? "";
}

function claudePresetsToSupplier(): SupplierPreset[] {
  const result: SupplierPreset[] = [];
  for (const preset of claudePresets) {
    if (preset.hidden || preset.isOfficial || preset.requiresOAuth) continue;
    const env = (
      preset.settingsConfig as { env?: Record<string, unknown> } | undefined
    )?.env;
    const raw =
      typeof env?.ANTHROPIC_BASE_URL === "string" ? env.ANTHROPIC_BASE_URL : "";
    if (!isUsableUrl(raw)) continue;
    result.push({
      id: `${preset.apiFormat ?? "anthropic"}|${normalizeUrl(raw)}`,
      name: preset.name,
      baseUrl: normalizeUrl(raw),
      protocol: (preset.apiFormat ?? "anthropic") as SupplierPresetProtocol,
      source: "claude",
      websiteUrl: preset.websiteUrl,
      apiKeyUrl: preset.apiKeyUrl,
    });
  }
  return result;
}

function codexPresetsToSupplier(): SupplierPreset[] {
  const result: SupplierPreset[] = [];
  for (const preset of codexProviderPresets) {
    if (preset.hidden || preset.isOfficial || preset.requiresOAuth) continue;
    const raw = codexBaseUrl(preset);
    if (!isUsableUrl(raw)) continue;
    result.push({
      id: `${preset.apiFormat ?? "openai_responses"}|${normalizeUrl(raw)}`,
      name: preset.name,
      baseUrl: normalizeUrl(raw),
      protocol: (preset.apiFormat ??
        "openai_responses") as SupplierPresetProtocol,
      source: "codex",
      websiteUrl: preset.websiteUrl,
      apiKeyUrl: preset.apiKeyUrl,
    });
  }
  return result;
}

function geminiPresetsToSupplier(): SupplierPreset[] {
  const result: SupplierPreset[] = [];
  for (const preset of geminiProviderPresets) {
    const flags = preset as { hidden?: boolean; isOfficial?: boolean };
    if (flags.hidden || flags.isOfficial) continue;
    const env = (
      preset.settingsConfig as { env?: Record<string, unknown> } | undefined
    )?.env;
    const raw =
      (typeof preset.baseURL === "string" && preset.baseURL) ||
      (typeof env?.GOOGLE_GEMINI_BASE_URL === "string"
        ? env.GOOGLE_GEMINI_BASE_URL
        : "");
    if (!isUsableUrl(raw)) continue;
    result.push({
      id: `gemini_native|${normalizeUrl(raw)}`,
      name: preset.name,
      baseUrl: normalizeUrl(raw),
      protocol: "gemini_native",
      source: "gemini",
      websiteUrl: preset.websiteUrl,
      apiKeyUrl: preset.apiKeyUrl,
    });
  }
  return result;
}

function buildPresets(): SupplierPreset[] {
  // 去重时保留先出现的：claude 系预设描述最全（名称/官网/key 链接），codex/gemini 补漏。
  const seen = new Set<string>();
  const result: SupplierPreset[] = [];
  for (const preset of [
    ...claudePresetsToSupplier(),
    ...codexPresetsToSupplier(),
    ...geminiPresetsToSupplier(),
  ]) {
    const key = preset.id.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    result.push(preset);
  }
  return result.sort((a, b) =>
    presetNameKey(a.name).localeCompare(presetNameKey(b.name)),
  );
}

export const supplierPresets: SupplierPreset[] = buildPresets();

/** 预设搜索：名称、地址、协议都参与匹配（大小写不敏感）。 */
export function searchSupplierPresets(
  keyword: string,
  presets: SupplierPreset[] = supplierPresets,
): SupplierPreset[] {
  const trimmed = keyword.trim().toLowerCase();
  if (!trimmed) return presets;
  return presets.filter(
    (preset) =>
      preset.name.toLowerCase().includes(trimmed) ||
      preset.baseUrl.toLowerCase().includes(trimmed) ||
      preset.protocol.toLowerCase().includes(trimmed),
  );
}
