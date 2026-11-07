import {
  useEffect,
  useMemo,
  useState,
  type FormEvent,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ChevronDown,
  ChevronRight,
  CircleAlert,
  Eye,
  EyeOff,
  Loader2,
  Plus,
  RefreshCw,
  Search,
  ShieldCheck,
  Trash2,
  X,
} from "lucide-react";

import type { Supplier, SupplierCredential } from "@/lib/api/suppliers";
import { suppliersApi } from "@/lib/api/suppliers";
import { gatewayApi } from "@/lib/api/gateway";
import {
  checkProviderModels,
  type ModelCheckResult,
} from "@/lib/api/connectivity-check";
import type { AppId } from "@/lib/api/types";
import { suppliersKeys } from "@/lib/query/suppliers";
import { getAppLabel } from "@/config/appConfig";
import { toast } from "@/lib/toast";
import { extractErrorMessage } from "@/utils/errorUtils";
import { cn } from "@/lib/utils";
import { FieldError, LABEL_CLASS } from "@/components/mcp/formBits";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import type { GatewayCatalogEntry, ModelOverride } from "@/types/gateway";
import { SupplierManagedAccounts } from "./SupplierManagedAccounts";
import { isCompleteHttpUrl } from "./supplierForm";
import { useSupplierAccountSwitch } from "./useSupplierAccountSwitch";
import { baseUrlTargets, type ProviderGroup } from "./providerGrouping";

type CredentialMode = "single" | "pool";

interface EditErrors {
  name?: string;
  baseUrl?: string;
  secret?: string;
}

export interface SupplierEditDialogProps {
  group: ProviderGroup | null;
  onClose: () => void;
  /** 凭据/模型等即时操作返回的新总览，交给页面刷新缓存与卡片 */
  onOverview: (suppliers: Supplier[]) => void;
}

function SectionHeader({
  title,
  description,
  aside,
}: {
  title: string;
  description?: string;
  aside?: ReactNode;
}) {
  return (
    <div className="flex min-w-0 items-start justify-between gap-3">
      <div className="min-w-0">
        <h3 className="m-0 text-strong font-semibold text-fg-1">{title}</h3>
        {description ? (
          <p className="m-0 mt-0.5 text-caption leading-5 text-fg-3">
            {description}
          </p>
        ) : null}
      </div>
      {aside ? <div className="shrink-0">{aside}</div> : null}
    </div>
  );
}

function MaskedKey({ value }: { value: string }) {
  return (
    <code className="rounded-sm bg-subtle px-1.5 py-0.5 font-mono text-caption text-fg-2">
      {value || "••••"}
    </code>
  );
}

/**
 * 编辑供应商：名称、地址、凭据、模型、连通性与端点诊断。
 *
 * 凭据的增删改是即时操作（每把 Key 是独立接入），名称/地址/单 Key 的统一替换在
 * 底部「保存」时提交。所有操作失败都留在弹窗里，输入不丢。
 */
export function SupplierEditDialog({
  group,
  onClose,
  onOverview,
}: SupplierEditDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [mode, setMode] = useState<CredentialMode>("single");
  const [singleSecret, setSingleSecret] = useState("");
  const [singleRevealed, setSingleRevealed] = useState(false);
  const [errors, setErrors] = useState<EditErrors>({});
  const [saving, setSaving] = useState(false);
  const [confirmClose, setConfirmClose] = useState(false);

  const literalCredentials = useMemo(() => {
    const fromBackend = (group?.credentials ?? []).filter(
      (credential) => credential.secretKind === "literal",
    );
    if (fromBackend.length > 0) return fromBackend;
    // 兼容没有 credentials 明细的旧数据 / 测试 fixture：只用指纹合成占位行
    if (!group?.credentialKinds.includes("literal")) return [];
    return (group.keyHashes ?? []).map((keyHash) => ({
      keyHash,
      maskedSecret: "••••",
      secretKind: "literal",
      envName: null,
      apps: [],
    }));
  }, [group?.credentials, group?.credentialKinds, group?.keyHashes]);
  const envCredentials = useMemo(
    () =>
      (group?.credentials ?? []).filter(
        (credential) => credential.secretKind === "env_ref",
      ),
    [group?.credentials],
  );

  // 打开一张卡时按凭据数量选默认模式；卡片刷新（加/删 Key）不重置用户的选择
  useEffect(() => {
    if (!group) return;
    setName(group.name);
    setBaseUrl(group.primary.baseUrl);
    setMode(literalCredentials.length > 1 ? "pool" : "single");
    setSingleSecret("");
    setSingleRevealed(false);
    setErrors({});
    setSaving(false);
    setConfirmClose(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [group?.id]);

  const urlTargets = group ? baseUrlTargets(group.suppliers) : [];
  const canEditUrl = urlTargets.length > 0;
  const canEditSingleKey =
    !group?.managedAuthProvider && literalCredentials.length > 0;

  const dirty =
    Boolean(group) &&
    (name.trim() !== group!.name ||
      (canEditUrl && baseUrl.trim() !== group!.primary.baseUrl.trim()) ||
      singleSecret.trim() !== "");

  const requestClose = () => {
    if (saving) return;
    if (dirty) {
      setConfirmClose(true);
      return;
    }
    onClose();
  };

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!group || saving) return;

    const trimmedName = name.trim();
    const trimmedUrl = baseUrl.trim();
    const trimmedSecret = singleSecret.trim();
    const nextErrors: EditErrors = {};

    if (!trimmedName) nextErrors.name = "suppliers.nameRequired";
    if (canEditUrl) {
      if (!trimmedUrl) nextErrors.baseUrl = "suppliers.baseUrlRequired";
      else if (!isCompleteHttpUrl(trimmedUrl)) {
        nextErrors.baseUrl = "suppliers.createBaseUrlInvalid";
      }
    }
    if (trimmedSecret && !canEditSingleKey) {
      nextErrors.secret = "suppliers.secretUnavailable";
    }
    if (Object.keys(nextErrors).length > 0) {
      setErrors(nextErrors);
      return;
    }
    setErrors({});

    const nameChanged = trimmedName !== group.name;
    const urlChanged =
      canEditUrl && trimmedUrl !== group.primary.baseUrl.trim();
    const secretChanged = trimmedSecret !== "" && canEditSingleKey;
    if (!nameChanged && !urlChanged && !secretChanged) {
      onClose();
      return;
    }

    setSaving(true);
    let updated: Supplier[] | null = null;
    let lastError: unknown = null;
    try {
      if (nameChanged) {
        const seenUpstreams = new Set<string>();
        for (const supplier of group.suppliers) {
          const upstreamId = supplier.upstreamIds?.[0];
          if (upstreamId) {
            if (seenUpstreams.has(upstreamId)) continue;
            seenUpstreams.add(upstreamId);
          } else if (seenUpstreams.size > 0) {
            continue;
          }
          updated = await suppliersApi.updateName(supplier.id, trimmedName);
        }
      }

      if (urlChanged) {
        for (const supplier of urlTargets) {
          updated = await suppliersApi.updateBaseUrl(supplier.id, trimmedUrl);
        }
      }

      if (secretChanged) {
        if (mode === "single") {
          // 单 Key 模式 = 把这家供应商的密钥统一成一把
          for (const supplier of group.suppliers) {
            if (!supplier.credentialKinds.includes("literal")) continue;
            updated = await suppliersApi.updateSecret(
              supplier.id,
              trimmedSecret,
            );
          }
        } else {
          // 池模式 = 只替换选中的那把 Key
          for (const credential of literalCredentials) {
            const owner =
              group.suppliers.find((supplier) =>
                supplier.credentials?.some(
                  (item) => item.keyHash === credential.keyHash,
                ),
              ) ?? group.primary;
            updated = await suppliersApi.updateCredential(
              owner.id,
              credential.keyHash,
              trimmedSecret,
            );
          }
        }
      }

      if (updated) {
        onOverview(updated);
        queryClient.setQueryData<Supplier[]>(suppliersKeys.overview, updated);
        void queryClient.invalidateQueries({ queryKey: ["providers"] });
        void queryClient.invalidateQueries({
          queryKey: ["gateway", "catalog"],
        });
        toast.success(t("suppliers.editSaved"));
        onClose();
      } else {
        toast.error(
          extractErrorMessage(lastError) || t("suppliers.editFailed"),
        );
      }
    } catch (error) {
      toast.error(extractErrorMessage(error) || t("suppliers.editFailed"));
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <Dialog
        open={group !== null}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) requestClose();
        }}
      >
        <DialogContent
          className="max-h-[min(860px,calc(100vh-48px))] max-w-[820px] gap-0 overflow-hidden p-0"
          onInteractOutside={(event) => {
            event.preventDefault();
            requestClose();
          }}
          onEscapeKeyDown={(event) => {
            event.preventDefault();
            requestClose();
          }}
        >
          <div className="flex shrink-0 items-start justify-between gap-4 border-b border-border px-6 py-4">
            <div className="min-w-0">
              <DialogTitle>{t("suppliers.editTitle")}</DialogTitle>
              <DialogDescription className="mt-0.5">
                {t("suppliers.editDescription")}
              </DialogDescription>
            </div>
            <DialogClose asChild>
              <Button
                type="button"
                variant="quiet"
                size="compact"
                aria-label={t("common.close")}
                title={t("common.close")}
                disabled={saving}
              >
                <X aria-hidden="true" className="h-4 w-4" />
              </Button>
            </DialogClose>
          </div>

          <form
            className="flex min-h-0 flex-1 flex-col"
            onSubmit={(event) => void submit(event)}
          >
            <div className="min-h-0 flex-1 space-y-6 overflow-y-auto px-6 py-5">
              {/* 身份与地址 */}
              <section className="space-y-3">
                <SectionHeader
                  title={t("suppliers.identityTitle")}
                  description={t("suppliers.identityDescription")}
                  aside={
                    <span className="rounded-full border border-border px-2 py-0.5 text-badge text-fg-2">
                      {group?.primary.upstreamProtocol === "unknown"
                        ? t("suppliers.protocolUnknown")
                        : group?.primary.upstreamProtocol}
                    </span>
                  }
                />

                <div className="grid gap-3 sm:grid-cols-2">
                  <div className="flex flex-col gap-1.5">
                    <label htmlFor="supplier-edit-name" className={LABEL_CLASS}>
                      {t("suppliers.nameLabel")}
                    </label>
                    <Input
                      id="supplier-edit-name"
                      value={name}
                      disabled={saving}
                      aria-invalid={errors.name ? true : undefined}
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
                    {errors.name && <FieldError>{t(errors.name)}</FieldError>}
                  </div>

                  <div className="flex flex-col gap-1.5">
                    <label htmlFor="supplier-edit-url" className={LABEL_CLASS}>
                      {t("suppliers.baseUrlLabel")}
                    </label>
                    {canEditUrl ? (
                      <>
                        <Input
                          id="supplier-edit-url"
                          className="font-mono"
                          value={baseUrl}
                          disabled={saving}
                          aria-invalid={errors.baseUrl ? true : undefined}
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
                          <FieldError>{t(errors.baseUrl)}</FieldError>
                        )}
                      </>
                    ) : (
                      <div className="flex h-9 min-w-0 items-center rounded-md border border-border bg-subtle px-3">
                        <code className="min-w-0 truncate font-mono text-caption text-fg-2">
                          {group?.primary.baseUrl ||
                            t("suppliers.noBaseUrlKey")}
                        </code>
                      </div>
                    )}
                  </div>
                </div>

                <p className="m-0 text-caption text-fg-3">
                  {canEditUrl
                    ? t("suppliers.editUrlHint")
                    : t("suppliers.editManagedAddressHint")}
                </p>
              </section>

              <div className="h-px bg-border" />

              <CredentialSection
                group={group}
                mode={mode}
                onModeChange={setMode}
                singleSecret={singleSecret}
                onSingleSecretChange={setSingleSecret}
                singleRevealed={singleRevealed}
                onSingleRevealedChange={setSingleRevealed}
                literalCredentials={literalCredentials}
                envCredentials={envCredentials}
                onOverview={onOverview}
                saving={saving}
              />

              <div className="h-px bg-border" />

              <ModelSection group={group} />

              <div className="h-px bg-border" />

              <ConnectionSection group={group} />

              <div className="h-px bg-border" />

              <AdvancedSection group={group} />
            </div>

            <DialogFooter className="shrink-0 border-t border-border px-6 py-3">
              <Button
                type="button"
                variant="quiet"
                size="regular"
                disabled={saving}
                onClick={requestClose}
              >
                {t("common.cancel")}
              </Button>
              <Button
                type="submit"
                variant="solid"
                size="regular"
                disabled={saving}
                aria-busy={saving}
              >
                {saving && (
                  <Loader2
                    aria-hidden="true"
                    className="h-4 w-4 animate-spin"
                  />
                )}
                {t("common.save")}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <ConfirmDialog
        isOpen={confirmClose}
        title={t("suppliers.discardTitle")}
        message={t("suppliers.discardMessage")}
        confirmText={t("suppliers.discardConfirm")}
        cancelText={t("common.cancel")}
        variant="destructive"
        onConfirm={() => {
          setConfirmClose(false);
          onClose();
        }}
        onCancel={() => setConfirmClose(false)}
      />
    </>
  );
}

/* ------------------------------- 凭据 ------------------------------- */

function CredentialSection({
  group,
  mode,
  onModeChange,
  singleSecret,
  onSingleSecretChange,
  singleRevealed,
  onSingleRevealedChange,
  literalCredentials,
  envCredentials,
  onOverview,
  saving,
}: {
  group: ProviderGroup | null;
  mode: CredentialMode;
  onModeChange: (mode: CredentialMode) => void;
  singleSecret: string;
  onSingleSecretChange: (value: string) => void;
  singleRevealed: boolean;
  onSingleRevealedChange: (value: boolean) => void;
  literalCredentials: SupplierCredential[];
  envCredentials: SupplierCredential[];
  onOverview: (suppliers: Supplier[]) => void;
  saving: boolean;
}) {
  const { t } = useTranslation();
  const [editingHash, setEditingHash] = useState<string | null>(null);
  const [editSecret, setEditSecret] = useState("");
  const [revealedHash, setRevealedHash] = useState<string | null>(null);
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [addApp, setAddApp] = useState("");
  const [addSecret, setAddSecret] = useState("");
  const [addRevealed, setAddRevealed] = useState(false);
  const [deleteHash, setDeleteHash] = useState<string | null>(null);
  const managedProvider = group?.managedAuthProvider ?? null;
  // 换账号与卡片共用同一套行为：供应商级切换，两个 app 一起变
  const {
    switchTo: switchManagedAccount,
    switchingGroupId,
    toggleEnabled: toggleManagedAccount,
    togglingGroupId,
    accountEnabledOf,
  } = useSupplierAccountSwitch(onOverview);

  const keyApps = useMemo(() => {
    if (!group) return [];
    return group.apps.filter((app) =>
      literalCredentials.some((credential) =>
        credential.apps.some((item) => item.appType === app),
      ),
    );
  }, [group, literalCredentials]);

  useEffect(() => {
    setEditingHash(null);
    setEditSecret("");
    setBusyKey(null);
    setAddSecret("");
    setAddApp(keyApps[0] ?? "");
    // 只在切换卡片时重置；凭据刷新不要清掉正在输入的内容
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [group?.id]);

  const ownerOf = (keyHash: string) =>
    group?.suppliers.find((supplier) =>
      supplier.credentials?.some((item) => item.keyHash === keyHash),
    ) ?? group?.primary;

  const run = async (key: string, action: () => Promise<Supplier[]>) => {
    if (busyKey) return;
    setBusyKey(key);
    try {
      const updated = await action();
      onOverview(updated);
      return true;
    } catch (error) {
      toast.error(extractErrorMessage(error) || t("suppliers.editFailed"));
      return false;
    } finally {
      setBusyKey(null);
    }
  };

  const replaceKey = async (credential: SupplierCredential) => {
    const secret = editSecret.trim();
    if (!secret || !group) return;
    const owner = ownerOf(credential.keyHash);
    if (!owner) return;
    const ok = await run(credential.keyHash, () =>
      suppliersApi.updateCredential(owner.id, credential.keyHash, secret),
    );
    if (ok) {
      setEditingHash(null);
      setEditSecret("");
      toast.success(t("suppliers.credentialSaved"));
    }
  };

  const deleteKey = async (credential: SupplierCredential) => {
    if (!group) return;
    const owner = ownerOf(credential.keyHash);
    if (!owner) return;
    const ok = await run(credential.keyHash, () =>
      suppliersApi.deleteCredential(owner.id, credential.keyHash),
    );
    if (ok) {
      setDeleteHash(null);
      toast.success(t("suppliers.credentialDeleted"));
    }
  };

  const toggleKey = async (
    credential: SupplierCredential,
    enabled: boolean,
  ) => {
    if (!group) return;
    const owner = ownerOf(credential.keyHash);
    if (!owner) return;
    await run(credential.keyHash, () =>
      suppliersApi.setCredentialEnabled(owner.id, credential.keyHash, enabled),
    );
  };

  const addKey = async () => {
    if (!group || !addApp || !addSecret.trim()) return;
    const owner =
      group.suppliers.find((supplier) =>
        supplier.bindings.some((binding) => binding.appType === addApp),
      ) ?? group.primary;
    const ok = await run("add", () =>
      suppliersApi.addCredential(owner.id, addApp, addSecret.trim()),
    );
    if (ok) {
      setAddSecret("");
      toast.success(t("suppliers.credentialAdded"));
    }
  };

  if (!group) {
    return null;
  }

  if (managedProvider) {
    // 托管供应商的凭据 = 账号池里的账号。这里和供应商卡片渲染同一个组件、同一套动作：
    // 列表是整池（不是「只显示已绑定的一两个」），切换 = 供应商级切换，
    // ⋯ 菜单里的「退出登录」= 从账号池删除。
    return (
      <section className="space-y-3">
        <SupplierManagedAccounts
          provider={managedProvider}
          headerLabel={t("suppliers.credentialsTitle")}
          onSetDefaultAccount={(accountId) =>
            void switchManagedAccount(group, accountId)
          }
          settingDefaultAccount={switchingGroupId === group.id}
          accountEnabled={(accountId) => accountEnabledOf(group, accountId)}
          onToggleAccountEnabled={(accountId, enabled) =>
            void toggleManagedAccount(group, accountId, enabled)
          }
          togglingAccountEnabled={togglingGroupId === group.id}
        />
      </section>
    );
  }

  return (
    <section className="space-y-3">
      <SectionHeader
        title={t("suppliers.credentialsTitle")}
        description={t("suppliers.credentialsDescription")}
        aside={
          <span className="text-caption text-fg-3">
            {t("suppliers.keyCount", { count: literalCredentials.length })}
          </span>
        }
      />

      <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
        <ModeCard
          selected={mode === "single"}
          title={t("suppliers.singleKeyTitle")}
          description={t("suppliers.singleKeyDescription")}
          onClick={() => onModeChange("single")}
        />
        <ModeCard
          selected={mode === "pool"}
          title={t("suppliers.poolTitle")}
          description={t("suppliers.poolDescription")}
          onClick={() => onModeChange("pool")}
        />
      </div>

      {mode === "single" ? (
        <div className="space-y-2 rounded-panel border border-border p-3">
          {literalCredentials.length === 0 ? (
            <p className="m-0 text-caption text-fg-3">
              {envCredentials.length > 0
                ? t("suppliers.editEnvHint")
                : t("suppliers.editNoSecretHint")}
            </p>
          ) : (
            <>
              {literalCredentials.length > 1 && (
                <Notice tone="warning">
                  {t("suppliers.secretMultiWarning", {
                    count: literalCredentials.length,
                  })}
                </Notice>
              )}
              <div className="flex flex-wrap items-center gap-2">
                <span className="text-caption text-fg-3">
                  {t("suppliers.currentKeys")}
                </span>
                {literalCredentials.map((credential) => (
                  <MaskedKey
                    key={credential.keyHash}
                    value={credential.maskedSecret}
                  />
                ))}
              </div>
              <div className="flex flex-col gap-1.5">
                <label htmlFor="supplier-single-secret" className={LABEL_CLASS}>
                  {t("suppliers.newSecretLabel")}
                </label>
                <div className="relative">
                  <Input
                    id="supplier-single-secret"
                    type={singleRevealed ? "text" : "password"}
                    value={singleSecret}
                    disabled={saving}
                    autoComplete="new-password"
                    className="pr-9 font-mono"
                    placeholder={t("suppliers.secretLeaveBlank")}
                    onChange={(event) =>
                      onSingleSecretChange(event.target.value)
                    }
                  />
                  <button
                    type="button"
                    className="absolute right-1 top-1/2 inline-flex h-7 w-7 -translate-y-1/2 items-center justify-center rounded-sm text-fg-3 hover:bg-subtle hover:text-fg-1"
                    aria-label={
                      singleRevealed
                        ? t("suppliers.hideSecret")
                        : t("suppliers.showSecret")
                    }
                    onClick={() => onSingleRevealedChange(!singleRevealed)}
                  >
                    {singleRevealed ? (
                      <EyeOff aria-hidden="true" className="h-3.5 w-3.5" />
                    ) : (
                      <Eye aria-hidden="true" className="h-3.5 w-3.5" />
                    )}
                  </button>
                </div>
                <p className="m-0 text-caption text-fg-3">
                  {t("suppliers.secretOnlyWrites")}
                </p>
              </div>
            </>
          )}
        </div>
      ) : (
        <div className="space-y-2">
          {literalCredentials.length === 0 ? (
            <div className="rounded-panel border border-dashed border-border px-4 py-4 text-center text-caption text-fg-3">
              {t("suppliers.poolEmpty")}
            </div>
          ) : (
            <ul className="m-0 list-none space-y-2 p-0">
              {literalCredentials.map((credential) => {
                const enabled = credential.apps.some(
                  (app) => app.inFailoverQueue,
                );
                const busy = busyKey === credential.keyHash;
                return (
                  <li
                    key={credential.keyHash}
                    className="rounded-panel border border-border p-3"
                  >
                    <div className="flex min-w-0 flex-wrap items-center gap-2">
                      <Switch
                        checked={enabled}
                        disabled={busy}
                        aria-label={t("suppliers.toggleCredential")}
                        onCheckedChange={(checked) =>
                          void toggleKey(credential, checked)
                        }
                      />
                      <MaskedKey value={credential.maskedSecret} />
                      <div className="flex min-w-0 flex-1 flex-wrap items-center gap-1">
                        {credential.apps.map((app) => (
                          <span
                            key={`${app.appType}:${app.providerId}`}
                            className="rounded-full bg-subtle px-1.5 py-0.5 text-badge text-fg-2"
                            title={app.providerName}
                          >
                            {getAppLabel(app.appType)}
                          </span>
                        ))}
                      </div>
                      <div className="flex shrink-0 items-center gap-1">
                        <Button
                          type="button"
                          variant="quiet"
                          size="compact"
                          disabled={busy}
                          onClick={() =>
                            setEditingHash(
                              editingHash === credential.keyHash
                                ? null
                                : credential.keyHash,
                            )
                          }
                        >
                          {t("suppliers.replaceKey")}
                        </Button>
                        <Button
                          type="button"
                          variant="quiet"
                          size="compact"
                          disabled={busy}
                          aria-label={t("suppliers.deleteKey")}
                          title={t("suppliers.deleteKey")}
                          onClick={() => setDeleteHash(credential.keyHash)}
                        >
                          <Trash2 aria-hidden="true" className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    </div>

                    {editingHash === credential.keyHash && (
                      <div className="mt-2 flex flex-col gap-2 border-t border-border pt-2 sm:flex-row sm:items-start">
                        <div className="relative min-w-0 flex-1">
                          <Input
                            autoFocus
                            type={
                              revealedHash === credential.keyHash
                                ? "text"
                                : "password"
                            }
                            value={editSecret}
                            className="pr-9 font-mono"
                            placeholder={t(
                              "suppliers.replaceSecretPlaceholder",
                            )}
                            onChange={(event) =>
                              setEditSecret(event.target.value)
                            }
                            onKeyDown={(event) => {
                              if (event.key === "Enter") {
                                event.preventDefault();
                                void replaceKey(credential);
                              }
                            }}
                          />
                          <button
                            type="button"
                            className="absolute right-1 top-1/2 inline-flex h-7 w-7 -translate-y-1/2 items-center justify-center rounded-sm text-fg-3 hover:bg-subtle hover:text-fg-1"
                            aria-label={t("suppliers.showSecret")}
                            onClick={() =>
                              setRevealedHash(
                                revealedHash === credential.keyHash
                                  ? null
                                  : credential.keyHash,
                              )
                            }
                          >
                            {revealedHash === credential.keyHash ? (
                              <EyeOff
                                aria-hidden="true"
                                className="h-3.5 w-3.5"
                              />
                            ) : (
                              <Eye aria-hidden="true" className="h-3.5 w-3.5" />
                            )}
                          </button>
                        </div>
                        <div className="flex shrink-0 items-center gap-2">
                          <Button
                            type="button"
                            variant="solid"
                            size="compact"
                            disabled={busy || !editSecret.trim()}
                            aria-busy={busy}
                            onClick={() => void replaceKey(credential)}
                          >
                            {busy && (
                              <Loader2
                                aria-hidden="true"
                                className="h-3.5 w-3.5 animate-spin"
                              />
                            )}
                            {t("suppliers.saveKey")}
                          </Button>
                          <Button
                            type="button"
                            variant="quiet"
                            size="compact"
                            onClick={() => {
                              setEditingHash(null);
                              setEditSecret("");
                            }}
                          >
                            {t("common.cancel")}
                          </Button>
                        </div>
                      </div>
                    )}

                    {deleteHash === credential.keyHash && (
                      <div className="mt-2 flex flex-wrap items-center justify-between gap-2 border-t border-border pt-2">
                        <p className="m-0 text-caption text-fg-2">
                          {t("suppliers.deleteKeyConfirm", {
                            apps: credential.apps
                              .map((app) => getAppLabel(app.appType))
                              .join("、"),
                          })}
                        </p>
                        <div className="flex items-center gap-2">
                          <Button
                            type="button"
                            variant="quiet"
                            size="compact"
                            disabled={busy}
                            onClick={() => setDeleteHash(null)}
                          >
                            {t("common.cancel")}
                          </Button>
                          <Button
                            type="button"
                            variant="solid"
                            size="compact"
                            disabled={busy}
                            aria-busy={busy}
                            onClick={() => void deleteKey(credential)}
                          >
                            {t("suppliers.deleteKey")}
                          </Button>
                        </div>
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          )}

          {envCredentials.length > 0 && (
            <div className="rounded-panel border border-border px-3 py-2 text-caption text-fg-3">
              {t("suppliers.envCredentials", {
                names: envCredentials
                  .map((credential) => credential.envName)
                  .filter(Boolean)
                  .join("、"),
              })}
            </div>
          )}

          <div className="rounded-panel border border-border p-3">
            <div className="flex flex-wrap items-end gap-2">
              <div className="flex min-w-[130px] flex-1 flex-col gap-1.5">
                <label htmlFor="supplier-add-app" className={LABEL_CLASS}>
                  {t("suppliers.addKeyAppLabel")}
                </label>
                <select
                  id="supplier-add-app"
                  className="h-9 rounded-md border border-border bg-surface px-2 text-body text-fg-1"
                  value={addApp}
                  disabled={busyKey !== null}
                  onChange={(event) => setAddApp(event.target.value)}
                >
                  {keyApps.map((app) => (
                    <option key={app} value={app}>
                      {getAppLabel(app)}
                    </option>
                  ))}
                </select>
              </div>
              <div className="flex min-w-[180px] flex-[2] flex-col gap-1.5">
                <label htmlFor="supplier-add-key" className={LABEL_CLASS}>
                  {t("suppliers.addKeyLabel")}
                </label>
                <div className="relative">
                  <Input
                    id="supplier-add-key"
                    type={addRevealed ? "text" : "password"}
                    value={addSecret}
                    className="pr-9 font-mono"
                    autoComplete="new-password"
                    placeholder={t("suppliers.secretLeaveBlank")}
                    disabled={busyKey !== null}
                    onChange={(event) => setAddSecret(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") {
                        event.preventDefault();
                        void addKey();
                      }
                    }}
                  />
                  <button
                    type="button"
                    className="absolute right-1 top-1/2 inline-flex h-7 w-7 -translate-y-1/2 items-center justify-center rounded-sm text-fg-3 hover:bg-subtle hover:text-fg-1"
                    aria-label={t("suppliers.showSecret")}
                    onClick={() => setAddRevealed((value) => !value)}
                  >
                    {addRevealed ? (
                      <EyeOff aria-hidden="true" className="h-3.5 w-3.5" />
                    ) : (
                      <Eye aria-hidden="true" className="h-3.5 w-3.5" />
                    )}
                  </button>
                </div>
              </div>
              <Button
                type="button"
                variant="neutral"
                size="regular"
                disabled={
                  busyKey !== null ||
                  !addSecret.trim() ||
                  !addApp ||
                  keyApps.length === 0
                }
                aria-busy={busyKey === "add"}
                onClick={() => void addKey()}
              >
                {busyKey === "add" ? (
                  <Loader2
                    aria-hidden="true"
                    className="h-4 w-4 animate-spin"
                  />
                ) : (
                  <Plus aria-hidden="true" className="h-4 w-4" />
                )}
                {t("suppliers.addKey")}
              </Button>
            </div>
            <p className="m-0 mt-2 text-caption text-fg-3">
              {keyApps.length === 0
                ? t("suppliers.addKeyNoApp")
                : t("suppliers.addKeyHint")}
            </p>
          </div>
        </div>
      )}
    </section>
  );
}

function ModeCard({
  selected,
  title,
  description,
  onClick,
}: {
  selected: boolean;
  title: string;
  description: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={title}
      aria-pressed={selected}
      onClick={onClick}
      className={cn(
        "flex min-h-[60px] flex-col items-start gap-0.5 rounded-panel border px-3 py-2 text-left transition-colors",
        selected
          ? "border-accent bg-accent/10"
          : "border-border hover:bg-subtle",
      )}
    >
      <span
        className={cn(
          "text-strong font-semibold",
          selected ? "text-accent-text" : "text-fg-1",
        )}
      >
        {title}
      </span>
      <span className="text-caption leading-5 text-fg-3">{description}</span>
    </button>
  );
}

function Notice({
  tone,
  children,
}: {
  tone: "warning" | "info";
  children: ReactNode;
}) {
  return (
    <div
      className={cn(
        "flex items-start gap-2 rounded-panel border px-3 py-2 text-caption leading-5",
        tone === "warning"
          ? "border-warning/40 bg-warning/10 text-fg-1"
          : "border-border bg-subtle text-fg-2",
      )}
    >
      <CircleAlert
        aria-hidden="true"
        className="mt-0.5 h-3.5 w-3.5 shrink-0 text-warning-text"
      />
      <span>{children}</span>
    </div>
  );
}

/* ------------------------------- 模型 ------------------------------- */

export const modelPickerKeys = {
  catalog: ["gateway", "catalog"] as const,
  /**
   * 含被隐藏条目的一份目录，只给这个弹窗用。
   *
   * 共享的 `catalog` 必须是「可见目录」—— AppConfigPage / AppEditorPage 的模型选择
   * 依赖它，混进隐藏条目会让那些地方冒出本该被隐藏的模型。子 key 依旧是
   * `catalog` 的前缀，所以 invalidateQueries({queryKey: catalog}) 两者一起刷新。
   */
  catalogWithHidden: ["gateway", "catalog", "withHidden"] as const,
  overrides: ["gateway", "modelOverrides"] as const,
};

function ModelSection({ group }: { group: ProviderGroup | null }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [expanded, setExpanded] = useState(true);
  const [sourceQuery, setSourceQuery] = useState("");
  const [addedQuery, setAddedQuery] = useState("");
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [drafts, setDrafts] = useState<Record<string, string>>({});

  const upstreamIds = group?.upstreamIds ?? [];
  const catalogQuery = useQuery({
    queryKey: modelPickerKeys.catalogWithHidden,
    // 左栏要能重新「加入」已隐藏的模型，所以这里必须带上隐藏条目。
    queryFn: () => gatewayApi.getCatalog({ includeHidden: true }),
    staleTime: 30_000,
  });
  const overridesQuery = useQuery({
    queryKey: modelPickerKeys.overrides,
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
  const enabled = (entry: GatewayCatalogEntry) =>
    !(overrides.get(keyOf(entry))?.hidden ?? false);

  /**
   * 同一个模型可能经多条上游进目录：Codex 订阅既挂 `chatgpt.com`，也挂官方托管上游，
   * 两条上游各回一份同样的模型名。对用户来说这是同一个模型，所以卡片里按 modelId 归并，
   * 否则同一行会既在左栏（这条上游还没启用）又在右栏（那条已启用）。
   */
  const entriesByModel = useMemo(() => {
    const map = new Map<string, GatewayCatalogEntry[]>();
    for (const entry of entries) {
      const list = map.get(entry.modelId);
      if (list) list.push(entry);
      else map.set(entry.modelId, [entry]);
    }
    return map;
  }, [entries]);

  /** 只要有一条上游启用了，这个模型就算启用（同一份订阅里的同一个模型）。 */
  const enabledModelIds = useMemo(() => {
    const ids = new Set<string>();
    for (const entry of entries) {
      if (enabled(entry)) ids.add(entry.modelId);
    }
    return ids;
  }, [entries, overrides]);

  /** 右栏 + 头部计数：每个模型只出现一次。 */
  const enabledEntries = useMemo(() => {
    const seen = new Set<string>();
    return entries.filter((entry) => {
      if (!enabled(entry) || seen.has(entry.modelId)) return false;
      seen.add(entry.modelId);
      return true;
    });
  }, [entries, overrides]);

  /**
   * 左栏 = 还没启用的候选，一个模型一行。
   *
   * 左栏不放已启用的模型：那一份在右栏，重复展示只会让人搞不清「这个模型到底启没启用」。
   * 点一下「加入」后它从左边消失、出现在右边，两栏数量互补。
   */
  const pendingEntries = useMemo(() => {
    const seen = new Set<string>();
    return entries.filter((entry) => {
      if (enabledModelIds.has(entry.modelId) || seen.has(entry.modelId)) {
        return false;
      }
      seen.add(entry.modelId);
      return true;
    });
  }, [entries, enabledModelIds]);

  /** 静默重读：改完人工覆盖（隐藏 / 改名）后刷新两份查询，不给提示。 */
  const reload = async () => {
    await queryClient.invalidateQueries({
      queryKey: modelPickerKeys.catalog,
    });
    await queryClient.invalidateQueries({
      queryKey: modelPickerKeys.overrides,
    });
  };

  /**
   * 用户点「刷新」：真去上游拉一遍模型列表并写库，再换掉查询缓存。
   *
   * 供应商就是上游：一张卡可能对应多条 upstream 行（同一家上游按 app 推导出来的），
   * 所以整组一起交给后端——后端按真实上游归并，一家只发一次请求，再写回它的每一行。
   * 新模型由后端写成 hidden（未启用），结果通常只体现在左栏；目录没变化时也要给
   * 可见反馈，否则这一步看起来像没反应。
   */
  const refresh = async () => {
    if (upstreamIds.length === 0) return;
    setRefreshing(true);
    try {
      const { fetched, added } =
        await gatewayApi.fetchSupplierModels(upstreamIds);
      setDrafts({});
      toast.success(
        added > 0
          ? t("suppliers.modelsFetchedAdded", {
              count: fetched,
              added,
              defaultValue:
                "已从上游获取 {{count}} 个模型，新增 {{added}} 个（未启用）",
            })
          : t("suppliers.modelsFetched", {
              count: fetched,
              defaultValue: "已从上游获取 {{count}} 个模型",
            }),
      );
    } catch (error) {
      toast.error(
        extractErrorMessage(error) ||
          t("suppliers.modelsFetchFailed", {
            defaultValue: "从上游获取模型失败",
          }),
      );
    } finally {
      // 中途失败时前面几条上游可能已经写库了，缓存必须照样重读，否则界面比库里旧。
      await queryClient.invalidateQueries({
        queryKey: modelPickerKeys.catalog,
      });
      await queryClient.invalidateQueries({
        queryKey: modelPickerKeys.overrides,
      });
      setRefreshing(false);
    }
  };

  const setHidden = async (entry: GatewayCatalogEntry, hidden: boolean) => {
    // 同一个模型可能对应多条上游条目：一起写，否则另一条上游会留下「未启用」的残影。
    const targets = entriesByModel.get(entry.modelId) ?? [entry];
    setBusyKey(entry.modelId);
    try {
      for (const target of targets) {
        await gatewayApi.setModelOverride({
          upstreamId: target.upstreamId,
          modelId: target.modelId,
          displayName: overrides.get(keyOf(target))?.displayName ?? null,
          hidden,
        });
      }
      await reload();
    } catch (error) {
      toast.error(
        t("suppliers.modelSaveFailed", {
          defaultValue: "更新失败：{{error}}",
          error: extractErrorMessage(error),
        }),
      );
    } finally {
      setBusyKey(null);
    }
  };

  const rename = async (entry: GatewayCatalogEntry) => {
    const key = entry.modelId;
    const displayName = drafts[key]?.trim() ?? "";
    const targets = entriesByModel.get(entry.modelId) ?? [entry];
    setBusyKey(key);
    try {
      for (const target of targets) {
        await gatewayApi.setModelOverride({
          upstreamId: target.upstreamId,
          modelId: target.modelId,
          displayName: displayName || null,
          hidden: overrides.get(keyOf(target))?.hidden ?? false,
        });
      }
      await reload();
      setDrafts((previous) => {
        const next = { ...previous };
        delete next[key];
        return next;
      });
    } catch (error) {
      toast.error(extractErrorMessage(error) || t("suppliers.modelSaveFailed"));
    } finally {
      setBusyKey(null);
    }
  };

  const sourceItems = pendingEntries.filter((entry) =>
    entry.modelId.toLowerCase().includes(sourceQuery.trim().toLowerCase()),
  );
  const addedItems = enabledEntries.filter((entry) =>
    entry.modelId.toLowerCase().includes(addedQuery.trim().toLowerCase()),
  );

  return (
    <section className="space-y-3">
      <button
        type="button"
        className="flex w-full items-center justify-between gap-3 text-left"
        onClick={() => setExpanded((value) => !value)}
        aria-expanded={expanded}
      >
        <SectionHeader
          title={t("suppliers.modelsTitle")}
          description={t("suppliers.modelsDescription")}
        />
        <span className="flex shrink-0 items-center gap-2 text-caption text-fg-3">
          {/* 头部计数是「对用户开放的模型数」：左栏现在含隐藏条目，不能拿它算。 */}
          <span>{enabledEntries.length}</span>
          <ChevronDown
            className={cn(
              "h-4 w-4 transition-transform",
              expanded && "rotate-180",
            )}
          />
        </span>
      </button>

      {expanded && (
        <>
          {upstreamIds.length === 0 ? (
            <p className="m-0 rounded-panel border border-dashed border-border px-4 py-4 text-center text-caption text-fg-3">
              {t("suppliers.noModels")}
            </p>
          ) : entries.length === 0 ? (
            <div className="flex flex-col items-center gap-2 rounded-panel border border-dashed border-border px-4 py-4 text-center">
              <p className="m-0 text-caption text-fg-3">
                {catalogQuery.isError
                  ? extractErrorMessage(catalogQuery.error) ||
                    t("suppliers.modelsRefreshFailed", {
                      defaultValue: "刷新模型目录失败",
                    })
                  : t("suppliers.noModels")}
              </p>
              {/* 空目录 / 读失败时左右两栏都不在，刷新入口得留一个，否则只能关掉重开。 */}
              <Button
                type="button"
                variant="quiet"
                size="compact"
                disabled={refreshing}
                aria-busy={refreshing}
                title={t("suppliers.modelsFetchHint", {
                  defaultValue: "从上游 /v1/models 获取模型列表",
                })}
                onClick={() => void refresh()}
              >
                <RefreshCw
                  aria-hidden="true"
                  className={cn(
                    "h-3.5 w-3.5",
                    refreshing && "motion-safe:animate-spin",
                  )}
                />
                {t("common.refresh")}
              </Button>
            </div>
          ) : (
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
              <div className="flex min-w-0 flex-col rounded-panel border border-border">
                <div className="flex items-center justify-between gap-2 border-b border-border px-3 py-2">
                  <div className="min-w-0">
                    <p className="m-0 text-strong font-semibold text-fg-1">
                      {t("suppliers.catalogModels")}
                    </p>
                    <p className="m-0 text-badge text-fg-3">
                      {t("suppliers.catalogModelsHint")}
                    </p>
                  </div>
                  <span className="shrink-0 text-caption text-fg-3">
                    {pendingEntries.length}
                  </span>
                  <Button
                    type="button"
                    variant="quiet"
                    size="compact"
                    aria-label={t("common.refresh")}
                    title={t("suppliers.modelsFetchHint", {
                      defaultValue: "从上游 /v1/models 获取模型列表",
                    })}
                    disabled={refreshing}
                    aria-busy={refreshing}
                    onClick={() => void refresh()}
                  >
                    <RefreshCw
                      aria-hidden="true"
                      className={cn(
                        "h-3.5 w-3.5",
                        refreshing && "motion-safe:animate-spin",
                      )}
                    />
                  </Button>
                </div>
                <div className="relative border-b border-border p-2">
                  <Search
                    aria-hidden="true"
                    className="absolute left-4 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-fg-3"
                  />
                  <Input
                    className="h-8 pl-7 text-caption"
                    value={sourceQuery}
                    placeholder={t("suppliers.searchModels")}
                    onChange={(event) => setSourceQuery(event.target.value)}
                  />
                </div>
                <ul className="m-0 max-h-56 list-none overflow-y-auto p-1">
                  {sourceItems.length === 0 ? (
                    <li className="px-2 py-3 text-center text-caption text-fg-3">
                      {sourceQuery.trim()
                        ? t("suppliers.noMatchingModels")
                        : t("suppliers.noPendingModels", {
                            defaultValue:
                              "没有待启用的模型，点刷新看看上游有没有新增",
                          })}
                    </li>
                  ) : (
                    sourceItems.map((entry) => {
                      const busy = busyKey === entry.modelId;
                      return (
                        <li key={entry.modelId}>
                          <button
                            type="button"
                            disabled={busy}
                            onClick={() => void setHidden(entry, false)}
                            className="flex w-full items-center justify-between gap-2 rounded-sm px-2 py-1.5 text-left hover:bg-subtle hover:text-fg-1"
                            title={entry.reference}
                          >
                            <span className="min-w-0 truncate font-mono text-caption">
                              {entry.modelId}
                            </span>
                            <span className="shrink-0 text-badge text-fg-3">
                              {t("suppliers.modelAdd")}
                            </span>
                          </button>
                        </li>
                      );
                    })
                  )}
                </ul>
              </div>

              <div className="flex min-w-0 flex-col rounded-panel border border-border">
                <div className="flex items-center justify-between gap-2 border-b border-border px-3 py-2">
                  <div className="min-w-0">
                    <p className="m-0 text-strong font-semibold text-fg-1">
                      {t("suppliers.addedModels")}
                    </p>
                    <p className="m-0 text-badge text-fg-3">
                      {t("suppliers.addedModelsHint")}
                    </p>
                  </div>
                  <span className="shrink-0 text-caption text-fg-3">
                    {addedItems.length}
                  </span>
                </div>
                <div className="relative border-b border-border p-2">
                  <Search
                    aria-hidden="true"
                    className="absolute left-4 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-fg-3"
                  />
                  <Input
                    className="h-8 pl-7 text-caption"
                    value={addedQuery}
                    placeholder={t("suppliers.searchModels")}
                    onChange={(event) => setAddedQuery(event.target.value)}
                  />
                </div>
                <ul className="m-0 max-h-56 list-none overflow-y-auto p-1">
                  {addedItems.length === 0 ? (
                    <li className="px-2 py-3 text-center text-caption text-fg-3">
                      {t("suppliers.noAddedModels")}
                    </li>
                  ) : (
                    addedItems.map((entry) => {
                      const key = entry.modelId;
                      const override = overrides.get(keyOf(entry));
                      const busy = busyKey === key;
                      return (
                        <li
                          key={key}
                          className="flex items-center gap-1 rounded-sm px-1 py-1 hover:bg-subtle"
                        >
                          <div className="min-w-0 flex-1">
                            <p className="m-0 truncate font-mono text-caption text-fg-2">
                              {entry.modelId}
                            </p>
                            <input
                              className="mt-0.5 h-6 w-full rounded-sm border border-transparent bg-transparent px-0 text-caption text-fg-3 outline-none focus:border-border focus:bg-surface focus:px-1"
                              value={drafts[key] ?? override?.displayName ?? ""}
                              placeholder={t("suppliers.modelDisplayName")}
                              disabled={busy}
                              onChange={(event) =>
                                setDrafts({
                                  ...drafts,
                                  [key]: event.target.value,
                                })
                              }
                              onBlur={() => {
                                if (drafts[key] !== undefined) {
                                  void rename(entry);
                                }
                              }}
                              onKeyDown={(event) => {
                                if (event.key === "Enter") {
                                  event.preventDefault();
                                  void rename(entry);
                                } else if (event.key === "Escape") {
                                  setDrafts((previous) => {
                                    const next = { ...previous };
                                    delete next[key];
                                    return next;
                                  });
                                }
                              }}
                            />
                          </div>
                          <Button
                            type="button"
                            variant="quiet"
                            size="compact"
                            disabled={busy}
                            aria-label={t("suppliers.removeModel")}
                            title={t("suppliers.removeModel")}
                            onClick={() => void setHidden(entry, true)}
                          >
                            <Trash2
                              aria-hidden="true"
                              className="h-3.5 w-3.5"
                            />
                          </Button>
                        </li>
                      );
                    })
                  )}
                </ul>
              </div>
            </div>
          )}
          <p className="m-0 text-caption text-fg-3">
            {t("suppliers.modelsFooterHint")}
          </p>
        </>
      )}
    </section>
  );
}

/* ------------------------------ 连通性 ------------------------------ */

function ConnectionSection({ group }: { group: ProviderGroup | null }) {
  const { t } = useTranslation();
  const [activeBinding, setActiveBinding] = useState<
    ProviderGroup["bindings"][number] | null
  >(null);
  const [selectedModels, setSelectedModels] = useState<string[]>([]);
  const [results, setResults] = useState<ModelCheckResult[] | null>(null);
  const [running, setRunning] = useState(false);

  const bindings = group?.bindings ?? [];
  const upstreamIds = group?.upstreamIds ?? [];
  const catalogQuery = useQuery({
    queryKey: ["gateway", "catalog", "withHidden"],
    queryFn: () => gatewayApi.getCatalog({ includeHidden: true }),
    staleTime: 30_000,
  });
  const models = useMemo(() => {
    const ids = new Set(upstreamIds);
    const values = new Set<string>();
    for (const entry of catalogQuery.data ?? []) {
      if (ids.has(entry.upstreamId)) values.add(entry.modelId);
    }
    return Array.from(values).sort();
  }, [catalogQuery.data, upstreamIds]);

  // 只有普通 API Key 接入可以真发请求；托管账号由授权中心维护，不探测。
  const literalBindings = useMemo(() => {
    const keys = new Set<string>();
    for (const credential of group?.credentials ?? []) {
      if (
        credential.secretKind !== "literal" &&
        credential.secretKind !== "env_ref"
      ) {
        continue;
      }
      for (const app of credential.apps) {
        keys.add(`${app.appType}:${app.providerId}`);
      }
    }
    return bindings.filter((binding) =>
      keys.has(`${binding.appType}:${binding.providerId}`),
    );
  }, [bindings, group?.credentials]);

  const checkableKeys = useMemo(
    () =>
      new Set(
        literalBindings.map(
          (binding) => `${binding.appType}:${binding.providerId}`,
        ),
      ),
    [literalBindings],
  );

  useEffect(() => {
    if (!activeBinding) {
      setSelectedModels([]);
      setResults(null);
      setRunning(false);
      return;
    }
    setSelectedModels(models);
    setResults(null);
  }, [activeBinding, models]);

  const startCheck = async () => {
    if (!activeBinding || running || selectedModels.length === 0) return;
    setRunning(true);
    try {
      const report = await checkProviderModels(
        activeBinding.appType as AppId,
        activeBinding.providerId,
        selectedModels,
      );
      setResults(report);
    } catch (error) {
      toast.error(extractErrorMessage(error) || t("suppliers.checkFailed"));
    } finally {
      setRunning(false);
    }
  };

  const passed = (results ?? []).filter((result) => result.success);
  const failed = (results ?? []).filter((result) => !result.success);

  return (
    <section className="space-y-3">
      <SectionHeader
        title={t("suppliers.connectivityTitle")}
        description={t("suppliers.connectivityDescription")}
      />

      {bindings.length === 0 ? (
        <p className="m-0 text-caption text-fg-3">
          {t("suppliers.noBindings")}
        </p>
      ) : (
        <ul className="m-0 list-none divide-y divide-border rounded-panel border border-border p-0">
          {bindings.map((binding) => {
            const checkable = checkableKeys.has(
              `${binding.appType}:${binding.providerId}`,
            );
            return (
              <li
                key={`${binding.appType}:${binding.providerId}`}
                className="flex flex-wrap items-center justify-between gap-2 px-3 py-2"
              >
                <div className="min-w-0">
                  <p className="m-0 text-caption text-fg-1">
                    {getAppLabel(binding.appType)}
                  </p>
                  <p className="m-0 truncate font-mono text-badge text-fg-3">
                    {binding.providerId}
                  </p>
                </div>
                {checkable ? (
                  <Button
                    type="button"
                    variant="neutral"
                    size="compact"
                    onClick={() => setActiveBinding(binding)}
                  >
                    <ShieldCheck aria-hidden="true" className="h-3.5 w-3.5" />
                    {t("suppliers.checkModels", {
                      defaultValue: "验证模型",
                    })}
                  </Button>
                ) : (
                  <span className="text-caption text-fg-3">
                    {t("suppliers.managedCheckNotApplicable", {
                      defaultValue: "托管登录，不适用真实模型检测",
                    })}
                  </span>
                )}
              </li>
            );
          })}
        </ul>
      )}

      <Dialog
        open={activeBinding !== null}
        onOpenChange={(open) => {
          if (!open && !running) setActiveBinding(null);
        }}
      >
        <DialogContent className="max-w-[540px]">
          <div className="flex items-start justify-between gap-3 border-b border-border px-5 py-4">
            <div className="min-w-0">
              <DialogTitle>
                {t("suppliers.checkModelsTitle", {
                  defaultValue: "验证模型连通性",
                })}
              </DialogTitle>
              <DialogDescription className="mt-0.5">
                {activeBinding
                  ? `${getAppLabel(activeBinding.appType)} · ${activeBinding.providerId}`
                  : ""}
              </DialogDescription>
            </div>
            <DialogClose asChild>
              <Button
                type="button"
                variant="quiet"
                size="compact"
                disabled={running}
                aria-label={t("common.close")}
              >
                <X aria-hidden="true" className="h-4 w-4" />
              </Button>
            </DialogClose>
          </div>

          <div className="min-h-0 flex-1 space-y-3 overflow-y-auto px-5 py-4">
            <Notice tone="warning">
              {t("suppliers.checkModelsWarning", {
                defaultValue:
                  "会使用该供应商的 API Key 向每个选中的模型各发一次真实请求，最多生成 1 token，可能消耗少量额度。",
              })}
            </Notice>

            <div className="rounded-panel border border-border">
              <div className="flex items-center justify-between gap-2 border-b border-border px-3 py-2">
                <span className="text-caption font-semibold text-fg-2">
                  {t("suppliers.checkModelsPick", {
                    defaultValue: "选择要验证的模型",
                  })}
                </span>
                <div className="flex items-center gap-1">
                  <Button
                    type="button"
                    variant="quiet"
                    size="compact"
                    disabled={running || models.length === 0}
                    onClick={() => setSelectedModels(models)}
                  >
                    {t("suppliers.checkModelsAll", {
                      defaultValue: "全选",
                    })}
                  </Button>
                  <Button
                    type="button"
                    variant="quiet"
                    size="compact"
                    disabled={running || selectedModels.length === 0}
                    onClick={() => setSelectedModels([])}
                  >
                    {t("suppliers.checkModelsClear", {
                      defaultValue: "清空",
                    })}
                  </Button>
                </div>
              </div>
              <div className="max-h-[180px] overflow-y-auto p-1">
                {models.length === 0 ? (
                  <p className="m-0 px-3 py-4 text-center text-caption text-fg-3">
                    {t("suppliers.checkModelsEmpty", {
                      defaultValue:
                        "模型目录为空。请先在「模型」区刷新并加入模型。",
                    })}
                  </p>
                ) : (
                  models.map((model) => {
                    const checked = selectedModels.includes(model);
                    return (
                      <label
                        key={model}
                        className={cn(
                          "flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-caption",
                          checked
                            ? "bg-accent/10 text-fg-1"
                            : "text-fg-2 hover:bg-subtle",
                        )}
                      >
                        <input
                          type="checkbox"
                          checked={checked}
                          disabled={running}
                          onChange={() =>
                            setSelectedModels((current) =>
                              current.includes(model)
                                ? current.filter((item) => item !== model)
                                : [...current, model],
                            )
                          }
                        />
                        <span className="min-w-0 flex-1 truncate font-mono">
                          {model}
                        </span>
                      </label>
                    );
                  })
                )}
              </div>
            </div>

            {results && (
              <div className="rounded-panel border border-border">
                <div className="flex items-center justify-between gap-2 border-b border-border px-3 py-2">
                  <span className="text-caption font-semibold text-fg-2">
                    {t("suppliers.checkModelsResults", {
                      defaultValue: "检测结果",
                    })}
                  </span>
                  <div className="flex items-center gap-2 text-badge">
                    <span className="text-success-text">
                      {passed.length}{" "}
                      {t("suppliers.checkModelsAvailable", {
                        defaultValue: "可用",
                      })}
                    </span>
                    <span className="text-danger-text">
                      {failed.length}{" "}
                      {t("suppliers.checkModelsUnavailable", {
                        defaultValue: "不可用",
                      })}
                    </span>
                  </div>
                </div>
                <ul className="m-0 max-h-[220px] list-none overflow-y-auto p-2">
                  {results.map((result) => (
                    <li
                      key={result.model}
                      className="rounded-sm border border-border px-2 py-1.5 text-caption"
                    >
                      <div className="flex items-center justify-between gap-2">
                        <span className="min-w-0 truncate font-mono font-semibold text-fg-1">
                          {result.model}
                        </span>
                        <span
                          className={cn(
                            "shrink-0",
                            result.success
                              ? "text-success-text"
                              : "text-danger-text",
                          )}
                        >
                          {result.success
                            ? t("suppliers.checkModelsAvailable", {
                                defaultValue: "可用",
                              })
                            : t("suppliers.checkModelsUnavailable", {
                                defaultValue: "不可用",
                              })}
                          {result.responseTimeMs != null
                            ? ` · ${result.responseTimeMs} ms`
                            : ""}
                        </span>
                      </div>
                      <p className="m-0 mt-0.5 break-words text-badge text-fg-3">
                        {result.message}
                      </p>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>

          <DialogFooter className="border-t border-border px-5 py-3">
            <Button
              type="button"
              variant="quiet"
              disabled={running}
              onClick={() => setActiveBinding(null)}
            >
              {t("common.close")}
            </Button>
            <Button
              type="button"
              variant="solid"
              disabled={
                running || selectedModels.length === 0 || models.length === 0
              }
              aria-busy={running}
              onClick={() => void startCheck()}
            >
              {running ? (
                <Loader2 aria-hidden="true" className="h-4 w-4 animate-spin" />
              ) : (
                <ShieldCheck aria-hidden="true" className="h-4 w-4" />
              )}
              {t("suppliers.checkModelsStart", {
                defaultValue: "开始检测",
              })}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}

/* ------------------------------ 高级 ------------------------------ */

function AdvancedSection({ group }: { group: ProviderGroup | null }) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const protocol = group?.primary.upstreamProtocol ?? "unknown";

  return (
    <section className="space-y-2">
      <button
        type="button"
        className="inline-flex items-center gap-1.5 text-caption font-semibold text-fg-2 hover:text-fg-1"
        aria-expanded={expanded}
        onClick={() => setExpanded((value) => !value)}
      >
        <ChevronRight
          className={cn(
            "h-3.5 w-3.5 transition-transform",
            expanded && "rotate-90",
          )}
        />
        {t("suppliers.advancedTitle")}
      </button>

      {expanded && (
        <div className="space-y-2 rounded-panel border border-border p-3">
          <div className="flex flex-wrap items-center gap-2 text-caption text-fg-2">
            <span className="text-fg-3">{t("suppliers.protocolLabel")}</span>
            <span className="rounded-full bg-subtle px-1.5 py-0.5">
              {protocol}
            </span>
            <span className="text-fg-3">{t("suppliers.protocolSource")}</span>
          </div>
          <ul className="m-0 list-none space-y-1 p-0">
            {(group?.suppliers ?? []).map((supplier) => (
              <li
                key={supplier.id}
                className="flex flex-wrap items-center gap-2 rounded-sm bg-subtle px-2 py-1.5 text-caption text-fg-2"
              >
                <code className="font-mono text-badge text-fg-3">
                  {supplier.id}
                </code>
                <span>{supplier.baseUrl || t("suppliers.noBaseUrlKey")}</span>
                <span className="text-fg-3">
                  {t("suppliers.bindingCount", {
                    count: supplier.bindings.length,
                  })}
                </span>
              </li>
            ))}
          </ul>
          <p className="m-0 text-caption text-fg-3">
            {t("suppliers.advancedHint")}
          </p>
        </div>
      )}
    </section>
  );
}
