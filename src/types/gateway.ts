/** 网关模型目录与每 app 模型配置（P1 后端命令的前端类型）。 */

/** 目录里的一条：模型引用 = `<供应商名>/<模型 id>`。 */
export interface GatewayCatalogEntry {
  reference: string;
  upstreamId: string;
  upstreamName: string;
  modelId: string;
  displayName?: string | null;
  oneM?: boolean;
  window?: number | null;
  /** 能承载这个模型的端点（按协议区分）。 */
  endpointIds: string[];
  /** 贡献这个模型的来源，形如 `claude:CTYun`。 */
  sources: string[];
}

/** 每 app 的模型配置，对齐 CCR `ProfileConfig` 的模型部分。 */
export interface AppModelConfig {
  /** 默认模型引用；允许列表非空时必填且必须在列表内。 */
  default?: string;
  /** 槽位 → 模型引用（claude 四档、codex 两档，其它 app 为空）。 */
  slots?: Record<string, string>;
  /** 允许模型列表；空 = 不限制。 */
  allowed?: string[];
  /** 增强路由 / 配置级路由，原样透传。 */
  routing?: unknown;
}

/** 目录里的一条人工覆盖：改名或隐藏（目录本身仍由 provider 行派生）。 */
export interface ModelOverride {
  upstreamId: string;
  modelId: string;
  displayName?: string | null;
  hidden: boolean;
}

/** Stack 配置迁移结果。 */
export interface AppMigration {
  app: string;
  status: "migrated" | "noStack" | "alreadyConfigured" | "unsupported" | "nothingToMigrate";
  default?: string | null;
  models: number;
}
