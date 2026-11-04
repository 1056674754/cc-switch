import { useLayoutEffect, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { Eye, EyeOff, Loader2 } from "lucide-react";

import { getAppLabel } from "@/config/appConfig";
import { suppliersApi, type Supplier } from "@/lib/api/suppliers";
import { suppliersKeys } from "@/lib/query/suppliers";
import { toast } from "@/lib/toast";
import { extractErrorMessage } from "@/utils/errorUtils";
import { FieldError, LABEL_CLASS } from "@/components/mcp/formBits";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";

/**
 * 后端 create_supplier 目前只接受这两个 app：其余 app 的配置要按供应商维护模型列表 / 认证，
 * 没法定向生成，后端会 reject 并给中文原因。
 */
const CREATABLE_APPS = ["claude", "claude-desktop"] as const;

/** 就地校验的结果：值是 i18n key，渲染时再翻译。 */
interface CreateSupplierErrors {
  name?: string;
  baseUrl?: string;
  secret?: string;
  apps?: string;
}

/**
 * 地址必须是完整的 http(s)：和后端 p1_normalize_url 的口径对齐（没有 scheme 的裸域名不算数），
 * 前端先挡一道，省一次往返。
 */
function isCompleteHttpUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return (
      (url.protocol === "http:" || url.protocol === "https:") &&
      url.hostname.trim() !== ""
    );
  } catch {
    return false;
  }
}

export interface CreateSupplierDialogProps {
  open: boolean;
  onClose: () => void;
}

/**
 * 新增供应商：在网关侧一次配好地址 + 密钥，并勾选哪些 agent 直接接入。
 *
 * 提交成功后用后端返回的总览替换缓存（不再多拉一次），并让各应用的供应商列表重读。
 * 失败时弹后端的中文原因且**不关**，用户改完可以直接重试。
 */
export function CreateSupplierDialog({
  open,
  onClose,
}: CreateSupplierDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  // 新建时密钥永远是空的：这里没有旧密钥可显示，后端也不会回传明文
  const [secret, setSecret] = useState("");
  const [revealed, setRevealed] = useState(false);
  // 默认全勾：最常见的用法就是「claude 和 claude-desktop 都接上」
  const [selectedApps, setSelectedApps] = useState<string[]>([
    ...CREATABLE_APPS,
  ]);
  const [errors, setErrors] = useState<CreateSupplierErrors>({});
  const [submitting, setSubmitting] = useState(false);

  // 每次打开都是干净的一份：上一轮的输入、勾选、密钥、报错一律不带走
  useLayoutEffect(() => {
    if (!open) return;
    setName("");
    setBaseUrl("");
    setSecret("");
    setRevealed(false);
    setSelectedApps([...CREATABLE_APPS]);
    setErrors({});
    setSubmitting(false);
  }, [open]);

  const requestClose = () => {
    if (submitting) return;
    onClose();
  };

  const toggleApp = (app: string) => {
    setSelectedApps((previous) =>
      previous.includes(app)
        ? previous.filter((current) => current !== app)
        : [...previous, app],
    );
    setErrors((previous) => ({ ...previous, apps: undefined }));
  };

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (submitting) return;

    const trimmedName = name.trim();
    const trimmedBaseUrl = baseUrl.trim();
    const trimmedSecret = secret.trim();
    // 按固定顺序回传，勾选顺序不影响后端（后端自己去重）
    const apps = CREATABLE_APPS.filter((app) => selectedApps.includes(app));

    const nextErrors: CreateSupplierErrors = {};
    if (!trimmedName) {
      nextErrors.name = "suppliers.createNameRequired";
    }
    if (!trimmedBaseUrl) {
      nextErrors.baseUrl = "suppliers.createBaseUrlRequired";
    } else if (!isCompleteHttpUrl(trimmedBaseUrl)) {
      nextErrors.baseUrl = "suppliers.createBaseUrlInvalid";
    }
    if (!trimmedSecret) {
      nextErrors.secret = "suppliers.createSecretRequired";
    }
    if (apps.length === 0) {
      nextErrors.apps = "suppliers.createAppsRequired";
    }

    setErrors(nextErrors);
    // 前端先挡一道：校验不过不发请求
    if (Object.keys(nextErrors).length > 0) return;

    setSubmitting(true);
    try {
      const updated = await suppliersApi.create(
        trimmedName,
        trimmedBaseUrl,
        trimmedSecret,
        apps,
      );
      // 后端返回的就是更新后的总览，直接落缓存，不再多拉一次
      queryClient.setQueryData<Supplier[]>(suppliersKeys.overview, updated);
      // 被勾选的 app 里已经多了一条供应商配置，让这些列表下次进入时重新读
      void queryClient.invalidateQueries({ queryKey: ["providers"] });
      toast.success(t("suppliers.createSuccess", { name: trimmedName }));
      onClose();
    } catch (createError) {
      // Tauri invoke 的 reject 值是后端序列化出的纯字符串（中文原因），原样透出
      toast.error(
        extractErrorMessage(createError) || t("suppliers.createFailed"),
      );
      // 不关对话框：用户改完名字 / 地址 / 密钥可以直接重试
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => {
        if (!nextOpen) requestClose();
      }}
    >
      <DialogContent
        className="max-w-md"
        // 项目里的对话框默认点遮罩不关（ui/dialog.tsx）；这一个按约定点遮罩退出，
        // 但提交中要拦住，免得用户以为请求没在跑
        onInteractOutside={(event) => {
          if (submitting) event.preventDefault();
        }}
        onEscapeKeyDown={(event) => {
          if (submitting) event.preventDefault();
        }}
      >
        <form onSubmit={(event) => void submit(event)} className="min-h-0">
          <DialogHeader>
            <DialogTitle>{t("suppliers.createTitle")}</DialogTitle>
            <DialogDescription>
              {t("suppliers.createDescription")}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 px-6 pb-1">
            <div className="flex flex-col gap-1.5">
              <label htmlFor="create-supplier-name" className={LABEL_CLASS}>
                {t("suppliers.createNameLabel")}
              </label>
              <Input
                id="create-supplier-name"
                autoFocus
                value={name}
                disabled={submitting}
                placeholder={t("suppliers.createNamePlaceholder")}
                aria-invalid={errors.name ? true : undefined}
                aria-describedby={
                  errors.name ? "create-supplier-name-error" : undefined
                }
                onChange={(event) => {
                  setName(event.target.value);
                  if (errors.name) {
                    setErrors((previous) => ({
                      ...previous,
                      name: undefined,
                    }));
                  }
                }}
              />
              {errors.name && (
                <FieldError id="create-supplier-name-error">
                  {t(errors.name)}
                </FieldError>
              )}
            </div>

            <div className="flex flex-col gap-1.5">
              <label htmlFor="create-supplier-base-url" className={LABEL_CLASS}>
                {t("suppliers.createBaseUrlLabel")}
              </label>
              <Input
                id="create-supplier-base-url"
                className="font-mono"
                value={baseUrl}
                disabled={submitting}
                placeholder={t("suppliers.createBaseUrlPlaceholder")}
                aria-invalid={errors.baseUrl ? true : undefined}
                aria-describedby={
                  errors.baseUrl ? "create-supplier-base-url-error" : undefined
                }
                onChange={(event) => {
                  setBaseUrl(event.target.value);
                  if (errors.baseUrl) {
                    setErrors((previous) => ({
                      ...previous,
                      baseUrl: undefined,
                    }));
                  }
                }}
              />
              {errors.baseUrl && (
                <FieldError id="create-supplier-base-url-error">
                  {t(errors.baseUrl)}
                </FieldError>
              )}
            </div>

            <div className="flex flex-col gap-1.5">
              <label htmlFor="create-supplier-secret" className={LABEL_CLASS}>
                {t("suppliers.createSecretLabel")}
              </label>
              <div className="relative">
                <Input
                  id="create-supplier-secret"
                  type={revealed ? "text" : "password"}
                  className="pe-8 font-mono"
                  value={secret}
                  disabled={submitting}
                  placeholder={t("suppliers.createSecretPlaceholder")}
                  aria-invalid={errors.secret ? true : undefined}
                  aria-describedby={
                    errors.secret ? "create-supplier-secret-error" : undefined
                  }
                  onChange={(event) => {
                    setSecret(event.target.value);
                    if (errors.secret) {
                      setErrors((previous) => ({
                        ...previous,
                        secret: undefined,
                      }));
                    }
                  }}
                />
                {secret && (
                  <button
                    type="button"
                    disabled={submitting}
                    aria-label={
                      revealed ? t("apiKeyInput.hide") : t("apiKeyInput.show")
                    }
                    className="absolute inset-y-0 end-0 flex items-center pe-2 text-fg-3 transition-colors hover:text-fg-1 disabled:cursor-not-allowed disabled:opacity-50"
                    onClick={() => setRevealed((value) => !value)}
                  >
                    {revealed ? (
                      <EyeOff aria-hidden="true" className="h-3.5 w-3.5" />
                    ) : (
                      <Eye aria-hidden="true" className="h-3.5 w-3.5" />
                    )}
                  </button>
                )}
              </div>
              {errors.secret && (
                <FieldError id="create-supplier-secret-error">
                  {t(errors.secret)}
                </FieldError>
              )}
            </div>

            <div className="flex flex-col gap-1.5">
              <span id="create-supplier-apps-label" className={LABEL_CLASS}>
                {t("suppliers.createAppsLabel")}
              </span>
              <div
                role="group"
                aria-labelledby="create-supplier-apps-label"
                aria-describedby={
                  errors.apps ? "create-supplier-apps-error" : undefined
                }
                className="flex flex-wrap items-center gap-x-5 gap-y-2"
              >
                {CREATABLE_APPS.map((app) => (
                  <label
                    key={app}
                    className="flex cursor-pointer items-center gap-2 text-body text-fg-1"
                  >
                    <Checkbox
                      checked={selectedApps.includes(app)}
                      disabled={submitting}
                      onCheckedChange={() => toggleApp(app)}
                    />
                    {getAppLabel(app)}
                  </label>
                ))}
              </div>
              <p className="m-0 text-caption text-fg-2">
                {t("suppliers.createAppsHint")}
              </p>
              {errors.apps && (
                <FieldError id="create-supplier-apps-error">
                  {t(errors.apps)}
                </FieldError>
              )}
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="neutral"
              size="regular"
              disabled={submitting}
              onClick={requestClose}
            >
              {t("common.cancel")}
            </Button>
            <Button
              type="submit"
              variant="solid"
              size="regular"
              disabled={submitting}
              aria-busy={submitting}
            >
              {submitting && (
                <Loader2 aria-hidden="true" className="h-4 w-4 animate-spin" />
              )}
              {t("suppliers.createSubmit")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

export default CreateSupplierDialog;
