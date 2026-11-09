import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { RotateCcw } from "lucide-react";
import { toast } from "sonner";
import { useCodexOauthQuotaByAccountId } from "@/lib/query/subscription";
import { subscriptionApi } from "@/lib/api/subscription";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { extractErrorMessage } from "@/utils/errorUtils";
import {
  AccountQuotaColumn,
  subscriptionQuotaState,
} from "@/components/settings/auth/AccountQuota";
import { useQuotaDisplay } from "@/components/quota/useQuotaDisplay";

interface CodexOauthAccountQuotaProps {
  /** cc-switch 自管的 ChatGPT 账号 ID */
  accountId: string;
  /** 账号名，拼进刷新按钮的无障碍名字 */
  login?: string;
}

/**
 * 授权中心里单个 ChatGPT (Codex OAuth) 账号的额度：账号行右侧的额度条 + 「x 分钟前 ↻」。
 *
 * 按 accountId 查 cc-switch 自管 OAuth token 的订阅额度（和绑定同一账号的供应商卡片共用缓存），
 * 打开页面时查一次，不轮询；点 ↻ 手动重查。
 *
 * 有存下的「限额重置卡」且已经有额度窗口用完时，额度下面显示「使用重置卡」：
 * 用一张清空已用完的窗口（不可恢复，二次确认；用完自动重查额度）。
 */
const CodexOauthAccountQuota: React.FC<CodexOauthAccountQuotaProps> = ({
  accountId,
  login = "",
}) => {
  const { t, i18n } = useTranslation();
  const display = useQuotaDisplay();
  const {
    data: quota,
    isFetching: loading,
    refetch,
  } = useCodexOauthQuotaByAccountId(accountId, {
    enabled: true,
    autoQuery: false,
  });

  const [confirming, setConfirming] = useState(false);
  const [using, setUsing] = useState(false);

  const credits = quota?.resetCredits?.credits ?? [];
  const exhausted = (quota?.tiers ?? []).some(
    (tier) => tier.utilization >= 100,
  );
  const canUseCredit = credits.length > 0 && exhausted;

  const useCredit = async () => {
    setUsing(true);
    try {
      const result = await subscriptionApi.consumeCodexResetCredit(accountId);
      toast.success(
        t("quota.resetCredits.useSuccess", {
          defaultValue: "已用掉一张重置卡，额度窗口已重置",
          windows: result.windowsReset ?? 0,
        }),
      );
      setConfirming(false);
      await refetch();
    } catch (error) {
      toast.error(
        t("quota.resetCredits.useFailed", {
          defaultValue: "使用重置卡失败：{{error}}",
          error: extractErrorMessage(error),
        }),
      );
    } finally {
      setUsing(false);
    }
  };

  return (
    <>
      <AccountQuotaColumn
        login={login}
        state={subscriptionQuotaState(
          t,
          quota,
          loading,
          i18n.language,
          display,
        )}
        queriedAt={quota?.queriedAt ?? null}
        loading={loading}
        onRefresh={() => void refetch()}
        resetCreditAction={
          canUseCredit ? (
            <Button
              type="button"
              variant="neutral"
              size="sm"
              className="h-7 w-full justify-center gap-1.5 text-caption"
              disabled={using}
              onClick={() => setConfirming(true)}
            >
              <RotateCcw className="size-3" strokeWidth={1.7} />
              {t("quota.resetCredits.useAction", {
                defaultValue: "使用重置卡",
              })}
            </Button>
          ) : undefined
        }
      />
      <ConfirmDialog
        isOpen={confirming}
        title={t("quota.resetCredits.useConfirmTitle", {
          defaultValue: "使用一张重置卡？",
        })}
        message={t("quota.resetCredits.useConfirmMessage", {
          defaultValue:
            "这会永久用掉一张重置卡（剩余 {{count}} 张），立即清空已用完的额度窗口，无法撤销。",
          count: credits.length,
        })}
        confirmText={t("quota.resetCredits.useConfirmAction", {
          defaultValue: "使用",
        })}
        pending={using}
        onConfirm={() => void useCredit()}
        onCancel={() => setConfirming(false)}
      />
    </>
  );
};

export default CodexOauthAccountQuota;
