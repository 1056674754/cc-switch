import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ManagedAuthAccount } from "@/lib/api/auth";
import type { Supplier } from "@/lib/api/suppliers";

const authListAccounts = vi.fn();

vi.mock("@/lib/api/auth", () => ({
  authListAccounts: (...args: unknown[]) => authListAccounts(...args),
}));

// 额度条各自会去查订阅接口，这里只验证账号列表，替换成空组件
vi.mock("@/components/CodexOauthAccountQuota", () => ({
  default: () => null,
}));
vi.mock("@/components/settings/auth/AccountQuota", () => ({
  AgyAccountQuota: () => null,
  ClaudeAccountQuota: () => null,
  CopilotAccountQuota: () => null,
  XaiAccountQuota: () => null,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    i18n: { language: "zh-CN" },
    t: (_key: string, options?: Record<string, unknown>) => {
      const raw = (options?.defaultValue as string | undefined) ?? _key;
      return Object.entries(options ?? {}).reduce(
        (text, [name, value]) =>
          text.replace(new RegExp(`\\{\\{${name}\\}\\}`, "g"), String(value)),
        raw,
      );
    },
  }),
}));

import { SupplierAccounts } from "./SupplierAccounts";

const account = (
  id: string,
  login: string,
  extra: Partial<ManagedAuthAccount> = {},
): ManagedAuthAccount => ({
  id,
  provider: "codex_oauth",
  login,
  avatar_url: null,
  authenticated_at: 1793416881,
  is_default: false,
  github_domain: "",
  requires_reauth: false,
  ...extra,
});

const supplier: Supplier = {
  id: "sup-codex",
  name: "Codex",
  host: "chatgpt.com",
  baseUrl: "https://chatgpt.com/backend-api/codex",
  upstreamProtocol: "openai_responses",
  enabled: true,
  apps: ["claude-desktop"],
  consumers: ["claude-desktop:codex-link"],
  bindings: [],
  credentialCount: 1,
  credentialKinds: ["managed"],
  managedAuthProvider: "codex_oauth",
  boundAccountIds: ["acc-default"],
  upstreamIds: [],
};

function renderAccounts(target: Supplier = supplier, onManage?: () => void) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <SupplierAccounts supplier={target} onManageAccounts={onManage} />
    </QueryClientProvider>,
  );
}

describe("SupplierAccounts", () => {
  it("把托管账号池里的账号逐条列出来，并标默认 / 使用中 / 需要重新登录", async () => {
    authListAccounts.mockResolvedValue([
      account("acc-default", "devgroup@suisqp.com", { is_default: true }),
      account("acc-teaching", "ai.teachingResearch@suisqp.com"),
      account("acc-dev2", "devgroup2@suisqp.com", {
        reauth_required: true,
        authenticated_at: 1792904208,
      }),
    ]);

    renderAccounts();

    expect(await screen.findByText("devgroup@suisqp.com")).toBeTruthy();
    expect(screen.getByText("ai.teachingResearch@suisqp.com")).toBeTruthy();
    expect(screen.getByText("devgroup2@suisqp.com")).toBeTruthy();
    // 标题是「账号」+ 数量，而不是「1 个凭据」
    expect(screen.getByText("账号")).toBeTruthy();
    expect(screen.getByText("3")).toBeTruthy();
    expect(screen.queryByText(/个凭据/)).toBeNull();
    // 默认账号 / 被供应商绑定的账号 / 失效账号都有标记
    expect(screen.getByText("默认")).toBeTruthy();
    expect(screen.getByText("使用中")).toBeTruthy();
    expect(screen.getByText("需要重新登录")).toBeTruthy();
  });

  it("点「管理账号」走传进来的入口", async () => {
    authListAccounts.mockResolvedValue([account("acc-default", "a@b.com")]);
    const onManage = vi.fn();
    const user = userEvent.setup();
    renderAccounts(supplier, onManage);

    await user.click(await screen.findByText("管理账号"));
    expect(onManage).toHaveBeenCalledTimes(1);
  });

  it("认不出认证供应商时不渲染，保留原来的凭据计数", () => {
    const { container } = renderAccounts({
      ...supplier,
      credentialKinds: ["none"],
      managedAuthProvider: null,
      boundAccountIds: [],
    });
    expect(container.firstChild).toBeNull();
    expect(authListAccounts).not.toHaveBeenCalled();
  });
});
