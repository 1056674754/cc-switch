import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Loader2, Pencil, Plug, RefreshCw } from "lucide-react";
import type { AppId } from "@/lib/api";
import {
  suppliersApi,
  type Supplier,
  type SupplierEndpoint,
} from "@/lib/api/suppliers";
import { suppliersKeys } from "@/lib/query/suppliers";
import { APP_IDS, getAppLabel } from "@/config/appConfig";
import { toast } from "@/lib/toast";
import { cn } from "@/lib/utils";
import { extractErrorMessage } from "@/utils/errorUtils";
import { AppGlyph } from "@/components/shell/AppGlyph";
import { AppPageHeader } from "@/components/shell/AppPageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { HelpTip } from "@/components/ui/help-tip";
import { Notice } from "@/components/ui/notice";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { protocolLabel } from "@/components/suppliers/protocolLabel";
import { ModelMapEditor } from "./ModelMapEditor";
import {
  findBinding,
  type ModelMapDraft,
  type ModelMapSlot,
  parseConsumer,
  summarizeModelMap,
  supportsModelMap,
  toModelMapDraft,
  toModelMapPayload,
} from "./modelMap";

/** 一个供应商端点，连同它属于哪家供应商（列表里要显示供应商名）。 */
interface EndpointRef {
  supplier: Supplier;
  endpoint: SupplierEndpoint;
}

/** 在用的端点：还带上这个 app 挂在这个端点上的 providerId（模型映射按它写）。 */
interface InUseRef extends EndpointRef {
  providerId: string;
}

/** 一个 app 的接入情况：已经在用哪些端点、还能选用哪些。 */
interface AppBinding {
  app: AppId;
  inUse: InUseRef[];
  candidates: EndpointRef[];
}

/** 正在编辑的模型映射：定位到某 app 的某条接入，草稿是六个槽位。 */
interface ModelMapEditing {
  endpointId: string;
  appType: AppId;
  providerId: string;
  draft: ModelMapDraft;
}

/**
 * 从供应商总览反查每个 app 的接入情况。
 *
 * - 有哪些 app：从各端点的 `apps` 汇总，顺序沿用 APP_IDS（不硬编码全量应用列表）；
 * - 在用什么：`consumers` 里 `"<app>:<providerId>"` 的项，一条接入一个条目
 *   （同一个端点被同一个 app 的两条配置挂上时，各自有独立的模型映射）；
 * - 能选什么：anthropic 协议、且这个 app 还没在用的端点。
 */
export function buildBindings(suppliers: Supplier[]): AppBinding[] {
  const apps = APP_IDS.filter((app) =>
    suppliers.some((supplier) =>
      supplier.endpoints.some((endpoint) => endpoint.apps.includes(app)),
    ),
  );

  return apps.map((app) => {
    const inUse: InUseRef[] = [];
    const candidates: EndpointRef[] = [];

    for (const supplier of suppliers) {
      for (const endpoint of supplier.endpoints) {
        const providerIds = endpoint.consumers
          .map(parseConsumer)
          .filter(
            (parsed): parsed is { appType: string; providerId: string } =>
              parsed !== null,
          )
          .filter((parsed) => parsed.appType === app)
          .map((parsed) => parsed.providerId);

        if (providerIds.length > 0) {
          for (const providerId of providerIds) {
            inUse.push({ supplier, endpoint, providerId });
          }
        } else if (endpoint.protocol === "anthropic") {
          candidates.push({ supplier, endpoint });
        }
      }
    }

    return { app, inUse, candidates };
  });
}

/**
 * 接入端页：倒过来的入口。
 *
 * 供应商（上游）在网关那边配一次地址 + 密钥；这里按 agent 看「这个应用现在用的是谁」，
 * 以及「还能选用谁」。选用时只挑端点，地址和密钥都从供应商读，用户不用填 key。
 */
/** 支持一键采用的 app：它们的配置能由网关忠实渲染出来（其余得手配）。 */
const ADOPT_APPS: AppId[] = ["claude", "claude-desktop"];

export function BindingsPage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { data, isPending, isError, error, refetch, isFetching } = useQuery({
    queryKey: suppliersKeys.overview,
    queryFn: () => suppliersApi.overview(),
  });

  // 正在采用到哪个 app：下拉在请求期间禁用，防止连点给同一个 agent 重复建配置
  const [adoptingApp, setAdoptingApp] = useState<string | null>(null);
  // 正在编辑模型映射的接入；同时只开一个编辑器，换一条就是换草稿
  const [editingMap, setEditingMap] = useState<ModelMapEditing | null>(null);
  const [savingMap, setSavingMap] = useState(false);
  // 只有能忠实渲染配置的 app 才给「选用」下拉；其余点了也只会被后端拒绝
  const canAdopt = (app: AppId) => ADOPT_APPS.includes(app);

  const bindings = useMemo(() => buildBindings(data ?? []), [data]);

  const openModelMap = (
    app: AppId,
    endpoint: SupplierEndpoint,
    providerId: string,
  ) => {
    // 预填当前映射：后端可能只返回配过的槽位，缺的槽位补空串
    const current = findBinding(endpoint, app, providerId)?.modelMap;
    setEditingMap({
      endpointId: endpoint.id,
      appType: app,
      providerId,
      draft: toModelMapDraft(current),
    });
  };

  const closeModelMap = () => {
    // 保存中不给关：请求已经发出去了，关了也拦不住
    if (savingMap) return;
    setEditingMap(null);
  };

  const changeModelMapSlot = (slot: ModelMapSlot, value: string) => {
    setEditingMap((previous) =>
      previous
        ? { ...previous, draft: { ...previous.draft, [slot]: value } }
        : previous,
    );
  };

  const saveModelMap = async () => {
    if (!editingMap || savingMap) return;

    setSavingMap(true);
    try {
      const updated = await suppliersApi.updateBindingModelMap(
        editingMap.appType,
        editingMap.providerId,
        toModelMapPayload(editingMap.draft),
      );
      // 后端返回的就是更新后的总览，直接落缓存，不再多拉一次
      queryClient.setQueryData<Supplier[]>(suppliersKeys.overview, updated);
      // 模型映射写在各 app 自己的配置里，供应商列表下次进入时要重读
      void queryClient.invalidateQueries({ queryKey: ["providers"] });
      setEditingMap(null);
      toast.success(t("bindings.modelMapSaved"));
    } catch (saveError) {
      // 非 claude 系、配置已被删掉等，后端会给出中文原因，原样透出
      toast.error(
        extractErrorMessage(saveError) || t("bindings.modelMapFailed"),
      );
    } finally {
      setSavingMap(false);
    }
  };

  const adoptFor = async (app: AppId, endpointId: string) => {
    if (adoptingApp) return;

    setAdoptingApp(app);
    try {
      const adopted = await suppliersApi.adopt(app, endpointId);
      // agent 侧多了一条配置、网关侧这个端点多了使用者：两边都要重新读
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
      // 后端会说清楚为什么拒（协议不匹配、密钥是环境变量引用 / 托管登录…），别吞掉
      toast.error(
        extractErrorMessage(adoptError) || t("suppliers.adoptFailed"),
      );
    } finally {
      setAdoptingApp(null);
    }
  };

  const renderBody = () => {
    if (isPending) {
      return (
        <div
          role="status"
          className="grid gap-3 lg:grid-cols-2"
          aria-busy="true"
          aria-label={t("bindings.loading")}
        >
          {[0, 1].map((index) => (
            <div
              key={index}
              className="h-[148px] w-full rounded-panel border border-dashed border-border bg-subtle"
            />
          ))}
        </div>
      );
    }

    if (isError) {
      return (
        <Notice
          tone="danger"
          title={t("bindings.loadFailed")}
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
          {extractErrorMessage(error) || t("bindings.loadFailedHint")}
        </Notice>
      );
    }

    if (bindings.length === 0) {
      return (
        <div className="rounded-panel border border-dashed border-border px-6 py-8 text-center text-body text-fg-2">
          {t("bindings.empty")}
        </div>
      );
    }

    return (
      <ul className="m-0 grid list-none gap-3 p-0 lg:grid-cols-2">
        {bindings.map(({ app, inUse, candidates }) => {
          const isBusy = adoptingApp === app;
          const label = getAppLabel(app);

          return (
            <li
              key={app}
              className="rounded-panel border border-border bg-surface p-4"
            >
              <div className="flex items-center gap-2">
                <AppGlyph app={app} size={16} badgeClassName="bg-surface" />
                <h2 className="m-0 min-w-0 flex-1 truncate text-strong text-fg-1">
                  {label}
                </h2>
              </div>

              <div className="mt-3 space-y-1.5">
                <div className="text-caption text-fg-3">
                  {t("bindings.currentSuppliers")}
                </div>
                {inUse.length === 0 ? (
                  <div className="rounded-control border border-dashed border-border px-3 py-2 text-caption text-fg-3">
                    {t("bindings.appEmpty")}
                  </div>
                ) : (
                  <ul className="m-0 list-none space-y-1.5 p-0">
                    {inUse.map(({ supplier, endpoint, providerId }) => {
                      const summary = summarizeModelMap(
                        findBinding(endpoint, app, providerId)?.modelMap,
                      );
                      // 同一个端点可能被同一个 app 的多条配置挂上，providerId 要进 key
                      const rowKey = `${supplier.id}:${endpoint.id}:${providerId}`;
                      const isEditingMap =
                        editingMap?.endpointId === endpoint.id &&
                        editingMap.appType === app &&
                        editingMap.providerId === providerId;

                      return (
                        <li
                          key={rowKey}
                          className="rounded-control bg-subtle px-3 py-2"
                        >
                          <div className="flex min-w-0 flex-wrap items-center gap-2">
                            <span className="min-w-0 truncate text-body text-fg-1">
                              {supplier.name}
                            </span>
                            <Badge
                              variant="outline"
                              className="h-[18px] shrink-0 rounded-full px-1.5 text-badge font-medium text-fg-2"
                            >
                              {protocolLabel(endpoint.protocol)}
                            </Badge>
                            <code
                              className="min-w-0 font-mono text-caption text-fg-3 [overflow-wrap:anywhere]"
                              title={endpoint.baseUrl || undefined}
                            >
                              {endpoint.baseUrl || t("suppliers.noBaseUrl")}
                            </code>
                          </div>

                          <div className="mt-1 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-caption">
                            <span className="shrink-0 text-fg-3">
                              {t("bindings.modelMap")}
                            </span>
                            <span
                              className={cn(
                                "min-w-0 truncate font-mono",
                                summary ? "text-fg-2" : "text-fg-3 italic",
                              )}
                              title={summary || undefined}
                            >
                              {summary || t("bindings.modelMapEmpty")}
                            </span>
                            {supportsModelMap(app) && !isEditingMap && (
                              <Button
                                type="button"
                                variant="quiet"
                                size="compact"
                                className="h-6 px-1.5 text-caption"
                                onClick={() =>
                                  openModelMap(app, endpoint, providerId)
                                }
                              >
                                <Pencil
                                  aria-hidden="true"
                                  className="h-3 w-3"
                                />
                                {t("bindings.modelMapEdit")}
                              </Button>
                            )}
                          </div>

                          {supportsModelMap(app) &&
                            isEditingMap &&
                            editingMap && (
                              <ModelMapEditor
                                appLabel={label}
                                draft={editingMap.draft}
                                saving={savingMap}
                                onChange={changeModelMapSlot}
                                onSave={() => void saveModelMap()}
                                onCancel={closeModelMap}
                              />
                            )}
                        </li>
                      );
                    })}
                  </ul>
                )}
              </div>

              <div className="mt-3 flex flex-wrap items-center gap-2">
                {!canAdopt(app) ? (
                  <p className="m-0 text-caption text-fg-3">
                    {t("bindings.manualOnly")}
                  </p>
                ) : candidates.length === 0 ? (
                  <p className="m-0 text-caption text-fg-3">
                    {t("bindings.noCandidates")}
                  </p>
                ) : (
                  <>
                    {/* 选中即采用：下拉始终显示占位文案，采用成功后端点会出现在上面 */}
                    <Select
                      value=""
                      // 采用进行中把各卡片的下拉都禁掉：并发采用会被上面的守卫丢掉，
                      // 能点却没反应更让人困惑
                      disabled={adoptingApp !== null}
                      onValueChange={(endpointId) =>
                        void adoptFor(app, endpointId)
                      }
                    >
                      <SelectTrigger
                        aria-label={t("bindings.pickSupplierFor", {
                          app: label,
                        })}
                        className="h-8 min-w-0 flex-1 text-caption"
                      >
                        <SelectValue
                          placeholder={t("bindings.pickPlaceholder")}
                        />
                      </SelectTrigger>
                      <SelectContent>
                        {candidates.map(({ supplier, endpoint }) => (
                          <SelectItem
                            key={`${supplier.id}:${endpoint.id}`}
                            value={endpoint.id}
                          >
                            <span className="flex min-w-0 items-baseline gap-2">
                              <span className="min-w-0 truncate">
                                {supplier.name}
                              </span>
                              <span className="min-w-0 truncate font-mono text-caption text-fg-3">
                                {endpoint.baseUrl || t("suppliers.noBaseUrl")}
                              </span>
                            </span>
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    {isBusy && (
                      <span
                        className="flex shrink-0 items-center gap-1.5 text-caption text-fg-2"
                        aria-live="polite"
                      >
                        <Loader2
                          aria-hidden="true"
                          className="h-3.5 w-3.5 animate-spin"
                        />
                        {t("bindings.adopting")}
                      </span>
                    )}
                  </>
                )}
              </div>
            </li>
          );
        })}
      </ul>
    );
  };

  return (
    <>
      <AppPageHeader
        icon={<Plug className="h-5 w-5" strokeWidth={1.5} />}
        title={t("nav.bindings")}
        titleExtra={
          <HelpTip title={t("bindings.helpTitle")}>
            {t("bindings.helpBody")}
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
          <p className="m-0 text-body text-fg-2">{t("bindings.description")}</p>
          {renderBody()}
        </div>
      </div>
    </>
  );
}

export default BindingsPage;
