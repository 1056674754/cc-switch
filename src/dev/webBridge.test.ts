/**
 * 浏览器 IPC 桥的单测。
 *
 * 注意：tests/setupTests.ts 里全局 mock 了 @tauri-apps/api/core 并挂了 msw，
 * 这里不依赖它们——测试直接操作 window.__TAURI_INTERNALS__，并自己
 * stub 掉 fetch / WebSocket，逐个用例显式 install()/uninstall()。
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  currentBaseUrl,
  currentEventsUrl,
  install,
  uninstall,
  type DevBridgeGlobal,
} from "./webBridge";

/** 只声明测试里会用到的 Tauri internals 字段。 */
interface TestInternals {
  invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
  transformCallback: (
    callback?: (data: unknown) => unknown,
    once?: boolean,
  ) => number;
  unregisterCallback: (id: number) => void;
  callbacks: Map<number, (data: unknown) => unknown>;
  metadata: {
    currentWindow: { label: string };
    currentWebview: { windowLabel: string; label: string };
  };
}

interface FakeSocketMessage {
  data: string;
}

/** 极简 WebSocket 替身：只保留桥用到的字段，测试里手动触发消息。 */
class FakeWebSocket {
  static instances: FakeWebSocket[] = [];

  url: string;
  onopen: (() => void) | null = null;
  onmessage: ((event: FakeSocketMessage) => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;
  close = vi.fn(() => {
    this.onclose?.();
  });

  constructor(url: string) {
    this.url = url;
    FakeWebSocket.instances.push(this);
  }

  emit(event: string, payload: unknown): void {
    this.onmessage?.({ data: JSON.stringify({ event, payload }) });
  }
}

const fetchMock = vi.fn();

function globals(): Record<string, unknown> {
  return window as unknown as Record<string, unknown>;
}

function internals(): TestInternals {
  const value = globals().__TAURI_INTERNALS__;
  if (!value) throw new Error("webBridge 未安装");
  return value as TestInternals;
}

function bridgeGlobal(): DevBridgeGlobal {
  const value = globals().__CC_DEV_BRIDGE__;
  if (!value) throw new Error("__CC_DEV_BRIDGE__ 未暴露");
  return value as DevBridgeGlobal;
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

beforeEach(() => {
  FakeWebSocket.instances = [];
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock);
  vi.stubGlobal("WebSocket", FakeWebSocket as unknown as typeof WebSocket);
});

afterEach(() => {
  uninstall();
  vi.unstubAllGlobals();
});

describe("install / uninstall", () => {
  it("测试环境（MODE=test）里模块导入不会自动安装", () => {
    expect(globals().__TAURI_INTERNALS__).toBeUndefined();
    expect(globals().isTauri).toBeUndefined();
  });

  it("安装后补齐 isTauri、internals 元数据和调试入口", () => {
    install();
    const target = globals();

    expect(target.isTauri).toBe(true);
    expect(internals().metadata.currentWindow.label).toBe("main");
    expect(internals().metadata.currentWebview.label).toBe("main");
    expect(internals().metadata.currentWebview.windowLabel).toBe("main");
    expect(bridgeGlobal().baseUrl).toBe(currentBaseUrl());
    expect(typeof bridgeGlobal().getHealth).toBe("function");
    expect(bridgeGlobal().version).toBe("");
  });

  it("卸载后清掉安装时写入的全局", () => {
    install();
    expect(globals().__TAURI_INTERNALS__).toBeDefined();

    uninstall();

    expect(globals().__TAURI_INTERNALS__).toBeUndefined();
    expect(globals().__CC_DEV_BRIDGE__).toBeUndefined();
    expect(globals().isTauri).toBeUndefined();
  });
});

describe("invoke 传输", () => {
  it("POST 到 /ipc/<command>，请求体是参数对象，成功后解包 data", async () => {
    fetchMock.mockResolvedValue(
      jsonResponse({ ok: true, data: { providers: ["a", "b"] } }),
    );
    install();

    const result = await internals().invoke("list_providers", {
      appType: "claude",
      includeHidden: false,
    });

    expect(result).toEqual({ providers: ["a", "b"] });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] as [
      string,
      { method: string; headers: Record<string, string>; body: string },
    ];
    expect(url).toBe(`${currentBaseUrl()}/ipc/list_providers`);
    expect(init.method).toBe("POST");
    expect(init.headers["Content-Type"]).toBe("application/json");
    expect(JSON.parse(init.body)).toEqual({
      appType: "claude",
      includeHidden: false,
    });
  });

  it("envelope 为 ok:false 时用 error 文本抛错", async () => {
    fetchMock.mockResolvedValue(
      jsonResponse({ ok: false, error: "provider-not-found" }),
    );
    install();

    await expect(
      internals().invoke("get_provider", { id: "missing" }),
    ).rejects.toThrow("provider-not-found");
  });

  it("HTTP 层连不上时抛出带命令名的错误，而不是卡死", async () => {
    fetchMock.mockRejectedValue(new TypeError("Failed to fetch"));
    install();

    await expect(internals().invoke("list_providers")).rejects.toThrow(
      /list_providers/,
    );
  });

  it("直连模式下从 /ipc/health 取版本并缓存", async () => {
    fetchMock.mockImplementation(async (url: unknown) => {
      if (String(url).endsWith("/ipc/health")) {
        return jsonResponse({
          ok: true,
          devIpc: true,
          version: "4.0.25+diy1",
          platform: "macos",
          home: "/tmp/dev-home",
          appConfigDir: "/tmp/dev-home/.cc-switch",
          commands: 405,
        });
      }
      return jsonResponse({ ok: true, data: null });
    });
    install({ baseUrl: "http://127.0.0.1:3999/" });

    await expect(internals().invoke("plugin:app|version")).resolves.toBe(
      "4.0.25+diy1",
    );
    await expect(internals().invoke("plugin:app|version")).resolves.toBe(
      "4.0.25+diy1",
    );

    const healthCalls = fetchMock.mock.calls.filter(([url]) =>
      String(url).endsWith("/ipc/health"),
    );
    expect(healthCalls).toHaveLength(1);
    expect(healthCalls[0][0]).toBe("http://127.0.0.1:3999/ipc/health");
    expect(bridgeGlobal().version).toBe("4.0.25+diy1");

    // 路径桩也从 health 派生，并且 join 会合并重复斜杠
    await expect(internals().invoke("plugin:path|home_dir")).resolves.toBe(
      "/tmp/dev-home",
    );
    await expect(
      internals().invoke("plugin:path|join", {
        paths: ["/tmp/dev-home/", ".cc-switch"],
      }),
    ).resolves.toBe("/tmp/dev-home/.cc-switch");
  });
});

describe("事件桥", () => {
  it("WS 消息按 { event, id, payload } 形状分发给 listen 的 handler，unlisten 后不再触发", async () => {
    install();
    const socket = FakeWebSocket.instances.at(-1);
    expect(socket).toBeDefined();
    expect(socket!.url).toBe(currentEventsUrl());

    const handler = vi.fn();
    const callbackId = internals().transformCallback(handler);
    const eventId = await internals().invoke("plugin:event|listen", {
      event: "usage-log-recorded",
      target: { kind: "Any" },
      handler: callbackId,
    });
    expect(eventId).toBe(callbackId);

    socket!.emit("usage-log-recorded", { appType: "claude" });
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith({
      event: "usage-log-recorded",
      id: callbackId,
      payload: { appType: "claude" },
    });

    // 其它事件不应该串台
    socket!.emit("provider-switched", { id: "p1" });
    expect(handler).toHaveBeenCalledTimes(1);

    await internals().invoke("plugin:event|unlisten", {
      event: "usage-log-recorded",
      eventId,
    });
    socket!.emit("usage-log-recorded", { appType: "claude" });
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it("plugin:event|emit 在本窗口内回环给已注册的 handler", async () => {
    install();
    const handler = vi.fn();
    const callbackId = internals().transformCallback(handler);
    await internals().invoke("plugin:event|listen", {
      event: "frontend-ready",
      target: { kind: "Any" },
      handler: callbackId,
    });

    await internals().invoke("plugin:event|emit", {
      event: "frontend-ready",
      payload: 1,
    });

    expect(handler).toHaveBeenCalledWith({
      event: "frontend-ready",
      id: callbackId,
      payload: 1,
    });
  });

  it("WS 断开后按 500ms → 1s 的退避自动重连", () => {
    vi.useFakeTimers();
    try {
      install();
      expect(FakeWebSocket.instances).toHaveLength(1);

      // 第一次断开：500ms 后重连
      FakeWebSocket.instances[0].onclose?.();
      vi.advanceTimersByTime(499);
      expect(FakeWebSocket.instances).toHaveLength(1);
      vi.advanceTimersByTime(1);
      expect(FakeWebSocket.instances).toHaveLength(2);

      // 第二次断开：退避翻倍到 1s
      FakeWebSocket.instances[1].onclose?.();
      vi.advanceTimersByTime(999);
      expect(FakeWebSocket.instances).toHaveLength(2);
      vi.advanceTimersByTime(1);
      expect(FakeWebSocket.instances).toHaveLength(3);
    } finally {
      vi.useRealTimers();
    }
  });
});

describe("plugin 桩", () => {
  it("window|is_maximized 返回 false，且完全不发 HTTP 请求", async () => {
    install();

    await expect(
      internals().invoke("plugin:window|is_maximized", { label: "main" }),
    ).resolves.toBe(false);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("未实现的插件命令返回 undefined 而不是抛错", async () => {
    install();
    const debug = vi.spyOn(console, "debug").mockImplementation(() => {});

    await expect(
      internals().invoke("plugin:unknown|whatever"),
    ).resolves.toBeUndefined();
    expect(debug).toHaveBeenCalledTimes(1);

    // 同一种命令只提示一次
    await expect(
      internals().invoke("plugin:unknown|whatever"),
    ).resolves.toBeUndefined();
    expect(debug).toHaveBeenCalledTimes(1);
    debug.mockRestore();
  });
});

describe("transformCallback", () => {
  it("自增发号，并支持注销", () => {
    install();
    const bridge = internals();

    const first = bridge.transformCallback(() => {});
    const second = bridge.transformCallback(() => {});

    expect(Number.isInteger(first)).toBe(true);
    expect(first).toBeGreaterThan(0);
    expect(second).toBe(first + 1);
    expect(bridge.callbacks.has(second)).toBe(true);

    bridge.unregisterCallback(first);
    expect(bridge.callbacks.has(first)).toBe(false);
    expect(bridge.callbacks.has(second)).toBe(true);
  });

  it("once=true 的回调执行一次后自动注销", async () => {
    install();
    const bridge = internals();
    const handler = vi.fn();
    const callbackId = bridge.transformCallback(handler, true);
    await bridge.invoke("plugin:event|listen", {
      event: "once-event",
      target: { kind: "Any" },
      handler: callbackId,
    });

    FakeWebSocket.instances.at(-1)!.emit("once-event", "first");
    FakeWebSocket.instances.at(-1)!.emit("once-event", "second");

    expect(handler).toHaveBeenCalledTimes(1);
    expect(bridge.callbacks.has(callbackId)).toBe(false);
  });
});
