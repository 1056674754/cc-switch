import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangle, Check, ChevronsUpDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
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
 * （`<供应商名>/<模型 id>`）。目录里没有的旧值照样显示，但标警告。
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

  const groups = useMemo(() => buildModelPickerGroups(entries), [entries]);
  const filtered = useMemo(
    () => filterModelGroups(groups, query),
    [groups, query],
  );
  const selected = useMemo(() => selectedLabel(groups, value), [groups, value]);

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
        className="z-[200] w-[--radix-popover-trigger-width] p-0"
      >
        <Command shouldFilter={false}>
          <CommandInput
            value={query}
            onValueChange={setQuery}
            placeholder={t("gateway.modelPicker.search", {
              defaultValue: "搜索模型或供应商…",
            })}
          />
          <CommandList className="max-h-72">
            <CommandEmpty>
              {t("gateway.modelPicker.empty", {
                defaultValue: "没有匹配的模型",
              })}
            </CommandEmpty>
            {filtered.map((group) => (
              <CommandGroup key={group.upstreamName} heading={group.upstreamName}>
                {group.options.map((option) => (
                  <CommandItem
                    key={option.value}
                    value={option.value}
                    onSelect={() => {
                      onChange(option.value);
                      setOpen(false);
                      setQuery("");
                    }}
                  >
                    <Check
                      className={cn(
                        "mr-2 h-4 w-4",
                        value === option.value ? "opacity-100" : "opacity-0",
                      )}
                    />
                    <span className="flex min-w-0 flex-col">
                      <span className="truncate">{option.label}</span>
                      {option.description && (
                        <span className="truncate text-xs text-muted-foreground">
                          {option.description}
                        </span>
                      )}
                    </span>
                    {option.oneM && (
                      <span className="ml-auto rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
                        1M
                      </span>
                    )}
                  </CommandItem>
                ))}
              </CommandGroup>
            ))}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
