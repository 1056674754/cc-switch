import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronDown, RotateCcw } from "lucide-react";
import { toast } from "@/lib/toast";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { gatewayApi } from "@/lib/api/gateway";
import type { GatewayCatalogEntry, ModelOverride } from "@/types/gateway";

export const supplierModelKeys = {
  catalog: ["gateway", "catalog"] as const,
  overrides: ["gateway", "modelOverrides"] as const,
};

/** 供应商卡片里的模型表：目录按上游过滤，改名 / 隐藏走人工覆盖，改完目录立刻失效。 */
export function SupplierModelsPanel({
  upstreamIds,
}: {
  upstreamIds: string[];
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [savingKey, setSavingKey] = useState<string | null>(null);

  const catalogQuery = useQuery({
    queryKey: supplierModelKeys.catalog,
    queryFn: () => gatewayApi.getCatalog(),
    staleTime: 30_000,
  });
  const overridesQuery = useQuery({
    queryKey: supplierModelKeys.overrides,
    queryFn: () => gatewayApi.getModelOverrides(),
    staleTime: 30_000,
  });

  const entries = useMemo(() => {
    const ids = new Set(upstreamIds);
    return (catalogQuery.data ?? []).filter((entry) =>
      ids.has(entry.upstreamId),
    );
  }, [catalogQuery.data, upstreamIds]);

  const overrides = useMemo(() => {
    const map = new Map<string, ModelOverride>();
    for (const override of overridesQuery.data ?? []) {
      map.set(`${override.upstreamId}\u0000${override.modelId}`, override);
    }
    return map;
  }, [overridesQuery.data]);

  const keyOf = (entry: GatewayCatalogEntry) =>
    `${entry.upstreamId}\u0000${entry.modelId}`;

  useEffect(() => {
    // 目录刷新后把草稿重置成当前显示名（已改过的保持覆盖值）
    setDrafts({});
  }, [catalogQuery.dataUpdatedAt]);

  const refresh = async () => {
    await queryClient.invalidateQueries({ queryKey: supplierModelKeys.catalog });
    await queryClient.invalidateQueries({
      queryKey: supplierModelKeys.overrides,
    });
  };

  const save = async (entry: GatewayCatalogEntry, hidden: boolean) => {
    const key = keyOf(entry);
    setSavingKey(key);
    try {
      await gatewayApi.setModelOverride({
        upstreamId: entry.upstreamId,
        modelId: entry.modelId,
        displayName: drafts[key]?.trim() || null,
        hidden,
      });
      await refresh();
      toast.success(t("suppliers.modelSaved", { defaultValue: "模型已更新" }));
    } catch (error) {
      toast.error(
        t("suppliers.modelSaveFailed", {
          defaultValue: "更新失败：{{error}}",
          error: String(error),
        }),
      );
    } finally {
      setSavingKey(null);
    }
  };

  const clear = async (entry: GatewayCatalogEntry) => {
    const key = keyOf(entry);
    setSavingKey(key);
    try {
      await gatewayApi.clearModelOverride({
        upstreamId: entry.upstreamId,
        modelId: entry.modelId,
      });
      await refresh();
    } catch (error) {
      toast.error(
        t("suppliers.modelSaveFailed", {
          defaultValue: "更新失败：{{error}}",
          error: String(error),
        }),
      );
    } finally {
      setSavingKey(null);
    }
  };

  if (upstreamIds.length === 0) return null;

  return (
    <div className="border-t border-border">
      <button
        type="button"
        className="flex w-full items-center justify-between gap-2 px-4 py-2 text-left text-caption text-fg-2"
        onClick={() => setOpen((value) => !value)}
      >
        <span>{t("suppliers.models", { defaultValue: "模型" })}</span>
        <span className="flex items-center gap-2 text-fg-3">
          <span>
            {catalogQuery.isLoading ? "…" : entries.length}
          </span>
          <ChevronDown
            className={`size-3.5 transition-transform ${open ? "rotate-180" : ""}`}
          />
        </span>
      </button>
      {open && (
        <div className="px-4 pb-3">
          {entries.length === 0 ? (
            <p className="py-2 text-caption text-fg-3">
              {t("suppliers.noModels", {
                defaultValue: "目录里没有这家供应商的模型（先跑一次请求或同步配置）",
              })}
            </p>
          ) : (
            <table className="w-full text-caption">
              <thead className="text-fg-3">
                <tr>
                  <th className="py-1 text-left font-normal">
                    {t("suppliers.modelId", { defaultValue: "模型 id" })}
                  </th>
                  <th className="py-1 text-left font-normal">
                    {t("suppliers.modelDisplayName", { defaultValue: "显示名" })}
                  </th>
                  <th className="py-1 text-right font-normal">
                    {t("suppliers.modelHidden", { defaultValue: "隐藏" })}
                  </th>
                  <th className="w-8" />
                </tr>
              </thead>
              <tbody>
                {entries.map((entry) => {
                  const key = keyOf(entry);
                  const override = overrides.get(key);
                  const dirty =
                    drafts[key] !== undefined &&
                    drafts[key] !== (override?.displayName ?? "");
                  return (
                    <tr key={key} className="border-t border-border/60">
                      <td className="max-w-[14rem] truncate py-1 pr-2 font-mono text-fg-2">
                        <span title={entry.reference}>{entry.modelId}</span>
                      </td>
                      <td className="py-1 pr-2">
                        <Input
                          className="h-7 text-caption"
                          value={drafts[key] ?? override?.displayName ?? ""}
                          placeholder={entry.displayName ?? undefined}
                          disabled={savingKey === key}
                          onChange={(event) =>
                            setDrafts({ ...drafts, [key]: event.target.value })
                          }
                          onKeyDown={(event) => {
                            if (event.key === "Enter") {
                              event.preventDefault();
                              void save(entry, override?.hidden ?? false);
                            } else if (event.key === "Escape") {
                              setDrafts({ ...drafts, [key]: override?.displayName ?? "" });
                            }
                          }}
                        />
                      </td>
                      <td className="py-1 text-right">
                        <Checkbox
                          checked={override?.hidden ?? false}
                          disabled={savingKey === key}
                          onCheckedChange={(checked) =>
                            void save(entry, checked === true)
                          }
                        />
                      </td>
                      <td className="py-1 text-right">
                        {override ? (
                          <Button
                            type="button"
                            variant="ghost"
                            size="compact"
                            aria-label={t("suppliers.modelReset", {
                              defaultValue: "还原",
                            })}
                            disabled={savingKey === key}
                            onClick={() => void clear(entry)}
                          >
                            <RotateCcw className="size-3.5" />
                          </Button>
                        ) : null}
                        {dirty ? (
                          <Button
                            type="button"
                            variant="ghost"
                            size="compact"
                            onClick={() =>
                              void save(entry, override?.hidden ?? false)
                            }
                          >
                            {t("suppliers.modelSave", { defaultValue: "保存" })}
                          </Button>
                        ) : null}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </div>
      )}
    </div>
  );
}
