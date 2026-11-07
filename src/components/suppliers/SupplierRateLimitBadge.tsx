import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { TimerReset } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { useRateLimitStates } from "@/lib/query/proxy";
import { cn } from "@/lib/utils";

/**
 * 供应商卡片的实时限流状态徽章。
 *
 * 数据来自自适应 429 退避控制器（get_rate_limit_states，内存快照 + 3s 轮询）。
 * 供应商组可能横跨多个端点（同一上游不同协议），按 providerId 匹配组内任一
 * 端点：冷却中显示剩余秒数（最严重的那条），没在冷却但近 60s 有 429 则显示
 * 次数。无状态渲染 null，不占布局。
 */
export function SupplierRateLimitBadge({
  providerIds,
}: {
  providerIds: string[];
}) {
  const { t } = useTranslation();
  const { data: states } = useRateLimitStates();

  const relevant = useMemo(() => {
    if (!states || providerIds.length === 0) return [];
    const idSet = new Set(providerIds);
    return states.filter((state) =>
      state.owners.some((owner) => idSet.has(owner.providerId)),
    );
  }, [states, providerIds]);

  if (relevant.length === 0) return null;

  const cooling = relevant
    .filter((state) => state.cooldownRemainingMs > 0)
    .sort((a, b) => b.cooldownRemainingMs - a.cooldownRemainingMs)[0];
  const recent429 = relevant.reduce(
    (sum, state) => sum + state.recent429Count,
    0,
  );

  if (cooling) {
    const secs = Math.max(1, Math.ceil(cooling.cooldownRemainingMs / 1000));
    return (
      <Badge
        variant="outline"
        title={t("suppliers.rateLimitCoolingHint", {
          secs,
          level: cooling.levelSecs,
          count: cooling.recent429Count,
          defaultValue:
            "该供应商的某个端点正在 429 冷却：退避档位 {{level}}s、近 60s {{count}} 次，冷却期内的请求由代理本地秒拒，不再打上游",
        })}
        className="h-[18px] shrink-0 gap-1 rounded-full px-1.5 text-badge font-medium text-danger-text"
      >
        <TimerReset aria-hidden="true" className="h-3 w-3" />
        {t("suppliers.rateLimitCooling", {
          secs,
          defaultValue: "429 冷却 剩{{secs}}s",
        })}
      </Badge>
    );
  }

  if (recent429 > 0) {
    return (
      <Badge
        variant="outline"
        title={t(
          "suppliers.rateLimitRecentHint",
          "近 60s 该供应商的端点出现过 429，退避控制器已记录",
        )}
        className={cn(
          "h-[18px] shrink-0 rounded-full px-1.5 text-badge font-medium text-fg-2",
        )}
      >
        {t("suppliers.rateLimitRecent", {
          count: recent429,
          defaultValue: "近60s {{count}}次429",
        })}
      </Badge>
    );
  }

  return null;
}
