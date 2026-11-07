import { useTranslation } from "react-i18next";
import { AgyOAuthSection } from "@/components/providers/forms/AgyOAuthSection";
import { ClaudeOfficialSection } from "@/components/providers/forms/ClaudeOfficialSection";
import { CodexOAuthSection } from "@/components/providers/forms/CodexOAuthSection";
import { CopilotAuthSection } from "@/components/providers/forms/CopilotAuthSection";
import { XaiOAuthSection } from "@/components/providers/forms/XaiOAuthSection";
import type { ManagedAuthProvider } from "@/lib/api/auth";

/** 供应商卡片能直接嵌入的托管账号服务（授权中心的那几组账号） */
export const MANAGED_AUTH_PROVIDERS: ManagedAuthProvider[] = [
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

/**
 * provider 卡片里的托管账号：把授权中心那一组账号的完整界面（头像、默认 / 需要重新登录、
 * 额度、⋯ 菜单、添加账号）嵌进卡片，只去掉外框、把服务名换成「凭据」。
 *
 * 账号不是另一张卡片的内容，而是这个 provider 的凭据：卡片本身已经是 provider，
 * 这里只负责它的登录态与账号操作。
 */
export function SupplierManagedAccounts({
  provider,
  headerLabel,
  onSetDefaultAccount,
  settingDefaultAccount,
  accountEnabled,
  onToggleAccountEnabled,
  togglingAccountEnabled,
}: {
  provider: ManagedAuthProvider;
  headerLabel?: string;
  /**
   * 供应商卡片里的「设为默认」：托管登录的凭据就是账号池里的某个账号，这里要让这家
   * 供应商真的改用该账号（改绑它的接入），而不是只改账号池的全局默认账号。
   * 不传就还是授权中心的全局默认语义。
   */
  onSetDefaultAccount?: (accountId: string) => void;
  settingDefaultAccount?: boolean;
  /**
   * 供应商页的「停用/启用」：账号留在池里，只是不参与使用。
   * 传了才显示这一项（授权中心自己没有这个概念）。
   */
  accountEnabled?: (accountId: string) => boolean | undefined;
  onToggleAccountEnabled?: (accountId: string, enabled: boolean) => void;
  togglingAccountEnabled?: boolean;
}) {
  const { t } = useTranslation();
  // 这张卡上的「设为默认」改的是这家供应商的账号绑定，不是账号池的全局默认账号，
  // 和授权中心里同一项的意义不同，所以在「?」里补一句。
  const helpNote = onSetDefaultAccount
    ? t("suppliers.credentialsSwitchNote", {
        defaultValue: "这里的「设为默认」会让这家供应商改用该账号。",
      })
    : undefined;

  switch (provider) {
    case "codex_oauth":
      return (
        <CodexOAuthSection
          embedded
          headerLabel={headerLabel}
          showAccountQuota
          onSetDefaultAccount={onSetDefaultAccount}
          settingDefaultAccount={settingDefaultAccount}
          helpNote={helpNote}
          accountEnabled={accountEnabled}
          onToggleAccountEnabled={onToggleAccountEnabled}
          togglingAccountEnabled={togglingAccountEnabled}
        />
      );
    case "claude_official":
      // Claude 官方账号没有「设为默认」：用哪个号由卡片绑定决定，绑定在供应商页改
      return (
        <ClaudeOfficialSection
          embedded
          headerLabel={headerLabel}
          accountEnabled={accountEnabled}
          onToggleAccountEnabled={onToggleAccountEnabled}
          togglingAccountEnabled={togglingAccountEnabled}
        />
      );
    case "github_copilot":
      return (
        <CopilotAuthSection
          embedded
          onSetDefaultAccount={onSetDefaultAccount}
          settingDefaultAccount={settingDefaultAccount}
          helpNote={helpNote}
          accountEnabled={accountEnabled}
          onToggleAccountEnabled={onToggleAccountEnabled}
          togglingAccountEnabled={togglingAccountEnabled}
        />
      );
    case "xai_oauth":
      return (
        <XaiOAuthSection
          embedded
          mode="manage"
          onSetDefaultAccount={onSetDefaultAccount}
          settingDefaultAccount={settingDefaultAccount}
          helpNote={helpNote}
          accountEnabled={accountEnabled}
          onToggleAccountEnabled={onToggleAccountEnabled}
          togglingAccountEnabled={togglingAccountEnabled}
        />
      );
    case "agy_oauth":
      return (
        <AgyOAuthSection
          embedded
          onSetDefaultAccount={onSetDefaultAccount}
          settingDefaultAccount={settingDefaultAccount}
          helpNote={helpNote}
          accountEnabled={accountEnabled}
          onToggleAccountEnabled={onToggleAccountEnabled}
          togglingAccountEnabled={togglingAccountEnabled}
        />
      );
    default:
      return null;
  }
}
