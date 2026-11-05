import type { QueryClient, QueryKey } from "@tanstack/react-query";

/**
 * `ccswitch` 命令执行成功后，桌面端会 emit `cli-command-applied`（见
 * `src-tauri/src/control/mod.rs`），payload 带命令所属的模块（源文件名）和 app。
 *
 * 这里把「哪个模块的写操作可能改了哪些界面的数据」翻译成 React Query 的查询根，只失效
 * 相关域的数据；**不做** `invalidateQueries()` 全量失效——那会把当前页面上无关的查询
 * 甚至界面状态查询一起重拉，看着像“刷新了一下界面”，实际是偷懒。
 */
export interface CliCommandAppliedPayload {
  command: string;
  /** 命令所属源文件（模块），例如 `provider`、`mcp`。 */
  module?: string | null;
  /** 命令参数里的 app（如 `claude`），用来收窄到具体应用。 */
  app?: string | null;
  ok: boolean;
}

/**
 * 模块没识别出来时的兜底：只失效后端数据根，绝不碰界面状态/一次性查询。
 * 只在前端比桌面版旧（协议漂移）时才会走到这里。
 */
const FALLBACK_DATA_ROOTS: QueryKey[] = [
  ["providers"],
  ["settings"],
  ["profiles"],
  ["mcp"],
  ["skills"],
  ["prompts"],
  ["sessions"],
  ["usage"],
  ["suppliersOverview"],
  ["db-backups"],
  ["subscription"],
  ["proxyStatus"],
  ["proxyTakeoverStatus"],
  ["appProxyConfig"],
  ["globalProxyConfig"],
  ["gateway"],
  ["failoverQueue"],
  ["availableProvidersForFailover"],
  ["autoFailoverEnabled"],
  ["circuitBreakerConfig"],
  ["circuitBreakerStats"],
  ["providerHealth"],
  ["managed-auth-status"],
];

/**
 * 供应商改动的影响面按 app 收窄：改 claude 供应商不该顺手重拉 codex 的账号额度。
 * 只有 CLI 没给 app（例如 universal provider 批量同步）时才放宽到全部。
 */
function providerRoots(app: string | null): QueryKey[] {
  const roots: QueryKey[] = [["providers"], ["suppliersOverview"]];
  if (app) {
    roots.push([app, "currentState"]);
  }
  if (app === "claude") {
    roots.push(["managed-auth-status"], ["subscription"], ["claude_official"]);
  } else if (app === "codex") {
    roots.push(["codex_oauth"], ["subscription"]);
  } else if (app === "claude-desktop") {
    roots.push(
      ["claudeDesktopStatus"],
      ["claude-desktop-status"],
      ["claudeDesktopDefaultRoutes"],
    );
  } else if (app === "opencode") {
    roots.push(["opencodeLiveProviderIds"], ["opencodeDefaultModel"]);
  } else if (app === "pi") {
    roots.push(["pi", "currentState"], ["pi", "sessionDiscovery"]);
  } else if (!app) {
    roots.push(["managed-auth-status"], ["subscription"]);
  }
  return roots;
}

/**
 * 模块（后端源文件名）→ 该模块写操作影响的查询根。
 *
 * 只列后端数据查询；`misc`/`tray`/`lib` 这类不动数据的模块显式映射成空数组，
 * 避免落到兜底逻辑。
 */
const MODULE_QUERY_ROOTS: Record<string, QueryKey[]> = {
  // 供应商：见 providerRoots（要做 app 收窄，不放在表里）
  // MCP
  mcp: [["mcp"]],
  // 技能
  skill: [["skills"]],
  // 设置
  settings: [["settings"]],
  config: [["settings"]],
  // 配置档案
  profile: [["profiles"]],
  // 提示词
  prompt: [["prompts"]],
  // 用量
  usage: [["usage"]],
  // 会话
  session_manager: [["sessions"]],
  // 数据库备份 / 导入导出：动的是整库
  import_export: [
    ["db-backups"],
    ["backup-locations"],
    ["providers"],
    ["settings"],
    ["mcp"],
    ["skills"],
    ["prompts"],
    ["profiles"],
  ],
  // 本地路由
  proxy: [
    ["proxyStatus"],
    ["proxyTakeoverStatus"],
    ["appProxyConfig"],
    ["globalProxyConfig"],
    ["providers"],
    ["providerHealth"],
    ["circuitBreakerStats"],
  ],
  // 网关聚合
  gateway: [["gateway"], ["providers"]],
  // 故障转移 / 熔断
  failover: [
    ["failoverQueue"],
    ["availableProvidersForFailover"],
    ["autoFailoverEnabled"],
    ["circuitBreakerConfig"],
    ["circuitBreakerStats"],
    ["providerHealth"],
    ["providers"],
  ],
  // 供应商市场
  suppliers: [["suppliersOverview"], ["providers"]],
  // 全局代理
  global_proxy: [
    ["globalProxyUrl"],
    ["upstreamProxyStatus"],
    ["providerHealth"],
  ],
  // 同步状态
  webdav_sync: [["settings"]],
  s3_sync: [["settings"]],
  // 账号 / 订阅额度
  subscription: [
    ["subscription"],
    ["claude_official"],
    ["codex_oauth"],
    ["xai_oauth"],
    ["agy_oauth"],
    ["kimi_oauth"],
    ["copilot"],
  ],
  claude_account: [["claude_official"], ["subscription"]],
  codex_oauth: [["codex_oauth"], ["subscription"]],
  xai_oauth: [["xai_oauth"], ["subscription"]],
  agy_account: [["agy_oauth"], ["agy-running"], ["subscription"]],
  kimi_oauth: [["kimi_oauth"], ["subscription"]],
  copilot: [["copilot"], ["subscription"]],
  balance: [["subscription"]],
  coding_plan: [["subscription"]],
  // 应用专属
  omo: [["omo"], ["omo-slim"], ["providers"]],
  pi: [
    ["pi"],
    ["pi", "currentState"],
    ["pi", "sessionDiscovery"],
    ["providers"],
  ],
  omp: [["omp"], ["providers"]],
  hermes: [["hermes"], ["providers"]],
  openclaw: [["openclaw"], ["providers"]],
  dsh: [["dsh"], ["providers"]],
  // 其他
  lightweight: [["settings"], ["providers"]],
  custom_icon: [["custom-icons"]],
  model_fetch: [["models-dev"]],
  stream_check: [["providerHealth"]],
  deeplink: [["providers"], ["settings"]],
  auth: [["auth"]],
  env: [["env"]],
  workspace: [["workspace"]],
  // 纯界面动作，不动数据：显式空数组，别走兜底
  misc: [],
  tray: [],
  tray_panel: [],
  lib: [],
};

/** 一条 CLI 变更需要刷新哪些查询根。失败的命令不改数据，返回空数组。 */
export function queryRootsForCliChange(
  payload: CliCommandAppliedPayload,
): QueryKey[] {
  if (!payload.ok) {
    return [];
  }
  const module = payload.module ?? null;
  if (!module) {
    return FALLBACK_DATA_ROOTS;
  }
  if (module === "provider") {
    return providerRoots(payload.app ?? null);
  }
  return MODULE_QUERY_ROOTS[module] ?? FALLBACK_DATA_ROOTS;
}

/** 多条命令可能命中同一个根，按 key 去重，避免同一查询被失效多次。 */
export function dedupeQueryRoots(roots: QueryKey[]): QueryKey[] {
  const seen = new Set<string>();
  const unique: QueryKey[] = [];
  for (const root of roots) {
    const key = JSON.stringify(root);
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    unique.push(root);
  }
  return unique;
}

/** 批量应用 CLI 变更：只失效相关的查询根。 */
export async function invalidateForCliChanges(
  queryClient: QueryClient,
  payloads: CliCommandAppliedPayload[],
): Promise<void> {
  const roots = dedupeQueryRoots(
    payloads.flatMap((payload) => queryRootsForCliChange(payload)),
  );
  if (roots.length === 0) {
    return;
  }
  await Promise.all(
    roots.map((queryKey) => queryClient.invalidateQueries({ queryKey })),
  );
}
