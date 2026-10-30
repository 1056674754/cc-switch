import { QueryClient } from "@tanstack/react-query";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      // 窗口获得焦点不再全量重取：以前这里是 true + staleTime 0，每次点回窗口都会把
      // 所有挂着的 query（含 get_providers 这类要摸数据库的命令）同时打一遍。
      refetchOnWindowFocus: false,
      // 15 秒内认为数据新鲜，页面间来回切不再重复请求；写操作仍走 invalidateQueries。
      // 需要「一进来就要最新」的 query 自己写了更小的 staleTime，不受影响。
      staleTime: 15_000,
    },
    mutations: {
      retry: false,
    },
  },
});
