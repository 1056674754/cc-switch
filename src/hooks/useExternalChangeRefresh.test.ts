import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  REFRESH_DEBOUNCE_MS,
  useExternalChangeRefresh,
} from "@/hooks/useExternalChangeRefresh";

/**
 * listen / useQueryClient 都用 mock：这里只验证 hook 自己的订阅、防抖、按域失效和清理，
 * 不依赖 tests/setupTests.ts 里的 msw 事件桩，也不要求 QueryClientProvider。
 */
const { invalidateQueries, listenMock, unlistenMock, handlers } = vi.hoisted(
  () => {
    const handlers = new Map<string, (event: { payload: unknown }) => void>();
    const unlistenMock = vi.fn(() => handlers.clear());
    const listenMock = vi.fn(
      async (event: string, handler: (event: { payload: unknown }) => void) => {
        handlers.set(event, handler);
        return unlistenMock;
      },
    );
    const invalidateQueries = vi.fn();
    return { invalidateQueries, listenMock, unlistenMock, handlers };
  },
);

vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));
vi.mock("@tanstack/react-query", () => ({
  useQueryClient: () => ({ invalidateQueries }),
}));

interface EmittedPayload {
  command: string;
  module?: string | null;
  app?: string | null;
  ok?: boolean;
}

/** 触发已注册的 cli-command-applied handler。 */
function emitCliCommandApplied(payload: EmittedPayload): void {
  handlers.get("cli-command-applied")?.({
    payload: { ok: true, ...payload },
  });
}

/** 已经传给 invalidateQueries 的查询根。 */
function invalidatedRoots(): unknown[] {
  return invalidateQueries.mock.calls.map(
    (call) => (call[0] as { queryKey: unknown }).queryKey,
  );
}

/** 等待 listen() 的 Promise resolve（注册完成）。 */
const flushListen = () => act(async () => {});

describe("useExternalChangeRefresh", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    handlers.clear();
    invalidateQueries.mockClear();
    listenMock.mockClear();
    unlistenMock.mockClear();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("订阅 cli-command-applied 事件", async () => {
    renderHook(() => useExternalChangeRefresh());
    await flushListen();

    expect(listenMock).toHaveBeenCalledTimes(1);
    expect(listenMock).toHaveBeenCalledWith(
      "cli-command-applied",
      expect.any(Function),
    );
  });

  it("写命令只失效对应域的数据，不全量 invalidate", async () => {
    renderHook(() => useExternalChangeRefresh());
    await flushListen();

    act(() =>
      emitCliCommandApplied({ command: "upsert_mcp_server", module: "mcp" }),
    );
    expect(invalidateQueries).not.toHaveBeenCalled();

    act(() => vi.advanceTimersByTime(REFRESH_DEBOUNCE_MS));
    expect(invalidatedRoots()).toEqual([["mcp"]]);
    // 关键：不是无参调用（全量失效）
    expect(invalidateQueries).not.toHaveBeenCalledWith();
  });

  it("供应商命令按 app 收窄，不顺手重拉别的应用", async () => {
    renderHook(() => useExternalChangeRefresh());
    await flushListen();

    act(() =>
      emitCliCommandApplied({
        command: "switch_provider",
        module: "provider",
        app: "claude",
      }),
    );
    act(() => vi.advanceTimersByTime(REFRESH_DEBOUNCE_MS));

    const roots = invalidatedRoots();
    expect(roots).toContainEqual(["providers"]);
    expect(roots).toContainEqual(["managed-auth-status"]);
    expect(roots).not.toContainEqual(["codex_oauth"]);
  });

  it("只在 150ms 窗口内按域去重合并，不多刷也不漏刷", async () => {
    renderHook(() => useExternalChangeRefresh());
    await flushListen();

    act(() => {
      emitCliCommandApplied({ command: "upsert_mcp_server", module: "mcp" });
      emitCliCommandApplied({ command: "toggle_mcp_app", module: "mcp" });
      emitCliCommandApplied({ command: "save_settings", module: "settings" });
    });

    act(() => vi.advanceTimersByTime(REFRESH_DEBOUNCE_MS - 1));
    expect(invalidateQueries).not.toHaveBeenCalled();

    act(() => vi.advanceTimersByTime(1));
    expect(invalidatedRoots()).toEqual([["mcp"], ["settings"]]);
  });

  it("失败命令、纯界面命令都不刷新", async () => {
    renderHook(() => useExternalChangeRefresh());
    await flushListen();

    // 只读命令由后端过滤（见 control::is_read_only_command），不会发到这里；
    // 这里只验证失败结果和不动数据的模块不会触发刷新。
    act(() => {
      emitCliCommandApplied({
        command: "save_settings",
        module: "settings",
        ok: false,
      });
      // 模块没变过数据：lib = update_tray_menu 之类
      emitCliCommandApplied({ command: "update_tray_menu", module: "lib" });
    });
    act(() => vi.advanceTimersByTime(REFRESH_DEBOUNCE_MS));

    expect(invalidateQueries).not.toHaveBeenCalled();
  });

  it("模块未识别时走兜底数据根", async () => {
    renderHook(() => useExternalChangeRefresh());
    await flushListen();

    act(() =>
      emitCliCommandApplied({ command: "some_future_write", module: null }),
    );
    act(() => vi.advanceTimersByTime(REFRESH_DEBOUNCE_MS));

    const roots = invalidatedRoots();
    expect(roots).toContainEqual(["providers"]);
    expect(roots).toContainEqual(["settings"]);
  });

  it("卸载时调用 unlisten", async () => {
    const { unmount } = renderHook(() => useExternalChangeRefresh());
    await flushListen();

    unmount();
    expect(unlistenMock).toHaveBeenCalledTimes(1);
  });

  it("listen 还没返回就卸载也会补调 unlisten", async () => {
    const { unmount } = renderHook(() => useExternalChangeRefresh());
    unmount();

    await flushListen();
    expect(unlistenMock).toHaveBeenCalledTimes(1);
  });

  it("卸载后不再触发刷新（含防抖中的事件）", async () => {
    const { unmount } = renderHook(() => useExternalChangeRefresh());
    await flushListen();

    act(() =>
      emitCliCommandApplied({ command: "upsert_mcp_server", module: "mcp" }),
    );
    unmount();
    act(() => vi.advanceTimersByTime(500));
    expect(invalidateQueries).not.toHaveBeenCalled();

    act(() =>
      emitCliCommandApplied({ command: "upsert_mcp_server", module: "mcp" }),
    );
    act(() => vi.advanceTimersByTime(500));
    expect(invalidateQueries).not.toHaveBeenCalled();
  });
});
