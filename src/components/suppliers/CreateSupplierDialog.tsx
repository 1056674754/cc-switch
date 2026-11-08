import { useLayoutEffect, useMemo, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  Eye,
  EyeOff,
  Loader2,
  Search,
  ShieldCheck,
  Sparkles,
  X,
} from "lucide-react";

import { getAppLabel } from "@/config/appConfig";
import { authApi, type ManagedAuthProvider } from "@/lib/api/auth";
import { suppliersApi, type Supplier } from "@/lib/api/suppliers";
import { suppliersKeys } from "@/lib/query/suppliers";
import { toast } from "@/lib/toast";
import { extractErrorMessage } from "@/utils/errorUtils";
import { FieldError, LABEL_CLASS } from "@/components/mcp/formBits";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { isCompleteHttpUrl } from "./supplierForm";
import {
  searchSupplierPresets,
  type SupplierPreset,
  type SupplierPresetProtocol,
} from "./supplierPresets";

/**
 * 创建时能连带接入的 app：后端只为 anthropic 协议渲染 claude 系配置。
 * 其它协议创建的供应商先不接应用，之后在卡片上按协议接入。
 */
const CREATABLE_APPS = ["claude", "claude-desktop"] as const;

const PROTOCOLS: SupplierPresetProtocol[] = [
  "anthropic",
  "openai_responses",
  "openai_chat",
  "gemini_native",
];

/**
 * 「导入本机 Agent」只列这三家官方订阅：它们的供应商语义是「一份订阅 = 一家供应商」，
 * 且协议由服务决定。其它托管登录（Copilot / xAI / agy）没有独立订阅语义，去应用页登录。
 */
const IMPORTABLE_PROVIDERS: {
  provider: ManagedAuthProvider;
  protocol: SupplierPresetProtocol;
}[] = [
  { provider: "codex_oauth", protocol: "openai_responses" },
  { provider: "claude_official", protocol: "anthropic" },
];

type Step = "choose" | "preset" | "import" | "form";

/** 就地校验的结果：值是 i18n key，渲染时再翻译。 */
interface CreateSupplierErrors {
  name?: string;
  baseUrl?: string;
  secret?: string;
}

export interface CreateSupplierDialogProps {
  open: boolean;
  onClose: () => void;
}

function CreateSection({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="space-y-3">
      <div>
        <h3 className="m-0 text-strong font-semibold text-fg-1">{title}</h3>
        {description ? (
          <p className="m-0 mt-0.5 text-caption leading-5 text-fg-3">
            {description}
          </p>
        ) : null}
      </div>
      {children}
    </section>
  );
}

/** 入口卡片：预设 / 导入 / 自定义三选一。 */
function SourceCard({
  title,
  description,
  icon,
  onClick,
}: {
  title: string;
  description: string;
  icon: React.ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex w-full items-start gap-3 rounded-lg border border-border bg-surface p-4 text-left transition-colors",
        "hover:border-primary/60 hover:bg-subtle focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
      )}
    >
      <span className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-subtle text-fg-2">
        {icon}
      </span>
      <span className="min-w-0">
        <span className="block text-strong font-medium text-fg-1">{title}</span>
        <span className="mt-0.5 block text-caption leading-5 text-fg-3">
          {description}
        </span>
      </span>
    </button>
  );
}

/**
 * 新增供应商向导：先选来源（预设 / 导入本机登录态 / 自定义地址），再进同一套表单。
 *
 * 表单字段与「编辑供应商」一致（名称、服务根、协议、密钥、接入应用），创建只是多了
 * 「从哪里开始」这一步；是否接入 app 是可选的，不勾就只建供应商本体。
 */
export function CreateSupplierDialog({
  open,
  onClose,
}: CreateSupplierDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [step, setStep] = useState<Step>("choose");
  const [search, setSearch] = useState("");
  const [preset, setPreset] = useState<SupplierPreset | null>(null);
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [protocol, setProtocol] = useState<SupplierPresetProtocol>("anthropic");
  // 新建时密钥永远是空的：这里没有旧密钥可显示，后端也不会回传明文
  const [secret, setSecret] = useState("");
  const [revealed, setRevealed] = useState(false);
  // 默认不勾：创建供应商与接入 app 解耦
  const [selectedApps, setSelectedApps] = useState<string[]>([]);
  const [errors, setErrors] = useState<CreateSupplierErrors>({});
  const [submitting, setSubmitting] = useState(false);
  const [importing, setImporting] = useState<ManagedAuthProvider | null>(null);

  // 每次打开都是干净的一份：上一轮的输入、勾选、密钥、报错一律不带走
  useLayoutEffect(() => {
    if (!open) return;
    setStep("choose");
    setSearch("");
    setPreset(null);
    setName("");
    setBaseUrl("");
    setProtocol("anthropic");
    setSecret("");
    setRevealed(false);
    setSelectedApps([]);
    setErrors({});
    setSubmitting(false);
    setImporting(null);
  }, [open]);

  const importStatus = useQuery({
    queryKey: ["managed-auth-status", "create-import"],
    queryFn: async () => {
      const statuses = await Promise.all(
        IMPORTABLE_PROVIDERS.map(async (target) => {
          try {
            const status = await authApi.authGetStatus(target.provider);
            return { ...target, accounts: status.accounts.length };
          } catch {
            // 没登录过 / 该 provider 不支持：当作没检测到，不挡向导
            return { ...target, accounts: 0 };
          }
        }),
      );
      return statuses;
    },
    enabled: open,
    staleTime: 30_000,
  });

  const detected = useMemo(
    () => (importStatus.data ?? []).filter((target) => target.accounts > 0),
    [importStatus.data],
  );

  const visiblePresets = useMemo(() => searchSupplierPresets(search), [search]);

  const requestClose = () => {
    if (submitting || importing) return;
    onClose();
  };

  const toggleApp = (app: string) => {
    setSelectedApps((previous) =>
      previous.includes(app)
        ? previous.filter((current) => current !== app)
        : [...previous, app],
    );
  };

  const goForm = (next: Partial<SupplierPreset> = {}) => {
    if (next.name) setName(next.name);
    if (next.baseUrl) setBaseUrl(next.baseUrl);
    if (next.protocol) setProtocol(next.protocol);
    setStep("form");
  };

  const pickPreset = (next: SupplierPreset) => {
    setPreset(next);
    goForm(next);
  };

  const pickCustom = () => {
    setPreset(null);
    goForm();
  };

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (submitting) return;

    const trimmedName = name.trim();
    const trimmedBaseUrl = baseUrl.trim();
    const trimmedSecret = secret.trim();
    const apps =
      protocol === "anthropic"
        ? CREATABLE_APPS.filter((app) => selectedApps.includes(app))
        : [];

    const nextErrors: CreateSupplierErrors = {};
    if (!trimmedName) nextErrors.name = "suppliers.createNameRequired";
    if (!trimmedBaseUrl) nextErrors.baseUrl = "suppliers.createBaseUrlRequired";
    else if (!isCompleteHttpUrl(trimmedBaseUrl)) {
      nextErrors.baseUrl = "suppliers.createBaseUrlInvalid";
    }
    if (!trimmedSecret) nextErrors.secret = "suppliers.createSecretRequired";

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
        protocol,
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

  const importAgent = async (target: (typeof IMPORTABLE_PROVIDERS)[number]) => {
    if (importing) return;
    setImporting(target.provider);
    try {
      const updated = await suppliersApi.ensureManaged(target.provider);
      queryClient.setQueryData<Supplier[]>(suppliersKeys.overview, updated);
      toast.success(t("suppliers.createImportSuccess"));
      onClose();
    } catch (importError) {
      toast.error(
        extractErrorMessage(importError) || t("suppliers.createImportFailed"),
      );
    } finally {
      setImporting(null);
    }
  };

  const busy = submitting || importing !== null;
  const showApps = protocol === "anthropic";

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => {
        if (!nextOpen) requestClose();
      }}
    >
      <DialogContent
        // WebKit 下 h-fit + flex-1 会把弹窗主体压成 0，只剩标题条；
        // 给弹窗确定高度，各步骤的 flex 滚动区才会正常分配。
        style={{ height: "min(860px, calc(100vh - 48px))" }}
        className="max-w-[820px] gap-0 overflow-hidden p-0"
        onInteractOutside={(event) => {
          // 遮罩点击由这里统一处理：提交中不关，其余情况交给 requestClose
          event.preventDefault();
          if (!busy) requestClose();
        }}
        onEscapeKeyDown={(event) => {
          // 提交中忽略 Escape；非提交中让 Radix 走 onOpenChange → requestClose
          if (busy) event.preventDefault();
        }}
      >
        <div className="flex shrink-0 items-start justify-between gap-4 border-b border-border px-6 py-4">
          <div className="flex min-w-0 items-start gap-2">
            {step !== "choose" ? (
              <Button
                type="button"
                variant="quiet"
                size="compact"
                className="-ml-2 mt-0.5"
                aria-label={t("suppliers.createBack")}
                title={t("suppliers.createBack")}
                disabled={busy}
                onClick={() => setStep("choose")}
              >
                <ArrowLeft aria-hidden="true" className="h-4 w-4" />
              </Button>
            ) : null}
            <div className="min-w-0">
              <DialogTitle>{t("suppliers.createTitle")}</DialogTitle>
              <DialogDescription className="mt-0.5">
                {step === "choose"
                  ? t("suppliers.createChooseDescription")
                  : t("suppliers.createDescription")}
              </DialogDescription>
            </div>
          </div>
          <DialogClose asChild>
            <Button
              type="button"
              variant="quiet"
              size="compact"
              aria-label={t("common.close")}
              title={t("common.close")}
              disabled={busy}
            >
              <X aria-hidden="true" className="h-4 w-4" />
            </Button>
          </DialogClose>
        </div>

        {step === "choose" ? (
          <div className="min-h-0 flex-1 space-y-2 overflow-y-auto px-6 py-5">
            <SourceCard
              icon={<Sparkles aria-hidden="true" className="h-4 w-4" />}
              title={t("suppliers.createSourcePreset")}
              description={t("suppliers.createSourcePresetHint")}
              onClick={() => setStep("preset")}
            />
            <SourceCard
              icon={<ShieldCheck aria-hidden="true" className="h-4 w-4" />}
              title={t("suppliers.createSourceImport")}
              description={t("suppliers.createSourceImportHint")}
              onClick={() => setStep("import")}
            />
            <SourceCard
              icon={<PlusIcon />}
              title={t("suppliers.createSourceCustom")}
              description={t("suppliers.createSourceCustomHint")}
              onClick={pickCustom}
            />
          </div>
        ) : null}

        {step === "preset" ? (
          <div className="flex min-h-0 flex-1 flex-col">
            <div className="shrink-0 px-6 pt-4">
              <div className="relative">
                <Search
                  aria-hidden="true"
                  className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-fg-3"
                />
                <Input
                  className="pl-8"
                  autoFocus
                  value={search}
                  placeholder={t("suppliers.createPresetSearch")}
                  aria-label={t("suppliers.createPresetSearch")}
                  onChange={(event) => setSearch(event.target.value)}
                />
              </div>
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto px-6 py-3">
              {visiblePresets.length === 0 ? (
                <p className="m-0 py-6 text-center text-caption text-fg-3">
                  {t("suppliers.createPresetEmpty")}
                </p>
              ) : (
                <ul className="m-0 list-none divide-y divide-border p-0">
                  {visiblePresets.map((item) => (
                    <li key={item.id}>
                      <button
                        type="button"
                        className="flex w-full items-center justify-between gap-3 rounded-md px-2 py-2.5 text-left hover:bg-subtle focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                        onClick={() => pickPreset(item)}
                      >
                        <span className="min-w-0">
                          <span className="block truncate text-fg-1">
                            {item.name}
                          </span>
                          <span className="block truncate font-mono text-caption text-fg-3">
                            {item.baseUrl}
                          </span>
                        </span>
                        <span className="shrink-0 rounded-sm bg-subtle px-1.5 py-0.5 font-mono text-caption text-fg-3">
                          {item.protocol}
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        ) : null}

        {step === "import" ? (
          <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
            {importStatus.isLoading ? (
              <p className="m-0 py-6 text-center text-caption text-fg-3">
                {t("suppliers.createImportLoading")}
              </p>
            ) : detected.length === 0 ? (
              <div className="space-y-1 py-6 text-center">
                <p className="m-0 text-caption text-fg-3">
                  {t("suppliers.createImportEmpty")}
                </p>
                <p className="m-0 text-caption text-fg-3">
                  {t("suppliers.createImportEmptyHint")}
                </p>
              </div>
            ) : (
              <ul className="m-0 list-none space-y-2 p-0">
                {detected.map((target) => (
                  <li
                    key={target.provider}
                    className="flex items-center justify-between gap-3 rounded-lg border border-border px-4 py-3"
                  >
                    <div className="min-w-0">
                      <p className="m-0 text-strong font-medium text-fg-1">
                        {t(`suppliers.createImportName.${target.provider}`)}
                      </p>
                      <p className="m-0 text-caption text-fg-3">
                        {t("suppliers.createImportAccounts", {
                          count: target.accounts,
                        })}
                      </p>
                    </div>
                    <Button
                      type="button"
                      variant="solid"
                      size="compact"
                      disabled={busy}
                      aria-busy={importing === target.provider}
                      onClick={() => void importAgent(target)}
                    >
                      {importing === target.provider ? (
                        <Loader2
                          aria-hidden="true"
                          className="h-4 w-4 animate-spin"
                        />
                      ) : null}
                      {t("suppliers.createImportAction")}
                    </Button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        ) : null}

        {step === "form" ? (
          <form
            className="flex min-h-0 flex-1 flex-col"
            onSubmit={(event) => void submit(event)}
          >
            <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
              <div className="grid gap-6 md:grid-cols-[minmax(0,1fr)_280px]">
                <div className="min-w-0 space-y-6">
                  <CreateSection
                    title={t("suppliers.createIdentityTitle")}
                    description={t("suppliers.createIdentityDescription")}
                  >
                    <div className="flex flex-col gap-1.5">
                      <label
                        htmlFor="create-supplier-name"
                        className={LABEL_CLASS}
                      >
                        {t("suppliers.createNameLabel")}
                      </label>
                      <Input
                        id="create-supplier-name"
                        autoFocus
                        value={name}
                        disabled={busy}
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
                      <label
                        htmlFor="create-supplier-base-url"
                        className={LABEL_CLASS}
                      >
                        {t("suppliers.createBaseUrlLabel")}
                      </label>
                      <Input
                        id="create-supplier-base-url"
                        className="font-mono"
                        value={baseUrl}
                        disabled={busy}
                        placeholder={t("suppliers.createBaseUrlPlaceholder")}
                        aria-invalid={errors.baseUrl ? true : undefined}
                        aria-describedby={
                          errors.baseUrl
                            ? "create-supplier-base-url-error"
                            : undefined
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
                      <p className="m-0 text-caption text-fg-3">
                        {t("suppliers.createBaseUrlHint")}
                      </p>
                      {errors.baseUrl && (
                        <FieldError id="create-supplier-base-url-error">
                          {t(errors.baseUrl)}
                        </FieldError>
                      )}
                    </div>

                    <div className="flex flex-col gap-1.5">
                      <label
                        htmlFor="create-supplier-protocol"
                        className={LABEL_CLASS}
                      >
                        {t("suppliers.createProtocolLabel")}
                      </label>
                      <select
                        id="create-supplier-protocol"
                        className="h-9 w-full rounded-md border border-border bg-surface px-2.5 text-caption text-fg-1"
                        value={protocol}
                        disabled={busy}
                        onChange={(event) =>
                          setProtocol(
                            event.target.value as SupplierPresetProtocol,
                          )
                        }
                      >
                        {PROTOCOLS.map((value) => (
                          <option key={value} value={value}>
                            {value}
                          </option>
                        ))}
                      </select>
                      {preset ? (
                        <p className="m-0 text-caption text-fg-3">
                          {t("suppliers.createPresetApplied", {
                            name: preset.name,
                          })}
                        </p>
                      ) : null}
                    </div>
                  </CreateSection>

                  <div className="h-px bg-border" />

                  <CreateSection
                    title={t("suppliers.createSecretLabel")}
                    description={t("suppliers.createSecretHint")}
                  >
                    <div className="relative">
                      <Input
                        id="create-supplier-secret"
                        type={revealed ? "text" : "password"}
                        className="pr-10"
                        value={secret}
                        disabled={busy}
                        placeholder={t("suppliers.createSecretPlaceholder")}
                        aria-label={t("suppliers.createSecretLabel")}
                        aria-invalid={errors.secret ? true : undefined}
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
                      {secret ? (
                        <button
                          type="button"
                          className="absolute right-2 top-1/2 -translate-y-1/2 rounded-sm p-1 text-fg-3 hover:text-fg-1"
                          aria-label={t(
                            revealed ? "apiKeyInput.hide" : "apiKeyInput.show",
                          )}
                          onClick={() => setRevealed((previous) => !previous)}
                        >
                          {revealed ? (
                            <EyeOff aria-hidden="true" className="h-4 w-4" />
                          ) : (
                            <Eye aria-hidden="true" className="h-4 w-4" />
                          )}
                        </button>
                      ) : null}
                    </div>
                    {errors.secret && (
                      <FieldError>{t(errors.secret)}</FieldError>
                    )}
                  </CreateSection>

                  <div className="h-px bg-border" />

                  <CreateSection
                    title={t("suppliers.createAppsLabel")}
                    description={
                      showApps
                        ? t("suppliers.createAppsHint")
                        : t("suppliers.createAppsProtocolHint", { protocol })
                    }
                  >
                    {showApps ? (
                      <div className="grid gap-2 sm:grid-cols-2">
                        {CREATABLE_APPS.map((app) => (
                          <label
                            key={app}
                            className="flex cursor-pointer items-center gap-2 rounded-md border border-border px-3 py-2 text-caption text-fg-1"
                          >
                            <Checkbox
                              checked={selectedApps.includes(app)}
                              disabled={busy}
                              aria-label={getAppLabel(app)}
                              onCheckedChange={() => toggleApp(app)}
                            />
                            <span>
                              {getAppLabel(app)}
                              <span className="block text-caption text-fg-3">
                                {t("suppliers.createAppProtocol", {
                                  app: getAppLabel(app),
                                })}
                              </span>
                            </span>
                          </label>
                        ))}
                      </div>
                    ) : (
                      <p className="m-0 text-caption text-fg-3">
                        {t("suppliers.createAppsLater")}
                      </p>
                    )}
                  </CreateSection>
                </div>

                <aside className="space-y-3 rounded-lg bg-subtle p-4">
                  <p className="m-0 text-caption font-medium text-fg-2">
                    {t("suppliers.createPreviewTitle")}
                  </p>
                  <dl className="m-0 space-y-2 text-caption">
                    <div>
                      <dt className="m-0 text-fg-3">
                        {t("suppliers.createPreviewName")}
                      </dt>
                      <dd className="m-0 break-words text-fg-1">
                        {name.trim() || t("suppliers.createPreviewEmpty")}
                      </dd>
                    </div>
                    <div>
                      <dt className="m-0 text-fg-3">
                        {t("suppliers.createPreviewUrl")}
                      </dt>
                      <dd className="m-0 break-all font-mono text-fg-1">
                        {baseUrl.trim() || t("suppliers.createPreviewEmpty")}
                      </dd>
                    </div>
                    <div>
                      <dt className="m-0 text-fg-3">
                        {t("suppliers.createPreviewProtocol")}
                      </dt>
                      <dd className="m-0 font-mono text-fg-1">{protocol}</dd>
                    </div>
                    <div>
                      <dt className="m-0 text-fg-3">
                        {t("suppliers.createPreviewApps")}
                      </dt>
                      <dd className="m-0 text-fg-1">
                        {showApps && selectedApps.length > 0
                          ? selectedApps.map(getAppLabel).join("、")
                          : t("suppliers.createPreviewEmpty")}
                      </dd>
                    </div>
                  </dl>
                  <p className="m-0 text-caption leading-5 text-fg-3">
                    {t("suppliers.createAfterHint")}
                  </p>
                </aside>
              </div>
            </div>

            <DialogFooter className="shrink-0 border-t border-border px-6 py-3">
              <Button
                type="button"
                variant="quiet"
                size="regular"
                disabled={busy}
                onClick={requestClose}
              >
                {t("common.cancel")}
              </Button>
              <Button
                type="submit"
                variant="solid"
                size="regular"
                disabled={busy}
                aria-busy={submitting}
              >
                {submitting && (
                  <Loader2
                    aria-hidden="true"
                    className="h-4 w-4 animate-spin"
                  />
                )}
                {t("suppliers.createSubmit")}
              </Button>
            </DialogFooter>
          </form>
        ) : null}

        {step === "choose" || step === "preset" || step === "import" ? (
          <DialogFooter className="shrink-0 border-t border-border px-6 py-3">
            <Button
              type="button"
              variant="quiet"
              size="regular"
              disabled={busy}
              onClick={requestClose}
            >
              {t("common.cancel")}
            </Button>
          </DialogFooter>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

function PlusIcon() {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      className="h-4 w-4"
    >
      <path d="M8 3v10M3 8h10" />
    </svg>
  );
}

export default CreateSupplierDialog;
