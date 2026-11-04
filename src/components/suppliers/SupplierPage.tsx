import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Boxes,
  ChevronRight,
  Download,
  Eye,
  EyeOff,
  KeyRound,
  Loader2,
  Pencil,
  RefreshCw,
} from "lucide-react";
import {
  suppliersApi,
  type Supplier,
  type SupplierEndpoint,
} from "@/lib/api/suppliers";
import { suppliersKeys } from "@/lib/query/suppliers";
import { getAppLabel } from "@/config/appConfig";
import { toast } from "@/lib/toast";
import { cn } from "@/lib/utils";
import { extractErrorMessage } from "@/utils/errorUtils";
import { AppPageHeader } from "@/components/shell/AppPageHeader";
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
import { protocolLabel } from "./protocolLabel";

/**
 * 「采用到…」的候选 app：后端 adopt_supplier 目前只支持 claude / claude-desktop，
 * 其它 app 会 reject 并给出中文原因（它要求配置里按供应商维护模型列表 / 认证）。
 */
const ADOPT_TARGET_APPS = ["claude", "claude-desktop"] as const;

interface EditingBaseUrl {
  endpointId: string;
  value: string;
}

/**
 * 换密钥的就地编辑器状态。
 *
 * 永远从空串开始：旧密钥后端不返回、界面也不保存，这里只接收「新密钥」。
 */
interface EditingSecret {
  endpointId: string;
  value: string;
  /** 是否切成明文显示（默认遮住） */
  revealed: boolean;
}

/**
 * 供应商页：网关级视角的供应商 / 端点总览。
 *
 * 每家供应商一行，点开看它的端点；端点上的地址改一次，所有接入它的 agent 一起被改写。
 */
export function SupplierPage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { data, isPending, isError, error, refetch, isFetching } = useQuery({
    queryKey: suppliersKeys.overview,
    queryFn: () => suppliersApi.overview(),
  });

  // 展开是纯界面状态：换一次数据（改完地址）也保持用户当前展开的那几家
  // 托管登录 / 无地址的账号不是网关，默认收起来，免得把真正的供应商淹掉
  const [showManaged, setShowManaged] = useState(false);
  const [expandedIds, setExpandedIds] = useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const [editing, setEditing] = useState<EditingBaseUrl | null>(null);
  const [editingSecret, setEditingSecret] = useState<EditingSecret | null>(
    null,
  );
  const [savingEndpointId, setSavingEndpointId] = useState<string | null>(null);
  // 换密钥单独记一个「正在保存的端点」：地址和密钥的请求不能互相顶掉禁用态
  const [savingSecretEndpointId, setSavingSecretEndpointId] = useState<
    string | null
  >(null);
  // 正在「采用到…」的端点：按钮期间禁用，防止连点给同一个 agent 重复建配置
  const [adoptingEndpointId, setAdoptingEndpointId] = useState<string | null>(
    null,
  );

  const suppliers = data ?? [];

  const toggleSupplier = (supplierId: string) => {
    setExpandedIds((previous) => {
      const next = new Set(previous);
      if (next.has(supplierId)) {
        next.delete(supplierId);
      } else {
        next.add(supplierId);
      }
      return next;
    });
  };

  const startEditing = (endpoint: SupplierEndpoint) => {
    // 一行只开一个编辑器：开地址编辑就把密钥编辑收起来
    setEditingSecret(null);
    setEditing({ endpointId: endpoint.id, value: endpoint.baseUrl });
  };

  const cancelEditing = () => setEditing(null);

  const startEditingSecret = (endpoint: SupplierEndpoint) => {
    // 不预填：有没有旧密钥、旧密钥是什么，界面都不知道（后端也不返回）
    setEditing(null);
    setEditingSecret({
      endpointId: endpoint.id,
      value: "",
      revealed: false,
    });
  };

  const cancelEditingSecret = () => setEditingSecret(null);

  const saveBaseUrl = async (endpoint: SupplierEndpoint) => {
    if (!editing || editing.endpointId !== endpoint.id) return;
    if (savingEndpointId) return;

    const nextUrl = editing.value.trim();
    if (!nextUrl) {
      toast.error(t("suppliers.baseUrlRequired"));
      return;
    }
    if (nextUrl === endpoint.baseUrl.trim()) {
      setEditing(null);
      return;
    }

    setSavingEndpointId(endpoint.id);
    try {
      const updated = await suppliersApi.updateBaseUrl(endpoint.id, nextUrl);
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
      setSavingEndpointId(null);
    }
  };

  // 换密钥：一把密钥换掉这个端点下所有 app 的；后端不接受空串，这里先拦一道
  const saveSecret = async (endpoint: SupplierEndpoint) => {
    if (!editingSecret || editingSecret.endpointId !== endpoint.id) return;
    if (savingSecretEndpointId) return;

    const nextSecret = editingSecret.value.trim();
    if (!nextSecret) {
      toast.error(t("suppliers.secretRequired"));
      return;
    }

    setSavingSecretEndpointId(endpoint.id);
    try {
      const updated = await suppliersApi.updateSecret(endpoint.id, nextSecret);
      // 后端返回的就是更新后的总览，直接落缓存
      queryClient.setQueryData<Supplier[]>(suppliersKeys.overview, updated);
      // 各应用里存的密钥已经被一起改写（界面只显示 credentialCount），让列表重读
      void queryClient.invalidateQueries({ queryKey: ["providers"] });
      setEditingSecret(null);
      toast.success(t("suppliers.secretUpdated"));
    } catch (saveError) {
      // 「密钥存在环境变量里，库里没有明文可换」「该端点没有任何接入」等中文原因原样透出
      toast.error(
        extractErrorMessage(saveError) || t("suppliers.secretFailed"),
      );
    } finally {
      setSavingSecretEndpointId(null);
    }
  };

  // 一键采用：地址和密钥都从网关读，用户在 agent 侧一个 key 都不用填
  const adoptInto = async (endpoint: SupplierEndpoint, app: string) => {
    if (adoptingEndpointId) return;

    setAdoptingEndpointId(endpoint.id);
    try {
      const adopted = await suppliersApi.adopt(app, endpoint.id);
      // agent 侧多了一条配置、网关侧这个端点多了使用者：两张表都要重新读
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
      setAdoptingEndpointId(null);
    }
  };

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

    // 有 host 的才是真正的网关上游；托管登录账号（官方 OAuth / 导入的账号）没有地址
    const gateways = suppliers.filter(
      (supplier) => supplier.host.trim() !== "",
    );
    const managed = suppliers.filter((supplier) => supplier.host.trim() === "");
    const shown = showManaged ? suppliers : gateways;

    return (
      <div>
        <ul className="m-0 list-none space-y-2 p-0">
          {shown.map((supplier) => {
            const open = expandedIds.has(supplier.id);
            const panelId = `supplier-endpoints-${supplier.id}`;
            const credentialTotal = supplier.endpoints.reduce(
              (total, endpoint) => total + endpoint.credentialCount,
              0,
            );

            return (
              <li
                key={supplier.id}
                className="overflow-hidden rounded-panel border border-border bg-surface"
              >
                <button
                  type="button"
                  aria-expanded={open}
                  aria-controls={open ? panelId : undefined}
                  onClick={() => toggleSupplier(supplier.id)}
                  className="flex w-full items-center gap-3 px-4 py-3 text-start transition-colors hover:bg-subtle"
                >
                  <ChevronRight
                    aria-hidden="true"
                    className={cn(
                      "h-4 w-4 shrink-0 text-fg-3 transition-transform",
                      open && "rotate-90",
                    )}
                  />
                  <span className="flex min-w-0 flex-1 items-baseline gap-2">
                    <span className="min-w-0 truncate text-strong text-fg-1">
                      {supplier.name}
                    </span>
                    <span
                      className={cn(
                        "min-w-0 truncate font-mono text-caption",
                        supplier.host ? "text-fg-3" : "text-fg-3 italic",
                      )}
                      title={supplier.host || undefined}
                    >
                      {supplier.host || t("suppliers.noHost")}
                    </span>
                  </span>
                  <span className="shrink-0 text-caption tabular-nums text-fg-2">
                    {t("suppliers.endpointCount", {
                      count: supplier.endpoints.length,
                    })}
                    {" · "}
                    {t("suppliers.appCount", { count: supplier.appCount })}
                    {" · "}
                    {t("suppliers.credentialCount", { count: credentialTotal })}
                  </span>
                </button>

                {open && (
                  <div id={panelId}>
                    {supplier.credentialKinds.length > 0 && (
                      <div className="flex flex-wrap items-center gap-1.5 border-t border-border px-4 py-2 text-caption text-fg-3">
                        <span>{t("suppliers.credentialKinds")}</span>
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

                    {supplier.endpoints.length === 0 ? (
                      <div className="border-t border-border px-4 py-3 text-caption text-fg-3">
                        {t("suppliers.noEndpoints")}
                      </div>
                    ) : (
                      <ul className="m-0 list-none p-0">
                        {supplier.endpoints.map((endpoint) => {
                          const isEditing = editing?.endpointId === endpoint.id;
                          const isSaving = savingEndpointId === endpoint.id;
                          const isAdopting = adoptingEndpointId === endpoint.id;
                          const isEditingSecret =
                            editingSecret?.endpointId === endpoint.id;
                          const isSavingSecret =
                            savingSecretEndpointId === endpoint.id;
                          return (
                            <li
                              key={endpoint.id}
                              className="border-t border-border px-4 py-3"
                            >
                              <div className="flex min-w-0 items-start gap-3">
                                <div className="min-w-0 flex-1 space-y-1.5">
                                  <div className="flex min-w-0 flex-wrap items-center gap-2">
                                    <Badge
                                      variant="outline"
                                      className="h-[18px] shrink-0 rounded-full px-1.5 text-badge font-medium text-fg-2"
                                    >
                                      {protocolLabel(endpoint.protocol)}
                                    </Badge>
                                    {isEditing ? (
                                      <Input
                                        // 展开就聚焦，省一次点击；地址可能很长，输入框吃掉整行
                                        autoFocus
                                        value={editing.value}
                                        disabled={isSaving}
                                        aria-label={t("suppliers.baseUrlLabel")}
                                        placeholder={t(
                                          "suppliers.baseUrlPlaceholder",
                                        )}
                                        className="h-7 min-w-[16rem] flex-1 font-mono text-caption"
                                        onChange={(event) =>
                                          setEditing({
                                            endpointId: endpoint.id,
                                            value: event.target.value,
                                          })
                                        }
                                        onKeyDown={(event) => {
                                          if (event.key === "Enter") {
                                            event.preventDefault();
                                            void saveBaseUrl(endpoint);
                                          } else if (event.key === "Escape") {
                                            event.preventDefault();
                                            cancelEditing();
                                          }
                                        }}
                                      />
                                    ) : endpoint.baseUrl ? (
                                      <code
                                        className="min-w-0 font-mono text-caption text-fg-2 [overflow-wrap:anywhere]"
                                        title={endpoint.baseUrl}
                                      >
                                        {endpoint.baseUrl}
                                      </code>
                                    ) : (
                                      <span className="text-caption text-fg-3">
                                        {t("suppliers.noBaseUrl")}
                                      </span>
                                    )}
                                  </div>

                                  <div className="flex flex-wrap items-center gap-1.5 text-caption text-fg-3">
                                    <span>{t("suppliers.usedBy")}</span>
                                    {endpoint.apps.length === 0 ? (
                                      <span>{t("suppliers.unused")}</span>
                                    ) : (
                                      endpoint.apps.map((app) => (
                                        <span
                                          key={app}
                                          className="rounded-full bg-subtle px-1.5 py-0.5 text-badge text-fg-2"
                                          title={app}
                                        >
                                          {getAppLabel(app)}
                                        </span>
                                      ))
                                    )}
                                    {endpoint.credentialCount > 0 && (
                                      <span>
                                        {" · "}
                                        {t("suppliers.credentialCount", {
                                          count: endpoint.credentialCount,
                                        })}
                                      </span>
                                    )}
                                  </div>
                                </div>

                                <div className="flex shrink-0 items-center gap-2">
                                  {isEditing ? (
                                    <>
                                      <Button
                                        type="button"
                                        variant="solid"
                                        size="compact"
                                        disabled={isSaving}
                                        aria-busy={isSaving}
                                        onClick={() =>
                                          void saveBaseUrl(endpoint)
                                        }
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
                                      <Button
                                        type="button"
                                        variant="neutral"
                                        size="compact"
                                        onClick={() => startEditing(endpoint)}
                                      >
                                        <Pencil
                                          aria-hidden="true"
                                          className="h-3.5 w-3.5"
                                        />
                                        {t("suppliers.editUrl")}
                                      </Button>
                                      {/* 「换密钥」：就地输入新密钥，一把换掉这个端点下所有 app 的 */}
                                      {!isEditingSecret && (
                                        <Button
                                          type="button"
                                          variant="neutral"
                                          size="compact"
                                          onClick={() =>
                                            startEditingSecret(endpoint)
                                          }
                                        >
                                          <KeyRound
                                            aria-hidden="true"
                                            className="h-3.5 w-3.5"
                                          />
                                          {t("suppliers.changeSecret")}
                                        </Button>
                                      )}
                                      {/* 「采用到…」：把这家端点一键接进某个 agent，key 从网关读 */}
                                      <DropdownMenu>
                                        <DropdownMenuTrigger asChild>
                                          <Button
                                            type="button"
                                            variant="neutral"
                                            size="compact"
                                            // 采用进行中把所有「采用到…」都禁掉：并发采用会被
                                            // 上面的守卫直接丢掉，按钮可点却没反应更让人困惑
                                            disabled={
                                              adoptingEndpointId !== null
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
                                        <DropdownMenuContent
                                          align="end"
                                          className="min-w-[160px]"
                                        >
                                          {ADOPT_TARGET_APPS.map((app) => (
                                            <DropdownMenuItem
                                              key={app}
                                              onSelect={() =>
                                                void adoptInto(endpoint, app)
                                              }
                                            >
                                              {getAppLabel(app)}
                                            </DropdownMenuItem>
                                          ))}
                                        </DropdownMenuContent>
                                      </DropdownMenu>
                                    </>
                                  )}
                                </div>
                              </div>

                              {/* 换密钥就地编辑器：只收新值，界面不显示也没拿到过旧密钥 */}
                              {isEditingSecret && editingSecret && (
                                <div className="mt-2 flex min-w-0 items-center gap-2">
                                  <div className="relative min-w-0 flex-1">
                                    <Input
                                      autoFocus
                                      type={
                                        editingSecret.revealed
                                          ? "text"
                                          : "password"
                                      }
                                      value={editingSecret.value}
                                      disabled={isSavingSecret}
                                      aria-label={t("suppliers.secretLabel")}
                                      placeholder={t(
                                        "suppliers.secretPlaceholder",
                                      )}
                                      className="h-7 pe-8 font-mono text-caption"
                                      onChange={(event) =>
                                        setEditingSecret({
                                          endpointId: endpoint.id,
                                          value: event.target.value,
                                          revealed: editingSecret.revealed,
                                        })
                                      }
                                      onKeyDown={(event) => {
                                        if (event.key === "Enter") {
                                          event.preventDefault();
                                          void saveSecret(endpoint);
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
                                    onClick={() => void saveSecret(endpoint)}
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
                            </li>
                          );
                        })}
                      </ul>
                    )}
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
    </>
  );
}

export default SupplierPage;
