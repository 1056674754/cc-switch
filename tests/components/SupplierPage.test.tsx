import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { SupplierPage } from "@/components/suppliers/SupplierPage";
import type { Supplier } from "@/lib/api/suppliers";

const mocks = vi.hoisted(() => ({
  overview: vi.fn(),
  updateBaseUrl: vi.fn(),
  updateSecret: vi.fn(),
  adopt: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
}));

vi.mock("@/lib/api/suppliers", () => ({
  suppliersApi: {
    overview: () => mocks.overview(),
    updateBaseUrl: (endpointId: string, baseUrl: string) =>
      mocks.updateBaseUrl(endpointId, baseUrl),
    updateSecret: (endpointId: string, secret: string) =>
      mocks.updateSecret(endpointId, secret),
    adopt: (appType: string, endpointId: string) =>
      mocks.adopt(appType, endpointId),
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

const supplier = (overrides: Partial<Supplier> = {}): Supplier => ({
  id: "sup-1",
  name: "Acme Gateway",
  host: "api.acme.dev",
  appCount: 2,
  credentialKinds: ["literal", "managed"],
  endpoints: [
    {
      id: "ep-1",
      protocol: "anthropic",
      baseUrl: "https://api.acme.dev",
      apps: ["claude", "codex"],
      consumers: ["claude:p1", "codex:p2"],
      credentialCount: 2,
      bindings: [
        { appType: "claude", providerId: "p1", modelMap: {} },
        { appType: "codex", providerId: "p2", modelMap: {} },
      ],
    },
    {
      id: "ep-2",
      protocol: "gemini_native",
      baseUrl: "",
      apps: [],
      consumers: [],
      credentialCount: 0,
      bindings: [],
    },
  ],
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

const supplierHeader = (name: string) =>
  screen.getByText(name).closest("button") as HTMLButtonElement;

const openSupplier = async (name: string) => {
  const user = userEvent.setup();
  await user.click(supplierHeader(name));
  return user;
};

describe("SupplierPage", () => {
  beforeEach(() => {
    mocks.overview.mockResolvedValue([supplier()]);
    mocks.updateBaseUrl.mockReset();
    mocks.updateSecret.mockReset();
    mocks.adopt.mockReset();
    mocks.success.mockReset();
    mocks.error.mockReset();
  });

  it("lists suppliers with endpoint, app and credential counts", async () => {
    renderPage();

    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    expect(screen.getByText("api.acme.dev")).toBeInTheDocument();
    expect(screen.getByText(/suppliers\.endpointCount:2/)).toBeInTheDocument();
    expect(screen.getByText(/suppliers\.appCount:2/)).toBeInTheDocument();
    expect(
      screen.getByText(/suppliers\.credentialCount:2/),
    ).toBeInTheDocument();
  });

  it("expands a supplier into endpoints with protocol, address and apps", async () => {
    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();

    await openSupplier("Acme Gateway");

    expect(screen.getByText("Anthropic")).toBeInTheDocument();
    expect(screen.getByText("https://api.acme.dev")).toBeInTheDocument();
    expect(screen.getByText("Claude")).toBeInTheDocument();
    expect(screen.getByText("Codex")).toBeInTheDocument();
    // 托管登录的端点没有地址，被谁使用也要说清楚
    expect(screen.getByText("Gemini")).toBeInTheDocument();
    expect(screen.getByText("suppliers.noBaseUrl")).toBeInTheDocument();
    expect(screen.getByText("suppliers.unused")).toBeInTheDocument();
  });

  it("saves an edited endpoint address and replaces the list with the response", async () => {
    const updated = [
      supplier({
        endpoints: [
          {
            id: "ep-1",
            protocol: "anthropic",
            baseUrl: "https://next.acme.dev",
            apps: ["claude", "codex"],
            consumers: ["claude:p1", "codex:p2"],
            credentialCount: 2,
            bindings: [
              { appType: "claude", providerId: "p1", modelMap: {} },
              { appType: "codex", providerId: "p2", modelMap: {} },
            ],
          },
        ],
      }),
    ];
    mocks.updateBaseUrl.mockResolvedValue(updated);

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = await openSupplier("Acme Gateway");

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.editUrl" })[0],
    );
    const input = screen.getByLabelText("suppliers.baseUrlLabel");
    await user.clear(input);
    await user.type(input, "  https://next.acme.dev  ");
    await user.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(mocks.updateBaseUrl).toHaveBeenCalledWith(
        "ep-1",
        "https://next.acme.dev",
      ),
    );
    expect(mocks.success).toHaveBeenCalledWith("suppliers.updateSuccess");
    expect(
      await screen.findByText("https://next.acme.dev"),
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
    const user = await openSupplier("Acme Gateway");

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.editUrl" })[0],
    );
    const input = screen.getByLabelText("suppliers.baseUrlLabel");
    await user.clear(input);
    await user.type(input, "https://next.acme.dev");
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

  it("offers the two adopt targets and adopts an endpoint without asking for a key", async () => {
    mocks.adopt.mockResolvedValue({
      appType: "claude-desktop",
      providerId: "sup-ep-1",
      providerName: "Acme Gateway",
    });

    const { invalidateQueries } = renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = await openSupplier("Acme Gateway");

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.adoptTo" })[0],
    );

    // 菜单里只有后端支持一键采用的两个 app
    expect(
      await screen.findByRole("menuitem", { name: "Claude" }),
    ).toBeInTheDocument();
    await user.click(
      await screen.findByRole("menuitem", { name: "Claude Desktop" }),
    );

    await waitFor(() =>
      expect(mocks.adopt).toHaveBeenCalledWith("claude-desktop", "ep-1"),
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
      "这个端点是 gemini_native 协议，claude 只吃 anthropic 协议，无法直接采用",
    );

    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = await openSupplier("Acme Gateway");

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.adoptTo" })[0],
    );
    await user.click(await screen.findByRole("menuitem", { name: "Claude" }));

    await waitFor(() =>
      expect(mocks.error).toHaveBeenCalledWith(
        "这个端点是 gemini_native 协议，claude 只吃 anthropic 协议，无法直接采用",
      ),
    );
    expect(mocks.success).not.toHaveBeenCalled();
    // 失败后按钮要恢复可用，不能卡在加载态
    await waitFor(() =>
      expect(
        screen.getAllByRole("button", { name: "suppliers.adoptTo" })[0],
      ).not.toBeDisabled(),
    );
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
    const user = await openSupplier("Acme Gateway");

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.adoptTo" })[0],
    );
    await user.click(await screen.findByRole("menuitem", { name: "Claude" }));

    const trigger = screen.getAllByRole("button", {
      name: "suppliers.adoptTo",
    })[0];
    await waitFor(() => expect(trigger).toBeDisabled());
    expect(trigger).toHaveAttribute("aria-busy", "true");

    await act(async () => {
      resolveAdopt({
        appType: "claude",
        providerId: "sup-ep-1",
        providerName: "Acme Gateway",
      });
    });

    await waitFor(() => expect(mocks.success).toHaveBeenCalled());
    await waitFor(() =>
      expect(
        screen.getAllByRole("button", { name: "suppliers.adoptTo" })[0],
      ).not.toBeDisabled(),
    );
  });

  it("never echoes a stored key: the inline secret editor starts empty and masked", async () => {
    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = await openSupplier("Acme Gateway");

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.changeSecret" })[0],
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
    const user = await openSupplier("Acme Gateway");

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.changeSecret" })[0],
    );
    const input = screen.getByLabelText("suppliers.secretLabel");
    await user.type(input, "  sk-new-key  ");
    await user.keyboard("{Enter}");

    await waitFor(() =>
      expect(mocks.updateSecret).toHaveBeenCalledWith("ep-1", "sk-new-key"),
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
    const user = await openSupplier("Acme Gateway");

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.changeSecret" })[0],
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
    const user = await openSupplier("Acme Gateway");

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.changeSecret" })[0],
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
    const user = await openSupplier("Acme Gateway");

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.changeSecret" })[0],
    );
    await user.type(screen.getByLabelText("suppliers.secretLabel"), "sk-new");
    await user.keyboard("{Escape}");

    expect(
      screen.queryByLabelText("suppliers.secretLabel"),
    ).not.toBeInTheDocument();
    expect(mocks.updateSecret).not.toHaveBeenCalled();
    // 收起后「换密钥」按钮回来
    expect(
      screen.getAllByRole("button", { name: "suppliers.changeSecret" })[0],
    ).toBeInTheDocument();
  });

  it("can reveal the new key while typing and hide it again", async () => {
    renderPage();
    expect(await screen.findByText("Acme Gateway")).toBeInTheDocument();
    const user = await openSupplier("Acme Gateway");

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.changeSecret" })[0],
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
    const user = await openSupplier("Acme Gateway");

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.changeSecret" })[0],
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
    const user = await openSupplier("Acme Gateway");

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.editUrl" })[0],
    );
    expect(screen.getByLabelText("suppliers.baseUrlLabel")).toBeInTheDocument();

    await user.click(
      screen.getAllByRole("button", { name: "suppliers.changeSecret" })[0],
    );

    // 一行只开一个编辑器：地址编辑器让位给密钥编辑器
    expect(
      screen.queryByLabelText("suppliers.baseUrlLabel"),
    ).not.toBeInTheDocument();
    expect(screen.getByLabelText("suppliers.secretLabel")).toBeInTheDocument();
  });
});
