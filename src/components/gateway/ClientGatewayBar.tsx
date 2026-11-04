import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "@/lib/toast";
import { Button } from "@/components/ui/button";
import { gatewayApi } from "@/lib/api/gateway";

export type GatewayClientBarApp = "opencode" | "hermes";

const APIS = {
  opencode: {
    label: "OpenCode",
    status: () => gatewayApi.getOpencodeGatewayStatus(),
    adopt: () => gatewayApi.adoptOpencodeToGateway(),
    release: () => gatewayApi.releaseOpencodeFromGateway(),
  },
  hermes: {
    label: "Hermes",
    status: () => gatewayApi.getHermesGatewayStatus(),
    adopt: () => gatewayApi.adoptHermesToGateway(),
    release: () => gatewayApi.releaseHermesFromGateway(),
  },
} as const;

export const gatewayClientBarKey = (app: GatewayClientBarApp) =>
  [app, "gatewayStatus"] as const;

/** 累加式 app 页顶的「走网关 / 直连」开关：接管时写 cc-switch 配置，释放时删掉。 */
export function ClientGatewayBar({ app }: { app: GatewayClientBarApp }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);
  const api = APIS[app];
  const { data: adopted = false } = useQuery({
    queryKey: gatewayClientBarKey(app),
    queryFn: () => api.status(),
    staleTime: 10_000,
  });

  const toggle = async () => {
    setBusy(true);
    try {
      if (adopted) {
        await api.release();
      } else {
        await api.adopt();
      }
      await queryClient.invalidateQueries({ queryKey: gatewayClientBarKey(app) });
      toast.success(
        adopted
          ? t("gateway.client.released", { defaultValue: "{{app}} 已回到直连", app: api.label })
          : t("gateway.client.adopted", {
              defaultValue: "{{app}} 已指向网关",
              app: api.label,
            }),
      );
    } catch (error) {
      toast.error(
        t("gateway.client.failed", {
          defaultValue: "切换失败：{{error}}",
          error: String(error),
        }),
      );
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex items-center justify-between gap-3 rounded-panel border border-border bg-surface px-4 py-2.5">
      <div className="min-w-0 text-caption">
        <span className="text-fg-1">
          {t("gateway.client.route", {
            defaultValue: "{{app}} 的模型来源",
            app: api.label,
          })}
        </span>
        <span className="ml-2 text-fg-3">
          {adopted
            ? t("gateway.client.viaGateway", {
                defaultValue: "全部来自网关（模型 = 网关里的引用）",
              })
            : t("gateway.client.direct", {
                defaultValue: "直连（各供应商自己配地址）",
              })}
        </span>
      </div>
      <Button
        variant="neutral"
        size="sm"
        disabled={busy}
        onClick={() => void toggle()}
      >
        {busy
          ? t("gateway.client.busy", { defaultValue: "处理中…" })
          : adopted
            ? t("gateway.client.release", { defaultValue: "释放" })
            : t("gateway.client.adopt", { defaultValue: "指向网关" })}
      </Button>
    </div>
  );
}
