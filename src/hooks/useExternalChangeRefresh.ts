import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import {
  invalidateForCliChanges,
  type CliCommandAppliedPayload,
} from "@/lib/cliCommandRefresh";

/** 后端在 CLI 写命令执行成功后 emit 的事件名。 */
export const CLI_COMMAND_APPLIED_EVENT = "cli-command-applied";

/** 连续多条 CLI 命令合并成一次刷新：先攒 150ms，再按域去重失效。 */
export const REFRESH_DEBOUNCE_MS = 150;

export type { CliCommandAppliedPayload };

/**
 * 监听 `cli-command-applied`：`ccswitch` 命令行把写命令交给正在运行的桌面进程执行后，
 * 这里只失效该命令影响的那个界面域的数据（见 `lib/cliCommandRefresh`），
 * 不整页 reload、也不全量 invalidate。
 */
export function useExternalChangeRefresh() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let pending: CliCommandAppliedPayload[] = [];

    const flush = () => {
      timer = undefined;
      const batch = pending;
      pending = [];
      if (batch.length === 0) {
        return;
      }
      void invalidateForCliChanges(queryClient, batch);
    };

    void (async () => {
      try {
        const off = await listen<CliCommandAppliedPayload>(
          CLI_COMMAND_APPLIED_EVENT,
          (event) => {
            if (disposed) return;
            pending.push(event.payload);
            // 防抖：连发多条命令（如 CLI 批量操作）合并成一次按域失效
            if (timer !== undefined) clearTimeout(timer);
            timer = setTimeout(flush, REFRESH_DEBOUNCE_MS);
          },
        );

        if (disposed) {
          off();
        } else {
          unlisten = off;
        }
      } catch (error) {
        // 非 Tauri 环境（例如没装浏览器桥的纯 web 预览）不应影响界面启动
        console.error(
          `Failed to subscribe ${CLI_COMMAND_APPLIED_EVENT} event`,
          error,
        );
      }
    })();

    return () => {
      disposed = true;
      if (timer !== undefined) clearTimeout(timer);
      timer = undefined;
      pending = [];
      unlisten?.();
    };
  }, [queryClient]);
}
