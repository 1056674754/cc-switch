import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown } from "lucide-react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useProxyTakeoverStatus } from "@/lib/query/proxy";
import { proxyKeys } from "@/lib/query/proxy";
import { toast } from "@/lib/toast";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Checkbox } from "@/components/ui/checkbox";
import { gatewayApi } from "@/lib/api/gateway";
import type { AppModelConfig, GatewayCatalogEntry } from "@/types/gateway";
import { ModelPicker } from "./ModelPicker";
import { ClientGatewayBar, type GatewayClientBarApp } from "./ClientGatewayBar";
import { applySlotChange, slotDefinitions, validateDraft } from "./appModelConfig";
import { gatewayKeys } from "./AppEditorPage";

/** 表单里非模型字段（作用范围 / 入口模式 / 两个路由开关 / 档案名）原样存进 routing。 */
interface FormMeta {
  profileName: string;
  scope: string;
  entryMode: string;
  enhancedRouting: boolean;
  configLevelRouting: boolean;
}

const DEFAULT_META: FormMeta = {
  profileName: "",
  scope: "system",
  entryMode: "cli-and-app",
  enhancedRouting: true,
  configLevelRouting: false,
};

function metaFromConfig(
  config: AppModelConfig | null | undefined,
): FormMeta {
  const saved = (config?.routing ?? {}) as Partial<FormMeta>;
  return { ...DEFAULT_META, ...saved };
}

/**
 * 应用页主体：CCR 那份「编辑配置」表单的内联版。
 * 不分直连 / 路由 / 聚合，也没有确认框：默认模型 + 槽位 + 允许列表直接编辑，
 * 保存后网关按模型引用路由，已接管的客户端当场重写。
 */
export function AppConfigPage({
  appType,
  appLabel,
  clientApp,
}: {
  appType: string;
  appLabel: string;
  /** 累加式 app：页面顶部显示「压网关 / 释放」开关，接管状态由该 app 自己的命令回答。 */
  clientApp?: GatewayClientBarApp;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const catalogQuery = useQuery({
    queryKey: gatewayKeys.catalog,
    queryFn: () => gatewayApi.getCatalog(),
    staleTime: 30_000,
  });
  const configQuery = useQuery({
    queryKey: gatewayKeys.appConfig(appType),
    queryFn: () => gatewayApi.getAppModelConfig(appType),
    staleTime: 30_000,
  });

  const [draft, setDraft] = useState<AppModelConfig>({});
  const [meta, setMeta] = useState<FormMeta>(DEFAULT_META);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [saving, setSaving] = useState(false);
  const [switchingRoute, setSwitchingRoute] = useState(false);
  const { data: takeoverStatus } = useProxyTakeoverStatus(false);
  const adopted =
    !clientApp &&
    Boolean(
      (takeoverStatus as Record<string, boolean> | undefined)?.[appType],
    );

  useEffect(() => {
    setDraft(configQuery.data ?? {});
    setMeta({
      ...metaFromConfig(configQuery.data),
      profileName: metaFromConfig(configQuery.data).profileName || appLabel,
    });
  }, [configQuery.data, appLabel]);

  // 只列这个 app 真的能用的模型：目录条目的 sources 形如 `claude:CTYun`。
  const entries = useMemo(
    () =>
      (catalogQuery.data ?? []).filter((entry) =>
        entry.sources.some(
          (source) => source === appType || source.startsWith(`${appType}:`),
        ),
      ),
    [catalogQuery.data, appType],
  );
  const slots = slotDefinitions(appType);
  const allowed = draft.allowed ?? [];

  const errors = useMemo(() => {
    if (!draft.default) {
      return [t("gateway.form.defaultRequired", { defaultValue: "默认模型必填" })];
    }
    return validateDraft(draft, entries);
  }, [draft, entries, t]);

  const filtered = useMemo(() => {
    const keyword = search.trim().toLowerCase();
    if (!keyword) return entries;
    return entries.filter((entry: GatewayCatalogEntry) =>
      `${entry.reference} ${entry.displayName}`.toLowerCase().includes(keyword),
    );
  }, [entries, search]);

  const toggleAllowed = (reference: string, checked: boolean) => {
    const next = checked
      ? [...allowed.filter((item) => item !== reference), reference]
      : allowed.filter((item) => item !== reference);
    setDraft({ ...draft, allowed: next });
  };

  const toggleRoute = async () => {
    if (clientApp) return;
    setSwitchingRoute(true);
    try {
      if (adopted) {
        await gatewayApi.release(appType);
      } else {
        await gatewayApi.adopt(appType);
      }
      await queryClient.invalidateQueries({ queryKey: proxyKeys.takeoverStatus });
      await queryClient.invalidateQueries({ queryKey: proxyKeys.status });
      toast.success(
        adopted
          ? t("gateway.form.released", { defaultValue: "已释放，客户端配置已还原" })
          : t("gateway.form.adopted", {
              defaultValue: "已接管，客户端指向网关",
            }),
      );
    } catch (error) {
      toast.error(
        t("gateway.form.routeFailed", {
          defaultValue: "切换失败：{{error}}",
          error: String(error),
        }),
      );
    } finally {
      setSwitchingRoute(false);
    }
  };

  const save = async () => {
    if (errors.length > 0) return;
    setSaving(true);
    try {
      await gatewayApi.setAppModelConfig(appType, {
        ...draft,
        default: draft.default,
        routing: meta,
      });
      await queryClient.invalidateQueries({
        queryKey: gatewayKeys.appConfig(appType),
      });
      toast.success(
        t("gateway.form.saved", { defaultValue: "已保存，已接管的客户端立即生效" }),
      );
    } catch (error) {
      toast.error(
        t("gateway.form.saveFailed", {
          defaultValue: "保存失败：{{error}}",
          error: String(error),
        }),
      );
    } finally {
      setSaving(false);
    }
  };

  const reset = () => {
    setDraft(configQuery.data ?? {});
    setMeta({
      ...metaFromConfig(configQuery.data),
      profileName: metaFromConfig(configQuery.data).profileName || appLabel,
    });
  };

  if (catalogQuery.isLoading || configQuery.isLoading) {
    return (
      <div className="p-6 text-sm text-muted-foreground">
        {t("gateway.form.loading", { defaultValue: "加载模型目录…" })}
      </div>
    );
  }

  return (
    <div
      id="main-content"
      className="flex min-h-0 flex-1 flex-col overflow-y-auto scroll-stable"
    >
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-5 p-6">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-lg font-semibold">
            {t("gateway.form.title", { defaultValue: "编辑配置" })}
          </h2>
          {clientApp ? null : (
            <div className="flex items-center gap-2">
              <span className={adopted ? "text-caption text-success" : "text-caption text-fg-3"}>
                {adopted
                  ? t("gateway.form.adoptedBadge", { defaultValue: "已接管" })
                  : t("gateway.form.directBadge", { defaultValue: "未接管" })}
              </span>
              <Button
                variant="neutral"
                size="sm"
                disabled={switchingRoute}
                onClick={() => void toggleRoute()}
              >
                {switchingRoute
                  ? t("gateway.form.routeSwitching", { defaultValue: "处理中…" })
                  : adopted
                    ? t("gateway.form.release", { defaultValue: "释放" })
                    : t("gateway.form.adopt", { defaultValue: "接管" })}
              </Button>
            </div>
          )}
        </div>

        {clientApp ? <ClientGatewayBar app={clientApp} /> : null}

        <div className="grid grid-cols-2 gap-4">
          <Field label={t("gateway.form.agent", { defaultValue: "Agent" })} required>
            <div className="flex h-10 items-center gap-2 rounded-md border border-border bg-muted px-3 text-sm">
              {appLabel}
            </div>
          </Field>
          <Field
            label={t("gateway.form.profileName", { defaultValue: "配置档案名称" })}
            required
          >
            <Input
              value={meta.profileName}
              onChange={(event) =>
                setMeta({ ...meta, profileName: event.target.value })
              }
            />
          </Field>
          <Field label={t("gateway.form.scope", { defaultValue: "作用范围" })} required>
            <select
              className="h-10 w-full rounded-md border border-border bg-input px-3 text-sm"
              value={meta.scope}
              onChange={(event) => setMeta({ ...meta, scope: event.target.value })}
            >
              <option value="system">
                {t("gateway.form.scopeSystem", { defaultValue: "系统默认" })}
              </option>
              <option value="project">
                {t("gateway.form.scopeProject", { defaultValue: "仅本项目" })}
              </option>
            </select>
          </Field>
          <Field label={t("gateway.form.entryMode", { defaultValue: "入口模式" })} required>
            <select
              className="h-10 w-full rounded-md border border-border bg-input px-3 text-sm"
              value={meta.entryMode}
              onChange={(event) => setMeta({ ...meta, entryMode: event.target.value })}
            >
              <option value="cli-and-app">
                {t("gateway.form.entryCliAndApp", { defaultValue: "CLI 和 APP" })}
              </option>
              <option value="cli">
                {t("gateway.form.entryCli", { defaultValue: "仅 CLI" })}
              </option>
              <option value="app">
                {t("gateway.form.entryApp", { defaultValue: "仅 APP" })}
              </option>
            </select>
          </Field>
        </div>

        <Field label={t("gateway.form.defaultModel", { defaultValue: "默认模型" })} required>
          <ModelPicker
            entries={entries}
            value={draft.default}
            onChange={(reference) => setDraft({ ...draft, default: reference })}
          />
        </Field>

        {slots.length > 0 && (
          <div className="grid grid-cols-2 gap-4">
            {[slots[0], slots[1], slots[2], slots[3]]
              .filter(Boolean)
              .map((slot) => (
                <Field
                  key={slot.key}
                  label={t("gateway.form.slotModel", {
                    defaultValue: "{{slot}} 模型",
                    slot: slot.label,
                  })}
                  optional
                >
                  <ModelPicker
                    entries={entries}
                    value={draft.slots?.[slot.key]}
                    onChange={(reference) =>
                      setDraft({
                        ...draft,
                        slots: applySlotChange(draft.slots, slot.key, reference),
                      })
                    }
                  />
                </Field>
              ))}
          </div>
        )}

        <div className="rounded-md border border-border">
          <button
            type="button"
            className="flex w-full items-center justify-between gap-2 px-4 py-3 text-left text-sm font-medium"
            onClick={() => setAdvancedOpen((open) => !open)}
          >
            <span>
              {t("gateway.form.advanced", { defaultValue: "高级设置" })}
              <span className="ml-2 text-caption text-fg-3">
                {t("gateway.form.advancedHint", {
                  defaultValue: "路由、允许模型列表",
                })}
              </span>
            </span>
            <ChevronDown
              className={`size-4 transition-transform ${advancedOpen ? "rotate-180" : ""}`}
            />
          </button>
          {advancedOpen && (
            <div className="flex flex-col gap-4 border-t border-border p-4">
              <div className="flex items-center justify-between gap-4">
                <span className="text-sm">
                  {t("gateway.form.enhancedRouting", { defaultValue: "增强路由" })}
                </span>
                <Switch
                  checked={meta.enhancedRouting}
                  onCheckedChange={(checked) =>
                    setMeta({ ...meta, enhancedRouting: checked })
                  }
                />
              </div>
              <div className="flex items-center justify-between gap-4">
                <span className="text-sm">
                  {t("gateway.form.configLevelRouting", {
                    defaultValue: "配置级路由",
                  })}
                </span>
                <Switch
                  checked={meta.configLevelRouting}
                  onCheckedChange={(checked) =>
                    setMeta({ ...meta, configLevelRouting: checked })
                  }
                />
              </div>

              <div className="flex flex-col gap-2">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-sm font-medium">
                    {t("gateway.form.allowedModels", {
                      defaultValue: "允许模型列表",
                    })}
                  </span>
                  <div className="flex items-center gap-1">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() =>
                        setDraft({ ...draft, allowed: entries.map((e) => e.reference) })
                      }
                    >
                      {t("gateway.form.selectAll", { defaultValue: "全部" })}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setDraft({ ...draft, allowed: [] })}
                    >
                      {t("gateway.form.clear", { defaultValue: "清除" })}
                    </Button>
                  </div>
                </div>
                <Input
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                  placeholder={t("gateway.form.searchPlaceholder", {
                    defaultValue: "搜索供应商或模型",
                  })}
                />
                <div className="grid max-h-72 grid-cols-2 gap-2 overflow-y-auto">
                  {filtered.map((entry) => (
                    <label
                      key={entry.reference}
                      className="flex items-center gap-2 rounded-md border border-border px-3 py-2 text-sm"
                    >
                      <Checkbox
                        checked={allowed.includes(entry.reference)}
                        onCheckedChange={(checked) =>
                          toggleAllowed(entry.reference, checked === true)
                        }
                      />
                      <span className="truncate">{entry.reference}</span>
                    </label>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>

        {errors.length > 0 && (
          <ul className="text-sm text-danger">
            {errors.map((error) => (
              <li key={error}>{error}</li>
            ))}
          </ul>
        )}

        <div className="flex items-center justify-end gap-2">
          <Button variant="neutral" onClick={reset} disabled={saving}>
            {t("gateway.form.cancel", { defaultValue: "取消" })}
          </Button>
          <Button onClick={save} disabled={saving || errors.length > 0}>
            {saving
              ? t("gateway.form.saving", { defaultValue: "保存中…" })
              : t("gateway.form.save", { defaultValue: "保存" })}
          </Button>
        </div>
      </div>
    </div>
  );
}

function Field({
  label,
  required,
  optional,
  children,
}: {
  label: string;
  required?: boolean;
  optional?: boolean;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <span className="flex items-center gap-1.5 text-sm text-fg-2">
        {label}
        {required && (
          <span className="rounded bg-warning/15 px-1 text-caption text-warning">
            必填
          </span>
        )}
        {optional && <span className="text-caption text-fg-3">可选</span>}
      </span>
      {children}
    </div>
  );
}
