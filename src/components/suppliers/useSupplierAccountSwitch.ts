import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "@/lib/toast";
import { authApi } from "@/lib/api";
import { suppliersApi, type Supplier } from "@/lib/api/suppliers";
import { extractErrorMessage } from "@/utils/errorUtils";
import type { ProviderGroup } from "./providerGrouping";

/**
 * 供应商级换账号：卡片和编辑弹窗共用同一套行为，避免两个入口语义分叉。
 *
 * - 账号池存在时只切 current，绝不把所有接入重绑到同一个账号；
 *   账号还不在池里就先加进池（历史数据），再切 current。
 * - 账号池为空时才回落到旧的单账号绑定。
 * - 切完同步账号池的全局默认账号，保证「一处切换、里外一起变」。
 */
export function useSupplierAccountSwitch(
  onOverview: (suppliers: Supplier[]) => void,
) {
  const [switchingGroupId, setSwitchingGroupId] = useState<string | null>(null);
  const [togglingGroupId, setTogglingGroupId] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  const switchTo = async (group: ProviderGroup, accountId: string) => {
    const provider = group.managedAuthProvider;
    if (!provider || switchingGroupId) return;

    setSwitchingGroupId(group.id);
    try {
      const supplierIds = group.suppliers.map((supplier) => supplier.id);
      const poolAccountIds = (group.credentials ?? [])
        .filter(
          (credential) =>
            credential.secretKind === "managed" && credential.accountId,
        )
        .map((credential) => credential.accountId!);
      const inPool = poolAccountIds.includes(accountId);
      const updated =
        poolAccountIds.length > 0
          ? await (async () => {
              const withAccount = inPool
                ? null
                : await suppliersApi.addAccount(
                    supplierIds,
                    accountId,
                    undefined,
                  );
              const nextIds = (withAccount ?? group.suppliers)
                .filter((supplier) => supplier.managedAuthProvider === provider)
                .map((supplier) => supplier.id);
              return suppliersApi.setCurrentAccount(
                withAccount ? nextIds : supplierIds,
                accountId,
              );
            })()
          : await suppliersApi.setManagedAccount(supplierIds, accountId);
      onOverview(updated);
      // Codex / Copilot / xAI 的全局默认只是「没绑账号的接入」的兜底和「默认」徽标，
      // 同步失败不影响这家供应商；agy 的默认就是它唯一的全局登录，切不过去等于没切。
      const syncDefault = authApi.authSetDefaultAccount(provider, accountId);
      await (provider === "agy_oauth"
        ? syncDefault
        : syncDefault.catch(() => undefined));
      await queryClient.invalidateQueries({
        queryKey: ["managed-auth-status", provider],
      });
      toast.success(
        t("suppliers.managedAccountSwitched", {
          defaultValue: "这家供应商已改用该账号",
        }),
      );
    } catch (error) {
      toast.error(
        extractErrorMessage(error) ||
          t("suppliers.managedAccountSwitchFailed", {
            defaultValue: "换账号失败，请重试",
          }),
      );
    } finally {
      setSwitchingGroupId(null);
    }
  };

  /**
   * 停用/启用一个账号：保留账号本身，只把它从「参与使用」里拿出去。
   * 后端语义是「这个账号的接入是否进故障转移队列」，所以最后一个可用账号不能被停用。
   */
  const toggleEnabled = async (
    group: ProviderGroup,
    accountId: string,
    enabled: boolean,
  ) => {
    if (!group.managedAuthProvider || togglingGroupId) return;
    setTogglingGroupId(group.id);
    try {
      const updated = await suppliersApi.setAccountEnabled(
        group.suppliers.map((supplier) => supplier.id),
        accountId,
        enabled,
      );
      onOverview(updated);
      toast.success(
        enabled
          ? t("suppliers.accountEnabled", { defaultValue: "账号已启用" })
          : t("suppliers.accountDisabled", {
              defaultValue: "账号已停用，切换时不会再选它",
            }),
      );
    } catch (error) {
      toast.error(
        extractErrorMessage(error) ||
          t("suppliers.accountToggleFailed", {
            defaultValue: "操作失败，请重试",
          }),
      );
    } finally {
      setTogglingGroupId(null);
    }
  };

  /** 供应商页里一个账号当前是否参与使用（没数据就返回 undefined，不显示停用/启用） */
  const accountEnabledOf = (
    group: ProviderGroup,
    accountId: string,
  ): boolean | undefined => {
    const credential = (group.credentials ?? []).find(
      (item) => item.secretKind === "managed" && item.accountId === accountId,
    );
    if (!credential) return undefined;
    return credential.apps.some((app) => app.inFailoverQueue || app.isCurrent);
  };

  return {
    switchTo,
    switchingGroupId,
    toggleEnabled,
    togglingGroupId,
    accountEnabledOf,
  };
}
