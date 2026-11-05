import { describe, expect, it } from "vitest";
import {
  dedupeQueryRoots,
  queryRootsForCliChange,
  type CliCommandAppliedPayload,
} from "@/lib/cliCommandRefresh";

function payload(
  overrides: Partial<CliCommandAppliedPayload> = {},
): CliCommandAppliedPayload {
  return {
    command: "switch_provider",
    module: "provider",
    ok: true,
    ...overrides,
  };
}

describe("queryRootsForCliChange", () => {
  it("失败的命令不改数据，不需要刷新", () => {
    expect(queryRootsForCliChange(payload({ ok: false }))).toEqual([]);
  });

  it("按模块只失效对应域的数据（MCP 不动设置/技能）", () => {
    const roots = queryRootsForCliChange(
      payload({ command: "upsert_mcp_server", module: "mcp" }),
    );
    expect(roots).toEqual([["mcp"]]);
    expect(roots).not.toContainEqual(["settings"]);
    expect(roots).not.toContainEqual(["skills"]);
  });

  it("供应商改动按 app 收窄", () => {
    const claude = queryRootsForCliChange(payload({ app: "claude" }));
    expect(claude).toContainEqual(["providers"]);
    expect(claude).toContainEqual(["managed-auth-status"]);
    expect(claude).toContainEqual(["subscription"]);
    expect(claude).not.toContainEqual(["codex_oauth"]);

    const codex = queryRootsForCliChange(
      payload({ command: "add_provider", app: "codex" }),
    );
    expect(codex).toContainEqual(["providers"]);
    expect(codex).toContainEqual(["codex_oauth"]);
    expect(codex).not.toContainEqual(["managed-auth-status"]);
  });

  it("没有 app 时放宽到账号相关根", () => {
    const roots = queryRootsForCliChange(payload({ app: null }));
    expect(roots).toContainEqual(["providers"]);
    expect(roots).toContainEqual(["subscription"]);
  });

  it("纯界面动作模块不刷新数据", () => {
    expect(
      queryRootsForCliChange(
        payload({ command: "update_tray_menu", module: "lib" }),
      ),
    ).toEqual([]);
    expect(
      queryRootsForCliChange(
        payload({ command: "set_window_theme", module: "misc" }),
      ),
    ).toEqual([]);
  });

  it("模块未识别时兜底只刷新数据根，不碰界面状态查询", () => {
    const roots = queryRootsForCliChange(payload({ module: null }));
    expect(roots).toContainEqual(["providers"]);
    expect(roots).toContainEqual(["settings"]);
    expect(roots).not.toContainEqual(["new-layout-notice", "has-providers"]);
    expect(roots).not.toContainEqual(["app-version"]);
  });
});

describe("dedupeQueryRoots", () => {
  it("按 key 去重且保持顺序", () => {
    expect(
      dedupeQueryRoots([
        ["providers"],
        ["mcp"],
        ["providers"],
        ["providers", "claude"],
      ]),
    ).toEqual([["providers"], ["mcp"], ["providers", "claude"]]);
  });
});
