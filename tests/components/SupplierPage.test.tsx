import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { SupplierPage } from "@/components/suppliers/SupplierPage";
import type { Supplier, SupplierBindingDetail } from "@/lib/api/suppliers";

const mocks = vi.hoisted(() => ({
  overview: vi.fn(),
  updateBaseUrl: vi.fn(),
  updateSecret: vi.fn(),
  updateBindingModelMap: vi.fn(),
  adopt: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
}));

vi.mock("@/lib/api/suppliers", () => ({
  suppliersApi: {
    overview: () => mocks.overview(),
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
  },
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
  credentialKinds: ["literal", "managed"],
  ...overrides,
});

const renderPage = () => {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const invalidateQueries = vi.spyOn(client, "invalidateQueries");
  render(
    <QueryClientProvider client={client}>
      <SupplierPage />
    </QueryClientProvider>,
  );
  return { client, invalidateQueries };
};

const adoptButton = (index = 0) =>
  screen.getAllByRole("button", { name: "suppliers.adoptTo" })[index];

const openAdoptMenu = async (index = 0) => {
  const user = userEvent.setup();
  await user.click(adoptButton(index));
  return user;
};

const openModelMapMenu = async () => {
  const user = userEvent.setup();
  await user.click(
    screen.getByRole("button", { name: "suppliers.modelMapAction" }),
  );
  return user;
};

describe("SupplierPage", () => {
  beforeEach(() => {
    mocks.overview.mockResolvedValue([supplier()]);
    mocks.updateBaseUrl.mockReset();
    mocks.updateSecret.mockReset();
    mocks.updateBindingModelMap.mockReset();
    mocks.adopt.mockReset();
    mocks.success.mockReset();
    mocks.error.mockReset();
  });

  it("renders one flat row per supplier with address, protocol, apps and credentials", async () => {
    renderPage();

    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    // 一家供应商只有一个地址，不再有「端点」这一层
    expect(screen.getByText("https://api.acme.dev/v1")).toBeInTheDocument();
    expect(
      screen.queryByText(/suppliers\.endpointCount/),
    ).not.toBeInTheDocument();
    // 上游协议徽标
    expect(screen.getByText("Anthropic")).toBeInTheDocument();
    // 被哪些 app 使用
    expect(screen.getByText("Claude")).toBeInTheDocument();
    expect(screen.getByText("Codex")).toBeInTheDocument();
    // 凭据数与凭据形态
    expect(screen.getByText("suppliers.credentialCount:2")).toBeInTheDocument();
    expect(
      screen.getByText("suppliers.credentialKind.literal"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("suppliers.credentialKind.managed"),
    ).toBeInTheDocument();
  });

  it("shows the row actions: edit address, change key, adopt and model mapping", async () => {
    renderPage();
    await screen.findByText("Acme Gateway");

    expect(
      screen.getByRole("button", { name: "suppliers.editUrl" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "suppliers.changeSecret" }),
    ).toBeInTheDocument();
    expect(adoptButton()).toBeEnabled();
    expect(
      screen.getByRole("button", { name: "suppliers.modelMapAction" }),
    ).toBeInTheDocument();
  });

  it("saves an edited supplier address and replaces the list with the response", async () => {
    const updated = [supplier({ baseUrl: "https://next.acme.dev/v1" })];
    mocks.updateBaseUrl.mockResolvedValue(updated);

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "suppliers.editUrl" }));
    const input = screen.getByLabelText("suppliers.baseUrlLabel");
    expect(input).toHaveValue("https://api.acme.dev/v1");
    await user.clear(input);
    await user.type(input, "  https://next.acme.dev/v1  ");
    await user.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(mocks.updateBaseUrl).toHaveBeenCalledWith(
        "sup-1",
        "https://next.acme.dev/v1",
      ),
    );
    expect(mocks.success).toHaveBeenCalledWith("suppliers.updateSuccess");
    expect(
      await screen.findByText("https://next.acme.dev/v1"),
    ).toBeInTheDocument();
    expect(
      screen.queryByLabelText("suppliers.baseUrlLabel"),
    ).not.toBeInTheDocument();
  });

  it("keeps the editor open and toasts the backend message when saving fails", async () => {
    mocks.updateBaseUrl.mockRejectedValue(
      "这些接入的地址不在配置里，无法自动改写：claude:p1",
    );

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "suppliers.editUrl" }));
    const input = screen.getByLabelText("suppliers.baseUrlLabel");
    await user.clear(input);
    await user.type(input, "https://next.acme.dev/v1");
    await user.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(mocks.error).toHaveBeenCalledWith(
        "这些接入的地址不在配置里，无法自动改写：claude:p1",
      ),
    );
    expect(screen.getByLabelText("suppliers.baseUrlLabel")).toBeInTheDocument();
  });

  it("shows an empty state when there are no suppliers", async () => {
    mocks.overview.mockResolvedValue([]);

    renderPage();

    expect(await screen.findByText("suppliers.empty")).toBeInTheDocument();
  });

  it("shows the load error with a retry action", async () => {
    mocks.overview.mockRejectedValueOnce("database is locked");

    renderPage();

    expect(await screen.findByText("suppliers.loadFailed")).toBeInTheDocument();
    expect(screen.getByText("database is locked")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "common.retry" }),
    ).toBeInTheDocument();
    // 重试再拉到数据时列表要能出来
    mocks.overview.mockResolvedValue([supplier()]);
    await userEvent
      .setup()
      .click(screen.getByRole("button", { name: "common.retry" }));
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
  });

  it("offers the two adopt targets and adopts a supplier without asking for a key", async () => {
    mocks.adopt.mockResolvedValue({
      appType: "claude-desktop",
      providerId: "sup-adopted",
      providerName: "Acme Gateway",
    });

    const { invalidateQueries } = renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = await openAdoptMenu();

    // 菜单里只有后端支持一键采用的两个 app
    expect(
      await screen.findByRole("menuitem", { name: "Claude" }),
    ).toBeInTheDocument();
    await user.click(
      await screen.findByRole("menuitem", { name: "Claude Desktop" }),
    );

    await waitFor(() =>
      expect(mocks.adopt).toHaveBeenCalledWith("claude-desktop", "sup-1"),
    );
    // 用户一个 key 都没填：成功提示说清楚加进了哪个 app、哪家供应商
    expect(mocks.success).toHaveBeenCalledWith(
      "suppliers.adoptSuccess:Claude Desktop|Acme Gateway",
    );
    // agent 侧和网关侧两个列表都要重读
    await waitFor(() => {
      const queryKeys = invalidateQueries.mock.calls.map(
        ([filters]) => filters?.queryKey,
      );
      expect(queryKeys).toContainEqual(["providers"]);
      expect(queryKeys).toContainEqual(["suppliers"]);
      expect(queryKeys).toContainEqual(["suppliersOverview"]);
    });
  });

  it("toasts the backend reason when adoption is rejected", async () => {
    mocks.adopt.mockRejectedValue(
      "这家供应商的密钥只存了环境变量名，库里没有明文，无法一键采用",
    );

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = await openAdoptMenu();
    await user.click(await screen.findByRole("menuitem", { name: "Claude" }));

    await waitFor(() =>
      expect(mocks.error).toHaveBeenCalledWith(
        "这家供应商的密钥只存了环境变量名，库里没有明文，无法一键采用",
      ),
    );
    expect(mocks.success).not.toHaveBeenCalled();
    // 失败后按钮要恢复可用，不能卡在加载态
    await waitFor(() => expect(adoptButton()).not.toBeDisabled());
  });

  it("disables the adopt button while a request is in flight", async () => {
    let resolveAdopt!: (value: {
      appType: string;
      providerId: string;
      providerName: string;
    }) => void;
    mocks.adopt.mockReturnValue(
      new Promise((resolve) => {
        resolveAdopt = resolve;
      }),
    );

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = await openAdoptMenu();
    await user.click(await screen.findByRole("menuitem", { name: "Claude" }));

    const trigger = adoptButton();
    await waitFor(() => expect(trigger).toBeDisabled());
    expect(trigger).toHaveAttribute("aria-busy", "true");

    await act(async () => {
      resolveAdopt({
        appType: "claude",
        providerId: "sup-adopted",
        providerName: "Acme Gateway",
      });
    });

    await waitFor(() => expect(mocks.success).toHaveBeenCalled());
    await waitFor(() => expect(adoptButton()).not.toBeDisabled());
  });

  it("disables adopt for non-Anthropic upstreams and says why in a title", async () => {
    mocks.overview.mockResolvedValue([
      supplier({
        id: "sup-2",
        name: "OpenAI Gate",
        upstreamProtocol: "openai_chat",
        baseUrl: "https://openai.example.com/v1",
      }),
      supplier({
        id: "sup-3",
        name: "Managed Claude",
        host: "",
        baseUrl: "",
        upstreamProtocol: "anthropic",
        apps: ["claude"],
        consumers: ["claude:official"],
        bindings: [binding({ appType: "claude", providerId: "official" })],
        credentialCount: 1,
        credentialKinds: ["managed"],
      }),
    ]);

    renderPage();
    expect(await screen.findByText("OpenAI Gate")).toBeInTheDocument();
    // 托管登录账号默认折叠，展开后才看得到
    const user = userEvent.setup();
    await user.click(
      screen.getByRole("button", { name: "suppliers.showManaged:1" }),
    );
    expect(screen.getByText("Managed Claude")).toBeInTheDocument();

    const [openAiTrigger, managedTrigger] = screen.getAllByRole("button", {
      name: "suppliers.adoptTo",
    });
    expect(openAiTrigger).toBeDisabled();
    expect(managedTrigger).toBeDisabled();
    // 直连模式没有代理转换，claude 只吃 anthropic 上游
    expect(openAiTrigger.closest("span[title]")).toHaveAttribute(
      "title",
      "suppliers.adoptDisabledHint",
    );
    expect(managedTrigger.closest("span[title]")).toHaveAttribute(
      "title",
      "suppliers.adoptDisabledHint",
    );
  });

  it("collapses managed sign-in accounts by default and hides their write actions", async () => {
    mocks.overview.mockResolvedValue([
      supplier(),
      supplier({
        id: "sup-managed",
        name: "Claude Official",
        host: "",
        baseUrl: "",
        upstreamProtocol: "anthropic",
        apps: ["claude"],
        consumers: ["claude:official"],
        bindings: [binding({ appType: "claude", providerId: "official" })],
        credentialCount: 1,
        credentialKinds: ["managed"],
      }),
    ]);

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    // 托管登录账号默认折叠，不把真正的网关淹掉
    expect(screen.queryByText("Claude Official")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "suppliers.showManaged:1" }),
    ).toBeInTheDocument();

    const user = userEvent.setup();
    await user.click(
      screen.getByRole("button", { name: "suppliers.showManaged:1" }),
    );

    expect(screen.getByText("Claude Official")).toBeInTheDocument();
    // 托管账号没有网关地址：不给「编辑地址」「换密钥」
    expect(
      screen.getAllByRole("button", { name: "suppliers.editUrl" }),
    ).toHaveLength(1);
    expect(
      screen.getAllByRole("button", { name: "suppliers.changeSecret" }),
    ).toHaveLength(1);
    expect(
      screen.getByRole("button", { name: "suppliers.hideManaged" }),
    ).toBeInTheDocument();
  });

  it("flags duplicate bindings of the same app on one supplier", async () => {
    mocks.overview.mockResolvedValue([
      supplier({
        apps: ["claude"],
        consumers: ["claude:p1", "claude:p1-old"],
        bindings: [
          binding({ appType: "claude", providerId: "p1" }),
          binding({ appType: "claude", providerId: "p1-old" }),
        ],
      }),
    ]);

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();

    expect(screen.getByText("bindings.duplicateBadge:2")).toBeInTheDocument();
    expect(screen.getByTitle("bindings.duplicateHint:2")).toBeInTheDocument();
  });

  it("hints that a binding is converted when downstream and upstream protocols differ", async () => {
    mocks.overview.mockResolvedValue([
      supplier({
        upstreamProtocol: "openai_chat",
        baseUrl: "https://ai.ctaigw.cn/v1",
        apps: ["claude"],
        consumers: ["claude:p1"],
        bindings: [
          binding({
            appType: "claude",
            providerId: "p1",
            downstreamProtocol: "anthropic",
          }),
        ],
      }),
    ]);

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();

    // 下游 Anthropic → 上游 OpenAI Chat：说明请求走了代理转换
    expect(
      screen.getByText("suppliers.protocolConversion:Anthropic|OpenAI Chat"),
    ).toBeInTheDocument();
    expect(
      screen.getByTitle("suppliers.protocolConversionHint"),
    ).toBeInTheDocument();
  });

  it("does not show a conversion hint when both protocols match", async () => {
    mocks.overview.mockResolvedValue([
      supplier({
        apps: ["claude"],
        consumers: ["claude:p1"],
        bindings: [binding({ appType: "claude", providerId: "p1" })],
      }),
    ]);

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();

    expect(
      screen.queryByText(/suppliers\.protocolConversion/),
    ).not.toBeInTheDocument();
  });

  it("only offers the model mapping action to Claude-family bindings", async () => {
    mocks.overview.mockResolvedValue([
      supplier({
        apps: ["codex"],
        consumers: ["codex:p2"],
        bindings: [
          binding({
            appType: "codex",
            providerId: "p2",
            downstreamProtocol: "openai_responses",
          }),
        ],
      }),
    ]);

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();

    expect(
      screen.queryByRole("button", { name: "suppliers.modelMapAction" }),
    ).not.toBeInTheDocument();
  });

  it("prefills and saves the model mapping of a Claude binding", async () => {
    mocks.updateBindingModelMap.mockResolvedValue([
      supplier({
        bindings: [
          binding({
            appType: "claude",
            providerId: "p1",
            modelMap: { haiku: "kimi-k2-small", sonnet: "kimi-k3" },
          }),
          binding({
            appType: "codex",
            providerId: "p2",
            downstreamProtocol: "openai_responses",
          }),
        ],
      }),
    ]);

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = await openModelMapMenu();

    await user.click(await screen.findByRole("menuitem", { name: /Claude/ }));

    // 六个槽位固定都在，已配的槽位按后端返回预填
    expect(screen.getByLabelText("bindings.modelSlot.haiku")).toHaveValue(
      "kimi-k2-small",
    );
    expect(screen.getByLabelText("bindings.modelSlot.sonnet")).toHaveValue(
      "kimi-k2",
    );

    await user.clear(screen.getByLabelText("bindings.modelSlot.sonnet"));
    await user.type(
      screen.getByLabelText("bindings.modelSlot.sonnet"),
      "kimi-k3",
    );
    await user.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(mocks.updateBindingModelMap).toHaveBeenCalledWith("claude", "p1", {
        haiku: "kimi-k2-small",
        sonnet: "kimi-k3",
      }),
    );
    expect(mocks.success).toHaveBeenCalledWith("bindings.modelMapSaved");
    // 编辑器收起
    await waitFor(() =>
      expect(
        screen.queryByLabelText("bindings.modelSlot.sonnet"),
      ).not.toBeInTheDocument(),
    );
  });

  it("keeps the mapping editor open when saving is rejected", async () => {
    mocks.updateBindingModelMap.mockRejectedValue(
      "claude 的这条配置里没有可写的模型字段（只有 claude 系支持模型映射）",
    );

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = await openModelMapMenu();
    await user.click(await screen.findByRole("menuitem", { name: /Claude/ }));
    await user.type(
      screen.getByLabelText("bindings.modelSlot.opus"),
      "kimi-k2-thinking",
    );
    await user.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(mocks.error).toHaveBeenCalledWith(
        "claude 的这条配置里没有可写的模型字段（只有 claude 系支持模型映射）",
      ),
    );
    expect(screen.getByLabelText("bindings.modelSlot.opus")).toHaveValue(
      "kimi-k2-thinking",
    );
  });

  it("never echoes a stored key: the inline secret editor starts empty and masked", async () => {
    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: "suppliers.changeSecret" }),
    );

    const input = screen.getByLabelText("suppliers.secretLabel");
    // 后端不返回明文，界面也不保留：编辑器永远是空的，默认遮住
    expect(input).toHaveValue("");
    expect(input).toHaveAttribute("type", "password");
    // 没有任何地方把密钥写进页面
    expect(screen.queryByDisplayValue(/sk-/)).not.toBeInTheDocument();
  });

  it("saves a new key on Enter, replaces the overview and tells every app followed", async () => {
    const updated = [supplier({ name: "Acme Gateway (rotated)" })];
    mocks.updateSecret.mockResolvedValue(updated);

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: "suppliers.changeSecret" }),
    );
    const input = screen.getByLabelText("suppliers.secretLabel");
    await user.type(input, "  sk-new-key  ");
    await user.keyboard("{Enter}");

    await waitFor(() =>
      expect(mocks.updateSecret).toHaveBeenCalledWith("sup-1", "sk-new-key"),
    );
    expect(mocks.success).toHaveBeenCalledWith("suppliers.secretUpdated");
    // 成功后用返回的总览替换缓存，编辑器收起
    expect(
      await screen.findByText("Acme Gateway (rotated)"),
    ).toBeInTheDocument();
    expect(
      screen.queryByLabelText("suppliers.secretLabel"),
    ).not.toBeInTheDocument();
  });

  it("refuses a blank key and keeps the editor open", async () => {
    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: "suppliers.changeSecret" }),
    );
    await user.type(screen.getByLabelText("suppliers.secretLabel"), "   ");
    await user.keyboard("{Enter}");

    expect(mocks.error).toHaveBeenCalledWith("suppliers.secretRequired");
    expect(mocks.updateSecret).not.toHaveBeenCalled();
    expect(screen.getByLabelText("suppliers.secretLabel")).toBeInTheDocument();
  });

  it("keeps the editor open and toasts the backend reason when the key change is rejected", async () => {
    mocks.updateSecret.mockRejectedValue(
      "这些接入的密钥存在环境变量里，库里没有明文可换：hermes:xxx",
    );

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: "suppliers.changeSecret" }),
    );
    await user.type(screen.getByLabelText("suppliers.secretLabel"), "sk-new");
    await user.keyboard("{Enter}");

    await waitFor(() =>
      expect(mocks.error).toHaveBeenCalledWith(
        "这些接入的密钥存在环境变量里，库里没有明文可换：hermes:xxx",
      ),
    );
    expect(mocks.success).not.toHaveBeenCalled();
    // 失败后不吞输入，也不卡在加载态
    expect(screen.getByLabelText("suppliers.secretLabel")).toHaveValue(
      "sk-new",
    );
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "common.save" }),
      ).not.toBeDisabled(),
    );
  });

  it("cancels the secret editor on Escape without sending anything", async () => {
    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: "suppliers.changeSecret" }),
    );
    await user.type(screen.getByLabelText("suppliers.secretLabel"), "sk-new");
    await user.keyboard("{Escape}");

    expect(
      screen.queryByLabelText("suppliers.secretLabel"),
    ).not.toBeInTheDocument();
    expect(mocks.updateSecret).not.toHaveBeenCalled();
    // 收起后「换密钥」按钮回来
    expect(
      screen.getByRole("button", { name: "suppliers.changeSecret" }),
    ).toBeInTheDocument();
  });

  it("can reveal the new key while typing and hide it again", async () => {
    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: "suppliers.changeSecret" }),
    );
    const input = screen.getByLabelText("suppliers.secretLabel");
    await user.type(input, "sk-live");
    // 空输入时不显示切换按钮，有内容才给
    await user.click(screen.getByRole("button", { name: "apiKeyInput.show" }));
    expect(screen.getByLabelText("suppliers.secretLabel")).toHaveAttribute(
      "type",
      "text",
    );

    await user.click(screen.getByRole("button", { name: "apiKeyInput.hide" }));
    expect(screen.getByLabelText("suppliers.secretLabel")).toHaveAttribute(
      "type",
      "password",
    );
  });

  it("disables the key editor while the request is in flight", async () => {
    let resolveSecret!: (value: Supplier[]) => void;
    mocks.updateSecret.mockReturnValue(
      new Promise<Supplier[]>((resolve) => {
        resolveSecret = resolve;
      }),
    );

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: "suppliers.changeSecret" }),
    );
    await user.type(screen.getByLabelText("suppliers.secretLabel"), "sk-new");
    await user.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() => {
      expect(screen.getByLabelText("suppliers.secretLabel")).toBeDisabled();
    });
    expect(screen.getByRole("button", { name: "common.save" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "common.save" })).toHaveAttribute(
      "aria-busy",
      "true",
    );

    await act(async () => {
      resolveSecret([supplier()]);
    });

    await waitFor(() =>
      expect(
        screen.queryByLabelText("suppliers.secretLabel"),
      ).not.toBeInTheDocument(),
    );
  });

  it("switches a row from the address editor to the key editor", async () => {
    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "suppliers.editUrl" }));
    expect(screen.getByLabelText("suppliers.baseUrlLabel")).toBeInTheDocument();

    await user.click(
      screen.getByRole("button", { name: "suppliers.changeSecret" }),
    );

    // 一行只开一个编辑器：地址编辑器让位给密钥编辑器
    expect(
      screen.queryByLabelText("suppliers.baseUrlLabel"),
    ).not.toBeInTheDocument();
    expect(screen.getByLabelText("suppliers.secretLabel")).toBeInTheDocument();
  });
});
