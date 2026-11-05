import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangle, Check, ChevronsUpDown, Search } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { cn } from "@/lib/utils";
import type { GatewayCatalogEntry } from "@/types/gateway";
import {
  buildModelPickerGroups,
  filterModelGroups,
  selectedLabel,
} from "./modelPickerOptions";

/**
 * 网关模型选择器：数据来自 `get_gateway_catalog`，值是模型引用
 * （`<供应商名>/<模型 id>`）。
 *
 * 布局照 CCR：左边供应商（带模型数），右边该供应商的模型，顶部一个
 * 「搜索供应商或模型」的框；选中项高亮 + 勾。目录里没有的旧值照样显示，但标警告。
 */
export function ModelPicker({
  entries,
  value,
  onChange,
  placeholder,
  disabled,
}: {
  entries: GatewayCatalogEntry[];
  value?: string;
  onChange: (reference: string | undefined) => void;
  placeholder?: string;
  disabled?: boolean;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [activeSupplier, setActiveSupplier] = useState("");

  const groups = useMemo(() => buildModelPickerGroups(entries), [entries]);
  const filtered = useMemo(
    () => filterModelGroups(groups, query),
    [groups, query],
  );
  const selected = useMemo(() => selectedLabel(groups, value), [groups, value]);
  const valueSupplier = value?.split("/")[0] ?? "";
  const activeGroup =
    filtered.find((group) => group.upstreamName === activeSupplier) ??
    filtered.find((group) => group.upstreamName === valueSupplier) ??
    filtered[0];

  // 打开时把左栏定位到当前值所在的供应商；搜索把选中的供应商筛掉了就退到第一个。
  useEffect(() => {
    if (!open) return;
    setActiveSupplier(valueSupplier || groups[0]?.upstreamName || "");
  }, [open, valueSupplier, groups]);

  useEffect(() => {
    if (filtered.length === 0) {
      if (activeSupplier) setActiveSupplier("");
      return;
    }
    if (!filtered.some((group) => group.upstreamName === activeSupplier)) {
      setActiveSupplier(filtered[0].upstreamName);
    }
  }, [filtered, activeSupplier]);

  const choose = (reference: string) => {
    onChange(reference);
    setOpen(false);
    setQuery("");
  };

  return (
    <Popover modal open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="outline"
          role="combobox"
          aria-expanded={open}
          disabled={disabled}
          className="w-full justify-between font-normal"
        >
          <span className="flex min-w-0 items-center gap-2">
            {selected.stale && (
              <AlertTriangle className="h-4 w-4 shrink-0 text-amber-500" />
            )}
            <span
              className={cn(
                "truncate",
                !value && "text-muted-foreground",
                selected.stale && "text-amber-600",
              )}
            >
              {value
                ? selected.label
                : (placeholder ??
                  t("gateway.modelPicker.placeholder", {
                    defaultValue: "选择模型",
                  }))}
            </span>
          </span>
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="start"
        sideOffset={4}
        collisionPadding={8}
        className="z-[200] w-[var(--radix-popover-trigger-width)] min-w-[380px] p-2"
      >
        <div className="relative">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-fg-3" />
          <Input
            autoFocus
            aria-label={t("gateway.modelPicker.searchLabel", {
              defaultValue: "搜索供应商或模型",
            })}
            className="h-9 pl-8"
            placeholder={t("gateway.modelPicker.search", {
              defaultValue: "搜索供应商或模型…",
            })}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </div>

        {groups.length === 0 ? (
          <div className="mt-2 rounded-md border border-dashed border-border bg-muted/30 px-3 py-8 text-center text-caption text-fg-3">
            {t("gateway.modelPicker.noModels", {
              defaultValue: "目录里还没有模型",
            })}
          </div>
        ) : (
          <div className="mt-2 grid h-64 grid-cols-[minmax(108px,0.38fr)_minmax(0,1fr)] overflow-hidden rounded-md border border-border">
            <div
              role="listbox"
              aria-label={t("gateway.modelPicker.suppliers", {
                defaultValue: "供应商",
              })}
              className="min-w-0 overflow-auto border-r border-border bg-muted/40 p-1"
            >
              {filtered.length === 0 ? (
                <div className="px-2 py-6 text-center text-caption text-fg-3">
                  {t("gateway.modelPicker.noMatch", {
                    defaultValue: "没有匹配项",
                  })}
                </div>
              ) : null}
              {filtered.map((group) => {
                const active = group.upstreamName === activeGroup?.upstreamName;
                return (
                  <button
                    key={group.upstreamName}
                    type="button"
                    role="option"
                    aria-selected={active}
                    className={cn(
                      "flex h-9 w-full min-w-0 items-center gap-2 rounded-[5px] px-2 text-left text-body outline-none transition-colors hover:bg-surface focus-visible:ring-2 focus-visible:ring-border",
                      active && "bg-surface text-fg-0",
                    )}
                    onClick={() => setActiveSupplier(group.upstreamName)}
                  >
                    <span className="min-w-0 flex-1 truncate">
                      {group.upstreamName}
                    </span>
                    <Badge variant="outline" className="shrink-0 px-1.5 py-0 text-[10px]">
                      {group.options.length}
                    </Badge>
                  </button>
                );
              })}
            </div>

            <div
              role="listbox"
              aria-label={t("gateway.modelPicker.models", {
                defaultValue: "模型",
              })}
              className="min-w-0 overflow-auto p-1"
            >
              {!activeGroup || activeGroup.options.length === 0 ? (
                <div className="px-2 py-10 text-center text-caption text-fg-3">
                  {t("gateway.modelPicker.noMatch", {
                    defaultValue: "没有匹配项",
                  })}
                </div>
              ) : null}
              {activeGroup?.options.map((option) => {
                const isSelected = option.value === value;
                return (
                  <button
                    key={option.value}
                    type="button"
                    role="option"
                    aria-selected={isSelected}
                    className={cn(
                      "flex h-9 w-full min-w-0 items-center gap-2 rounded-[5px] px-2 text-left text-body outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-border",
                      isSelected && "bg-muted text-fg-0",
                    )}
                    onClick={() => choose(option.value)}
                    title={option.value}
                  >
                    <span className="min-w-0 flex-1 truncate">{option.label}</span>
                    {option.window ? (
                      <span className="shrink-0 text-[11px] text-fg-3">
                        {option.window}
                      </span>
                    ) : null}
                    {option.oneM ? (
                      <span className="shrink-0 rounded bg-muted px-1.5 py-0.5 text-[10px] text-fg-3">
                        1M
                      </span>
                    ) : null}
                    {isSelected ? (
                      <Check className="h-3.5 w-3.5 shrink-0" />
                    ) : null}
                  </button>
                );
              })}
            </div>
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}
