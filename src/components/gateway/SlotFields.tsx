import { useTranslation } from "react-i18next";
import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { GatewayCatalogEntry } from "@/types/gateway";
import { ModelPicker } from "./ModelPicker";
import { slotDefinitions } from "./appModelConfig";

/**
 * 槽位字段：Claude Code 四档，其余 app 不渲染。
 * 空 = 不配置；清空会从配置里删掉这个键，而不是写空串。
 */
export function SlotFields({
  appType,
  entries,
  slots,
  onChange,
  disabled,
}: {
  appType: string;
  entries: GatewayCatalogEntry[];
  slots?: Record<string, string>;
  onChange: (slots: Record<string, string>) => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation();
  const definitions = slotDefinitions(appType);
  if (definitions.length === 0) return null;

  return (
    <div className="grid gap-3 sm:grid-cols-2">
      {definitions.map((definition) => {
        const value = slots?.[definition.key];
        return (
          <div key={definition.key} className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between gap-2">
              <span className="text-sm font-medium">{definition.label}</span>
              {value ? (
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="h-5 w-5"
                  aria-label={t("gateway.slots.clear", {
                    defaultValue: "清空 {{slot}}",
                    slot: definition.label,
                  })}
                  disabled={disabled}
                  onClick={() => {
                    const next = { ...(slots ?? {}) };
                    delete next[definition.key];
                    onChange(next);
                  }}
                >
                  <X className="h-3.5 w-3.5" />
                </Button>
              ) : null}
            </div>
            <ModelPicker
              entries={entries}
              value={value}
              disabled={disabled}
              placeholder={t("gateway.slots.empty", {
                defaultValue: "不配置（用默认模型）",
              })}
              onChange={(reference) => {
                const next = { ...(slots ?? {}) };
                if (reference) next[definition.key] = reference;
                else delete next[definition.key];
                onChange(next);
              }}
            />
            <span className="text-xs text-muted-foreground">{definition.hint}</span>
          </div>
        );
      })}
    </div>
  );
}
