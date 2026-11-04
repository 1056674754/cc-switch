import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "@/lib/toast";
import { Button } from "@/components/ui/button";
import { gatewayApi } from "@/lib/api/gateway";

export const opencodeGatewayKey = ["opencode", "gatewayStatus"] as const;

/** OpenCode 页顶的「走网关 / 直连」开关：接管时写 cc-switch provider，释放时删掉。 */
export function OpencodeGatewayBar() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);
  const { data: adopted = false } = useQuery({
    queryKey: opencodeGatewayKey,
    queryFn: () => gatewayApi.getOpencodeGatewayStatus(),
    staleTime: 10_000,
  });

  const toggle = async () => {
    setBusy(true);
    try {
      if (adopted) {
        await gatewayApi.releaseOpencodeFromGateway();
      } else {
        await gatewayApi.adoptOpencodeToGateway();
      }
      await queryClient.invalidateQueries({ queryKey: opencodeGatewayKey });
      toast.success(
        adopted
          ? t("gateway.opencode.released", { defaultValue: "OpenCode 已回到直连" })
          : t("gateway.opencode.adopted", {
              defaultValue: "OpenCode 已指向网关",
            }),
      );
    } catch (error) {
      toast.error(
        t("gateway.opencode.failed", {
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
          {t("gateway.opencode.route", { defaultValue: "OpenCode 的模型来源" })}
        </span>
        <span className="ml-2 text-fg-3">
          {adopted
            ? t("gateway.opencode.viaGateway", {
                defaultValue: "全部来自网关（模型 = 网关里的引用）",
              })
            : t("gateway.opencode.direct", {
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
          ? t("gateway.opencode.busy", { defaultValue: "处理中…" })
          : adopted
            ? t("gateway.opencode.release", { defaultValue: "释放" })
            : t("gateway.opencode.adopt", { defaultValue: "指向网关" })}
      </Button>
    </div>
  );
}
