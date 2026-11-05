/**
 * Tauri 插件命令在「浏览器 + headless core」模式下的本地替身。
 *
 * 浏览器里没有 Tauri runtime，`@tauri-apps/plugin-*` 与 `@tauri-apps/api` 的
 * 窗口/路径 API 发出的 `plugin:*` 命令到不了 Rust 侧（headless core 只暴露应用
 * 自身的命令），所以在这里兜底：能给合理默认值的给默认值，做不到的记一条日志
 * 后返回 undefined —— 原则是「不抛错、不打断页面渲染」。
 *
 * 只有 `src/dev/webBridge.ts` 会调用本模块；生产构建不会走到这里。
 */

/** `GET /ipc/health` 的响应；headless core 用它自报家门。 */
export interface DevIpcHealth {
  ok?: boolean;
  devIpc?: boolean;
  pid?: number;
  version?: string;
  platform?: string;
  home?: string;
  appConfigDir?: string;
  commands?: number;
}

/** 桩命令需要的运行时依赖，由桥注入，避免和 webBridge 循环引用。 */
export interface StubContext {
  /** 读取 health；core 没起时返回 undefined，而不是抛错。 */
  getHealth: () => Promise<DevIpcHealth | undefined>;
}

/** 已提示过的未知插件命令：同一种命令只 console.debug 一次，避免刷屏。 */
const reportedUnknown = new Set<string>();

function reportUnknown(cmd: string): void {
  if (reportedUnknown.has(cmd)) return;
  reportedUnknown.add(cmd);
  console.debug(
    `[cc-dev-bridge] 浏览器模式未实现插件命令 ${cmd}，已返回 undefined`,
  );
}

function toText(value: unknown): string {
  if (typeof value === "string") return value;
  if (value === null || value === undefined) return "";
  return String(value);
}

/**
 * 按 `@tauri-apps/api/path` 的 join 语义拼路径：
 * 用 "/" 连接各段，并合并重复的斜杠（因为 dev 场景都在 macOS/Linux 上）。
 */
export function joinPathSegments(paths: unknown): string {
  const segments = (Array.isArray(paths) ? paths : [paths]).map(toText);
  const parts = segments.filter((segment) => segment.length > 0);
  if (parts.length === 0) return "";
  return parts.join("/").replace(/\/{2,}/g, "/");
}

/**
 * BaseDirectory 枚举值（见 @tauri-apps/api/path）里可以直接从 home 派生的目录。
 * 13/14/15/16/17 等 App 相关目录走 health.appConfigDir。
 */
const HOME_RELATIVE_DIRECTORIES: Record<number, string> = {
  6: "Documents",
  7: "Downloads",
  8: "Pictures",
  9: "Public",
  10: "Movies",
  18: "Desktop",
};

async function resolveBaseDirectory(
  directory: unknown,
  ctx: StubContext,
): Promise<string> {
  const health = await ctx.getHealth();
  switch (Number(directory)) {
    case 21: // Home
      return health?.home ?? "";
    case 13: // AppConfig
    case 14: // AppData
    case 15: // AppLocalData
    case 16: // AppCache
    case 17: // AppLog
      return health?.appConfigDir ?? "";
    default: {
      const folder = HOME_RELATIVE_DIRECTORIES[Number(directory)];
      const home = health?.home ?? "";
      return folder && home ? joinPathSegments([home, folder]) : "";
    }
  }
}

const LOG_LEVELS: Record<number, "debug" | "info" | "warn" | "error"> = {
  1: "debug",
  2: "debug",
  3: "info",
  4: "warn",
  5: "error",
};

/** `plugin:log|log`：前端转给 Rust 的日志在浏览器里直接写 console。 */
function writeConsoleLog(args: Record<string, unknown>): void {
  const message = toText(args.message);
  const level = LOG_LEVELS[Number(args.level)] ?? "info";
  console[level](`[core:log] ${message}`, {
    location: args.location,
    file: args.file,
    line: args.line,
    keyValues: args.keyValues,
  });
}

/**
 * 处理 `plugin:*` 命令（事件插件的命令由 webBridge 先一步截走，到不了这里）。
 * 返回值即 Tauri 侧 resolve 给前端的值；只有明确「浏览器模式不支持」的命令才 reject。
 */
export async function handlePluginCommand(
  cmd: string,
  args: Record<string, unknown>,
  ctx: StubContext,
): Promise<unknown> {
  switch (cmd) {
    // ---- 窗口：浏览器没有原生窗口，给一组「正常但不可操作」的默认值 ----
    case "plugin:window|is_maximized":
    case "plugin:window|is_fullscreen":
    case "plugin:window|is_minimizable":
    case "plugin:window|is_maximizable":
      return false;
    case "plugin:window|is_resizable":
      // 浏览器窗口由浏览器自己管理，应用内边缘缩放不该接管。
      return false;
    case "plugin:window|is_decorated":
      return true;
    case "plugin:window|scale_factor":
      return 1;
    case "plugin:window|inner_size":
      return { width: 1280, height: 800 };
    case "plugin:window|inner_position":
    case "plugin:window|outer_position":
      return { x: 0, y: 0 };
    case "plugin:window|theme":
      return null;
    case "plugin:window|title":
      return "";
    case "plugin:window|minimize":
    case "plugin:window|maximize":
    case "plugin:window|unmaximize":
    case "plugin:window|toggle_maximize":
    case "plugin:window|close":
    case "plugin:window|destroy":
    case "plugin:window|start_dragging":
    case "plugin:window|set_decorations":
    case "plugin:window|set_focus":
    case "plugin:window|show":
    case "plugin:window|hide":
    case "plugin:window|set_shadow":
    case "plugin:window|set_title":
      return undefined;

    // ---- 应用信息：版本来自 core 的 health，名称与 tauri.conf 一致 ----
    case "plugin:app|version": {
      const health = await ctx.getHealth();
      return health?.version ?? "";
    }
    case "plugin:app|name":
      return "CC Switch";

    // ---- 路径：能对上 health 的走 health，其余按 POSIX 语义拼 / 尽力而为 ----
    case "plugin:path|home_dir": {
      const health = await ctx.getHealth();
      return health?.home ?? "";
    }
    case "plugin:path|app_config_dir":
    case "plugin:path|app_data_dir": {
      const health = await ctx.getHealth();
      return health?.appConfigDir ?? "";
    }
    case "plugin:path|join":
      return joinPathSegments(args.paths);
    case "plugin:path|resolve_directory":
      return resolveBaseDirectory(args.directory, ctx);

    // ---- 对话框：浏览器里没有原生弹窗，按「用户取消 / 已确认」处理 ----
    case "plugin:dialog|message":
    case "plugin:dialog|ask":
    case "plugin:dialog|confirm": {
      const title = toText(args.title);
      const message = toText(args.message);
      console.info(
        `[cc-dev-bridge] 浏览器模式自动关闭对话框${
          title ? `「${title}」` : ""
        }：${message}`,
      );
      return null;
    }
    case "plugin:dialog|open":
    case "plugin:dialog|save":
      console.info(
        "[cc-dev-bridge] 浏览器模式没有原生文件选择框，按「用户取消」返回 null",
      );
      return null;

    // ---- 进程：退出只告警，重启用刷新页面模拟 ----
    case "plugin:process|exit":
      console.warn("[cc-dev-bridge] 浏览器模式忽略退出请求（不关闭标签页）");
      return undefined;
    case "plugin:process|restart":
    case "plugin:process|relaunch":
      console.info("[cc-dev-bridge] 浏览器模式用刷新页面模拟应用重启");
      window.location.reload();
      return undefined;

    // ---- 更新器：浏览器模式永远「没有更新」，安装动作明确报错 ----
    case "plugin:updater|check":
      return null;
    case "plugin:updater|download":
    case "plugin:updater|download_and_install":
    case "plugin:updater|install":
      throw new Error("更新器在浏览器开发模式下不可用");

    // ---- 日志 / 打开类 ----
    case "plugin:log|log":
      writeConsoleLog(args);
      return undefined;
    case "plugin:opener|open_url": {
      const url = toText(args.url);
      if (url) {
        console.info(`[cc-dev-bridge] 浏览器模式用新标签页打开：${url}`);
        window.open(url, "_blank", "noopener");
      }
      return undefined;
    }
    case "plugin:opener|open_path":
    case "plugin:opener|reveal_item_in_dir":
      console.info(
        "[cc-dev-bridge] 浏览器模式无法打开本地路径：",
        args.path ?? args,
      );
      return undefined;

    default:
      break;
  }

  // 路径类命令按约定「尽力而返」，给不出值时返回空串而不是 undefined。
  if (cmd.startsWith("plugin:path|")) {
    reportUnknown(cmd);
    return "";
  }
  reportUnknown(cmd);
  return undefined;
}
