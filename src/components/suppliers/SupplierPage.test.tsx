import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { Supplier } from "@/lib/api/suppliers";

const overview = vi.fn();
const updateBaseUrl = vi.fn();
const updateSecret = vi.fn();
const authGetStatus = vi.fn();
const kimiStatus = vi.fn();

vi.mock("@/lib/api/suppliers", () => ({
  suppliersApi: {
    overview: (...args: unknown[]) => overview(...args),
    create: vi.fn(),
    updateBaseUrl: (...args: unknown[]) => updateBaseUrl(...args),
    updateName: vi.fn(),
    updateSecret: (...args: unknown[]) => updateSecret(...args),
    setEnabled: vi.fn(),
    updateBindingModelMap: vi.fn(),
    adopt: vi.fn(),
    remove: vi.fn(),
  },
}));

vi.mock("@/lib/api", () => ({
  authApi: {
    authGetStatus: (...args: unknown[]) => authGetStatus(...args),
  },
}));

vi.mock("@/lib/api/kimiOAuth", () => ({
  kimiOAuthApi: {
    status: (...args: unknown[]) => kimiStatus(...args),
  },
}));

vi.mock("@/lib/toast", () => ({
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn() },
}));

// 卡片底部的模型面板会去查目录和覆盖；这里只验证卡片里的账号组
vi.mock("./SupplierModelsPanel", () => ({ SupplierModelsPanel: () => null }));

// 账号组本身由授权中心的组件负责，这里验证卡片嵌的是对应那一组、标题换成了「凭据」
vi.mock("@/components/providers/forms/CodexOAuthSection", () => ({
  CodexOAuthSection: ({ headerLabel }: { headerLabel?: string }) => (
    <div>CHATGPT-ACCOUNTS:{headerLabel}</div>
  ),
}));
vi.mock("@/components/providers/forms/ClaudeOfficialSection", () => ({
  ClaudeOfficialSection: ({ headerLabel }: { headerLabel?: string }) => (
    <div>CLAUDE-ACCOUNTS:{headerLabel}</div>
  ),
}));
vi.mock("@/components/providers/KimiAccountLogin", () => ({
  KimiAccountLogin: ({ headerLabel }: { headerLabel?: string }) => (
    <div>KIMI-ACCOUNTS:{headerLabel}</div>
  ),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    i18n: { language: "zh-CN" },
    t: (key: string, options?: Record<string, unknown>) => {
      const count = options?.count;
      if (typeof count === "number") return `${key}:${count}`;
      return key;
    },
  }),
}));

import { SupplierPage } from "./SupplierPage";

const supplier = (extra: Partial<Supplier> = {}): Supplier => ({
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
  credentialKeyHashes: [],
  managedAuthProvider: "codex_oauth",
  boundAccountIds: ["acc-default"],
  upstreamIds: [],
  ...extra,
});

function renderPage() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <SupplierPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  overview.mockReset();
  updateBaseUrl.mockReset();
  updateSecret.mockReset();
  authGetStatus.mockReset();
  kimiStatus.mockReset();
  authGetStatus.mockResolvedValue({
    accounts: [{ id: "acc-1" }, { id: "acc-2" }, { id: "acc-3" }],
    currentAccountId: "acc-1",
    authenticated: true,
  });
  kimiStatus.mockResolvedValue({
    accounts: [{ id: "kimi-1" }],
    currentAccountId: "kimi-1",
    primaryHidden: false,
  });
});

describe("供应商页：provider 卡片", () => {
  it("Codex 和 OpenAI Official 只出一张卡，账号就是它的凭据", async () => {
    overview.mockResolvedValue([
      supplier(),
      supplier({
        id: "sup-codex-official",
        name: "OpenAI Official",
        host: "",
        baseUrl: "",
        upstreamProtocol: "openai_responses",
        apps: ["codex"],
        consumers: ["codex:codex-official"],
        bindings: [
          {
            appType: "codex",
            providerId: "codex-official",
            modelMap: {},
            downstreamProtocol: "openai_responses",
          },
        ],
      }),
    ]);

    renderPage();

    expect(
      await screen.findByText("CHATGPT-ACCOUNTS:suppliers.credentialsTitle"),
    ).toBeTruthy();
    // 同一个 ChatGPT 账号服务下的两个官方接入合成一张卡，不再多一张 OpenAI Official
    expect(screen.queryByText("OpenAI Official")).toBeNull();
    // 被哪些 app 使用：两个接入合成后都在
    expect(screen.getByText("Claude Desktop")).toBeTruthy();
    // 「被使用」里有 Codex（卡片标题也叫 Codex，所以这里允许不止一个）
    expect(screen.getAllByText("Codex").length).toBeGreaterThan(0);
    // 托管账号不是「1 个凭据」，也没有换密钥入口
    expect(screen.queryByText("suppliers.changeSecret")).toBeNull();
  });

  it("没登录的托管 provider 不显示", async () => {
    authGetStatus.mockResolvedValue({
      accounts: [],
      currentAccountId: null,
      authenticated: false,
    });
    overview.mockResolvedValue([
      supplier(),
      supplier({
        id: "sup-openai-official",
        name: "OpenAI Official",
        host: "",
        baseUrl: "",
        apps: ["codex"],
        consumers: ["codex:codex-official"],
        bindings: [],
      }),
    ]);

    renderPage();

    expect(
      await screen.findByText("suppliers.empty", { exact: false }),
    ).toBeTruthy();
    expect(
      screen.queryByText("CHATGPT-ACCOUNTS:suppliers.credentialsTitle"),
    ).toBeNull();
  });

  it("Claude 官方登录有账号才出卡，且账号组标题是「凭据」", async () => {
    authGetStatus.mockResolvedValue({
      accounts: [{ id: "claude-1" }],
      currentAccountId: null,
      authenticated: true,
    });
    overview.mockResolvedValue([
      supplier({
        id: "sup-claude-official",
        name: "Claude Official",
        host: "",
        baseUrl: "",
        upstreamProtocol: "anthropic",
        apps: ["claude"],
        consumers: ["claude:claude-official"],
        bindings: [],
        managedAuthProvider: "claude_official",
        credentialKinds: ["managed"],
        boundAccountIds: ["claude-1"],
      }),
    ]);

    renderPage();

    expect(
      await screen.findByText("CLAUDE-ACCOUNTS:suppliers.credentialsTitle"),
    ).toBeTruthy();
  });

  it("Kimi 一张卡里同时有托管账号和密钥", async () => {
    overview.mockResolvedValue([
      supplier({
        id: "sup-kimi-key",
        name: "Kimi",
        host: "api.kimi.com",
        baseUrl: "https://api.kimi.com/coding",
        upstreamProtocol: "anthropic",
        apps: ["claude"],
        consumers: ["claude:kimi-link"],
        bindings: [
          {
            appType: "claude",
            providerId: "kimi-link",
            modelMap: {},
            downstreamProtocol: "anthropic",
          },
        ],
        credentialCount: 4,
        credentialKinds: ["literal"],
        credentialKeyHashes: ["kimi-key-1", "kimi-key-2"],
        managedAuthProvider: null,
        boundAccountIds: [],
      }),
      supplier({
        id: "sup-kimi-managed",
        name: "Kimi",
        host: "api.kimi.com",
        baseUrl: "https://api.kimi.com/coding/v1",
        upstreamProtocol: "kimi_managed",
        apps: ["kimi"],
        consumers: ["kimi:managed:kimi-code"],
        bindings: [],
        credentialCount: 1,
        credentialKinds: ["managed"],
        credentialKeyHashes: [],
        managedAuthProvider: null,
        boundAccountIds: [],
      }),
    ]);

    renderPage();

    expect(
      await screen.findByText("KIMI-ACCOUNTS:suppliers.credentialsTitle"),
    ).toBeTruthy();
    // 同一个 Key 在多个 app 里存了多份，只算一把
    expect(screen.getByText("suppliers.keyCount:2")).toBeTruthy();
    // 换密钥收进编辑弹窗，卡片上不再有独立入口
    expect(screen.queryByText("suppliers.changeSecret")).toBeNull();
  });

  it("同名同服务根的两个协议合成一张卡（阿里云百炼）", async () => {
    overview.mockResolvedValue([
      supplier({
        id: "sup-aliyun-1",
        name: "阿里云百炼",
        host: "dashscope.aliyuncs.com",
        baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        upstreamProtocol: "alibaba",
        apps: ["opencode"],
        consumers: ["opencode:aliyun"],
        bindings: [],
        credentialCount: 1,
        credentialKinds: ["literal"],
        credentialKeyHashes: ["ali-key"],
        managedAuthProvider: null,
        boundAccountIds: [],
      }),
      supplier({
        id: "sup-aliyun-2",
        name: "阿里云百炼",
        host: "dashscope.aliyuncs.com",
        baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        upstreamProtocol: "openai_chat",
        apps: ["opencode"],
        consumers: ["opencode:aliyun-2"],
        bindings: [],
        credentialCount: 1,
        credentialKinds: ["literal"],
        credentialKeyHashes: ["ali-key"],
        managedAuthProvider: null,
        boundAccountIds: [],
      }),
    ]);

    renderPage();

    expect(await screen.findByText("阿里云百炼")).toBeTruthy();
    expect(screen.getAllByText("阿里云百炼")).toHaveLength(1);
    expect(screen.getByText("suppliers.keyCount:1")).toBeTruthy();
  });

  it("改地址、换密钥会写回同一张卡底下的每条供应商行", async () => {
    const aliyun = [
      supplier({
        id: "sup-aliyun-1",
        name: "阿里云百炼",
        baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        upstreamProtocol: "alibaba",
        apps: ["opencode"],
        consumers: ["opencode:aliyun"],
        bindings: [],
        credentialKinds: ["literal"],
        credentialKeyHashes: ["ali-key"],
        managedAuthProvider: null,
      }),
      supplier({
        id: "sup-aliyun-2",
        name: "阿里云百炼",
        baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        upstreamProtocol: "openai_chat",
        apps: ["opencode"],
        consumers: ["opencode:aliyun-2"],
        bindings: [],
        credentialKinds: ["literal"],
        managedAuthProvider: null,
      }),
    ];
    overview.mockResolvedValue(aliyun);
    updateBaseUrl.mockResolvedValue(aliyun);
    updateSecret.mockResolvedValue(aliyun);

    renderPage();
    await screen.findByText("阿里云百炼");

    const user = userEvent.setup();
    await user.click(
      screen.getByRole("button", { name: "suppliers.editAction" }),
    );
    const input = screen.getByLabelText("suppliers.baseUrlLabel");
    await user.clear(input);
    await user.type(input, "https://next.example.com/v1");
    const secret = screen.getByLabelText("suppliers.newSecretLabel");
    await user.type(secret, "sk-new");
    await user.click(screen.getByRole("button", { name: "common.save" }));

    expect(updateBaseUrl).toHaveBeenCalledTimes(2);
    expect(updateBaseUrl).toHaveBeenCalledWith(
      "sup-aliyun-1",
      "https://next.example.com/v1",
    );
    expect(updateBaseUrl).toHaveBeenCalledWith(
      "sup-aliyun-2",
      "https://next.example.com/v1",
    );
    expect(updateSecret).toHaveBeenCalledTimes(2);
    expect(updateSecret).toHaveBeenCalledWith("sup-aliyun-1", "sk-new");
    expect(updateSecret).toHaveBeenCalledWith("sup-aliyun-2", "sk-new");
  });

  it("没有任何凭据的占位供应商不显示", async () => {
    overview.mockResolvedValue([
      supplier({
        id: "sup-default",
        name: "default",
        host: "",
        baseUrl: "",
        upstreamProtocol: "anthropic",
        apps: ["claude"],
        consumers: ["claude:default"],
        bindings: [],
        credentialCount: 0,
        credentialKinds: ["none"],
        managedAuthProvider: null,
        boundAccountIds: [],
      }),
      supplier({
        id: "sup-google",
        name: "Google Official",
        host: "",
        baseUrl: "",
        upstreamProtocol: "gemini_native",
        apps: ["gemini"],
        consumers: ["gemini:google-official"],
        bindings: [],
        credentialCount: 1,
        credentialKinds: ["managed"],
        managedAuthProvider: null,
        boundAccountIds: [],
      }),
    ]);

    renderPage();

    expect(
      await screen.findByText("suppliers.empty", { exact: false }),
    ).toBeTruthy();
    expect(screen.queryByText("default")).toBeNull();
    expect(screen.queryByText("Google Official")).toBeNull();
  });
});
