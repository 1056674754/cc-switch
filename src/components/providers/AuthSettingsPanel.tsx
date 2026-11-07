import { useTranslation } from "react-i18next";

import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { DshAccountLogin } from "@/components/providers/DshAccount";
import { KimiAccountLogin } from "@/components/providers/KimiAccountLogin";
import {
  isManagedAuthProvider,
  SupplierManagedAccounts,
} from "@/components/suppliers/SupplierManagedAccounts";
import type { AuthCenterTarget } from "@/lib/api";

interface AuthSettingsPanelProps {
  target: AuthCenterTarget | null;
  onClose: () => void;
}

/** 服务显示名：面板标题用，不再出现「授权中心」这种第二套叫法。 */
function serviceTitle(target: AuthCenterTarget): string | null {
  switch (target) {
    case "kimi_oauth":
      return "Kimi Code";
    case "dsh_account":
      return "DeepSeek Harness";
    case "codex_oauth":
      return "OpenAI Official";
    case "claude_official":
      return "Claude Official";
    case "github_copilot":
      return "GitHub Copilot";
    case "xai_oauth":
      return "xAI";
    case "agy_oauth":
      return "Antigravity";
    default:
      return null;
  }
}

/**
 * 应用页供应商表单里的「管理账号」全屏页。
 *
 * 早期这里嵌的是授权中心（同一批账号的另一套页面），账号因此有两个入口、两套操作习惯。
 * 现在统一成供应商卡片里那套账号区：托管服务走 `SupplierManagedAccounts`，Kimi / DeepSeek
 * Harness 走各自的账号组，都是同一行组件，登录 / 切换 / 停用 / 退出登录语义一致。
 */
export function AuthSettingsPanel({ target, onClose }: AuthSettingsPanelProps) {
  const { t } = useTranslation();
  const isOpen = target !== null;
  const title = (target && serviceTitle(target)) || t("nav.auth");

  return (
    <FullScreenPanel
      isOpen={isOpen}
      title={title}
      onClose={onClose}
      motionPreset="slide-from-right"
    >
      {target ? (
        <div className="px-5 py-4">
          {isManagedAuthProvider(target) ? (
            <SupplierManagedAccounts provider={target} />
          ) : target === "kimi_oauth" ? (
            <KimiAccountLogin />
          ) : (
            <DshAccountLogin />
          )}
        </div>
      ) : null}
    </FullScreenPanel>
  );
}
