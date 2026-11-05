import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ManagedAuthAccount } from "@/lib/api/auth";
import type { Supplier } from "@/lib/api/suppliers";

const overview = vi.fn();
const authListAccounts = vi.fn();

vi.mock("@/lib/api/suppliers", () => ({
  suppliersApi: {
    overview: (...args: unknown[]) => overview(...args),
    create: vi.fn(),
    updateBaseUrl: vi.fn(),
    updateSecret: vi.fn(),
    setEnabled: vi.fn(),
    updateBindingModelMap: vi.fn(),
    adopt: vi.fn(),
    remove: vi.fn(),
  },
}));

vi.mock("@/lib/api/auth", () => ({
  authListAccounts: (...args: unknown[]) => authListAccounts(...args),
}));

vi.mock("@/lib/toast", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

// 卡片底部的模型面板会去查目录和覆盖；这里只验证账号列表
vi.mock("./SupplierModelsPanel", () => ({ SupplierModelsPanel: () => null }));

// 额度条会去查订阅接口；这里验证的是供应商卡片把账号列出来，额度组件替换为空
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

import { SupplierPage } from "./SupplierPage";

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

const managedSupplier: Supplier = {
  id: "sup-codex",
  name: "Codex",
  host: "chatgpt.com",
  baseUrl: "https://chatgpt.com/backend-api/codex",
  upstreamProtocol: "openai_responses",
  enabled: true,
  apps: ["claude-desktop"],
  consumers: ["claude-desktop:codex-link"],
  bindings: [
    {
      appType: "claude-desktop",
      providerId: "codex-link",
      modelMap: {},
      downstreamProtocol: "anthropic",
    },
  ],
  credentialCount: 1,
  credentialKinds: ["managed"],
  managedAuthProvider: "codex_oauth",
  boundAccountIds: ["acc-default"],
  upstreamIds: [],
};

function renderPage(onOpenAuthCenter = vi.fn()) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return {
    onOpenAuthCenter,
    ...render(
      <QueryClientProvider client={client}>
        <SupplierPage onOpenAuthCenter={onOpenAuthCenter} />
      </QueryClientProvider>,
    ),
  };
}

describe("SupplierPage 托管登录供应商卡片", () => {
  it("列出账号池里的 3 个账号，不再只显示「1 个凭据」", async () => {
    overview.mockResolvedValue([managedSupplier]);
    authListAccounts.mockResolvedValue([
      account("acc-default", "devgroup@suisqp.com", { is_default: true }),
      account("acc-teaching", "ai.teachingResearch@suisqp.com"),
      account("acc-dev2", "devgroup2@suisqp.com"),
    ]);

    renderPage();

    expect(await screen.findByText("Codex")).toBeTruthy();
    expect(await screen.findByText("devgroup@suisqp.com")).toBeTruthy();
    expect(screen.getByText("ai.teachingResearch@suisqp.com")).toBeTruthy();
    expect(screen.getByText("devgroup2@suisqp.com")).toBeTruthy();
    expect(screen.getByText("账号")).toBeTruthy();
    expect(screen.getByText("默认")).toBeTruthy();
    expect(screen.getByText("使用中")).toBeTruthy();
    // 托管登录没有密钥可换，也不该再出现「1 个凭据」
    expect(screen.queryByText(/个凭据/)).toBeNull();
    expect(screen.queryByText("换密钥")).toBeNull();
  });

  it("「管理账号」跳授权中心", async () => {
    overview.mockResolvedValue([managedSupplier]);
    authListAccounts.mockResolvedValue([
      account("acc-default", "devgroup@suisqp.com", { is_default: true }),
    ]);
    const user = userEvent.setup();
    const { onOpenAuthCenter } = renderPage();

    await user.click(await screen.findByText("管理账号"));
    expect(onOpenAuthCenter).toHaveBeenCalledTimes(1);
  });
});
