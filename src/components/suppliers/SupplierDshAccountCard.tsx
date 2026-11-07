import { useTranslation } from "react-i18next";

import { DshAccountLogin } from "@/components/providers/DshAccount";
import { Badge } from "@/components/ui/badge";

/**
 * 供应商页里的 DeepSeek Harness 账号卡。
 *
 * DSH 的官方账号不是「供应商行」：模型由 DSH 自带的账号插件提供，登录写在
 * `$DSH_HOME/.credentials.yaml`，Web 与 Desktop 共用。所以这张卡只有账号区，
 * 没有地址 / 协议 / 启停 / 删除——和 Kimi 卡里的账号区长得一模一样，账号操作也同一套。
 */
export function SupplierDshAccountCard() {
  const { t } = useTranslation();

  return (
    <li
      className="rounded-panel border border-border bg-surface"
      data-testid="dsh-account-supplier"
    >
      <div className="px-4 py-3">
        <div className="flex min-w-0 items-start gap-3">
          <div className="min-w-0 flex-1">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <span className="min-w-0 truncate text-strong text-fg-1">
                DeepSeek Harness
              </span>
              <Badge
                variant="outline"
                className="h-[18px] shrink-0 rounded-full px-1.5 text-badge font-medium text-fg-2"
              >
                {t("providerCard.chip.official")}
              </Badge>
              <span className="text-caption text-fg-3">
                {t("suppliers.dshAccountUsage")}
              </span>
            </div>
          </div>
        </div>
      </div>

      <DshAccountLogin embedded headerLabel={t("suppliers.credentialsTitle")} />
    </li>
  );
}
