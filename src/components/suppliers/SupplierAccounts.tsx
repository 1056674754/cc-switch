import { useQuery } from "@tanstack/react-query";
import { AlertTriangle, KeyRound, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import CodexOauthAccountQuota from "@/components/CodexOauthAccountQuota";
import {
  AgyAccountQuota,
  ClaudeAccountQuota,
  CopilotAccountQuota,
  XaiAccountQuota,
} from "@/components/settings/auth/AccountQuota";
import { signedInDate } from "@/components/settings/auth/accountDetails";
import { authListAccounts } from "@/lib/api/auth";
import type { ManagedAuthAccount, ManagedAuthProvider } from "@/lib/api/auth";
import type { Supplier } from "@/lib/api/suppliers";

/**
 * 供应商卡片上的托管账号列表。
 *
 * 托管登录的供应商（Codex 官方、ChatGPT 预设、Claude 官方…）的凭据不是一条密钥，
 * 而是授权中心里的一个账号池；卡片必须把池子里的账号一个个列出来——只写
 * 「1 个凭据」会把「登录过 3 个账号」这件事藏掉。行里的额度和授权中心共用同一份缓存。
 */
const MANAGED_AUTH_PROVIDERS: ManagedAuthProvider[] = [
  "github_copilot",
  "codex_oauth",
  "xai_oauth",
  "agy_oauth",
  "claude_official",
];

export function isManagedAuthProvider(
  value: string | null | undefined,
): value is ManagedAuthProvider {
  return (
    typeof value === "string" &&
    (MANAGED_AUTH_PROVIDERS as string[]).includes(value)
  );
}

function AccountQuota({
  provider,
  account,
}: {
  provider: ManagedAuthProvider;
  account: ManagedAuthAccount;
}) {
  switch (provider) {
    case "codex_oauth":
      return (
        <CodexOauthAccountQuota accountId={account.id} login={account.login} />
      );
    case "claude_official":
      return (
        <ClaudeAccountQuota accountId={account.id} login={account.login} />
      );
    case "github_copilot":
      return (
        <CopilotAccountQuota accountId={account.id} login={account.login} />
      );
    case "xai_oauth":
      return <XaiAccountQuota accountId={account.id} login={account.login} />;
    case "agy_oauth":
      return <AgyAccountQuota accountId={account.id} login={account.login} />;
    default:
      return null;
  }
}

function AccountRow({
  provider,
  account,
  inUse,
}: {
  provider: ManagedAuthProvider;
  account: ManagedAuthAccount;
  inUse: boolean;
}) {
  const { t, i18n } = useTranslation();
  const needsReauth = Boolean(account.reauth_required || account.requires_reauth);
  const signedIn = signedInDate(account.authenticated_at, i18n.language);
  const details = [account.plan, signedIn, account.github_domain].filter(
    (detail): detail is string => Boolean(detail && detail.trim()),
  );

  return (
    <li className="flex items-center gap-3 rounded-md border border-border px-3 py-2">
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 flex-wrap items-center gap-1.5">
          <span className="min-w-0 truncate text-sm" title={account.login}>
            {account.login}
          </span>
          {account.is_default && (
            <span className="shrink-0 rounded-full bg-subtle px-1.5 py-0.5 text-badge text-fg-2">
              {t("authCenter.default", { defaultValue: "默认" })}
            </span>
          )}
          {inUse && (
            <span className="shrink-0 rounded-full bg-success-soft px-1.5 py-0.5 text-badge text-success-text">
              {t("suppliers.accountInUse", { defaultValue: "使用中" })}
            </span>
          )}
          {needsReauth && (
            <span className="flex shrink-0 items-center gap-1 text-caption text-warning-text">
              <AlertTriangle aria-hidden="true" className="h-3 w-3" />
              {t("codexOauth.reauthBadge", { defaultValue: "需要重新登录" })}
            </span>
          )}
        </div>
        {details.length > 0 && (
          <div className="mt-0.5 truncate text-caption text-fg-3">
            {details.join(" · ")}
          </div>
        )}
      </div>
      <AccountQuota provider={provider} account={account} />
    </li>
  );
}

export function SupplierAccounts({
  supplier,
  onManageAccounts,
}: {
  supplier: Supplier;
  onManageAccounts?: () => void;
}) {
  const { t } = useTranslation();
  const provider = supplier.managedAuthProvider;
  const manageable = isManagedAuthProvider(provider);
  const boundAccountIds = supplier.boundAccountIds ?? [];
  // 后端认不出认证供应商（老数据 / 未支持的账号池）时不猜，保留原来的凭据计数
  const { data, isPending, isError } = useQuery({
    queryKey: ["supplier-managed-accounts", provider],
    queryFn: () => authListAccounts(provider as ManagedAuthProvider),
    enabled: manageable,
    staleTime: 30_000,
  });

  if (!manageable) return null;
  const accounts = data ?? [];

  return (
    <div className="mt-3 rounded-md border border-border px-3 py-2">
      <div className="flex flex-wrap items-center gap-1.5 text-caption text-fg-3">
        <KeyRound aria-hidden="true" className="h-3.5 w-3.5" />
        <span className="font-medium text-fg-2">
          {t("suppliers.accountsTitle", { defaultValue: "账号" })}
        </span>
        {!isPending && accounts.length > 0 && (
          <span className="rounded-full bg-subtle px-1.5 py-0.5 text-badge text-fg-2">
            {accounts.length}
          </span>
        )}
        <span className="rounded-full bg-subtle px-1.5 py-0.5 text-badge text-fg-2">
          {t("suppliers.credentialKind.managed", {
            defaultValue: "托管登录",
          })}
        </span>
        {onManageAccounts && (
          <Button
            type="button"
            variant="link"
            size="compact"
            className="ms-auto h-auto px-0 text-caption"
            onClick={onManageAccounts}
          >
            {t("suppliers.manageAccounts", { defaultValue: "管理账号" })}
          </Button>
        )}
      </div>

      {isPending ? (
        <div className="flex items-center gap-2 py-2 text-caption text-fg-3">
          <Loader2 aria-hidden="true" className="h-3.5 w-3.5 animate-spin" />
          {t("authCenter.statusLoading", { defaultValue: "正在加载账号…" })}
        </div>
      ) : isError ? (
        <div className="py-2 text-caption text-danger-text">
          {t("suppliers.accountsLoadFailed", {
            defaultValue: "读取账号失败",
          })}
        </div>
      ) : accounts.length === 0 ? (
        <div className="py-2 text-caption text-fg-3">
          {t("suppliers.accountsEmpty", { defaultValue: "这个服务还没有账号" })}
        </div>
      ) : (
        <ul className="m-0 mt-1.5 list-none space-y-1.5 p-0">
          {accounts.map((account) => (
            <AccountRow
              key={account.id}
              provider={provider}
              account={account}
              inUse={boundAccountIds.includes(account.id)}
            />
          ))}
        </ul>
      )}
    </div>
  );
}
