import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Boxes,
  Download,
  Eye,
  EyeOff,
  KeyRound,
  Loader2,
  MoreHorizontal,
  Pencil,
  Plus,
  RefreshCw,
  Trash2,
} from "lucide-react";
import {
  suppliersApi,
  type DeleteSupplierSkipped,
  type Supplier,
  type SupplierBindingDetail,
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
import { Input } from "@/components/ui/input";
import { Notice } from "@/components/ui/notice";
import { Switch } from "@/components/ui/switch";
import { ModelMapEditor } from "@/components/bindings/ModelMapEditor";
import { SupplierModelsPanel } from "@/components/suppliers/SupplierModelsPanel";
import {
  SupplierAccounts,
  isManagedAuthProvider,
} from "@/components/suppliers/SupplierAccounts";
import {
  supportsModelMap,
  toModelMapDraft,
  toModelMapPayload,
  type ModelMapDraft,
  type ModelMapSlot,
} from "@/components/bindings/modelMap";
import { CreateSupplierDialog } from "./CreateSupplierDialog";
import { protocolLabel } from "./protocolLabel";

/**
 * 「采用到…」的候选 app：后端 adopt_supplier 目前只支持 claude / claude-desktop，
 * 其它 app 会 reject 并给出中文原因（它要求配置里按供应商维护模型列表 / 认证）。
 */
const ADOPT_TARGET_APPS = ["claude", "claude-desktop"] as const;

interface EditingBaseUrl {
  supplierId: string;
  value: string;
}

/**
 * 换密钥的就地编辑器状态。
 *
 * 永远从空串开始：旧密钥后端不返回、界面也不保存，这里只接收「新密钥」。
 */
interface EditingSecret {
  supplierId: string;
  value: string;
  /** 是否切成明文显示（默认遮住） */
  revealed: boolean;
}

/** 正在编辑的模型映射：定位到某 app 的某条接入，草稿是六个槽位。 */
interface EditingModelMap {
  supplierId: string;
  appType: string;
  providerId: string;
  draft: ModelMapDraft;
}

/**
 * 被哪些 app 使用：以 apps 为准（应用名可能重名），bindings 里多出来的 app 补在后面。
 */
function collectAppTypes(supplier: Supplier): string[] {
  const appTypes = [...supplier.apps];
  for (const binding of supplier.bindings) {
    if (!appTypes.includes(binding.appType)) {
      appTypes.push(binding.appType);
    }
  }
  return appTypes;
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
 * 供应商页：网关级视角的供应商总览。
 *
 * 一家供应商 = 一个确切的上游 API（服务根 + 上游协议），所以这里是一张扁平列表：
 * 每行一家，地址改一次，所有接入它的 agent 一起被改写。
 */
export function SupplierPage({
  onOpenAuthCenter,
}: {
  /** 托管登录的供应商卡片上「管理账号」要跳到授权中心 */
  onOpenAuthCenter?: () => void;
} = {}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { data, isPending, isError, error, refetch, isFetching } = useQuery({
    queryKey: suppliersKeys.overview,
    queryFn: () => suppliersApi.overview(),
  });

  // 托管登录 / 无地址的账号不是网关，默认收起来，免得把真正的供应商淹掉
  const [showManaged, setShowManaged] = useState(false);
  const [editing, setEditing] = useState<EditingBaseUrl | null>(null);
  const [editingSecret, setEditingSecret] = useState<EditingSecret | null>(
    null,
  );
  const [savingSupplierId, setSavingSupplierId] = useState<string | null>(null);
  // 换密钥单独记一个「正在保存的供应商」：地址和密钥的请求不能互相顶掉禁用态
  const [savingSecretSupplierId, setSavingSecretSupplierId] = useState<
    string | null
  >(null);
  // 正在「采用到…」的供应商：按钮期间禁用，防止连点给同一个 agent 重复建配置
  const [adoptingSupplierId, setAdoptingSupplierId] = useState<string | null>(
    null,
  );
  // 正在编辑映射的那条接入；同时只开一个编辑器，换一条就是换草稿
  const [editingModelMap, setEditingModelMap] =
    useState<EditingModelMap | null>(null);
  const [savingModelMap, setSavingModelMap] = useState(false);
  // 正在切换启用状态的供应商：请求期间把开关禁掉，避免连点把开关拨来拨去
  const [togglingSupplierId, setTogglingSupplierId] = useState<string | null>(
    null,
  );
  // 「删除供应商」确认框的目标：非空即打开；请求期间保持打开，让按钮转成禁用态
  const [deleteTarget, setDeleteTarget] = useState<Supplier | null>(null);
  const [deleting, setDeleting] = useState(false);
  // 「新增供应商」对话框：网关侧一次配好地址 + 密钥，勾选要直接接入的 agent
  const [createOpen, setCreateOpen] = useState(false);

  const suppliers = data ?? [];

  /** 托管登录账号没有网关地址：不显示编辑地址 / 换密钥。 */
  const isManagedSupplier = (supplier: Supplier) =>
    supplier.baseUrl.trim() === "";

  /** 凭据全在账号池里（托管登录）时没有密钥可换，不摆「换密钥」。 */
  const hasEditableSecret = (supplier: Supplier) =>
    supplier.credentialKinds.some(
      (kind) => kind === "literal" || kind === "env_ref",
    );

  /** 有没有启用：老 fixture / 更早的后端不带 enabled，缺省按启用处理，免得整页变灰。 */
  const isEnabled = (supplier: Supplier) => supplier.enabled !== false;

  /**
   * 「采用到…」只在直连模式下成立：没有代理做协议转换，claude / claude-desktop
   * 的下游是 Anthropic Messages，只能接 anthropic 上游，且必须有网关地址。
   */
  const canAdopt = (supplier: Supplier) =>
    supplier.upstreamProtocol === "anthropic" && supplier.baseUrl.trim() !== "";

  const startEditing = (supplier: Supplier) => {
    // 一行只开一个编辑器：开地址编辑就把密钥编辑收起来
    setEditingSecret(null);
    setEditing({ supplierId: supplier.id, value: supplier.baseUrl });
  };

  const cancelEditing = () => setEditing(null);

  const startEditingSecret = (supplier: Supplier) => {
    // 不预填：有没有旧密钥、旧密钥是什么，界面都不知道（后端也不返回）
    setEditing(null);
    setEditingSecret({
      supplierId: supplier.id,
      value: "",
      revealed: false,
    });
  };

  const cancelEditingSecret = () => setEditingSecret(null);

  const saveBaseUrl = async (supplier: Supplier) => {
    if (!editing || editing.supplierId !== supplier.id) return;
    if (savingSupplierId) return;

    const nextUrl = editing.value.trim();
    if (!nextUrl) {
      toast.error(t("suppliers.baseUrlRequired"));
      return;
    }
    if (nextUrl === supplier.baseUrl.trim()) {
      setEditing(null);
      return;
    }

    setSavingSupplierId(supplier.id);
    try {
      const updated = await suppliersApi.updateBaseUrl(supplier.id, nextUrl);
      // 后端返回的就是更新后的总览，直接落缓存，不再多拉一次
      queryClient.setQueryData<Supplier[]>(suppliersKeys.overview, updated);
      // 各应用里存的供应商配置已经被一起改写，让这些列表下次进入时重新读
      void queryClient.invalidateQueries({ queryKey: ["providers"] });
      setEditing(null);
      toast.success(t("suppliers.updateSuccess"));
    } catch (saveError) {
      // Tauri invoke 的 reject 值是后端序列化出的纯字符串，extractErrorMessage 能取到
      toast.error(
        extractErrorMessage(saveError) || t("suppliers.updateFailed"),
      );
    } finally {
      setSavingSupplierId(null);
    }
  };

  // 换密钥：一把密钥换掉这家供应商下所有 app 的；后端不接受空串，这里先拦一道
  const saveSecret = async (supplier: Supplier) => {
    if (!editingSecret || editingSecret.supplierId !== supplier.id) return;
    if (savingSecretSupplierId) return;

    const nextSecret = editingSecret.value.trim();
    if (!nextSecret) {
      toast.error(t("suppliers.secretRequired"));
      return;
    }

    setSavingSecretSupplierId(supplier.id);
    try {
      const updated = await suppliersApi.updateSecret(supplier.id, nextSecret);
      // 后端返回的就是更新后的总览，直接落缓存
      queryClient.setQueryData<Supplier[]>(suppliersKeys.overview, updated);
      // 各应用里存的密钥已经被一起改写（界面只显示 credentialCount），让列表重读
      void queryClient.invalidateQueries({ queryKey: ["providers"] });
      setEditingSecret(null);
      toast.success(t("suppliers.secretUpdated"));
    } catch (saveError) {
      // 「密钥存在环境变量里，库里没有明文可换」「这家供应商没有任何接入」等中文原因原样透出
      toast.error(
        extractErrorMessage(saveError) || t("suppliers.secretFailed"),
      );
    } finally {
      setSavingSecretSupplierId(null);
    }
  };

  // 一键采用：地址和密钥都从网关读，用户在 agent 侧一个 key 都不用填
  const adoptInto = async (supplier: Supplier, app: string) => {
    if (adoptingSupplierId) return;
    // 按钮已经按协议 / 启用态禁用，这里再挡一道，避免任何入口绕过状态
    if (!canAdopt(supplier) || !isEnabled(supplier)) return;

    setAdoptingSupplierId(supplier.id);
    try {
      const adopted = await suppliersApi.adopt(app, supplier.id);
      // agent 侧多了一条配置、网关侧这家供应商多了使用者：两张表都要重新读
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["providers"] }),
        queryClient.invalidateQueries({ queryKey: ["suppliers"] }),
        queryClient.invalidateQueries({
          queryKey: suppliersKeys.overview,
        }),
      ]);
      toast.success(
        t("suppliers.adoptSuccess", {
          app: getAppLabel(app),
          name: adopted.providerName,
        }),
      );
    } catch (adoptError) {
      // 后端会说清楚为什么拒（协议不匹配、密钥是环境变量引用 / 托管登录…），别吞掉
      toast.error(
        extractErrorMessage(adoptError) || t("suppliers.adoptFailed"),
      );
    } finally {
      setAdoptingSupplierId(null);
    }
  };

  const openModelMap = (supplier: Supplier, binding: SupplierBindingDetail) => {
    // 预填当前映射：后端可能只返回配过的槽位，缺的槽位补空串
    setEditingModelMap({
      supplierId: supplier.id,
      appType: binding.appType,
      providerId: binding.providerId,
      draft: toModelMapDraft(binding.modelMap),
    });
  };

  const closeModelMap = () => {
    // 保存中不给关：请求已经发出去了，关了也拦不住
    if (savingModelMap) return;
    setEditingModelMap(null);
  };

  const changeModelMapSlot = (slot: ModelMapSlot, value: string) => {
    setEditingModelMap((previous) =>
      previous
        ? { ...previous, draft: { ...previous.draft, [slot]: value } }
        : previous,
    );
  };

  const saveModelMap = async () => {
    if (!editingModelMap || savingModelMap) return;

    setSavingModelMap(true);
    try {
      const updated = await suppliersApi.updateBindingModelMap(
        editingModelMap.appType,
        editingModelMap.providerId,
        toModelMapPayload(editingModelMap.draft),
      );
      // 后端返回的就是更新后的总览，直接落缓存，不再多拉一次
      queryClient.setQueryData<Supplier[]>(suppliersKeys.overview, updated);
      // 模型映射写在各 app 自己的配置里，供应商列表下次进入时要重读
      void queryClient.invalidateQueries({ queryKey: ["providers"] });
      setEditingModelMap(null);
      toast.success(t("bindings.modelMapSaved"));
    } catch (saveError) {
      // 非 claude 系、配置已被删掉等，后端会给出中文原因，原样透出
      toast.error(
        extractErrorMessage(saveError) || t("bindings.modelMapFailed"),
      );
    } finally {
      setSavingModelMap(false);
    }
  };

  // 启用 / 停用：只拨开关，地址和密钥都不动；失败时开关自然停回原样
  const toggleEnabled = async (supplier: Supplier) => {
    if (togglingSupplierId) return;
    const nextEnabled = !isEnabled(supplier);

    setTogglingSupplierId(supplier.id);
    try {
      const updated = await suppliersApi.setEnabled(supplier.id, nextEnabled);
      // 后端返回的就是更新后的总览，直接落缓存，不再多拉一次
      queryClient.setQueryData<Supplier[]>(suppliersKeys.overview, updated);
      // 各应用里的供应商列表带着启用态，让它们下次进入时重读
      void queryClient.invalidateQueries({ queryKey: ["providers"] });
      toast.success(
        t(nextEnabled ? "suppliers.enabledToast" : "suppliers.disabledToast", {
          name: supplier.name,
        }),
      );
    } catch (toggleError) {
      // 供应商已经不存在之类的，后端给的是中文原因，原样透出
      toast.error(
        extractErrorMessage(toggleError) || t("suppliers.toggleFailed"),
      );
    } finally {
      setTogglingSupplierId(null);
    }
  };

  // 删除供应商会连它在各 app 里的配置一起删，所以只从这里（确认框）走
  const confirmDelete = async () => {
    if (!deleteTarget || deleting) return;
    const supplier = deleteTarget;

    setDeleting(true);
    try {
      const outcome = await suppliersApi.remove(supplier.id);
      // 后端返回的就是删后的总览，直接落缓存；模型目录也少了一家上游，一并重读
      queryClient.setQueryData<Supplier[]>(
        suppliersKeys.overview,
        outcome.suppliers,
      );
      void queryClient.invalidateQueries({ queryKey: ["providers"] });
      void queryClient.invalidateQueries({ queryKey: ["gateway", "catalog"] });
      setDeleteTarget(null);

      if (outcome.deleted > 0) {
        toast.success(
          t("suppliers.deleteSuccess", {
            name: supplier.name,
            count: outcome.deleted,
          }),
        );
      }
      // 后端删不掉的（配置已失效、写盘失败…）逐条报原因，不假装删干净了
      if (outcome.skipped.length > 0) {
        toast.warning(
          t("suppliers.deleteSkipped", {
            count: outcome.skipped.length,
            detail: formatSkippedDetail(outcome.skipped),
          }),
        );
      }
    } catch (deleteError) {
      // 后端说清楚为什么不能删，原样透出；确认框留着，用户可以重试或取消
      toast.error(
        extractErrorMessage(deleteError) || t("suppliers.deleteFailed"),
      );
    } finally {
      setDeleting(false);
    }
  };

  // 删除确认框的正文：先摊开会一起删掉的 app 和配置条数，用户点头前就知道代价
  const deleteAppTypes = deleteTarget ? collectAppTypes(deleteTarget) : [];
  const deleteMessage = deleteTarget
    ? t("suppliers.deleteDescription", {
        name: deleteTarget.name,
        count: deleteAppTypes.length,
        bindings: deleteTarget.bindings.length,
        apps: deleteAppTypes.map((appType) => getAppLabel(appType)).join("、"),
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

    if (suppliers.length === 0) {
      return (
        <div className="rounded-panel border border-dashed border-border px-6 py-8 text-center text-body text-fg-2">
          {t("suppliers.empty")}
        </div>
      );
    }

    // 有地址的才是真正的网关上游；托管登录账号（官方 OAuth / 导入的账号）没有地址
    const gateways = suppliers.filter(
      (supplier) => !isManagedSupplier(supplier),
    );
    const managed = suppliers.filter(isManagedSupplier);
    const shown = showManaged ? suppliers : gateways;

    return (
      <div>
        <ul className="m-0 list-none space-y-2 p-0">
          {shown.map((supplier) => {
            const isManaged = isManagedSupplier(supplier);
            const isEditing = editing?.supplierId === supplier.id;
            const isSaving = savingSupplierId === supplier.id;
            const isAdopting = adoptingSupplierId === supplier.id;
            const isEditingSecret = editingSecret?.supplierId === supplier.id;
            const isSavingSecret = savingSecretSupplierId === supplier.id;
            const isEditingMap = editingModelMap?.supplierId === supplier.id;
            const isToggling = togglingSupplierId === supplier.id;
            const enabled = isEnabled(supplier);
            const adoptEnabled = canAdopt(supplier) && enabled;
            const modelBindings = supplier.bindings.filter((binding) =>
              supportsModelMap(binding.appType),
            );

            const appTypes = collectAppTypes(supplier);

            return (
              <li
                key={supplier.id}
                className="rounded-panel border border-border bg-surface"
              >
                <div className="flex min-w-0 items-start gap-3 px-4 py-3">
                  {/* 停用只灰信息区，右侧动作区保持可点（编辑地址 / 换密钥 / 模型映射仍然可用） */}
                  <div
                    className={cn(
                      "min-w-0 flex-1 space-y-1.5",
                      !enabled && "opacity-60",
                    )}
                  >
                    <div className="flex min-w-0 flex-wrap items-center gap-2">
                      <span className="min-w-0 truncate text-strong text-fg-1">
                        {supplier.name}
                      </span>
                      <Badge
                        variant="outline"
                        title={t("suppliers.upstreamProtocol")}
                        className="h-[18px] shrink-0 rounded-full px-1.5 text-badge font-medium text-fg-2"
                      >
                        {protocolLabel(supplier.upstreamProtocol)}
                      </Badge>
                      {/* 停用徽标：和协议徽标同一套 outline 样式，只降一档文字色，不另造颜色 */}
                      {!enabled && (
                        <Badge
                          variant="outline"
                          title={t("suppliers.disabledToggleHint")}
                          className="h-[18px] shrink-0 rounded-full px-1.5 text-badge font-medium text-fg-3"
                        >
                          {t("suppliers.disabledBadge")}
                        </Badge>
                      )}
                      {isEditing ? (
                        <Input
                          // 展开就聚焦，省一次点击；地址可能很长，输入框吃掉整行
                          autoFocus
                          value={editing.value}
                          disabled={isSaving}
                          aria-label={t("suppliers.baseUrlLabel")}
                          placeholder={t("suppliers.baseUrlPlaceholder")}
                          className="h-7 min-w-[16rem] flex-1 font-mono text-caption"
                          onChange={(event) =>
                            setEditing({
                              supplierId: supplier.id,
                              value: event.target.value,
                            })
                          }
                          onKeyDown={(event) => {
                            if (event.key === "Enter") {
                              event.preventDefault();
                              void saveBaseUrl(supplier);
                            } else if (event.key === "Escape") {
                              event.preventDefault();
                              cancelEditing();
                            }
                          }}
                        />
                      ) : supplier.baseUrl ? (
                        <code
                          className="min-w-0 font-mono text-caption text-fg-2 [overflow-wrap:anywhere]"
                          title={supplier.baseUrl}
                        >
                          {supplier.baseUrl}
                        </code>
                      ) : (
                        <span className="text-caption text-fg-3">
                          {t("suppliers.noBaseUrl")}
                        </span>
                      )}
                    </div>

                    <div className="flex flex-wrap items-center gap-1.5 text-caption text-fg-3">
                      <span>{t("suppliers.usedBy")}</span>
                      {appTypes.length === 0 ? (
                        <span>{t("suppliers.unused")}</span>
                      ) : (
                        appTypes.map((appType) => {
                          const bindings = supplier.bindings.filter(
                            (binding) => binding.appType === appType,
                          );
                          const first = bindings[0];
                          // 下游是 app 决定的、上游是供应商决定的：两者不同说明走了代理转换
                          const conversion =
                            first &&
                            first.downstreamProtocol !==
                              supplier.upstreamProtocol
                              ? first
                              : null;
                          return (
                            <span
                              key={appType}
                              className="flex flex-wrap items-center gap-1.5"
                            >
                              <span
                                className="rounded-full bg-subtle px-1.5 py-0.5 text-badge text-fg-2"
                                title={appType}
                              >
                                {getAppLabel(appType)}
                              </span>
                              {conversion && (
                                <span
                                  title={t("suppliers.protocolConversionHint")}
                                  className="rounded-full bg-subtle px-1.5 py-0.5 text-badge text-fg-3"
                                >
                                  {t("suppliers.protocolConversion", {
                                    downstream: protocolLabel(
                                      conversion.downstreamProtocol,
                                    ),
                                    upstream: protocolLabel(
                                      supplier.upstreamProtocol,
                                    ),
                                  })}
                                </span>
                              )}
                              {bindings.length > 1 && (
                                <Badge
                                  variant="outline"
                                  title={t("bindings.duplicateHint", {
                                    count: bindings.length,
                                  })}
                                  className="h-[18px] shrink-0 rounded-full border-warning/40 px-1.5 text-badge font-medium text-warning"
                                >
                                  {t("bindings.duplicateBadge", {
                                    count: bindings.length,
                                  })}
                                </Badge>
                              )}
                            </span>
                          );
                        })
                      )}
                    </div>

                    {isManagedAuthProvider(supplier.managedAuthProvider) ? (
                      // 托管登录：凭据是账号池，列账号而不是只报「1 个凭据」
                      <SupplierAccounts
                        supplier={supplier}
                        onManageAccounts={onOpenAuthCenter}
                      />
                    ) : (
                      <div className="flex flex-wrap items-center gap-1.5 text-caption text-fg-3">
                        <span>
                          {t("suppliers.credentialCount", {
                            count: supplier.credentialCount,
                          })}
                        </span>
                        {supplier.credentialKinds.map((kind) => (
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
                    )}

                    {/* 换密钥就地编辑器：只收新值，界面不显示也没拿到过旧密钥 */}
                    {isEditingSecret && editingSecret && (
                      <div className="mt-2 flex min-w-0 items-center gap-2">
                        <div className="relative min-w-0 flex-1">
                          <Input
                            autoFocus
                            type={editingSecret.revealed ? "text" : "password"}
                            value={editingSecret.value}
                            disabled={isSavingSecret}
                            aria-label={t("suppliers.secretLabel")}
                            placeholder={t("suppliers.secretPlaceholder")}
                            className="h-7 pe-8 font-mono text-caption"
                            onChange={(event) =>
                              setEditingSecret({
                                supplierId: supplier.id,
                                value: event.target.value,
                                revealed: editingSecret.revealed,
                              })
                            }
                            onKeyDown={(event) => {
                              if (event.key === "Enter") {
                                event.preventDefault();
                                void saveSecret(supplier);
                              } else if (event.key === "Escape") {
                                event.preventDefault();
                                cancelEditingSecret();
                              }
                            }}
                          />
                          {editingSecret.value && (
                            <button
                              type="button"
                              disabled={isSavingSecret}
                              aria-label={
                                editingSecret.revealed
                                  ? t("apiKeyInput.hide")
                                  : t("apiKeyInput.show")
                              }
                              className="absolute inset-y-0 end-0 flex items-center pe-2 text-fg-3 transition-colors hover:text-fg-1 disabled:cursor-not-allowed disabled:opacity-50"
                              onClick={() =>
                                setEditingSecret((previous) =>
                                  previous
                                    ? {
                                        ...previous,
                                        revealed: !previous.revealed,
                                      }
                                    : previous,
                                )
                              }
                            >
                              {editingSecret.revealed ? (
                                <EyeOff
                                  aria-hidden="true"
                                  className="h-3.5 w-3.5"
                                />
                              ) : (
                                <Eye
                                  aria-hidden="true"
                                  className="h-3.5 w-3.5"
                                />
                              )}
                            </button>
                          )}
                        </div>
                        <Button
                          type="button"
                          variant="solid"
                          size="compact"
                          disabled={isSavingSecret}
                          aria-busy={isSavingSecret}
                          onClick={() => void saveSecret(supplier)}
                        >
                          {isSavingSecret && (
                            <Loader2
                              aria-hidden="true"
                              className="h-3.5 w-3.5 animate-spin"
                            />
                          )}
                          {t("common.save")}
                        </Button>
                        <Button
                          type="button"
                          variant="neutral"
                          size="compact"
                          disabled={isSavingSecret}
                          onClick={cancelEditingSecret}
                        >
                          {t("common.cancel")}
                        </Button>
                      </div>
                    )}
                  </div>

                  <div className="flex shrink-0 flex-wrap items-center justify-end gap-2">
                    {/* 启用 / 停用：停用后网关不再把请求路由到这家上游，配置一条不动。
                        托管登录账号没有网关可关，不显示。 */}
                    {!isManaged && (
                      <Switch
                        checked={enabled}
                        // 有请求在飞时把所有开关一起禁掉：连点会把开关拨乱
                        disabled={togglingSupplierId !== null}
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
                        onCheckedChange={() => void toggleEnabled(supplier)}
                      />
                    )}
                    {isEditing ? (
                      <>
                        {/* 编辑地址时仍可切到换密钥：一行只开一个编辑器 */}
                        {!isManaged && hasEditableSecret(supplier) && (
                          <Button
                            type="button"
                            variant="neutral"
                            size="compact"
                            disabled={isSaving}
                            onClick={() => startEditingSecret(supplier)}
                          >
                            <KeyRound
                              aria-hidden="true"
                              className="h-3.5 w-3.5"
                            />
                            {t("suppliers.changeSecret")}
                          </Button>
                        )}
                        <Button
                          type="button"
                          variant="solid"
                          size="compact"
                          disabled={isSaving}
                          aria-busy={isSaving}
                          onClick={() => void saveBaseUrl(supplier)}
                        >
                          {isSaving && (
                            <Loader2
                              aria-hidden="true"
                              className="h-3.5 w-3.5 animate-spin"
                            />
                          )}
                          {t("common.save")}
                        </Button>
                        <Button
                          type="button"
                          variant="neutral"
                          size="compact"
                          disabled={isSaving}
                          onClick={cancelEditing}
                        >
                          {t("common.cancel")}
                        </Button>
                      </>
                    ) : (
                      <>
                        {!isManaged && (
                          <Button
                            type="button"
                            variant="neutral"
                            size="compact"
                            onClick={() => startEditing(supplier)}
                          >
                            <Pencil
                              aria-hidden="true"
                              className="h-3.5 w-3.5"
                            />
                            {t("suppliers.editUrl")}
                          </Button>
                        )}
                        {/* 「换密钥」：就地输入新密钥，一把换掉这家供应商下所有 app 的 */}
                        {!isManaged && !isEditingSecret && (
                          <Button
                            type="button"
                            variant="neutral"
                            size="compact"
                            onClick={() => startEditingSecret(supplier)}
                          >
                            <KeyRound
                              aria-hidden="true"
                              className="h-3.5 w-3.5"
                            />
                            {t("suppliers.changeSecret")}
                          </Button>
                        )}
                        {/* 「采用到…」：把这家供应商一键接进某个 agent，key 从网关读。
                            直连模式下没有代理转换，协议不是 anthropic、没有地址或已停用时直接禁用。 */}
                        {!isEditingSecret && (
                          <DropdownMenu>
                            <span
                              title={
                                !enabled
                                  ? t("suppliers.disabledAdoptHint")
                                  : adoptEnabled
                                    ? undefined
                                    : t("suppliers.adoptDisabledHint")
                              }
                              className="inline-flex"
                            >
                              <DropdownMenuTrigger asChild>
                                <Button
                                  type="button"
                                  variant="neutral"
                                  size="compact"
                                  // 采用进行中把所有「采用到…」都禁掉：并发采用会被
                                  // 上面的守卫直接丢掉，按钮可点却没反应更让人困惑
                                  disabled={
                                    !adoptEnabled || adoptingSupplierId !== null
                                  }
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
                              {ADOPT_TARGET_APPS.map((app) => (
                                <DropdownMenuItem
                                  key={app}
                                  onSelect={() => void adoptInto(supplier, app)}
                                >
                                  {getAppLabel(app)}
                                </DropdownMenuItem>
                              ))}
                            </DropdownMenuContent>
                          </DropdownMenu>
                        )}
                        {/* 模型映射只对 claude 系接入有意义，按接入逐条选 */}
                        {modelBindings.length > 0 && !isEditingSecret && (
                          <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                              <Button
                                type="button"
                                variant="neutral"
                                size="compact"
                              >
                                {t("suppliers.modelMapAction")}
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent
                              align="end"
                              className="min-w-[180px]"
                            >
                              {modelBindings.map((binding) => (
                                <DropdownMenuItem
                                  key={`${binding.appType}:${binding.providerId}`}
                                  onSelect={() =>
                                    openModelMap(supplier, binding)
                                  }
                                >
                                  <span className="flex min-w-0 items-baseline gap-2">
                                    <span className="min-w-0 truncate">
                                      {getAppLabel(binding.appType)}
                                    </span>
                                    <span className="min-w-0 truncate font-mono text-caption text-fg-3">
                                      {binding.providerId}
                                    </span>
                                  </span>
                                </DropdownMenuItem>
                              ))}
                            </DropdownMenuContent>
                          </DropdownMenu>
                        )}
                      </>
                    )}
                    {/* 删除只放在这个菜单里：动作区唯一一处破坏性入口，避免误触 */}
                    {!isManaged && (
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
                            onSelect={() => setDeleteTarget(supplier)}
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

                <SupplierModelsPanel upstreamIds={supplier.upstreamIds ?? []} />

                {isEditingMap && editingModelMap && (
                  <div className="border-t border-border px-4 py-3">
                    <ModelMapEditor
                      appLabel={getAppLabel(editingModelMap.appType)}
                      draft={editingModelMap.draft}
                      saving={savingModelMap}
                      onChange={changeModelMapSlot}
                      onSave={() => void saveModelMap()}
                      onCancel={closeModelMap}
                    />
                  </div>
                )}
              </li>
            );
          })}
        </ul>
        {managed.length > 0 ? (
          <button
            type="button"
            onClick={() => setShowManaged((value) => !value)}
            className="mt-3 w-full rounded-panel border border-dashed border-border px-4 py-2 text-caption text-fg-3 transition-colors hover:text-fg-2"
          >
            {showManaged
              ? t("suppliers.hideManaged")
              : t("suppliers.showManaged", { count: managed.length })}
          </button>
        ) : null}
      </div>
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
            {/* 主按钮放最后（页头的约定）：一次建好供应商并接进勾选的 agent */}
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

      {/* 新建成功后对话框自己把返回的总览写进缓存，这里只负责开关 */}
      <CreateSupplierDialog
        open={createOpen}
        onClose={() => setCreateOpen(false)}
      />

      {/* 删除确认：把会一起删掉的 app 和配置条数先摊开，删除中不让关也不让再点 */}
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
