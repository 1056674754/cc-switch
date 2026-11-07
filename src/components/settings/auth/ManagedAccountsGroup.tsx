import React, { useEffect, useId, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "@/lib/toast";
import {
  Check,
  ChevronDown,
  CircleAlert,
  Copy,
  ExternalLink,
  Link2,
  Loader2,
  MoreHorizontal,
  Plus,
  RefreshCw,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { HelpTip } from "@/components/ui/help-tip";
import { HoverTip } from "@/components/ui/hover-tip";
import { Notice } from "@/components/ui/notice";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { ManagedAccountUsage } from "@/components/providers/forms/ManagedAccountRemoveDialog";
import type { ManagedAccountUser } from "@/lib/managedAccountUsage";
import type { ManagedAuthDeviceCodeResponse } from "@/lib/api";
import { copyText } from "@/lib/clipboard";
import { settingsApi } from "@/lib/api";
import { readAuthOpenBrowser } from "@/lib/authLoginLink";
import { cn } from "@/lib/utils";
import { ProviderIcon } from "@/components/ProviderIcon";

export interface GroupAccountRow {
  id: string;
  login: string;
  avatarUrl?: string | null;
  /** 第二行的片段，用「·」连起来：域名、登录日期；需要重新登录时是原因 */
  details: React.ReactNode[];
  isDefault: boolean;
  needsReauth: boolean;
  /** 在用这个账号的供应商（第二行末尾写「N 个供应商在用」，悬停列名字） */
  users: ManagedAccountUser[];
  /** 右侧的额度（AccountQuotaColumn）；需要重新登录的账号不传 */
  quota?: React.ReactNode;
  /**
   * 供应商页：这个账号是否参与使用（没传就没有「停用/启用」这一项）。
   * 与「退出登录」不同：停用只是保留账号、别用它。
   */
  enabled?: boolean;
  /** ⋯ 里删除项的文案（Kimi Code 自己的登录是「移出列表」） */
  removeLabel?: string;
  /** 删除项不能点的原因（Kimi 正在用的号要先切走），照常列出、原因写在下面 */
  removeDisabledReason?: string;
}

export interface GroupLoginFlow {
  /** 正在向服务要验证码（还没拿到） */
  starting: boolean;
  /** 已拿到验证码，等用户在浏览器里授权 */
  polling: boolean;
  pollingState: "idle" | "polling" | "success" | "error";
  deviceCode: Pick<
    ManagedAuthDeviceCodeResponse,
    "user_code" | "verification_uri"
  > | null;
  error: string | null;
  onCancel: () => void;
  onRetry: () => void;
  /** 等待块里浏览器说明下面的附加内容（Claude 官方账号：浏览器回不到本机时粘贴代码） */
  extra?: React.ReactNode;
}

interface ManagedAccountsGroupProps {
  /** DOM id 前缀 */
  slug: string;
  name: string;
  /** ProviderIcon 的图标名（githubcopilot / openai / xai） */
  iconName: string;
  /** 「?」里的说明：这个服务的账号用在哪些预设上、没指定账号时用哪个 */
  help: React.ReactNode;
  /**
   * 「?」里再补一句：嵌进供应商卡片时，「设为默认」是让这家供应商改用该账号，
   * 和授权中心里改全局默认账号不是一回事。
   */
  helpNote?: React.ReactNode;
  helpSide?: "top" | "bottom";
  accounts: GroupAccountRow[];
  status: "loading" | "error" | "ready";
  statusErrorText: string;
  onRetryStatus: () => void;
  /** 标题行下面的提示（Copilot 旧数据迁移失败） */
  notice?: React.ReactNode;
  /** 不能登录的原因（Claude 官方账号：配置目录在 WSL 里）：「添加账号」「重新登录」置灰，原因显示在标题行下面 */
  loginDisabledReason?: string;
  emptyText: string;
  /** 标题行的「添加账号」，有没有账号都在（Copilot 先打开部署类型选择） */
  onAdd: () => void;
  /** Copilot 选部署类型的那块；有它时不显示空状态那行 */
  chooser?: React.ReactNode;
  /**
   * 「添加账号」旁边的登录方式：传了就把「添加账号」做成一个整体控件——
   * 左边点击按默认方式登录，右边 ▾ 选「浏览器登录 / 设备代码登录」。
   * 不再单独摆一个下拉框，避免看起来像两个入口。
   */
  addOptions?: {
    key: string;
    label: React.ReactNode;
    hint?: React.ReactNode;
    checked?: boolean;
    onSelect: () => void;
  }[];
  /** 「默认」徽标和 ⋯ 里「设为默认」的文案（Antigravity 是「agy 在用」「切换 agy 到这个账号」） */
  defaultLabel?: string;
  setDefaultLabel?: string;
  /** ⋯ 里有没有「重新登录」（Copilot 没有） */
  canReauth: boolean;
  /** 只给「需要重新登录」的账号显示（Claude 官方账号：重新登录会替换掉原来的登录，凭据好好的时候不该点到） */
  reauthOnlyWhenExpired?: boolean;
  onReauth: (accountId: string) => void;
  /** 不传就没有「设为默认」（Claude 官方账号：用哪个号由卡片决定） */
  onSetDefault?: (accountId: string) => void;
  settingDefault: boolean;
  onRemove: (accountId: string, login: string) => void;
  removing: boolean;
  /** 传了才显示 ⋯ 里的「停用/启用」；供应商页用它表达「保留但不参与使用」 */
  onToggleEnabled?: (accountId: string, enabled: boolean) => void;
  togglingEnabled?: boolean;
  login: GroupLoginFlow;
  /** 嵌进别的卡片（供应商卡片）时去掉自己的外框，只留一条上分隔线 */
  embedded?: boolean;
  /** 嵌进 provider 卡片时把「服务名 + 图标」换成一段自己的标题（如「凭据」），账号操作保留 */
  headerLabel?: string;
  /** 确认框等 */
  children?: React.ReactNode;
}

const PILL =
  "inline-flex h-[18px] shrink-0 items-center rounded-full px-1.5 text-badge";

/**
 * 授权中心的一个服务（v7 Auth 画板）：标题行（图标、名字、「?」、添加账号），
 * 每个账号一行（头像、名字 + 默认 / 需要重新登录、第二行、额度、⋯），空状态，
 * 登录等待块。删账号走确认框（列出受影响的供应商），由调用方传进 children。
 */
export function ManagedAccountsGroup({
  slug,
  name,
  iconName,
  help,
  helpNote,
  helpSide,
  accounts,
  status,
  statusErrorText,
  onRetryStatus,
  notice,
  loginDisabledReason,
  emptyText,
  onAdd,
  chooser,
  addOptions,
  defaultLabel,
  setDefaultLabel,
  canReauth,
  reauthOnlyWhenExpired = false,
  onReauth,
  onSetDefault,
  settingDefault,
  onRemove,
  removing,
  onToggleEnabled,
  togglingEnabled = false,
  login,
  embedded = false,
  headerLabel,
  children,
}: ManagedAccountsGroupProps) {
  const { t } = useTranslation();
  const pendingId = `auth-pending-${slug}`;
  // 正在重新登录哪个账号（等待块的标题、成功后的 toast 用）
  const [reauthTarget, setReauthTarget] = useState<string | null>(null);
  const busy = login.starting || login.polling;
  const blockedId = `auth-blocked-${slug}`;
  const loginBlocked = Boolean(loginDisabledReason);

  const targetAccount = reauthTarget
    ? accounts.find((account) => account.id === reauthTarget)
    : undefined;

  // 登录成功的 toast：pollingState 走到 success 时报一次
  const prevPolling = useRef(login.pollingState);
  useEffect(() => {
    const prev = prevPolling.current;
    prevPolling.current = login.pollingState;
    if (prev === "success" || login.pollingState !== "success") return;
    toast.success(
      targetAccount
        ? t("authCenter.toast.reauthed", {
            defaultValue: "{{login}} 已重新登录。",
            login: targetAccount.login,
          })
        : t("authCenter.toast.added", {
            defaultValue: "已添加 {{service}} 账号。",
            service: name,
          }),
    );
    setReauthTarget(null);
  }, [login.pollingState, name, t, targetAccount]);

  const add = () => {
    if (busy || loginBlocked) return;
    setReauthTarget(null);
    onAdd();
  };

  const reauth = (accountId: string) => {
    if (busy || loginBlocked) return;
    setReauthTarget(accountId);
    onReauth(accountId);
  };

  const pendingTitle = targetAccount
    ? t("authCenter.pending.reauth", {
        defaultValue: "重新登录 {{login}} · 等待授权中…",
        login: targetAccount.login,
      })
    : t("authCenter.pending.add", {
        defaultValue: "添加 {{service}} 账号 · 等待授权中…",
        service: name,
      });

  const copyCode = async () => {
    if (!login.deviceCode?.user_code) return;
    try {
      await copyText(login.deviceCode.user_code);
      toast.success(
        t("authCenter.pending.copied", { defaultValue: "验证码已复制。" }),
      );
    } catch (e) {
      console.debug("[AuthCenter] Failed to copy user code:", e);
    }
  };

  // 这次登录开始时有没有打开浏览器（授权中心的开关），换一次登录链接才重新读：
  // 中途拨开关不改这次的说明
  const verificationUri = login.deviceCode?.verification_uri;
  const linkOnly = useMemo(
    () => Boolean(verificationUri) && !readAuthOpenBrowser(),
    [verificationUri],
  );

  const copyLink = async () => {
    const uri = login.deviceCode?.verification_uri;
    if (!uri) return;
    try {
      await copyText(uri);
      toast.success(
        t("authCenter.pending.linkCopied", {
          defaultValue: "登录链接已复制。",
        }),
      );
    } catch (e) {
      console.debug("[AuthCenter] Failed to copy login link:", e);
    }
  };

  const openPage = () => {
    const uri = login.deviceCode?.verification_uri;
    if (uri) void settingsApi.openExternal(uri);
  };

  const ready = status === "ready";
  const showEmpty = ready && accounts.length === 0 && !busy && !chooser;

  return (
    <section
      aria-label={name}
      className={cn(
        "auth-group shrink-0",
        embedded
          ? "border-t border-border"
          : "rounded-panel border border-border bg-surface",
      )}
    >
      <div
        className={cn(
          "flex items-center gap-3 pe-2.5 ps-4",
          headerLabel ? "h-[44px]" : "h-[52px]",
        )}
      >
        {headerLabel ? null : (
          <span
            aria-hidden="true"
            className="flex h-7 w-7 shrink-0 items-center justify-center text-fg-1"
          >
            <ProviderIcon icon={iconName} name={name} size={20} />
          </span>
        )}
        <div className="flex min-w-0 flex-1 items-center gap-0.5">
          <h2 className="m-0 truncate text-strong font-semibold text-fg-1">
            {headerLabel ?? name}
          </h2>
          <HelpTip
            title={
              headerLabel
                ? t("suppliers.credentialsHelpTitle", {
                    defaultValue: "这些账号怎么用",
                  })
                : t("authCenter.group.helpTitle", {
                    defaultValue: "{{service}} 账号怎么用",
                    service: name,
                  })
            }
            side={helpSide}
          >
            {help}
            {helpNote ? (
              <span className="mt-1 block text-fg-2">{helpNote}</span>
            ) : null}
          </HelpTip>
        </div>
        {ready && addOptions?.length ? (
          <div className="flex items-stretch">
            <Button
              type="button"
              id={`auth-add-${slug}`}
              variant="neutral"
              size="compact"
              className="gap-1.5 rounded-e-none pe-3 ps-2.5"
              aria-disabled={busy || loginBlocked || undefined}
              aria-describedby={
                busy ? pendingId : loginBlocked ? blockedId : undefined
              }
              onClick={add}
            >
              <Plus className="h-3.5 w-3.5" strokeWidth={2} />
              {t("authCenter.addAccount", { defaultValue: "添加账号" })}
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  type="button"
                  variant="neutral"
                  size="compact"
                  className="rounded-s-none border-s-0 px-1.5"
                  aria-label={t("authCenter.addAccountMethod", {
                    defaultValue: "选择添加账号的方式",
                  })}
                  aria-disabled={busy || loginBlocked || undefined}
                >
                  <ChevronDown
                    className="h-3.5 w-3.5"
                    strokeWidth={2}
                    aria-hidden="true"
                  />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {addOptions.map((option) => (
                  <DropdownMenuItem
                    key={option.key}
                    disabled={busy || loginBlocked}
                    onSelect={() => {
                      if (busy || loginBlocked) return;
                      setReauthTarget(null);
                      option.onSelect();
                    }}
                  >
                    <Check
                      aria-hidden="true"
                      className={cn(
                        "h-3.5 w-3.5",
                        option.checked ? "opacity-100" : "opacity-0",
                      )}
                    />
                    <span className="flex flex-col">
                      <span>{option.label}</span>
                      {option.hint ? (
                        <span className="text-caption text-fg-3">
                          {option.hint}
                        </span>
                      ) : null}
                    </span>
                  </DropdownMenuItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        ) : ready ? (
          <Button
            type="button"
            id={`auth-add-${slug}`}
            variant="neutral"
            size="compact"
            className="gap-1.5 pe-3 ps-2.5"
            aria-disabled={busy || loginBlocked || undefined}
            aria-describedby={
              busy ? pendingId : loginBlocked ? blockedId : undefined
            }
            onClick={add}
          >
            <Plus className="h-3.5 w-3.5" strokeWidth={2} />
            {t("authCenter.addAccount", { defaultValue: "添加账号" })}
          </Button>
        ) : null}
      </div>

      {notice && <div className="px-3 pb-3">{notice}</div>}
      {loginDisabledReason && (
        <div id={blockedId} className="px-3 pb-3">
          <Notice tone="warning" title={loginDisabledReason} />
        </div>
      )}

      {status === "loading" && (
        <div className="flex items-center gap-2 border-t border-border py-3.5 pe-3 ps-4 text-caption text-fg-2">
          <Loader2 className="h-3.5 w-3.5 motion-safe:animate-spin" />
          {t("authCenter.statusLoading", { defaultValue: "正在加载账号…" })}
        </div>
      )}

      {status === "error" && (
        <div className="flex items-center gap-3 border-t border-border py-3 pe-3 ps-4">
          <CircleAlert
            aria-hidden="true"
            strokeWidth={1.5}
            className="h-4 w-4 shrink-0 text-danger-text"
          />
          <span className="min-w-0 flex-1 text-caption text-fg-1">
            {statusErrorText}
          </span>
          <Button
            type="button"
            variant="neutral"
            size="compact"
            onClick={onRetryStatus}
          >
            <RefreshCw className="h-3.5 w-3.5" strokeWidth={1.5} />
            {t("authCenter.retry", { defaultValue: "重试" })}
          </Button>
        </div>
      )}

      {ready &&
        accounts.map((account) => (
          <AccountRow
            key={account.id}
            account={account}
            defaultLabel={defaultLabel}
            setDefaultLabel={setDefaultLabel}
            canReauth={
              canReauth && (!reauthOnlyWhenExpired || account.needsReauth)
            }
            busy={busy}
            pendingId={pendingId}
            loginBlockedId={loginBlocked ? blockedId : undefined}
            settingDefault={settingDefault}
            removing={removing}
            onReauth={reauth}
            onSetDefault={onSetDefault}
            onRemove={onRemove}
            onToggleEnabled={onToggleEnabled}
            togglingEnabled={togglingEnabled}
          />
        ))}

      {showEmpty && (
        <div className="border-t border-border py-3.5 pe-3 ps-4 text-caption text-fg-2">
          {emptyText}
        </div>
      )}

      {chooser}

      <div role="status">
        {busy && (
          <div className="mx-3 mb-3 flex flex-col gap-2 rounded-[8px] bg-subtle px-3.5 py-3">
            <div className="flex items-center gap-2">
              <Loader2
                aria-hidden="true"
                className="h-[15px] w-[15px] shrink-0 text-fg-2 motion-safe:animate-spin"
              />
              <span
                id={pendingId}
                className="min-w-0 flex-1 text-body font-medium text-fg-1"
              >
                {pendingTitle}
              </span>
              <Button
                type="button"
                variant="neutral"
                size="compact"
                onClick={login.onCancel}
              >
                {t("common.cancel", { defaultValue: "取消" })}
              </Button>
            </div>
            {login.deviceCode && !login.deviceCode.user_code && (
              <div className="flex flex-wrap items-center gap-2 text-caption text-fg-2">
                <span className="min-w-0 flex-1">
                  {linkOnly
                    ? t("authCenter.pending.linkNote", {
                        defaultValue:
                          "登录链接已复制。在你要用的浏览器里打开，登录并同意授权后这里会自动更新。",
                      })
                    : t("authCenter.pending.browserNote", {
                        defaultValue:
                          "已在浏览器打开 {{service}} 的登录页，登录并同意授权后这里会自动更新。",
                        service: name,
                      })}
                </span>
                <Button
                  type="button"
                  variant="quiet"
                  size="compact"
                  className="gap-1"
                  onClick={() => void copyLink()}
                >
                  <Copy className="h-3.5 w-3.5" strokeWidth={1.5} />
                  {t("authCenter.pending.copyLink", {
                    defaultValue: "复制登录链接",
                  })}
                </Button>
                {!linkOnly && (
                  <Button
                    type="button"
                    variant="quiet"
                    size="compact"
                    className="gap-1"
                    onClick={openPage}
                  >
                    <ExternalLink className="h-3.5 w-3.5" strokeWidth={1.5} />
                    {t("authCenter.pending.reopen", {
                      defaultValue: "重新打开登录页",
                    })}
                  </Button>
                )}
              </div>
            )}
            {login.deviceCode && !login.deviceCode.user_code && login.extra}
            {login.deviceCode?.user_code && (
              <>
                <div className="flex flex-wrap items-center gap-2 text-caption text-fg-2">
                  <span>
                    {t("authCenter.pending.enterCode", {
                      defaultValue: "在浏览器中输入验证码：",
                    })}
                  </span>
                  <code className="inline-flex h-7 items-center rounded-control border border-border-strong bg-surface px-2.5 font-mono text-section tracking-[1px] text-fg-1">
                    {login.deviceCode.user_code}
                  </code>
                  <HoverTip
                    content={t("authCenter.pending.copy", {
                      defaultValue: "复制验证码",
                    })}
                  >
                    <Button
                      type="button"
                      variant="quiet"
                      size="icon-compact"
                      aria-label={t("authCenter.pending.copy", {
                        defaultValue: "复制验证码",
                      })}
                      onClick={() => void copyCode()}
                    >
                      <Copy className="h-3.5 w-3.5" strokeWidth={1.5} />
                    </Button>
                  </HoverTip>
                  <HoverTip
                    content={t("authCenter.pending.copyLink", {
                      defaultValue: "复制登录链接",
                    })}
                  >
                    <Button
                      type="button"
                      variant="quiet"
                      size="icon-compact"
                      aria-label={t("authCenter.pending.copyLink", {
                        defaultValue: "复制登录链接",
                      })}
                      onClick={() => void copyLink()}
                    >
                      <Link2 className="h-3.5 w-3.5" strokeWidth={1.5} />
                    </Button>
                  </HoverTip>
                  {!linkOnly && (
                    <HoverTip
                      content={t("authCenter.pending.open", {
                        defaultValue: "打开授权页面",
                      })}
                    >
                      <Button
                        type="button"
                        variant="quiet"
                        size="icon-compact"
                        aria-label={t("authCenter.pending.open", {
                          defaultValue: "打开授权页面",
                        })}
                        onClick={openPage}
                      >
                        <ExternalLink
                          className="h-3.5 w-3.5"
                          strokeWidth={1.5}
                        />
                      </Button>
                    </HoverTip>
                  )}
                </div>
                <span className="text-caption text-fg-2">
                  {linkOnly
                    ? t("authCenter.pending.linkCodeNote", {
                        defaultValue:
                          "登录链接已复制。在你要用的浏览器里打开并输入验证码，完成后这里会自动更新。注意 {{service}} 的使用条款。",
                        service: name,
                      })
                    : t("authCenter.pending.note", {
                        defaultValue:
                          "已在浏览器打开授权页面，完成后这里会自动更新。注意 {{service}} 的使用条款。",
                        service: name,
                      })}
                </span>
              </>
            )}
          </div>
        )}
        {!busy && login.pollingState === "error" && login.error && (
          <div className="mx-3 mb-3">
            <Notice
              tone="danger"
              title={t("authCenter.loginFailed", {
                defaultValue: "登录没有完成",
              })}
              actions={
                <>
                  <Button
                    type="button"
                    variant="neutral"
                    size="compact"
                    onClick={login.onRetry}
                  >
                    {t("authCenter.retry", { defaultValue: "重试" })}
                  </Button>
                  <Button
                    type="button"
                    variant="quiet"
                    size="compact"
                    onClick={login.onCancel}
                  >
                    {t("common.close", { defaultValue: "关闭" })}
                  </Button>
                </>
              }
            >
              <span className="[overflow-wrap:anywhere]">{login.error}</span>
            </Notice>
          </div>
        )}
      </div>

      {children}
    </section>
  );
}

function AccountAvatar({
  login,
  avatarUrl,
}: {
  login: string;
  avatarUrl?: string | null;
}) {
  const [failed, setFailed] = useState(false);
  return (
    <span
      aria-hidden="true"
      className="flex h-7 w-7 shrink-0 items-center justify-center overflow-hidden rounded-full border border-border bg-subtle text-caption font-semibold text-fg-2"
    >
      {avatarUrl && !failed ? (
        <img
          src={avatarUrl}
          alt=""
          className="h-full w-full object-cover"
          loading="lazy"
          referrerPolicy="no-referrer"
          onError={() => setFailed(true)}
        />
      ) : (
        login.charAt(0).toUpperCase()
      )}
    </span>
  );
}

interface AccountRowProps {
  account: GroupAccountRow;
  defaultLabel?: string;
  setDefaultLabel?: string;
  canReauth: boolean;
  busy: boolean;
  pendingId: string;
  /** 不能登录时，说明原因的元素 id（「重新登录」置灰） */
  loginBlockedId?: string;
  settingDefault: boolean;
  removing: boolean;
  onReauth: (accountId: string) => void;
  onSetDefault?: (accountId: string) => void;
  onRemove: (accountId: string, login: string) => void;
  /** 供应商页：账号是否参与使用（undefined 就不显示停用/启用） */
  onToggleEnabled?: (accountId: string, enabled: boolean) => void;
  togglingEnabled: boolean;
}

function AccountRow({
  account,
  defaultLabel,
  setDefaultLabel,
  canReauth,
  busy,
  pendingId,
  loginBlockedId,
  settingDefault,
  removing,
  onReauth,
  onSetDefault,
  onRemove,
  onToggleEnabled,
  togglingEnabled,
}: AccountRowProps) {
  const { t } = useTranslation();
  const subId = useId();
  const canSetDefault =
    onSetDefault !== undefined && !account.isDefault && !account.needsReauth;
  const separator = " · ";

  return (
    <div className="flex min-h-[56px] items-center gap-3 border-t border-border py-2 pe-2.5 ps-4">
      <AccountAvatar login={account.login} avatarUrl={account.avatarUrl} />
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="flex min-w-0 items-center gap-1.5">
          <span
            title={account.login}
            className="min-w-0 truncate text-body font-medium text-fg-1"
          >
            {account.login}
          </span>
          {account.isDefault && (
            <span className={cn(PILL, "border border-border-strong text-fg-2")}>
              {defaultLabel ??
                t("authCenter.default", { defaultValue: "默认" })}
            </span>
          )}
          {account.needsReauth && (
            <span className={cn(PILL, "bg-danger-soft text-danger-text")}>
              {t("authCenter.needsReauth", { defaultValue: "需要重新登录" })}
            </span>
          )}
        </div>
        <div
          id={subId}
          className="flex min-w-0 items-center text-caption text-fg-2"
        >
          <span className="min-w-0 truncate">
            {account.details.map((part, index) => (
              <React.Fragment key={index}>
                {index > 0 && separator}
                {part}
              </React.Fragment>
            ))}
            {account.details.length > 0 && account.users.length > 0
              ? separator
              : null}
          </span>
          <ManagedAccountUsage users={account.users} />
        </div>
      </div>

      {!account.needsReauth && account.quota}

      {account.needsReauth && canReauth && (
        <Button
          type="button"
          variant="neutral"
          size="compact"
          aria-disabled={busy || Boolean(loginBlockedId) || undefined}
          aria-describedby={busy ? pendingId : (loginBlockedId ?? subId)}
          onClick={() => onReauth(account.id)}
        >
          {t("authCenter.reauth", { defaultValue: "重新登录" })}
        </Button>
      )}

      <DropdownMenu>
        <HoverTip content={t("common.more", { defaultValue: "更多" })}>
          <DropdownMenuTrigger asChild>
            <Button
              type="button"
              variant="quiet"
              size="icon-compact"
              aria-label={t("authCenter.accountMenu", {
                defaultValue: "{{login}} 的更多操作",
                login: account.login,
              })}
            >
              <MoreHorizontal className="h-[15px] w-[15px]" />
            </Button>
          </DropdownMenuTrigger>
        </HoverTip>
        <DropdownMenuContent
          align="end"
          className="min-w-[148px] rounded-panel bg-surface p-1 shadow-v7-md"
        >
          {canSetDefault && (
            <DropdownMenuItem
              disabled={settingDefault}
              onSelect={() => onSetDefault?.(account.id)}
              className="h-[30px] rounded-control px-2.5 text-body"
            >
              {setDefaultLabel ??
                t("authCenter.setDefault", { defaultValue: "设为默认" })}
            </DropdownMenuItem>
          )}
          {canReauth && (
            <DropdownMenuItem
              disabled={busy || Boolean(loginBlockedId)}
              onSelect={() => onReauth(account.id)}
              className="min-h-[30px] flex-col items-start justify-center gap-0 rounded-control px-2.5 text-body data-[disabled]:opacity-100"
            >
              <span className={cn((busy || loginBlockedId) && "text-fg-3")}>
                {t("authCenter.reauth", { defaultValue: "重新登录" })}
              </span>
              {busy && (
                <span className="text-caption text-fg-3">
                  {t("authCenter.busyHint", {
                    defaultValue: "先完成或取消正在进行的登录",
                  })}
                </span>
              )}
            </DropdownMenuItem>
          )}
          {onToggleEnabled && account.enabled !== undefined && (
            <DropdownMenuItem
              disabled={togglingEnabled || busy}
              onSelect={() => onToggleEnabled(account.id, !account.enabled)}
              className="h-[30px] rounded-control px-2.5 text-body"
            >
              {account.enabled
                ? t("authCenter.disableAccount", { defaultValue: "停用账号" })
                : t("authCenter.enableAccount", { defaultValue: "启用账号" })}
            </DropdownMenuItem>
          )}
          <DropdownMenuItem
            disabled={removing || Boolean(account.removeDisabledReason)}
            onSelect={() => onRemove(account.id, account.login)}
            className={cn(
              "min-h-[30px] flex-col items-start justify-center gap-0 rounded-control px-2.5 text-body text-danger-text focus:text-danger-text",
              account.removeDisabledReason && "data-[disabled]:opacity-100",
            )}
          >
            <span className={cn(account.removeDisabledReason && "text-fg-3")}>
              {account.removeLabel ??
                t("authCenter.removeAccount", { defaultValue: "退出登录…" })}
            </span>
            {account.removeDisabledReason && (
              <span className="text-caption text-fg-3">
                {account.removeDisabledReason}
              </span>
            )}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
