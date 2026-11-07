import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Switch } from "@/components/ui/switch";
import { useAuthOpenBrowser, writeAuthOpenBrowser } from "@/lib/authLoginLink";

/**
 * 「登录时自动打开浏览器」：所有托管账号服务的登录共用。关掉后只复制登录链接，用户自己选
 * 浏览器打开（用独立浏览器环境管多个账号时，不让登录混进系统默认浏览器）。
 */
export function OpenBrowserToggle() {
  const { t } = useTranslation();
  const openBrowser = useAuthOpenBrowser();
  const titleId = useId();
  const descriptionId = useId();
  return (
    <div className="flex shrink-0 items-center gap-4 rounded-panel border border-border bg-surface py-3 pe-4 ps-4">
      <div className="min-w-0 flex-1 space-y-0.5">
        <p id={titleId} className="m-0 text-body font-medium text-fg-1">
          {t("authCenter.openBrowser.title", {
            defaultValue: "登录时自动打开浏览器",
          })}
        </p>
        <p id={descriptionId} className="m-0 text-caption text-fg-2">
          {t("authCenter.openBrowser.description", {
            defaultValue:
              "关掉后只复制登录链接，由你自己选浏览器打开，适合用独立的浏览器环境管理多个账号。",
          })}
        </p>
      </div>
      <Switch
        checked={openBrowser}
        onCheckedChange={writeAuthOpenBrowser}
        aria-labelledby={titleId}
        aria-describedby={descriptionId}
      />
    </div>
  );
}

