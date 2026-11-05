/**
 * 浏览器开发模式（普通浏览器 UI + headless core）的 Tauri IPC 适配层。
 *
 * 目标是不改动 343 处 `invoke()` 调用点：dev 且非 Tauri 环境时，这里把
 * `window.__TAURI_INTERNALS__` 补齐，`@tauri-apps/api` 原样工作，只是传输层换成
 *
 *   invoke(cmd, args)  --POST-->  /ipc/<cmd>      （默认经 vite 同源代理）
 *   `emit` 事件         <--WS----  /ipc/events
 *
 * `plugin:event|*` 在本地实现（回调和 id 都在前端），其它 `plugin:*` 交给
 * `./tauriStubs`，都不打后端。生产构建里 `import.meta.env.DEV === false`，
 * 本模块不会安装任何东西。
 */
import { handlePluginCommand, type DevIpcHealth } from "./tauriStubs";

/** 直连 core 时的默认地址；正常走 vite 代理时用不到。 */
const DEFAULT_CORE_BASE_URL = "http://127.0.0.1:3999";
/** WS 断线重连：500ms 起步，翻倍，5s 封顶。 */
const RECONNECT_MIN_DELAY_MS = 500;
const RECONNECT_MAX_DELAY_MS = 5000;

type JsonObject = Record<string, unknown>;
type TauriCallback = (data: unknown) => unknown;
type GlobalBag = Record<string, unknown>;

/** Tauri 传给 JS 事件 handler 的形状。 */
interface TauriEventPayload {
  event: string;
  id: number;
  payload: unknown;
}

/** `window.__TAURI_INTERNALS__` 里前端真正会用到的那部分。 */
interface TauriInternals {
  invoke: (
    cmd: string,
    args?: JsonObject,
    options?: unknown,
  ) => Promise<unknown>;
  transformCallback: (callback?: TauriCallback, once?: boolean) => number;
  unregisterCallback: (id: number) => void;
  callbacks: Map<number, TauriCallback>;
  convertFileSrc: (filePath: string, protocol?: string) => string;
  metadata: {
    currentWindow: { label: string };
    currentWebview: { windowLabel: string; label: string };
  };
  /** 少数 API（如 `path.sep()`）直接读这里；浏览器里固定按 POSIX 处理。 */
  plugins: { path: { sep: string; delimiter: string } };
}

/** 暴露到 `window.__CC_DEV_BRIDGE__` 的手测入口。 */
export interface DevBridgeGlobal {
  /** 当前 IPC 的 HTTP 基地址（代理模式是同源 origin，直连模式是 core 地址）。 */
  readonly baseUrl: string;
  /** 请求 `GET /ipc/health`，成功后缓存。 */
  getHealth: () => Promise<DevIpcHealth>;
  /** health 里报告的 core 版本；还没取到 health 时是空串。 */
  readonly version: string;
}

export interface InstallOptions {
  /** 直连 core 地址；不传则走 vite 同源代理或 `VITE_DEV_IPC_BASE`。 */
  baseUrl?: string;
  /** 是否建立 WS 事件连接，默认 true（测试里可以关掉）。 */
  connectEvents?: boolean;
}

interface BridgeState {
  /** 显式传入的直连地址（测试 / 特殊场景）。 */
  baseUrlOverride?: string;
  callbacks: Map<number, TauriCallback>;
  /** callback id 自增发号，从 1 开始，和 Tauri 保持一致。 */
  nextCallbackId: number;
  /** event 名 → 监听它的 callback id 集合。 */
  eventHandlers: Map<string, Set<number>>;
  health?: DevIpcHealth;
  healthPromise?: Promise<DevIpcHealth>;
  healthFailureReported: boolean;
  socket?: WebSocket;
  reconnectTimer?: ReturnType<typeof setTimeout>;
  reconnectDelayMs: number;
  bridge: DevBridgeGlobal;
  /** 安装前的全局值，卸载时原样还回去。 */
  previous: {
    isTauri: unknown;
    globalIsTauri: unknown;
    internals: unknown;
    eventInternals: unknown;
    bridge: unknown;
  };
}

let state: BridgeState | undefined;

// ---------------------------------------------------------------------------
// 小工具
// ---------------------------------------------------------------------------

function globalBag(target: object = window): GlobalBag {
  return target as unknown as GlobalBag;
}

function restoreProperty(
  target: GlobalBag,
  key: string,
  previous: unknown,
): void {
  if (previous === undefined) {
    delete target[key];
  } else {
    target[key] = previous;
  }
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
  return typeof value === "object" && value !== null
    ? (value as Record<string, unknown>)
    : undefined;
}

function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function normalizeBaseUrl(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim().replace(/\/+$/, "");
  return trimmed.length > 0 ? trimmed : undefined;
}

/** 浏览器同源地址；jsdom 等没有 origin 的环境里退回默认 core 地址。 */
function sameOriginBaseUrl(): string {
  try {
    const origin = window.location?.origin;
    if (origin && origin !== "null") return origin.replace(/\/+$/, "");
  } catch {
    // 忽略读取 location 失败
  }
  return DEFAULT_CORE_BASE_URL;
}

/**
 * IPC 的 HTTP 基地址，优先级：
 * `install({ baseUrl })` > `VITE_DEV_IPC_BASE`（直连）> 同源（vite 代理）。
 */
export function currentBaseUrl(): string {
  const override =
    state?.baseUrlOverride ??
    normalizeBaseUrl(import.meta.env.VITE_DEV_IPC_BASE);
  return override ?? sameOriginBaseUrl();
}

/** WS 事件地址：跟着 HTTP 基地址走（直连时也直连 core）。 */
export function currentEventsUrl(): string {
  const base = currentBaseUrl();
  if (/^https?:\/\//i.test(base)) {
    // http:// → ws://，https:// → wss://
    return `${base.replace(/^http/i, "ws")}/ipc/events`;
  }
  const protocol = window.location.protocol === "https:" ? "wss" : "ws";
  return `${protocol}://${window.location.host}/ipc/events`;
}

async function readJson(response: Response, label: string): Promise<unknown> {
  const text = await response.text();
  if (!text) {
    if (response.ok) return undefined;
    throw new Error(`${label} 失败：HTTP ${response.status}`);
  }
  try {
    return JSON.parse(text) as unknown;
  } catch {
    throw new Error(`${label} 返回了非 JSON 内容：${text.slice(0, 200)}`);
  }
}

// ---------------------------------------------------------------------------
// HTTP 传输
// ---------------------------------------------------------------------------

/** POST /ipc/<cmd>，按 envelope 解包（成功取 data，失败抛 error）。 */
async function postCommand(cmd: string, args: JsonObject): Promise<unknown> {
  const url = `${currentBaseUrl()}/ipc/${encodeURIComponent(cmd)}`;
  let response: Response;
  try {
    response = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(args ?? {}),
    });
  } catch (error) {
    throw new Error(
      `IPC ${cmd} 请求失败（headless core 没起？${currentBaseUrl()}）：${errorText(error)}`,
    );
  }
  const payload = await readJson(response, `IPC ${cmd}`);
  const envelope = asRecord(payload);
  if (envelope) {
    if (envelope.ok === false) {
      const reason =
        typeof envelope.error === "string" ? envelope.error.trim() : "";
      throw new Error(reason || `IPC ${cmd} 执行失败`);
    }
    if (envelope.ok === true) return envelope.data;
  }
  // 容错：不是标准 envelope 时原样返回，至少不吞掉数据。
  if (!response.ok) throw new Error(`IPC ${cmd} 失败：HTTP ${response.status}`);
  return payload;
}

/** GET /ipc/health：core 自报版本、隔离 home、appConfigDir 等。 */
async function requestHealth(): Promise<DevIpcHealth> {
  const url = `${currentBaseUrl()}/ipc/health`;
  let response: Response;
  try {
    response = await fetch(url, {
      method: "GET",
      headers: { Accept: "application/json" },
    });
  } catch (error) {
    throw new Error(`无法连接 headless core（${url}）：${errorText(error)}`);
  }
  const payload = await readJson(response, "health 检查");
  const envelope = asRecord(payload);
  if (envelope?.ok === false) {
    const reason =
      typeof envelope.error === "string" ? envelope.error.trim() : "";
    throw new Error(reason || "health 检查失败");
  }
  if (!response.ok) throw new Error(`health 检查失败：HTTP ${response.status}`);
  return (envelope ?? {}) as DevIpcHealth;
}

/** 读取 health 并缓存（失败不缓存，下次调用会重试）。 */
export async function getHealth(): Promise<DevIpcHealth> {
  const current = state;
  if (!current) throw new Error("dev web bridge 尚未安装");
  if (current.health) return current.health;
  if (!current.healthPromise) {
    current.healthPromise = requestHealth().then(
      (health) => {
        current.health = health;
        current.healthPromise = undefined;
        return health;
      },
      (error: unknown) => {
        current.healthPromise = undefined;
        throw error;
      },
    );
  }
  return current.healthPromise;
}

/** 桩命令用：core 没起时静默返回 undefined，只提示一次。 */
async function getHealthForStubs(): Promise<DevIpcHealth | undefined> {
  try {
    return await getHealth();
  } catch (error) {
    if (state && !state.healthFailureReported) {
      state.healthFailureReported = true;
      console.debug(
        "[cc-dev-bridge] 读取 /ipc/health 失败（headless core 没起？）",
        error,
      );
    }
    return undefined;
  }
}

// ---------------------------------------------------------------------------
// 事件：plugin:event|* 全部在前端实现，WS 只负责把 core 侧 emit 转给 handler
// ---------------------------------------------------------------------------

/** 按 Tauri 的语义把事件发给本窗口注册的 handler：`{ event, id, payload }`。 */
function dispatchEvent(event: string, payload: unknown): void {
  const current = state;
  if (!current) return;
  const handlers = current.eventHandlers.get(event);
  if (!handlers || handlers.size === 0) return;
  for (const id of Array.from(handlers)) {
    const callback = current.callbacks.get(id);
    if (!callback) {
      handlers.delete(id);
      continue;
    }
    const eventPayload: TauriEventPayload = { event, id, payload };
    try {
      callback(eventPayload);
    } catch (error) {
      console.error(`[cc-dev-bridge] 事件 ${event} 的处理器抛错`, error);
    }
  }
}

function unregisterEventCallback(event: string, eventId: number): void {
  const current = state;
  if (!current) return;
  current.eventHandlers.get(event)?.delete(eventId);
  current.callbacks.delete(eventId);
}

async function handleEventCommand(
  cmd: string,
  args: JsonObject,
): Promise<unknown> {
  const current = state;
  if (!current) return null;

  switch (cmd) {
    case "plugin:event|listen": {
      const event = typeof args.event === "string" ? args.event : "";
      const handler = Number(args.handler);
      if (!event || !Number.isFinite(handler)) return null;
      let handlers = current.eventHandlers.get(event);
      if (!handlers) {
        handlers = new Set<number>();
        current.eventHandlers.set(event, handlers);
      }
      handlers.add(handler);
      // Tauri 的 listen() 拿这个 id 作为 unlisten 的句柄。
      return handler;
    }
    case "plugin:event|unlisten": {
      const event = typeof args.event === "string" ? args.event : "";
      const eventId = Number(args.eventId);
      unregisterEventCallback(event, eventId);
      return null;
    }
    case "plugin:event|emit":
    case "plugin:event|emit_to": {
      const event = typeof args.event === "string" ? args.event : "";
      if (event) dispatchEvent(event, args.payload);
      return null;
    }
    default:
      return undefined;
  }
}

function handleSocketMessage(data: unknown): void {
  let parsed: unknown = data;
  if (typeof data === "string") {
    try {
      parsed = JSON.parse(data) as unknown;
    } catch {
      return;
    }
  }
  const record = asRecord(parsed);
  if (!record) return;
  const { event, payload } = record;
  if (typeof event === "string" && event.length > 0) {
    dispatchEvent(event, payload);
  }
}

function connectEvents(): void {
  const current = state;
  if (!current || current.socket || typeof WebSocket === "undefined") return;

  let socket: WebSocket;
  try {
    socket = new WebSocket(currentEventsUrl());
  } catch (error) {
    console.debug("[cc-dev-bridge] 连接事件通道失败，稍后重试", error);
    scheduleReconnect();
    return;
  }
  current.socket = socket;
  socket.onopen = () => {
    if (state === current) current.reconnectDelayMs = RECONNECT_MIN_DELAY_MS;
  };
  socket.onmessage = (event: MessageEvent) => handleSocketMessage(event.data);
  socket.onerror = () => {
    // 浏览器随后会触发 onclose，重连统一在那里处理。
  };
  socket.onclose = () => {
    if (state !== current) return;
    current.socket = undefined;
    scheduleReconnect();
  };
}

function scheduleReconnect(): void {
  const current = state;
  if (!current || current.reconnectTimer !== undefined) return;
  const delay = current.reconnectDelayMs;
  current.reconnectDelayMs = Math.min(
    Math.ceil(delay * 2),
    RECONNECT_MAX_DELAY_MS,
  );
  current.reconnectTimer = setTimeout(() => {
    if (state !== current) return;
    current.reconnectTimer = undefined;
    connectEvents();
  }, delay);
}

function disconnectEvents(current: BridgeState): void {
  if (current.reconnectTimer !== undefined) {
    clearTimeout(current.reconnectTimer);
    current.reconnectTimer = undefined;
  }
  const socket = current.socket;
  current.socket = undefined;
  if (!socket) return;
  // 先摘掉回调再关闭，避免 close 触发自动重连。
  socket.onopen = null;
  socket.onmessage = null;
  socket.onerror = null;
  socket.onclose = null;
  try {
    socket.close();
  } catch {
    // 忽略关闭异常
  }
}

// ---------------------------------------------------------------------------
// __TAURI_INTERNALS__ 的实现
// ---------------------------------------------------------------------------

function transformCallback(callback?: TauriCallback, once = false): number {
  const current = state;
  if (!current) return 0;
  const id = current.nextCallbackId++;
  current.callbacks.set(id, (data: unknown) => {
    if (once) unregisterCallback(id);
    return callback?.(data);
  });
  return id;
}

function unregisterCallback(id: number): void {
  const current = state;
  if (!current) return;
  current.callbacks.delete(id);
  for (const handlers of current.eventHandlers.values()) handlers.delete(id);
}

function convertFileSrc(filePath: string, protocol = "asset"): string {
  // 浏览器里没有 asset: 协议，给出与 Tauri mocks 一致的 URL，至少不抛错。
  return `${protocol}://localhost/${encodeURIComponent(String(filePath ?? ""))}`;
}

async function invoke(
  cmd: string,
  args: JsonObject = {},
  _options?: unknown,
): Promise<unknown> {
  if (typeof cmd !== "string" || cmd.length === 0) {
    throw new Error("IPC 命令名必须是非空字符串");
  }
  const payload = args ?? {};
  // 顺序很重要：先事件插件（本地事件总线），再其它插件（本地桩），剩下的才进 HTTP。
  if (cmd.startsWith("plugin:event|")) {
    return handleEventCommand(cmd, payload);
  }
  if (cmd.startsWith("plugin:")) {
    return handlePluginCommand(cmd, payload, { getHealth: getHealthForStubs });
  }
  return postCommand(cmd, payload);
}

// ---------------------------------------------------------------------------
// 安装 / 卸载
// ---------------------------------------------------------------------------

/** 安装桥接（幂等）。返回 `window.__CC_DEV_BRIDGE__` 同样的对象。 */
export function install(options: InstallOptions = {}): DevBridgeGlobal {
  if (state) return state.bridge;

  const callbacks = new Map<number, TauriCallback>();
  const previous: BridgeState["previous"] = {
    isTauri: globalBag(window).isTauri,
    globalIsTauri: globalBag(globalThis).isTauri,
    internals: globalBag(window).__TAURI_INTERNALS__,
    eventInternals: globalBag(window).__TAURI_EVENT_PLUGIN_INTERNALS__,
    bridge: globalBag(window).__CC_DEV_BRIDGE__,
  };

  const bridge: DevBridgeGlobal = {
    get baseUrl() {
      return currentBaseUrl();
    },
    getHealth,
    get version() {
      return state?.health?.version ?? "";
    },
  };

  const current: BridgeState = {
    baseUrlOverride: normalizeBaseUrl(options.baseUrl),
    callbacks,
    nextCallbackId: 1,
    eventHandlers: new Map(),
    healthFailureReported: false,
    reconnectDelayMs: RECONNECT_MIN_DELAY_MS,
    bridge,
    previous,
  };
  state = current;

  const internals: TauriInternals = {
    invoke,
    transformCallback,
    unregisterCallback,
    callbacks,
    convertFileSrc,
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { windowLabel: "main", label: "main" },
    },
    plugins: { path: { sep: "/", delimiter: ":" } },
  };

  const target = globalBag(window);
  target.__TAURI_INTERNALS__ = internals;
  // @tauri-apps/api/event 的 _unlisten 会先调这里，再 invoke unlisten。
  target.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: (event: string, eventId: number) =>
      unregisterEventCallback(event, eventId),
  };
  target.isTauri = true;
  globalBag(globalThis).isTauri = true;
  target.__CC_DEV_BRIDGE__ = bridge;

  console.info(
    `[cc-dev-bridge] 浏览器开发模式 IPC 桥已启用：${currentBaseUrl()}`,
  );

  if (options.connectEvents !== false) connectEvents();
  return bridge;
}

/** 卸载桥接，把安装前的全局值和状态都还回去（测试 afterEach 用）。 */
export function uninstall(): void {
  const current = state;
  if (!current) return;
  state = undefined;

  disconnectEvents(current);
  current.callbacks.clear();
  current.eventHandlers.clear();

  const target = globalBag(window);
  restoreProperty(target, "__TAURI_INTERNALS__", current.previous.internals);
  restoreProperty(
    target,
    "__TAURI_EVENT_PLUGIN_INTERNALS__",
    current.previous.eventInternals,
  );
  restoreProperty(target, "__CC_DEV_BRIDGE__", current.previous.bridge);
  restoreProperty(target, "isTauri", current.previous.isTauri);
  restoreProperty(
    globalBag(globalThis),
    "isTauri",
    current.previous.globalIsTauri,
  );
}

export function isInstalled(): boolean {
  return state !== undefined;
}

// ---------------------------------------------------------------------------
// 自动安装：只在 vite dev、非测试、非 Tauri 环境生效
// ---------------------------------------------------------------------------

function shouldAutoInstall(): boolean {
  if (typeof window === "undefined") return false;
  if (!import.meta.env.DEV) return false;
  // vitest 的 MODE 是 test，测试统一显式 install()/uninstall()。
  if (import.meta.env.MODE === "test") return false;
  const target = globalBag(window);
  if (target.isTauri || globalBag(globalThis).isTauri) return false;
  // 真实 Tauri 环境里已经有 __TAURI_INTERNALS__，不要覆盖。
  if (target.__TAURI_INTERNALS__) return false;
  return true;
}

if (shouldAutoInstall()) {
  install();
}
