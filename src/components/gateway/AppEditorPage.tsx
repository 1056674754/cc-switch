import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "@/lib/toast";
import { Button } from "@/components/ui/button";
import { gatewayApi } from "@/lib/api/gateway";
import type { AppModelConfig } from "@/types/gateway";
import { AllowedModelsEditor } from "./AllowedModelsEditor";
import { ModelPicker } from "./ModelPicker";
import { SlotFields } from "./SlotFields";
import { validateDraft } from "./appModelConfig";

export const gatewayKeys = {
  catalog: ["gateway", "catalog"] as const,
  appConfig: (appType: string) => ["gateway", "appConfig", appType] as const,
};

/**
 * 应用页的模型配置编辑器（CCR 那份表单的内联版）。
 * 默认只读；点「编辑」才能改，保存走 `set_app_model_config`，后端再校验一次。
 */
export function AppEditorPage({
  appType,
  appLabel,
}: {
  appType: string;
  appLabel: string;
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

  const initial = configQuery.data ?? {};
  const [draft, setDraft] = useState<AppModelConfig>(initial);
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setDraft(configQuery.data ?? {});
  }, [configQuery.data]);

  const entries = catalogQuery.data ?? [];
  const errors = useMemo(
    () => (editing ? validateDraft(draft, entries) : []),
    [draft, editing, entries],
  );

  const onlyRead = !editing;
  const hasConfig = Boolean(
    configQuery.data?.default ||
      Object.keys(configQuery.data?.slots ?? {}).length > 0 ||
      (configQuery.data?.allowed ?? []).length > 0,
  );

  const save = async () => {
    if (errors.length > 0) return;
    setSaving(true);
    try {
      await gatewayApi.setAppModelConfig(appType, draft);
      await queryClient.invalidateQueries({
        queryKey: gatewayKeys.appConfig(appType),
      });
      setEditing(false);
      toast.success(
        t("gateway.editor.saved", { defaultValue: "已保存模型配置" }),
      );
    } catch (error) {
      toast.error(
        t("gateway.editor.saveFailed", {
          defaultValue: "保存失败：{{error}}",
          error: String(error),
        }),
      );
    } finally {
      setSaving(false);
    }
  };

  const migrate = async () => {
    setSaving(true);
    try {
      const result = await gatewayApi.migrateFromStack();
      const mine = result.find((item) => item.app === appType);
      await queryClient.invalidateQueries({ queryKey: gatewayKeys.catalog });
      await queryClient.invalidateQueries({
        queryKey: gatewayKeys.appConfig(appType),
      });
      toast.success(
        mine?.status === "migrated"
          ? t("gateway.editor.migrated", {
              defaultValue: "已从聚合配置迁移 {{count}} 个模型",
              count: mine.models,
            })
          : t("gateway.editor.nothingToMigrate", {
              defaultValue: "没有可迁移的聚合配置",
            }),
      );
    } catch (error) {
      toast.error(
        t("gateway.editor.migrateFailed", {
          defaultValue: "迁移失败：{{error}}",
          error: String(error),
        }),
      );
    } finally {
      setSaving(false);
    }
  };

  if (catalogQuery.isLoading || configQuery.isLoading) {
    return (
      <div className="p-6 text-sm text-muted-foreground">
        {t("gateway.editor.loading", { defaultValue: "加载模型目录…" })}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6 p-6">
      <header className="flex items-center justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">
            {t("gateway.editor.title", {
              defaultValue: "{{app}} 模型配置",
              app: appLabel,
            })}
          </h2>
          <p className="text-sm text-muted-foreground">
            {t("gateway.editor.subtitle", {
              defaultValue:
                "网关按模型名选上游：选哪个模型，就走哪个供应商。目录里共 {{count}} 个模型。",
              count: entries.length,
            })}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {editing ? (
            <>
              <Button
                variant="ghost"
                onClick={() => {
                  setDraft(configQuery.data ?? {});
                  setEditing(false);
                }}
                disabled={saving}
              >
                {t("gateway.editor.cancel", { defaultValue: "取消" })}
              </Button>
              <Button
                onClick={save}
                disabled={saving || errors.length > 0}
              >
                {t("gateway.editor.save", { defaultValue: "保存" })}
              </Button>
            </>
          ) : (
            <>
              {!hasConfig && (
                <Button variant="outline" onClick={migrate} disabled={saving}>
                  {t("gateway.editor.migrate", {
                    defaultValue: "从聚合配置迁移",
                  })}
                </Button>
              )}
              <Button onClick={() => setEditing(true)} disabled={saving}>
                {t("gateway.editor.edit", { defaultValue: "编辑" })}
              </Button>
            </>
          )}
        </div>
      </header>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-medium">
          {t("gateway.editor.defaultModel", { defaultValue: "默认模型" })}
        </h3>
        <div className="max-w-md">
          <ModelPicker
            entries={entries}
            value={draft.default}
            disabled={onlyRead}
            onChange={(reference) => setDraft({ ...draft, default: reference })}
          />
        </div>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-medium">
          {t("gateway.editor.slots", { defaultValue: "槽位模型" })}
        </h3>
        <SlotFields
          appType={appType}
          entries={entries}
          slots={draft.slots}
          disabled={onlyRead}
          onChange={(slots) => setDraft({ ...draft, slots })}
        />
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-medium">
          {t("gateway.editor.allowed", { defaultValue: "允许的模型" })}
        </h3>
        <AllowedModelsEditor
          entries={entries}
          allowed={draft.allowed}
          defaultReference={draft.default}
          disabled={onlyRead}
          onChange={(allowed) => setDraft({ ...draft, allowed })}
        />
      </section>

      {errors.length > 0 && (
        <ul className="text-sm text-amber-600">
          {errors.map((error) => (
            <li key={error}>{error}</li>
          ))}
        </ul>
      )}
    </div>
  );
}
