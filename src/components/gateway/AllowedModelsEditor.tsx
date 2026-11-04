import { useTranslation } from "react-i18next";
import { AlertTriangle, X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { GatewayCatalogEntry } from "@/types/gateway";
import { ModelPicker } from "./ModelPicker";
import { addAllowedModel, removeAllowedModel } from "./appModelConfig";

/**
 * 允许模型列表：空 = 不限制；非空时默认模型必须在列表内（后端与本地校验都会挡）。
 * 目录里没有的旧引用显示为警告色并注明，不隐藏。
 */
export function AllowedModelsEditor({
  entries,
  allowed,
  defaultReference,
  onChange,
  disabled,
}: {
  entries: GatewayCatalogEntry[];
  allowed?: string[];
  defaultReference?: string;
  onChange: (allowed: string[]) => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation();
  const known = new Set(entries.map((entry) => entry.reference));
  const list = allowed ?? [];

  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap gap-1.5">
        {list.length === 0 ? (
          <span className="text-sm text-muted-foreground">
            {t("gateway.allowed.empty", {
              defaultValue: "未限制（所有目录模型都能选）",
            })}
          </span>
        ) : (
          list.map((reference) => {
            const stale = !known.has(reference);
            const isDefault = reference === defaultReference;
            return (
              <Badge
                key={reference}
                variant={stale ? "outline" : isDefault ? "default" : "secondary"}
                className="gap-1 font-normal"
              >
                {stale && <AlertTriangle className="h-3 w-3 text-amber-500" />}
                <span className="max-w-[16rem] truncate">{reference}</span>
                {isDefault && (
                  <span className="text-[10px] opacity-70">
                    {t("gateway.allowed.defaultTag", { defaultValue: "默认" })}
                  </span>
                )}
                {!disabled && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="h-4 w-4"
                    aria-label={t("gateway.allowed.remove", {
                      defaultValue: "移除 {{model}}",
                      model: reference,
                    })}
                    onClick={() => onChange(removeAllowedModel(list, reference))}
                  >
                    <X className="h-3 w-3" />
                  </Button>
                )}
              </Badge>
            );
          })
        )}
      </div>
      <div className="max-w-sm">
        <ModelPicker
          entries={entries}
          disabled={disabled}
          placeholder={t("gateway.allowed.add", {
            defaultValue: "添加允许的模型…",
          })}
          onChange={(reference) => {
            if (reference) onChange(addAllowedModel(list, reference));
          }}
        />
      </div>
    </div>
  );
}
