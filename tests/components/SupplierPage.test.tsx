import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { SupplierPage } from "@/components/suppliers/SupplierPage";
import type { Supplier, SupplierBindingDetail } from "@/lib/api/suppliers";

const mocks = vi.hoisted(() => ({
  overview: vi.fn(),
  updateName: vi.fn(),
  updateBaseUrl: vi.fn(),
  updateSecret: vi.fn(),
  updateBindingModelMap: vi.fn(),
  adopt: vi.fn(),
  setEnabled: vi.fn(),
  setManagedAccount: vi.fn(),
  remove: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
  warning: vi.fn(),
  authGetStatus: vi.fn(),
  authSetDefaultAccount: vi.fn(),
  dshStatus: vi.fn(),
}));

vi.mock("@/lib/api/suppliers", () => ({
  suppliersApi: {
    overview: () => mocks.overview(),
    updateName: (supplierId: string, name: string) =>
      mocks.updateName(supplierId, name),
    updateBaseUrl: (supplierId: string, baseUrl: string) =>
      mocks.updateBaseUrl(supplierId, baseUrl),
    updateSecret: (supplierId: string, secret: string) =>
      mocks.updateSecret(supplierId, secret),
    updateBindingModelMap: (
      appType: string,
      providerId: string,
      modelMap: Record<string, string>,
    ) => mocks.updateBindingModelMap(appType, providerId, modelMap),
    adopt: (appType: string, supplierId: string) =>
      mocks.adopt(appType, supplierId),
    setEnabled: (supplierId: string, enabled: boolean) =>
      mocks.setEnabled(supplierId, enabled),
    setManagedAccount: (supplierIds: string[], accountId: string) =>
      mocks.setManagedAccount(supplierIds, accountId),
    remove: (supplierId: string) => mocks.remove(supplierId),
  },
}));

// 托管账号的接口：页面用它判断「有没有登录」来决定要不要出卡
vi.mock("@/lib/api", () => ({
  authApi: {
    authGetStatus: (...args: unknown[]) => mocks.authGetStatus(...args),
    authSetDefaultAccount: (authProvider: string, accountId: string) =>
      mocks.authSetDefaultAccount(authProvider, accountId),
  },
}));

// 账号组本身由授权中心的组件负责，这里只验证卡片嵌的是对应那一组
// 账号组本身由授权中心的组件负责，这里只验证卡片嵌的是对应那一组；「设为默认」
// 由供应商卡片接管，所以 mock 里留一个按钮把它按出去。
vi.mock("@/components/providers/forms/CodexOAuthSection", () => ({
  CodexOAuthSection: (props: {
    onSetDefaultAccount?: (accountId: string) => void;
  }) => (
    <div>
      CHATGPT-ACCOUNTS
      <button
        type="button"
        onClick={() => props.onSetDefaultAccount?.("acc-2")}
      >
        SET-DEFAULT-ACC-2
      </button>
    </div>
  ),
}));
vi.mock("@/components/providers/forms/ClaudeOfficialSection", () => ({
  ClaudeOfficialSection: () => <div>CLAUDE-ACCOUNTS</div>,
}));
vi.mock("@/components/providers/KimiAccountLogin", () => ({
  KimiAccountLogin: () => <div>KIMI-ACCOUNTS</div>,
}));
// DeepSeek Harness 账号：登录了才多一张账号卡，卡片里嵌的就是这一组账号
vi.mock("@/components/providers/DshAccount", () => ({
  useDshAccountStatus: () => ({ data: mocks.dshStatus() }),
  DshAccountLogin: () => <div>DSH-ACCOUNTS</div>,
}));

// 卡片底部的模型面板会去查目录和覆盖；弹窗里的模型表另有测试
vi.mock("@/components/suppliers/SupplierModelsPanel", () => ({
  SupplierModelsPanel: () => <div>MODELS-PANEL</div>,
}));

vi.mock("@/lib/toast", () => ({
  toast: {
    success: (...args: unknown[]) => mocks.success(...args),
    error: (...args: unknown[]) => mocks.error(...args),
    warning: (...args: unknown[]) => mocks.warning(...args),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      const count = options?.count;
      if (typeof count === "number") return `${key}:${count}`;
      // defaultValue 只是给 i18n 兜底的名字，不算插值参数
      const entries = Object.entries(options ?? {}).filter(
        ([name]) => name !== "defaultValue",
      );
      return entries.length === 0
        ? key
        : `${key}:${entries.map(([, value]) => String(value)).join("|")}`;
    },
  }),
}));

const binding = (
  overrides: Partial<SupplierBindingDetail> &
    Pick<SupplierBindingDetail, "appType" | "providerId">,
): SupplierBindingDetail => ({
  modelMap: {},
  downstreamProtocol: "anthropic",
  ...overrides,
});

const supplier = (overrides: Partial<Supplier> = {}): Supplier => ({
  id: "sup-1",
  name: "Acme Gateway",
  host: "api.acme.dev",
  baseUrl: "https://api.acme.dev/v1",
  upstreamProtocol: "anthropic",
  enabled: true,
  apps: ["claude", "codex"],
  consumers: ["claude:p1", "codex:p2"],
  bindings: [
    binding({
      appType: "claude",
      providerId: "p1",
      modelMap: { haiku: "kimi-k2-small", sonnet: "kimi-k2" },
    }),
    binding({
      appType: "codex",
      providerId: "p2",
      downstreamProtocol: "openai_responses",
    }),
  ],
  credentialCount: 2,
  credentialKinds: ["literal"],
  credentialKeyHashes: ["key-1"],
  managedAuthProvider: null,
  ...overrides,
});

const renderPage = () => {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={client}>
      <SupplierPage />
    </QueryClientProvider>,
  );
  return { client };
};

const openEditDialog = async () => {
  const user = userEvent.setup();
  await user.click(
    screen.getByRole("button", { name: "suppliers.editAction" }),
  );
  return user;
};

describe("SupplierPage", () => {
  beforeEach(() => {
    mocks.overview.mockResolvedValue([supplier()]);
    mocks.updateName.mockReset();
    mocks.updateBaseUrl.mockReset();
    mocks.updateSecret.mockReset();
    mocks.updateBindingModelMap.mockReset();
    mocks.adopt.mockReset();
    mocks.setEnabled.mockReset();
    mocks.setManagedAccount.mockReset();
    mocks.authSetDefaultAccount.mockReset();
    mocks.authSetDefaultAccount.mockResolvedValue([]);
    mocks.remove.mockReset();
    mocks.success.mockReset();
    mocks.error.mockReset();
    mocks.warning.mockReset();
    mocks.authGetStatus.mockReset();
    mocks.authGetStatus.mockResolvedValue({
      accounts: [{ id: "acc-1" }],
      currentAccountId: "acc-1",
      authenticated: true,
    });
    mocks.dshStatus.mockReset();
    mocks.dshStatus.mockReturnValue(undefined);
  });

  it("renders a provider card with address, protocol, apps and distinct keys", async () => {
    renderPage();

    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    expect(screen.getByText("https://api.acme.dev/v1")).toBeInTheDocument();
    expect(screen.getByText("Anthropic")).toBeInTheDocument();
    expect(screen.getByText("Claude")).toBeInTheDocument();
    expect(screen.getByText("Codex")).toBeInTheDocument();
    // 去重后的明文密钥数，而不是每个 app 的接入条数
    expect(screen.getByText("suppliers.keyCount:1")).toBeInTheDocument();
    // 编辑器收进弹窗，卡片上不再有「编辑地址」「换密钥」
    expect(
      screen.getByRole("button", { name: "suppliers.editAction" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "suppliers.editUrl" }),
    ).toBeNull();
    expect(screen.queryByText("suppliers.changeSecret")).toBeNull();
  });

  it("shows the DeepSeek Harness account card inside the same account area when signed in", async () => {
    mocks.dshStatus.mockReturnValue({
      signedIn: true,
      expired: false,
      wallets: [],
      bonusWallets: [],
      profile: { name: "松松" },
    });
    renderPage();

    const card = await screen.findByTestId("dsh-account-supplier");
    expect(within(card).getByText("DSH-ACCOUNTS")).toBeInTheDocument();
    // 账号卡不是供应商行：没有编辑 / 删除这类只有供应商才有的动作
    expect(
      within(card).queryByRole("button", { name: "suppliers.editAction" }),
    ).toBeNull();
    expect(
      within(card).queryByRole("button", { name: "suppliers.deleteAction" }),
    ).toBeNull();
  });

  it("hides the DeepSeek Harness account card when its account is signed out", async () => {
    renderPage();

    await screen.findByText("Acme Gateway");
    expect(screen.queryByTestId("dsh-account-supplier")).toBeNull();
  });

  it("opens the edit dialog with the current values and saves name, address and key", async () => {
    const updated = [
      supplier({ name: "Next Acme", baseUrl: "https://next.acme.dev/v1" }),
    ];
    mocks.updateName.mockResolvedValue(updated);
    mocks.updateBaseUrl.mockResolvedValue(updated);
    mocks.updateSecret.mockResolvedValue(updated);

    renderPage();
    await screen.findByText("Acme Gateway");
    const user = await openEditDialog();

    const dialog = await screen.findByRole("dialog");
    expect(dialog).toBeInTheDocument();
    const name = screen.getByLabelText("suppliers.nameLabel");
    expect(name).toHaveValue("Acme Gateway");
    expect(screen.getByLabelText("suppliers.baseUrlLabel")).toHaveValue(
      "https://api.acme.dev/v1",
    );

    await user.clear(name);
    await user.type(name, "Next Acme");
    const url = screen.getByLabelText("suppliers.baseUrlLabel");
    await user.clear(url);
    await user.type(url, "https://next.acme.dev/v1");
    await user.type(
      screen.getByLabelText("suppliers.newSecretLabel"),
      "sk-new",
    );
    await user.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(mocks.updateName).toHaveBeenCalledWith("sup-1", "Next Acme"),
    );
    expect(mocks.updateBaseUrl).toHaveBeenCalledWith(
      "sup-1",
      "https://next.acme.dev/v1",
    );
    expect(mocks.updateSecret).toHaveBeenCalledWith("sup-1", "sk-new");
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    expect(mocks.success).toHaveBeenCalled();
  });

  it("does not save when the name or address is invalid", async () => {
    renderPage();
    await screen.findByText("Acme Gateway");
    const user = await openEditDialog();

    const name = screen.getByLabelText("suppliers.nameLabel");
    await user.clear(name);
    await user.click(screen.getByRole("button", { name: "common.save" }));

    expect(screen.getByText("suppliers.nameRequired")).toBeInTheDocument();
    expect(mocks.updateName).not.toHaveBeenCalled();

    const url = screen.getByLabelText("suppliers.baseUrlLabel");
    await user.type(name, "Acme Gateway");
    await user.clear(url);
    await user.type(url, "not-a-url");
    await user.click(screen.getByRole("button", { name: "common.save" }));

    expect(
      screen.getByText("suppliers.createBaseUrlInvalid"),
    ).toBeInTheDocument();
    expect(mocks.updateBaseUrl).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("switches this supplier to the account picked from the credential menu", async () => {
    const codexSupplier = (accountId: string) =>
      supplier({
        id: "sup-codex",
        name: "Codex",
        baseUrl: "https://chatgpt.com/backend-api/codex",
        managedAuthProvider: "codex_oauth",
        credentialKinds: ["managed"],
        credentialKeyHashes: [],
        boundAccountIds: [accountId],
        consumers: ["claude-desktop:codex", "codex:codex-official"],
        bindings: [
          binding({ appType: "claude-desktop", providerId: "codex" }),
          binding({ appType: "codex", providerId: "codex-official" }),
        ],
      });
    mocks.overview.mockResolvedValue([codexSupplier("acc-1")]);
    mocks.setManagedAccount.mockResolvedValue([codexSupplier("acc-2")]);

    renderPage();
    await screen.findByText("CHATGPT-ACCOUNTS");
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "SET-DEFAULT-ACC-2" }));

    // 改的是这家供应商的所有接入，而不是只改账号池的默认账号
    await waitFor(() =>
      expect(mocks.setManagedAccount).toHaveBeenCalledWith(
        ["sup-codex"],
        "acc-2",
      ),
    );
    // 全局默认跟着同步，「默认」徽标才一起动
    expect(mocks.authSetDefaultAccount).toHaveBeenCalledWith(
      "codex_oauth",
      "acc-2",
    );
    expect(mocks.success).toHaveBeenCalled();
  });

  it("reports a failed account switch instead of silently doing nothing", async () => {
    mocks.overview.mockResolvedValue([
      supplier({
        id: "sup-codex",
        name: "Codex",
        baseUrl: "https://chatgpt.com/backend-api/codex",
        managedAuthProvider: "codex_oauth",
        credentialKinds: ["managed"],
        credentialKeyHashes: [],
        consumers: ["codex:codex-official"],
        bindings: [binding({ appType: "codex", providerId: "codex-official" })],
      }),
    ]);
    mocks.setManagedAccount.mockRejectedValue(new Error("改了 0 条接入"));

    renderPage();
    await screen.findByText("CHATGPT-ACCOUNTS");
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "SET-DEFAULT-ACC-2" }));

    await waitFor(() => expect(mocks.error).toHaveBeenCalled());
    expect(mocks.success).not.toHaveBeenCalled();
  });

  it("shows the same account pool inside the edit dialog", async () => {
    mocks.overview.mockResolvedValue([
      supplier({
        id: "sup-codex",
        name: "Codex",
        baseUrl: "https://chatgpt.com/backend-api/codex",
        managedAuthProvider: "codex_oauth",
        credentialKinds: ["managed"],
        credentialKeyHashes: [],
      }),
    ]);

    renderPage();
    await screen.findByText("CHATGPT-ACCOUNTS");
    const user = await openEditDialog();

    // 弹窗和卡片渲染同一个账号组件（同一套登录 / 切换 / 退出登录），不再是两套口径
    expect(screen.getAllByText("CHATGPT-ACCOUNTS").length).toBeGreaterThan(1);
    const dialog = screen.getByRole("dialog");
    expect(
      within(dialog).queryByLabelText(/suppliers\.addAccountLabel/),
    ).toBeNull();
    expect(
      within(dialog).queryByLabelText("suppliers.newSecretLabel"),
    ).toBeNull();

    await user.click(screen.getByRole("button", { name: "common.cancel" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("merges protocols of the same provider and edits every underlying row", async () => {
    const aliyun = [
      supplier({
        id: "sup-aliyun-1",
        name: "阿里云百炼",
        baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        upstreamProtocol: "alibaba",
        apps: ["opencode"],
        consumers: ["opencode:aliyun"],
        bindings: [],
        credentialCount: 1,
        credentialKeyHashes: ["ali-key"],
      }),
      supplier({
        id: "sup-aliyun-2",
        name: "阿里云百炼",
        baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        upstreamProtocol: "openai_chat",
        apps: ["opencode"],
        consumers: ["opencode:aliyun-2"],
        bindings: [],
        credentialCount: 1,
        credentialKeyHashes: ["ali-key"],
      }),
    ];
    mocks.overview.mockResolvedValue(aliyun);
    mocks.updateBaseUrl.mockResolvedValue(aliyun);
    mocks.updateSecret.mockResolvedValue(aliyun);

    renderPage();
    await screen.findByText("阿里云百炼");
    expect(screen.getAllByText("阿里云百炼")).toHaveLength(1);
    expect(screen.getByText("suppliers.keyCount:1")).toBeInTheDocument();

    const user = await openEditDialog();
    const url = screen.getByLabelText("suppliers.baseUrlLabel");
    await user.clear(url);
    await user.type(url, "https://next.example.com/v1");
    await user.type(
      screen.getByLabelText("suppliers.newSecretLabel"),
      "sk-new",
    );
    await user.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() => expect(mocks.updateBaseUrl).toHaveBeenCalledTimes(2));
    expect(mocks.updateBaseUrl).toHaveBeenCalledWith(
      "sup-aliyun-1",
      "https://next.example.com/v1",
    );
    expect(mocks.updateBaseUrl).toHaveBeenCalledWith(
      "sup-aliyun-2",
      "https://next.example.com/v1",
    );
    expect(mocks.updateSecret).toHaveBeenCalledTimes(2);
  });

  it("hides managed providers without accounts and providers without credentials", async () => {
    mocks.authGetStatus.mockResolvedValue({
      accounts: [],
      currentAccountId: null,
      authenticated: false,
    });
    mocks.overview.mockResolvedValue([
      supplier({
        id: "sup-claude",
        name: "Claude Official",
        host: "",
        baseUrl: "",
        managedAuthProvider: "claude_official",
        credentialKinds: ["managed"],
        credentialKeyHashes: [],
      }),
      supplier({
        id: "sup-default",
        name: "default",
        host: "",
        baseUrl: "",
        credentialKinds: ["none"],
        credentialKeyHashes: [],
        credentialCount: 0,
        bindings: [],
        apps: [],
        consumers: [],
      }),
    ]);

    renderPage();

    expect(
      await screen.findByText("suppliers.empty", { exact: false }),
    ).toBeInTheDocument();
    expect(screen.queryByText("Claude Official")).toBeNull();
    expect(screen.queryByText("default")).toBeNull();
    expect(screen.queryByText("CLAUDE-ACCOUNTS")).toBeNull();
  });

  it("adopts an anthropic provider into an app without asking for a key", async () => {
    mocks.adopt.mockResolvedValue({
      appType: "claude",
      providerId: "sup-adopted",
      providerName: "Acme Gateway",
    });

    renderPage();
    await screen.findByText("Acme Gateway");

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "suppliers.adoptTo" }));
    await user.click(await screen.findByRole("menuitem", { name: "Claude" }));

    await waitFor(() =>
      expect(mocks.adopt).toHaveBeenCalledWith("claude", "sup-1"),
    );
    expect(mocks.success).toHaveBeenCalled();
  });

  it("adopts an openai_chat provider into Codex", async () => {
    mocks.overview.mockResolvedValue([
      supplier({ upstreamProtocol: "openai_chat" }),
    ]);
    mocks.adopt.mockResolvedValue({
      appType: "codex",
      providerId: "sup-adopted",
      providerName: "Acme Gateway",
    });

    renderPage();
    await screen.findByText("Acme Gateway");

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "suppliers.adoptTo" }));
    await user.click(await screen.findByRole("menuitem", { name: "Codex" }));

    await waitFor(() =>
      expect(mocks.adopt).toHaveBeenCalledWith("codex", "sup-1"),
    );
  });

  it("hides adopt for protocols without a direct renderer", async () => {
    mocks.overview.mockResolvedValue([
      supplier({ upstreamProtocol: "kimi_managed" }),
    ]);

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "suppliers.adoptTo" }),
    ).toBeNull();
  });

  it("does not offer model mapping on the supplier card (it belongs to the app side)", async () => {
    renderPage();
    await screen.findByText("Acme Gateway");

    // 模型映射是「某个 app 的这条接入」自己的配置，只在接入端改；供应商卡上不再有入口
    expect(
      screen.queryByRole("button", { name: "suppliers.modelMapAction" }),
    ).toBeNull();
    expect(screen.queryByLabelText("bindings.modelMapFor:Claude")).toBeNull();
  });

  it("disables a supplier from the card switch", async () => {
    mocks.setEnabled.mockResolvedValue([supplier({ enabled: false })]);

    renderPage();
    await screen.findByText("Acme Gateway");
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("switch", { name: "suppliers.disableAction" }),
    );

    await waitFor(() =>
      expect(mocks.setEnabled).toHaveBeenCalledWith("sup-1", false),
    );
    expect(
      await screen.findByText("suppliers.disabledBadge"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("switch", { name: "suppliers.enableAction" }),
    ).toBeInTheDocument();
  });

  it("shows an empty state when there are no suppliers", async () => {
    mocks.overview.mockResolvedValue([]);
    renderPage();

    expect(await screen.findByText("suppliers.empty")).toBeInTheDocument();
  });

  it("shows the load error with a retry action", async () => {
    mocks.overview.mockRejectedValue("db locked");
    renderPage();

    expect(await screen.findByText("suppliers.loadFailed")).toBeInTheDocument();
    expect(screen.getByText("db locked")).toBeInTheDocument();
  });

  it("deletes a provider only after the confirmation and warns about skipped rows", async () => {
    mocks.remove.mockResolvedValue({
      suppliers: [],
      deleted: 2,
      skipped: [{ appType: "claude", providerId: "p1", reason: "配置已过期" }],
    });

    renderPage();
    await screen.findByText("Acme Gateway");
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: "suppliers.moreActions" }),
    );
    await user.click(
      await screen.findByRole("menuitem", { name: "suppliers.deleteAction" }),
    );
    await user.click(
      screen.getByRole("button", { name: "suppliers.deleteConfirm" }),
    );

    await waitFor(() => expect(mocks.remove).toHaveBeenCalledWith("sup-1"));
    await waitFor(() => expect(mocks.success).toHaveBeenCalled());
    expect(mocks.warning).toHaveBeenCalled();
  });
});
