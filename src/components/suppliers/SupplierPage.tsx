import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Boxes, ChevronRight, Loader2, Pencil, RefreshCw } from "lucide-react";
import {
  suppliersApi,
  type Supplier,
  type SupplierEndpoint,
} from "@/lib/api/suppliers";
import { getAppLabel } from "@/config/appConfig";
import { toast } from "@/lib/toast";
import { cn } from "@/lib/utils";
import { extractErrorMessage } from "@/utils/errorUtils";
import { AppPageHeader } from "@/components/shell/AppPageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { HelpTip } from "@/components/ui/help-tip";
import { Input } from "@/components/ui/input";
import { Notice } from "@/components/ui/notice";

/** 供应商总览的查询键；改完地址直接用后端返回的新总览覆盖它。 */
const SUPPLIERS_QUERY_KEY = ["suppliersOverview"] as const;

/** 协议标识是技术名词，不做翻译；后端也可能拼出 `opencode:<npm>` 这类值，原样显示。 */
const PROTOCOL_LABELS: Record<string, string> = {
  anthropic: "Anthropic",
  openai_chat: "OpenAI Chat",
  openai_responses: "OpenAI Responses",
  gemini_native: "Gemini",
  kimi_managed: "Kimi",
};

function protocolLabel(protocol: string): string {
  return PROTOCOL_LABELS[protocol] ?? protocol;
}

interface EditingBaseUrl {
  endpointId: string;
  value: string;
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
    queryKey: SUPPLIERS_QUERY_KEY,
    queryFn: () => suppliersApi.overview(),
  });

  // 展开是纯界面状态：换一次数据（改完地址）也保持用户当前展开的那几家
  const [expandedIds, setExpandedIds] = useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const [editing, setEditing] = useState<EditingBaseUrl | null>(null);
  const [savingEndpointId, setSavingEndpointId] = useState<string | null>(null);

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
    setEditing({ endpointId: endpoint.id, value: endpoint.baseUrl });
  };

  const cancelEditing = () => setEditing(null);

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
      queryClient.setQueryData<Supplier[]>(SUPPLIERS_QUERY_KEY, updated);
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

    return (
      <ul className="m-0 list-none space-y-2 p-0">
        {suppliers.map((supplier) => {
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
                                      onClick={() => void saveBaseUrl(endpoint)}
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
                                )}
                              </div>
                            </div>
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
