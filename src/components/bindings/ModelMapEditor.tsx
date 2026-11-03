import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  MODEL_MAP_SLOTS,
  type ModelMapDraft,
  type ModelMapSlot,
} from "./modelMap";

export interface ModelMapEditorProps {
  /** 只用于无障碍标签：「<app> 的模型映射」 */
  appLabel: string;
  draft: ModelMapDraft;
  saving: boolean;
  onChange: (slot: ModelMapSlot, value: string) => void;
  onSave: () => void;
  onCancel: () => void;
}

/**
 * 模型映射的就地编辑器：六个固定槽位。
 *
 * 槽位留空表示清掉那个模型字段——后端会把对应的 env 键删掉，所以这里不做
 * 「必填」校验；六个都空保存等于清空整份映射。
 */
export function ModelMapEditor({
  appLabel,
  draft,
  saving,
  onChange,
  onSave,
  onCancel,
}: ModelMapEditorProps) {
  const { t } = useTranslation();

  return (
    <div
      role="group"
      aria-label={t("bindings.modelMapFor", { app: appLabel })}
      className="mt-2 rounded-control border border-border bg-surface p-2.5"
    >
      <div className="grid gap-2 sm:grid-cols-2">
        {MODEL_MAP_SLOTS.map((slot) => (
          <label key={slot} className="flex min-w-0 items-center gap-2">
            <span className="w-14 shrink-0 font-mono text-caption text-fg-3">
              {t(`bindings.modelSlot.${slot}`)}
            </span>
            <Input
              value={draft[slot]}
              disabled={saving}
              placeholder={t("bindings.modelMapPlaceholder")}
              className="h-7 min-w-0 flex-1 font-mono text-caption"
              onChange={(event) => onChange(slot, event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  onSave();
                } else if (event.key === "Escape") {
                  event.preventDefault();
                  onCancel();
                }
              }}
            />
          </label>
        ))}
      </div>

      <p className="mt-1.5 mb-0 text-caption text-fg-3">
        {t("bindings.modelMapHint")}
      </p>

      <div className="mt-2 flex justify-end gap-2">
        <Button
          type="button"
          variant="solid"
          size="compact"
          disabled={saving}
          aria-busy={saving}
          onClick={onSave}
        >
          {saving && (
            <Loader2 aria-hidden="true" className="h-3.5 w-3.5 animate-spin" />
          )}
          {t("common.save")}
        </Button>
        <Button
          type="button"
          variant="neutral"
          size="compact"
          disabled={saving}
          onClick={onCancel}
        >
          {t("common.cancel")}
        </Button>
      </div>
    </div>
  );
}

export default ModelMapEditor;
