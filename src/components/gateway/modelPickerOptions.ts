import type { GatewayCatalogEntry } from "@/types/gateway";

/** 选择器里一项：value 就是模型引用（写进配置的值）。 */
export interface ModelPickerOption {
  value: string;
  label: string;
  /** 次要说明：供应商 · 窗口。 */
  description?: string;
  oneM: boolean;
  /** 找不到对应目录项时为 true（配置里的旧引用/手工改过的值）。 */
  stale?: boolean;
}

export interface ModelPickerGroup {
  upstreamName: string;
  options: ModelPickerOption[];
}

function windowLabel(window?: number | null): string | undefined {
  if (!window) return undefined;
  if (window % 1_000_000 === 0) return `${window / 1_000_000}M`;
  return `${Math.round(window / 1_000)}K`;
}

function optionOf(entry: GatewayCatalogEntry): ModelPickerOption {
  const label = entry.displayName?.trim() || entry.modelId;
  const window = windowLabel(entry.window);
  return {
    value: entry.reference,
    label,
    description: window
      ? `${entry.upstreamName} · ${window}`
      : entry.upstreamName,
    oneM: Boolean(entry.oneM),
  };
}

/** 按供应商分组，组内保持目录顺序（目录本身已按上游 + 模型名排序）。 */
export function buildModelPickerGroups(
  entries: GatewayCatalogEntry[],
): ModelPickerGroup[] {
  const groups = new Map<string, ModelPickerOption[]>();
  for (const entry of entries) {
    const options = groups.get(entry.upstreamName) ?? [];
    options.push(optionOf(entry));
    groups.set(entry.upstreamName, options);
  }
  return [...groups.entries()].map(([upstreamName, options]) => ({
    upstreamName,
    options,
  }));
}

/**
 * 搜索过滤：label / 模型 id / 引用 / 供应商名任一命中即可，大小写不敏感。
 * 空查询返回原分组。
 */
export function filterModelGroups(
  groups: ModelPickerGroup[],
  query: string,
): ModelPickerGroup[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return groups;
  return groups
    .map((group) => ({
      upstreamName: group.upstreamName,
      options: group.options.filter((option) =>
        `${option.label}\n${option.value}\n${group.upstreamName}`
          .toLowerCase()
          .includes(needle),
      ),
    }))
    .filter((group) => group.options.length > 0);
}

/** 当前值对应的显示名；目录里没有就原样返回并标 stale。 */
export function selectedLabel(
  groups: ModelPickerGroup[],
  value?: string,
): { label: string; stale: boolean } {
  if (!value) return { label: "", stale: false };
  for (const group of groups) {
    const hit = group.options.find((option) => option.value === value);
    if (hit) return { label: hit.label, stale: false };
  }
  return { label: value, stale: true };
}
