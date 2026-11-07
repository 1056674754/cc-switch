import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { SupplierEditDialog } from "@/components/suppliers/SupplierEditDialog";
import { gatewayApi } from "@/lib/api/gateway";
import type { ProviderGroup } from "@/components/suppliers/providerGrouping";
import type { Supplier } from "@/lib/api/suppliers";

const mocks = vi.hoisted(() => ({
  updateCredential: vi.fn(),
  addCredential: vi.fn(),
  deleteCredential: vi.fn(),
  setCredentialEnabled: vi.fn(),
  addAccount: vi.fn(),
  deleteAccount: vi.fn(),
  setAccountEnabled: vi.fn(),
  setCurrentAccount: vi.fn(),
  authGetStatus: vi.fn(),
  checkProviderModels: vi.fn(),
  updateName: vi.fn(),
  updateBaseUrl: vi.fn(),
  updateSecret: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
}));

vi.mock("@/lib/api/suppliers", () => ({
  suppliersApi: {
    updateCredential: (...args: unknown[]) => mocks.updateCredential(...args),
    addCredential: (...args: unknown[]) => mocks.addCredential(...args),
    deleteCredential: (...args: unknown[]) => mocks.deleteCredential(...args),
    setCredentialEnabled: (...args: unknown[]) =>
      mocks.setCredentialEnabled(...args),
    addAccount: (...args: unknown[]) => mocks.addAccount(...args),
    deleteAccount: (...args: unknown[]) => mocks.deleteAccount(...args),
    setAccountEnabled: (...args: unknown[]) => mocks.setAccountEnabled(...args),
    setCurrentAccount: (...args: unknown[]) => mocks.setCurrentAccount(...args),
    updateName: (...args: unknown[]) => mocks.updateName(...args),
    updateBaseUrl: (...args: unknown[]) => mocks.updateBaseUrl(...args),
    updateSecret: (...args: unknown[]) => mocks.updateSecret(...args),
  },
}));

vi.mock("@/lib/api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api")>();
  return {
    ...actual,
    authApi: {
      ...actual.authApi,
      authGetStatus: (...args: unknown[]) => mocks.authGetStatus(...args),
      authSetDefaultAccount: vi.fn().mockResolvedValue(undefined),
    },
  };
});

// 托管账号区在卡片和弹窗里是同一个组件（授权中心那套）；这里只验证弹窗嵌的是它，
// 以及「设为默认」确实走供应商级切换。
vi.mock("@/components/providers/forms/CodexOAuthSection", () => ({
  CodexOAuthSection: (props: {
    onSetDefaultAccount?: (accountId: string) => void;
    accountEnabled?: (accountId: string) => boolean | undefined;
    onToggleAccountEnabled?: (accountId: string, enabled: boolean) => void;
  }) => (
    <div>
      CHATGPT-ACCOUNTS
      <span>ENABLED-{String(props.accountEnabled?.("acc-a"))}</span>
      <span>UNKNOWN-{String(props.accountEnabled?.("acc-z"))}</span>
      <button
        type="button"
        onClick={() => props.onSetDefaultAccount?.("acc-b")}
      >
        SET-DEFAULT-ACC-B
      </button>
      <button
        type="button"
        onClick={() => props.onToggleAccountEnabled?.("acc-a", false)}
      >
        DISABLE-ACC-A
      </button>
    </div>
  ),
}));

vi.mock("@/lib/api/gateway", () => ({
  gatewayApi: {
    getCatalog: vi.fn().mockResolvedValue([]),
    getModelOverrides: vi.fn().mockResolvedValue([]),
    setModelOverride: vi.fn().mockResolvedValue(undefined),
    fetchSupplierModels: vi
      .fn()
      .mockResolvedValue({ upstreamIds: ["up-1"], fetched: 0, added: 0 }),
  },
}));

vi.mock("@/lib/api/connectivity-check", () => ({
  checkProviderModels: (...args: unknown[]) =>
    mocks.checkProviderModels(...args),
}));

vi.mock("@/lib/toast", () => ({
  toast: {
    success: (...args: unknown[]) => mocks.success(...args),
    error: (...args: unknown[]) => mocks.error(...args),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      if (!options) return key;
      return `${key}:${Object.entries(options)
        .map(([, value]) => String(value))
        .join("|")}`;
    },
  }),
}));

const supplier: Supplier = {
  id: "sup-1",
  name: "Pool Gateway",
  host: "pool.example.com",
  baseUrl: "https://pool.example.com/v1",
  upstreamProtocol: "anthropic",
  enabled: true,
  apps: ["claude", "claude-desktop"],
  consumers: ["claude:a", "claude-desktop:b"],
  credentialCount: 2,
  credentialKinds: ["literal"],
  credentialKeyHashes: ["h1", "h2"],
  credentials: [
    {
      keyHash: "h1",
      maskedSecret: "sk-…1111",
      secretKind: "literal",
      envName: null,
      apps: [
        {
          appType: "claude",
          providerId: "a",
          providerName: "A",
          inFailoverQueue: true,
        },
      ],
    },
    {
      keyHash: "h2",
      maskedSecret: "sk-…2222",
      secretKind: "literal",
      envName: null,
      apps: [
        {
          appType: "claude-desktop",
          providerId: "b",
          providerName: "B",
          inFailoverQueue: true,
        },
      ],
    },
  ],
  bindings: [
    {
      appType: "claude",
      providerId: "a",
      modelMap: {},
      downstreamProtocol: "anthropic",
    },
    {
      appType: "claude-desktop",
      providerId: "b",
      modelMap: {},
      downstreamProtocol: "anthropic",
    },
  ],
  upstreamIds: ["up-1"],
};

const group: ProviderGroup = {
  id: "key:Pool Gateway|https://pool.example.com",
  name: "Pool Gateway",
  managedAuthProvider: null,
  suppliers: [supplier],
  primary: supplier,
  apps: ["claude", "claude-desktop"],
  bindings: supplier.bindings,
  credentialKinds: ["literal"],
  credentialCount: 2,
  keyHashes: ["h1", "h2"],
  distinctKeyCount: 2,
  credentials: supplier.credentials ?? [],
  upstreamIds: ["up-1"],
  kimiManaged: false,
};

function catalogEntry(modelId: string, upstreamId = "up-1") {
  return {
    reference: `Pool/${modelId}`,
    upstreamId,
    upstreamName: "Pool",
    modelId,
    displayName: null,
    oneM: false,
    window: null,
    endpointIds: ["e1"],
    sources: ["claude:a"],
  };
}

const managedSupplier: Supplier = {
  ...supplier,
  id: "sup-codex",
  name: "Codex",
  host: "chatgpt.com",
  baseUrl: "",
  apps: ["claude-desktop", "codex"],
  consumers: ["claude-desktop:codex-link", "codex:codex-official"],
  credentialCount: 2,
  credentialKinds: ["managed"],
  credentialKeyHashes: [],
  managedAuthProvider: "codex_oauth",
  boundAccountIds: ["acc-a"],
  credentials: [
    {
      keyHash: "account:acc-a",
      maskedSecret: "",
      secretKind: "managed",
      envName: null,
      accountId: "acc-a",
      apps: [
        {
          appType: "claude-desktop",
          providerId: "codex-link",
          providerName: "Codex",
          inFailoverQueue: false,
          isCurrent: true,
        },
        {
          appType: "codex",
          providerId: "codex-official",
          providerName: "OpenAI Official",
          inFailoverQueue: false,
          isCurrent: true,
        },
      ],
    },
  ],
  bindings: [
    {
      appType: "claude-desktop",
      providerId: "codex-link",
      modelMap: {},
      downstreamProtocol: "anthropic",
    },
    {
      appType: "codex",
      providerId: "codex-official",
      modelMap: {},
      downstreamProtocol: "codex",
    },
  ],
};

const managedGroup: ProviderGroup = {
  id: "managed:codex_oauth",
  name: "Codex",
  managedAuthProvider: "codex_oauth",
  suppliers: [managedSupplier],
  primary: managedSupplier,
  apps: ["claude-desktop", "codex"],
  bindings: managedSupplier.bindings,
  credentialKinds: ["managed"],
  credentialCount: 2,
  keyHashes: [],
  distinctKeyCount: 0,
  credentials: managedSupplier.credentials ?? [],
  upstreamIds: ["up-codex"],
  kimiManaged: false,
};

function authStatus() {
  return {
    provider: "codex_oauth",
    authenticated: true,
    default_account_id: "acc-a",
    accounts: [
      {
        id: "acc-a",
        provider: "codex_oauth",
        login: "a@example.com",
        avatar_url: null,
        authenticated_at: 0,
        is_default: true,
        github_domain: "",
        requires_reauth: false,
      },
      {
        id: "acc-b",
        provider: "codex_oauth",
        login: "b@example.com",
        avatar_url: null,
        authenticated_at: 0,
        is_default: false,
        github_domain: "",
        requires_reauth: false,
      },
    ],
  };
}

function renderDialog(onClose = vi.fn(), groupOverride: ProviderGroup = group) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const onOverview = vi.fn();
  render(
    <QueryClientProvider client={client}>
      <SupplierEditDialog
        group={groupOverride}
        onClose={onClose}
        onOverview={onOverview}
      />
    </QueryClientProvider>,
  );
  return { onClose, onOverview };
}

describe("SupplierEditDialog", () => {
  beforeEach(() => {
    for (const mock of Object.values(mocks)) mock.mockReset();
    mocks.updateCredential.mockResolvedValue([]);
    mocks.addCredential.mockResolvedValue([]);
    mocks.deleteCredential.mockResolvedValue([]);
    mocks.setCredentialEnabled.mockResolvedValue([]);
    mocks.addAccount.mockResolvedValue([]);
    mocks.deleteAccount.mockResolvedValue([]);
    mocks.setAccountEnabled.mockResolvedValue([]);
    mocks.setCurrentAccount.mockResolvedValue([]);
    mocks.authGetStatus.mockResolvedValue(authStatus());
    mocks.checkProviderModels.mockResolvedValue([
      { model: "m1", success: true, message: "可用", responseTimeMs: 30 },
      { model: "m2", success: false, message: "HTTP 401: bad key" },
    ]);
    // 网关相关的 mock 定义在 vi.mock 工厂里，不受上面那轮 reset 影响，单独复位。
    vi.mocked(gatewayApi.getCatalog).mockReset().mockResolvedValue([]);
    vi.mocked(gatewayApi.getModelOverrides).mockReset().mockResolvedValue([]);
    vi.mocked(gatewayApi.setModelOverride)
      .mockReset()
      .mockResolvedValue(undefined);
    vi.mocked(gatewayApi.fetchSupplierModels)
      .mockReset()
      .mockResolvedValue({ upstreamIds: ["up-1"], fetched: 0, added: 0 });
  });

  it("shows both keys in the pool with masked values and their apps", () => {
    renderDialog();

    expect(screen.getByText("suppliers.editTitle")).toBeInTheDocument();
    expect(screen.getByText("sk-…1111")).toBeInTheDocument();
    expect(screen.getByText("sk-…2222")).toBeInTheDocument();
    // 默认两把 Key 进入密钥池模式
    expect(
      screen.getByRole("button", { name: "suppliers.poolTitle" }),
    ).toHaveAttribute("aria-pressed", "true");
    expect(
      screen.getAllByRole("button", { name: "suppliers.deleteKey" }),
    ).toHaveLength(2);
  });

  it("replaces only the selected key", async () => {
    const { onOverview } = renderDialog();
    const user = userEvent.setup();

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.replaceKey" })[0],
    );
    await user.type(
      screen.getByPlaceholderText("suppliers.replaceSecretPlaceholder"),
      "sk-new-1111",
    );
    await user.click(screen.getByRole("button", { name: "suppliers.saveKey" }));

    await waitFor(() =>
      expect(mocks.updateCredential).toHaveBeenCalledWith(
        "sup-1",
        "h1",
        "sk-new-1111",
      ),
    );
    expect(onOverview).toHaveBeenCalledWith([]);
  });

  it("adds a key for the chosen app and reports the target app", async () => {
    renderDialog();
    const user = userEvent.setup();

    await user.selectOptions(
      screen.getByLabelText("suppliers.addKeyAppLabel"),
      "claude",
    );
    await user.type(screen.getByLabelText("suppliers.addKeyLabel"), "sk-added");
    await user.click(screen.getByRole("button", { name: "suppliers.addKey" }));

    await waitFor(() =>
      expect(mocks.addCredential).toHaveBeenCalledWith(
        "sup-1",
        "claude",
        "sk-added",
      ),
    );
  });

  it("asks before deleting a key and then deletes only that key", async () => {
    renderDialog();
    const user = userEvent.setup();

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.deleteKey" })[1],
    );
    expect(
      screen.getByText(/suppliers.deleteKeyConfirm:Claude Desktop/),
    ).toBeInTheDocument();
    const confirmButtons = screen.getAllByRole("button", {
      name: "suppliers.deleteKey",
    });
    await user.click(confirmButtons[confirmButtons.length - 1]);

    await waitFor(() =>
      expect(mocks.deleteCredential).toHaveBeenCalledWith("sup-1", "h2"),
    );
  });

  it("guards closing when identity fields are dirty", async () => {
    const { onClose } = renderDialog();
    const user = userEvent.setup();

    await user.type(screen.getByLabelText("suppliers.nameLabel"), "-edited");
    await user.click(screen.getByRole("button", { name: "common.close" }));

    expect(screen.getByText("suppliers.discardTitle")).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();

    await user.click(
      screen.getByRole("button", { name: "suppliers.discardConfirm" }),
    );
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
  });

  it("pulls models from the upstream when the refresh button is used", async () => {
    vi.mocked(gatewayApi.getCatalog).mockResolvedValue([
      catalogEntry("gpt-6-astra"),
    ]);
    vi.mocked(gatewayApi.fetchSupplierModels).mockResolvedValue({
      upstreamIds: ["up-1"],
      fetched: 3,
      added: 2,
    });
    renderDialog();
    const user = userEvent.setup();

    // 等目录真正落到界面上再点：目录为空时会渲染另一颗「刷新」按钮
    await screen.findAllByText("gpt-6-astra");
    await user.click(screen.getByRole("button", { name: "common.refresh" }));

    await waitFor(() =>
      expect(gatewayApi.fetchSupplierModels).toHaveBeenCalledWith(["up-1"]),
    );
    // 含隐藏条目的目录必须走独立查询，共享缓存保持可见目录
    expect(gatewayApi.getCatalog).toHaveBeenCalledWith({ includeHidden: true });
    await waitFor(() => expect(mocks.success).toHaveBeenCalled());
    expect(mocks.success.mock.calls[0][0]).toContain(
      "suppliers.modelsFetchedAdded",
    );
  });

  it("offers a refresh entry when the catalog comes back empty", async () => {
    vi.mocked(gatewayApi.getCatalog).mockResolvedValue([]);
    renderDialog();
    const user = userEvent.setup();

    await user.click(
      await screen.findByRole("button", { name: "common.refresh" }),
    );

    await waitFor(() =>
      expect(gatewayApi.fetchSupplierModels).toHaveBeenCalledWith(["up-1"]),
    );
    // 一条都没新增时退回不带「新增」的文案，别弹「新增 0 个」
    await waitFor(() => expect(mocks.success).toHaveBeenCalled());
    expect(mocks.success.mock.calls[0][0]).toContain(
      "suppliers.modelsFetched:",
    );
  });

  it("lists hidden models on the left and re-adds them by click", async () => {
    vi.mocked(gatewayApi.getCatalog).mockResolvedValue([
      catalogEntry("gpt-visible"),
      catalogEntry("gpt-hidden"),
    ]);
    vi.mocked(gatewayApi.getModelOverrides).mockResolvedValue([
      {
        upstreamId: "up-1",
        modelId: "gpt-hidden",
        displayName: null,
        hidden: true,
      },
    ]);
    renderDialog();
    const user = userEvent.setup();

    const row = await screen.findByRole("button", { name: /gpt-hidden/ });
    expect(within(row).getByText("suppliers.modelAdd")).toBeInTheDocument();
    // 左栏只放还没启用的候选：已启用的那条在右栏，不该再出现在左边（否则「启用没启用」看不出来）
    expect(
      screen.queryByRole("button", { name: /gpt-visible/ }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("suppliers.modelAdded")).not.toBeInTheDocument();

    await user.click(row);

    await waitFor(() =>
      expect(gatewayApi.setModelOverride).toHaveBeenCalledWith({
        upstreamId: "up-1",
        modelId: "gpt-hidden",
        displayName: null,
        hidden: false,
      }),
    );
  });

  it("renders the same managed-account section as the supplier card", async () => {
    renderDialog(vi.fn(), managedGroup);

    expect(await screen.findByText("CHATGPT-ACCOUNTS")).toBeInTheDocument();
    // 旧弹窗那套「只列已绑定账号 + 自己加账号」不再出现
    expect(screen.queryByLabelText(/suppliers\.addAccountLabel/)).toBeNull();
  });

  it("passes account availability and the toggle handler to the account section", async () => {
    mocks.setAccountEnabled.mockResolvedValue([]);
    const { onOverview } = renderDialog(vi.fn(), managedGroup);
    const user = userEvent.setup();

    // 正在使用的账号 = 启用；池里没有的账号不给状态（不显示停用/启用）
    expect(await screen.findByText("ENABLED-true")).toBeInTheDocument();
    expect(screen.getByText("UNKNOWN-undefined")).toBeInTheDocument();

    await user.click(screen.getByText("DISABLE-ACC-A"));
    await waitFor(() =>
      expect(mocks.setAccountEnabled).toHaveBeenCalledWith(
        ["sup-codex"],
        "acc-a",
        false,
      ),
    );
    expect(onOverview).toHaveBeenCalledWith([]);
  });

  it("switches accounts through the supplier-level handler", async () => {
    const withBothAccounts: ProviderGroup = {
      ...managedGroup,
      credentials: [
        ...managedGroup.credentials,
        {
          keyHash: "account:acc-b",
          maskedSecret: "",
          secretKind: "managed",
          envName: null,
          accountId: "acc-b",
          apps: [
            {
              appType: "codex",
              providerId: "sup-codex",
              providerName: "Codex",
              inFailoverQueue: false,
            },
          ],
        },
      ],
    };
    mocks.setCurrentAccount.mockResolvedValue([]);
    const { onOverview } = renderDialog(vi.fn(), withBothAccounts);
    const user = userEvent.setup();

    await user.click(await screen.findByText("SET-DEFAULT-ACC-B"));

    await waitFor(() =>
      expect(mocks.setCurrentAccount).toHaveBeenCalledWith(
        ["sup-codex"],
        "acc-b",
      ),
    );
    expect(onOverview).toHaveBeenCalledWith([]);
  });

  it("merges the same model coming from several upstreams into one row", async () => {
    // Codex 订阅同时挂 chatgpt.com 和官方托管上游：两条上游都回同一份模型名。
    // 左栏按模型名去重，且已启用的模型不能因为另一条上游还没启用就又冒出来。
    const twoUpstreams = { ...group, upstreamIds: ["up-1", "up-2"] };
    vi.mocked(gatewayApi.getCatalog).mockResolvedValue([
      catalogEntry("gpt-6-astra", "up-1"),
      catalogEntry("gpt-6-astra", "up-2"),
      catalogEntry("gpt-5.6-sol", "up-1"),
      catalogEntry("gpt-5.6-sol", "up-2"),
    ]);
    vi.mocked(gatewayApi.getModelOverrides).mockResolvedValue([
      {
        upstreamId: "up-2",
        modelId: "gpt-6-astra",
        displayName: null,
        hidden: true,
      },
      {
        upstreamId: "up-1",
        modelId: "gpt-5.6-sol",
        displayName: null,
        hidden: true,
      },
      {
        upstreamId: "up-2",
        modelId: "gpt-5.6-sol",
        displayName: null,
        hidden: true,
      },
    ]);
    renderDialog(vi.fn(), twoUpstreams);
    const user = userEvent.setup();

    const pendingRows = await screen.findAllByRole("button", {
      name: /gpt-5\.6-sol/,
    });
    expect(pendingRows).toHaveLength(1);
    // 已启用（up-1 上没有 hidden）→ 只在右栏留一行，不能因为 up-2 还隐藏着就进左栏
    expect(
      screen.queryByRole("button", { name: /gpt-6-astra/ }),
    ).not.toBeInTheDocument();
    expect(screen.getAllByText("gpt-6-astra")).toHaveLength(1);

    await user.click(pendingRows[0]);

    // 一个模型对应两条上游条目：两条都要写成已启用，否则另一条上游留着未启用的残影
    await waitFor(() =>
      expect(gatewayApi.setModelOverride).toHaveBeenCalledTimes(2),
    );
    expect(gatewayApi.setModelOverride).toHaveBeenCalledWith({
      upstreamId: "up-1",
      modelId: "gpt-5.6-sol",
      displayName: null,
      hidden: false,
    });
    expect(gatewayApi.setModelOverride).toHaveBeenCalledWith({
      upstreamId: "up-2",
      modelId: "gpt-5.6-sol",
      displayName: null,
      hidden: false,
    });
  });

  it("saves a renamed supplier through updateName", async () => {
    mocks.updateName.mockResolvedValue([]);
    const { onOverview, onClose } = renderDialog();
    const user = userEvent.setup();

    const name = screen.getByLabelText("suppliers.nameLabel");
    await user.clear(name);
    await user.type(name, "Renamed");
    await user.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(mocks.updateName).toHaveBeenCalledWith("sup-1", "Renamed"),
    );
    expect(onOverview).toHaveBeenCalledWith([]);
    expect(onClose).toHaveBeenCalled();
  });
  it("verifies models through a confirmation dialog and shows per-model results", async () => {
    vi.mocked(gatewayApi.getCatalog).mockResolvedValue([
      catalogEntry("m1"),
      catalogEntry("m2"),
    ]);
    renderDialog();
    const user = userEvent.setup();

    const checkButtons = await screen.findAllByRole("button", {
      name: /验证模型/,
    });
    await user.click(checkButtons[0]);

    expect(
      await screen.findByText(/suppliers\.checkModelsWarning/),
    ).toBeInTheDocument();
    expect(screen.getAllByText("m1").length).toBeGreaterThan(0);
    expect(screen.getAllByText("m2").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: /开始检测/ }));

    await waitFor(() =>
      expect(mocks.checkProviderModels).toHaveBeenCalledWith("claude", "a", [
        "m1",
        "m2",
      ]),
    );
    expect(await screen.findByText("HTTP 401: bad key")).toBeInTheDocument();
    expect(
      screen.getAllByText(/suppliers\.checkModelsAvailable/).length,
    ).toBeGreaterThan(1);
  });
});
