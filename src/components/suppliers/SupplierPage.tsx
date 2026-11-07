import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Boxes,
  Download,
  Loader2,
  MoreHorizontal,
  Pencil,
  Plus,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { authApi } from "@/lib/api";
import { kimiOAuthApi } from "@/lib/api/kimiOAuth";
import type { ManagedAuthProvider } from "@/lib/api/auth";
import {
  suppliersApi,
  type DeleteSupplierSkipped,
  type Supplier,
} from "@/lib/api/suppliers";
import { suppliersKeys } from "@/lib/query/suppliers";
import { getAppLabel } from "@/config/appConfig";
import { toast } from "@/lib/toast";
import { cn } from "@/lib/utils";
import { extractErrorMessage } from "@/utils/errorUtils";
import { AppPageHeader } from "@/components/shell/AppPageHeader";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { HelpTip } from "@/components/ui/help-tip";
import { Notice } from "@/components/ui/notice";
import { Switch } from "@/components/ui/switch";
import { SupplierModelsPanel } from "@/components/suppliers/SupplierModelsPanel";
import { SupplierManagedAccounts } from "@/components/suppliers/SupplierManagedAccounts";
import { SupplierDshAccountCard } from "@/components/suppliers/SupplierDshAccountCard";
import { SupplierEditDialog } from "@/components/suppliers/SupplierEditDialog";
import {
  adoptTarget,
  collectAppTypes,
  groupSuppliers,
  isEnabled,
  toggleTargets,
  type ProviderGroup,
} from "@/components/suppliers/providerGrouping";
import { KimiAccountLogin } from "@/components/providers/KimiAccountLogin";
import { useDshAccountStatus } from "@/components/providers/DshAccount";
import { CreateSupplierDialog } from "./CreateSupplierDialog";
import { protocolLabel } from "./protocolLabel";
import { useSupplierAccountSwitch } from "./useSupplierAccountSwitch";

/**
 * 「采用到…」的候选 app：直连模式下没有代理做转换，下游协议必须等于上游协议。
 * 后端 adopt_supplier 对每种 app 都有忠实渲染器；这里只列出协议匹配的应用，
 * 协议不在表里的供应商（如 opencode / kimi 这类要 models 列表的）不显示按钮。
 */
const ADOPT_TARGET_APPS: Record<string, readonly string[]> = {
  anthropic: ["claude", "claude-desktop"],
  openai_responses: ["codex"],
  openai_chat: ["codex"],
  gemini_native: ["gemini"],
};

function adoptTargetApps(protocol: string): readonly string[] {
  return ADOPT_TARGET_APPS[protocol] ?? [];
}

/**
 * 删除时被跳过的配置：只列前三条「App（后端原因）」，剩下的折成「等 N 条」，
 * 免得一次弹一整屏；原因由后端给（中文），这里不翻译。
 */
function formatSkippedDetail(skipped: DeleteSupplierSkipped[]): string {
  const head = skipped
    .slice(0, 3)
    .map((item) => `${getAppLabel(item.appType)}（${item.reason}）`);
  const remaining = skipped.length - head.length;
  return remaining > 0
    ? `${head.join("、")} 等 ${remaining} 条`
    : head.join("、");
}

/**
 * 供应商页：一家人 = 一组凭据 + 一组接入。
 *
 * - 「供应商」是网关级的上游（服务根 + 上游协议 + 凭据/账号），与 app 无关；
 * - 「接入」是某个 app 里指向这家供应商的那条配置，属于 app；
 * - key 型供应商的凭据是密钥，托管型供应商的凭据是账号池里的账号；
 * - 卡片只显示「被使用」的 app 列表，不再铺开端点 / 协议 / 每条接入的操作；
 * - 名称、地址、密钥、模型都收进「编辑」弹窗，卡片本身只保留开关和常规动作；
 * - 没有凭据的供应商（没登录的账号、只有 official 占位）不出现。
 */
export function SupplierPage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { data, isPending, isError, error, refetch, isFetching } = useQuery({
    queryKey: suppliersKeys.overview,
    queryFn: () => suppliersApi.overview(),
  });

  const [editGroup, setEditGroup] = useState<ProviderGroup | null>(null);
  const [adoptingGroupId, setAdoptingGroupId] = useState<string | null>(null);
  const [togglingGroupId, setTogglingGroupId] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<ProviderGroup | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);

  const suppliers = data ?? [];
  const groups = useMemo(() => groupSuppliers(suppliers), [suppliers]);

  /**
   * 页面上出现哪些 provider：托管型要求账号池里至少有一个账号；key 型要求至少有一把
   * 可写的密钥；Kimi 这种混合型，只要有账号或密钥就保留。
   * 没有登录的 Claude / Copilot / xAI … 不再显示占位卡。
   */
  const managedProviders = useMemo(() => {
    const providers: ManagedAuthProvider[] = [];
    for (const group of groups) {
      if (
        group.managedAuthProvider &&
        !providers.includes(group.managedAuthProvider)
      ) {
        providers.push(group.managedAuthProvider);
      }
    }
    return providers;
  }, [groups]);

  const authStatusQueries = useQueries({
    queries: managedProviders.map((provider) => ({
      queryKey: ["managed-auth-status", provider],
      queryFn: () => authApi.authGetStatus(provider),
      staleTime: 30_000,
    })),
  });

  const accountCounts = useMemo(() => {
    const counts = new Map<ManagedAuthProvider, number>();
    managedProviders.forEach((provider, index) => {
      counts.set(
        provider,
        authStatusQueries[index]?.data?.accounts.length ?? 0,
      );
    });
    return counts;
  }, [managedProviders, authStatusQueries]);

  // DeepSeek Harness 的官方账号不是供应商行（模型由它自带的账号插件提供），
  // 但账号区要和 Kimi / 托管账号长在同一处，所以登录了就在列表里补一张账号卡。
  const dshAccount = useDshAccountStatus();
  const dshSignedIn = dshAccount.data?.signedIn === true;

  const hasKimiGroup = groups.some((group) => group.kimiManaged);
  const kimiStatus = useQuery({
    queryKey: ["kimi-oauth-status"],
    queryFn: () => kimiOAuthApi.status(),
    staleTime: 30_000,
    enabled: hasKimiGroup,
  });

  const visibleGroups = useMemo(() => {
    return groups.filter((group) => {
      if (group.managedAuthProvider) {
        return (accountCounts.get(group.managedAuthProvider) ?? 0) > 0;
      }
      if (
        group.credentialKinds.some(
          (kind) => kind === "literal" || kind === "env_ref",
        )
      ) {
        return true;
      }
      if (group.kimiManaged) {
        const accounts = kimiStatus.data;
        return Boolean(
          accounts && (accounts.accounts.length > 0 || accounts.primaryHidden),
        );
      }
      return false;
    });
  }, [groups, accountCounts, kimiStatus.data]);

  const saveOverview = (updated: Supplier[]) => {
    queryClient.setQueryData<Supplier[]>(suppliersKeys.overview, updated);
    void queryClient.invalidateQueries({ queryKey: ["providers"] });
  };

  // 凭据/模型等即时操作返回新总览后，同步缓存并把弹窗切到更新后的卡片
  const handleEditOverview = (updated: Supplier[]) => {
    saveOverview(updated);
    if (!editGroup) return;
    const ids = new Set(editGroup.suppliers.map((supplier) => supplier.id));
    const next = groupSuppliers(updated).find((group) =>
      group.suppliers.some((supplier) => ids.has(supplier.id)),
    );
    if (next) setEditGroup(next);
  };

  // 供应商级换账号：与编辑弹窗共用同一套行为（见 useSupplierAccountSwitch）
  const {
    switchTo: switchManagedAccount,
    switchingGroupId,
    toggleEnabled: toggleManagedAccount,
    togglingGroupId: togglingAccountGroupId,
    accountEnabledOf,
  } = useSupplierAccountSwitch(saveOverview);

  // 一键采用：用组里那条 anthropic 上游把 provider 接进 agent
  const adoptInto = async (group: ProviderGroup, app: string) => {
    if (adoptingGroupId) return;
    const supplier = adoptTarget(group.suppliers);
    if (!supplier) return;

    setAdoptingGroupId(group.id);
    try {
      const adopted = await suppliersApi.adopt(app, supplier.id);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["providers"] }),
        queryClient.invalidateQueries({ queryKey: ["suppliers"] }),
        queryClient.invalidateQueries({ queryKey: suppliersKeys.overview }),
      ]);
      toast.success(
        t("suppliers.adoptSuccess", {
          app: getAppLabel(app),
          name: adopted.providerName,
        }),
      );
    } catch (adoptError) {
      toast.error(
        extractErrorMessage(adoptError) || t("suppliers.adoptFailed"),
      );
    } finally {
      setAdoptingGroupId(null);
    }
  };

  // 启停：一张卡的底层 supplier 行一起拨；托管登录行没有网关，跳过
  const toggleEnabled = async (group: ProviderGroup) => {
    if (togglingGroupId) return;
    const targets = toggleTargets(group.suppliers);
    if (targets.length === 0) return;
    const nextEnabled = !group.suppliers.every(isEnabled);

    setTogglingGroupId(group.id);
    let updated: Supplier[] | null = null;
    let lastError: unknown = null;
    try {
      for (const supplier of targets) {
        try {
          updated = await suppliersApi.setEnabled(supplier.id, nextEnabled);
        } catch (toggleError) {
          lastError = toggleError;
        }
      }
      if (updated) {
        saveOverview(updated);
        toast.success(
          t(
            nextEnabled ? "suppliers.enabledToast" : "suppliers.disabledToast",
            { name: group.name },
          ),
        );
      } else {
        toast.error(
          extractErrorMessage(lastError) || t("suppliers.toggleFailed"),
        );
      }
    } finally {
      setTogglingGroupId(null);
    }
  };

  // 删除整张卡：把它底下的每条 supplier 行一起删，逐条汇总结果
  const confirmDelete = async () => {
    if (!deleteTarget || deleting) return;
    const group = deleteTarget;

    setDeleting(true);
    try {
      let updated: Supplier[] | null = null;
      let deleted = 0;
      const skipped: DeleteSupplierSkipped[] = [];
      let lastError: unknown = null;

      for (const supplier of group.suppliers) {
        try {
          const outcome = await suppliersApi.remove(supplier.id);
          updated = outcome.suppliers;
          deleted += outcome.deleted;
          skipped.push(...outcome.skipped);
        } catch (deleteError) {
          lastError = deleteError;
        }
      }

      if (updated) {
        queryClient.setQueryData<Supplier[]>(suppliersKeys.overview, updated);
        void queryClient.invalidateQueries({ queryKey: ["providers"] });
        void queryClient.invalidateQueries({
          queryKey: ["gateway", "catalog"],
        });
        setDeleteTarget(null);

        if (deleted > 0) {
          toast.success(
            t("suppliers.deleteSuccess", {
              name: group.name,
              count: deleted,
            }),
          );
        }
        if (skipped.length > 0) {
          toast.warning(
            t("suppliers.deleteSkipped", {
              count: skipped.length,
              detail: formatSkippedDetail(skipped),
            }),
          );
        }
      } else {
        toast.error(
          extractErrorMessage(lastError) || t("suppliers.deleteFailed"),
        );
      }
    } finally {
      setDeleting(false);
    }
  };

  const deleteMessage = deleteTarget
    ? t("suppliers.deleteDescription", {
        name: deleteTarget.name,
        count: collectAppTypes(deleteTarget).length,
        bindings: deleteTarget.bindings.length,
        apps: collectAppTypes(deleteTarget)
          .map((appType) => getAppLabel(appType))
          .join("、"),
      })
    : "";

  const renderBody = () => {
    if (isPending) {
      return (
        <div className="space-y-2" aria-busy="true">
          {[0, 1, 2].map((index) => (
            <div
              key={index}
              className="h-[60px] w-full rounded-panel border border-dashed border-border bg-subtle"
            />
          ))}
        </div>
      );
    }

    if (isError) {
      return (
        <Notice
          tone="danger"
          title={t("suppliers.loadFailed")}
          actions={
            <Button
              type="button"
              variant="neutral"
              size="compact"
              onClick={() => void refetch()}
            >
              {t("common.retry")}
            </Button>
          }
        >
          {extractErrorMessage(error) || t("suppliers.loadFailedHint")}
        </Notice>
      );
    }

    if (visibleGroups.length === 0 && !dshSignedIn) {
      return (
        <div className="rounded-panel border border-dashed border-border px-6 py-8 text-center text-body text-fg-2">
          {t("suppliers.empty")}
        </div>
      );
    }

    return (
      <ul className="m-0 list-none space-y-2 p-0">
        {dshSignedIn ? <SupplierDshAccountCard /> : null}
        {visibleGroups.map((group) => {
          const primary = group.primary;
          const managed = Boolean(group.managedAuthProvider);
          const enabled = group.suppliers.every(isEnabled);
          const isToggling = togglingGroupId === group.id;
          const isAdopting = adoptingGroupId === group.id;
          const protocols = Array.from(
            new Set(
              group.suppliers.map((supplier) => supplier.upstreamProtocol),
            ),
          );
          const adoptSupplier = adoptTarget(group.suppliers);
          const adoptApps = adoptSupplier
            ? adoptTargetApps(adoptSupplier.upstreamProtocol)
            : [];
          const appTypes = collectAppTypes(group);
          const showProtocolBadge =
            protocols.length === 1 && protocols[0] !== "unknown";

          return (
            <li
              key={group.id}
              className="rounded-panel border border-border bg-surface"
            >
              <div className="px-4 py-3">
                <div className="flex min-w-0 items-start gap-3">
                  <div
                    className={cn("min-w-0 flex-1", !enabled && "opacity-60")}
                  >
                    <div className="flex min-w-0 flex-wrap items-center gap-2">
                      <span className="min-w-0 truncate text-strong text-fg-1">
                        {group.name}
                      </span>
                      {showProtocolBadge && (
                        <Badge
                          variant="outline"
                          title={t("suppliers.upstreamProtocol")}
                          className="h-[18px] shrink-0 rounded-full px-1.5 text-badge font-medium text-fg-2"
                        >
                          {protocolLabel(protocols[0])}
                        </Badge>
                      )}
                      {!enabled && (
                        <Badge
                          variant="outline"
                          title={t("suppliers.disabledToggleHint")}
                          className="h-[18px] shrink-0 rounded-full px-1.5 text-badge font-medium text-fg-3"
                        >
                          {t("suppliers.disabledBadge")}
                        </Badge>
                      )}
                      {primary.baseUrl ? (
                        <code
                          className="min-w-0 font-mono text-caption text-fg-2 [overflow-wrap:anywhere]"
                          title={primary.baseUrl}
                        >
                          {primary.baseUrl}
                        </code>
                      ) : (
                        <span className="text-caption text-fg-3">
                          {t(
                            managed
                              ? "suppliers.noBaseUrl"
                              : "suppliers.noBaseUrlKey",
                          )}
                        </span>
                      )}
                    </div>
                  </div>

                  <div className="flex shrink-0 flex-wrap items-center justify-end gap-2">
                    {toggleTargets(group.suppliers).length > 0 && (
                      <Switch
                        checked={enabled}
                        disabled={togglingGroupId !== null}
                        aria-busy={isToggling}
                        aria-label={
                          enabled
                            ? t("suppliers.disableAction")
                            : t("suppliers.enableAction")
                        }
                        title={
                          enabled
                            ? t("suppliers.enabledToggleHint")
                            : t("suppliers.disabledToggleHint")
                        }
                        onCheckedChange={() => void toggleEnabled(group)}
                      />
                    )}
                    <Button
                      type="button"
                      variant="neutral"
                      size="compact"
                      onClick={() => setEditGroup(group)}
                    >
                      <Pencil aria-hidden="true" className="h-3.5 w-3.5" />
                      {t("suppliers.editAction")}
                    </Button>
                    {!managed && adoptSupplier && adoptApps.length > 0 && (
                      <DropdownMenu>
                        <span
                          title={
                            !enabled
                              ? t("suppliers.disabledAdoptHint")
                              : undefined
                          }
                          className="inline-flex"
                        >
                          <DropdownMenuTrigger asChild>
                            <Button
                              type="button"
                              variant="neutral"
                              size="compact"
                              disabled={!enabled || adoptingGroupId !== null}
                              aria-busy={isAdopting}
                            >
                              {isAdopting ? (
                                <Loader2
                                  aria-hidden="true"
                                  className="h-3.5 w-3.5 animate-spin"
                                />
                              ) : (
                                <Download
                                  aria-hidden="true"
                                  className="h-3.5 w-3.5"
                                />
                              )}
                              {t("suppliers.adoptTo")}
                            </Button>
                          </DropdownMenuTrigger>
                        </span>
                        <DropdownMenuContent
                          align="end"
                          className="min-w-[160px]"
                        >
                          {adoptApps.map((app) => (
                            <DropdownMenuItem
                              key={app}
                              onSelect={() => void adoptInto(group, app)}
                            >
                              {getAppLabel(app)}
                            </DropdownMenuItem>
                          ))}
                        </DropdownMenuContent>
                      </DropdownMenu>
                    )}
                    {!managed && (
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button
                            type="button"
                            variant="neutral"
                            size="compact"
                            aria-label={t("suppliers.moreActions")}
                            title={t("suppliers.moreActions")}
                          >
                            <MoreHorizontal
                              aria-hidden="true"
                              className="h-3.5 w-3.5"
                            />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent
                          align="end"
                          className="min-w-[160px]"
                        >
                          <DropdownMenuItem
                            onSelect={() => setDeleteTarget(group)}
                            className="text-danger-text focus:text-danger-text"
                          >
                            <Trash2
                              aria-hidden="true"
                              className="h-3.5 w-3.5"
                            />
                            {t("suppliers.deleteAction")}
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    )}
                  </div>
                </div>
              </div>

              {group.managedAuthProvider ? (
                <div className={cn(!enabled && "opacity-60")}>
                  <SupplierManagedAccounts
                    provider={group.managedAuthProvider}
                    headerLabel={t("suppliers.credentialsTitle")}
                    onSetDefaultAccount={(accountId) =>
                      void switchManagedAccount(group, accountId)
                    }
                    settingDefaultAccount={switchingGroupId === group.id}
                    accountEnabled={(accountId) =>
                      accountEnabledOf(group, accountId)
                    }
                    onToggleAccountEnabled={(accountId, enabled) =>
                      void toggleManagedAccount(group, accountId, enabled)
                    }
                    togglingAccountEnabled={togglingAccountGroupId === group.id}
                  />
                </div>
              ) : (
                <div className={cn(!enabled && "opacity-60")}>
                  {group.kimiManaged && (
                    <KimiAccountLogin
                      embedded
                      headerLabel={t("suppliers.credentialsTitle")}
                    />
                  )}
                  <div className="flex flex-wrap items-center gap-2 border-t border-border px-4 py-3 text-caption text-fg-3">
                    {!group.kimiManaged && (
                      <span className="text-fg-2">
                        {t("suppliers.credentialsTitle")}
                      </span>
                    )}
                    {group.distinctKeyCount > 0 && (
                      <span>
                        {t("suppliers.keyCount", {
                          count: group.distinctKeyCount,
                        })}
                      </span>
                    )}
                    {group.credentialKinds
                      .filter(
                        (kind) =>
                          kind !== "literal" &&
                          kind !== "none" &&
                          !(group.kimiManaged && kind === "managed"),
                      )
                      .map((kind) => (
                        <span
                          key={kind}
                          className="rounded-full bg-subtle px-1.5 py-0.5 text-badge text-fg-2"
                        >
                          {t(`suppliers.credentialKind.${kind}`, {
                            defaultValue: kind,
                          })}
                        </span>
                      ))}
                  </div>
                </div>
              )}

              <div
                className={cn(
                  "flex flex-wrap items-center gap-1.5 px-4 pb-3 text-caption text-fg-3",
                  !enabled && "opacity-60",
                )}
              >
                <span>{t("suppliers.usedBy")}</span>
                {appTypes.length === 0 ? (
                  <span>{t("suppliers.unused")}</span>
                ) : (
                  appTypes.map((appType) => (
                    <span
                      key={appType}
                      className="rounded-full bg-subtle px-1.5 py-0.5 text-badge text-fg-2"
                      title={appType}
                    >
                      {getAppLabel(appType)}
                    </span>
                  ))
                )}
              </div>

              <SupplierModelsPanel upstreamIds={group.upstreamIds} />
            </li>
          );
        })}
      </ul>
    );
  };

  return (
    <>
      <AppPageHeader
        icon={<Boxes className="h-5 w-5" strokeWidth={1.5} />}
        title={t("nav.suppliers")}
        titleExtra={
          <HelpTip title={t("suppliers.helpTitle")}>
            {t("suppliers.helpBody")}
          </HelpTip>
        }
        actions={
          <>
            <Button
              type="button"
              variant="quiet"
              size="regular"
              disabled={isFetching}
              aria-busy={isFetching}
              onClick={() => void refetch()}
            >
              <RefreshCw
                aria-hidden="true"
                className={cn("h-4 w-4", isFetching && "animate-spin")}
                strokeWidth={2}
              />
              {t("common.refresh")}
            </Button>
            <Button
              type="button"
              variant="solid"
              size="regular"
              onClick={() => setCreateOpen(true)}
            >
              <Plus aria-hidden="true" className="h-4 w-4" strokeWidth={2} />
              {t("suppliers.createAction")}
            </Button>
          </>
        }
      />

      <div
        id="main-content"
        className="min-h-0 flex-1 overflow-y-auto scroll-stable"
      >
        <div className="space-y-4 px-6 pb-10 pt-4">
          <p className="m-0 text-body text-fg-2">
            {t("suppliers.description")}
          </p>
          {renderBody()}
        </div>
      </div>

      <CreateSupplierDialog
        open={createOpen}
        onClose={() => setCreateOpen(false)}
      />

      <SupplierEditDialog
        group={editGroup}
        onClose={() => setEditGroup(null)}
        onOverview={handleEditOverview}
      />

      <ConfirmDialog
        isOpen={deleteTarget !== null}
        title={t("suppliers.deleteTitle")}
        message={deleteMessage}
        confirmText={t("suppliers.deleteConfirm")}
        variant="destructive"
        pending={deleting}
        onConfirm={() => void confirmDelete()}
        onCancel={() => {
          if (!deleting) setDeleteTarget(null);
        }}
      />
    </>
  );
}

export default SupplierPage;
