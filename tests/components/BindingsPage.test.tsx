import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BindingsPage } from "@/components/bindings/BindingsPage";
import type { Supplier, SupplierBindingDetail } from "@/lib/api/suppliers";

const mocks = vi.hoisted(() => ({
  overview: vi.fn(),
  adopt: vi.fn(),
  updateBindingModelMap: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
}));

vi.mock("@/lib/api/suppliers", () => ({
  suppliersApi: {
    overview: () => mocks.overview(),
    adopt: (appType: string, supplierId: string) =>
      mocks.adopt(appType, supplierId),
    updateBindingModelMap: (
      appType: string,
      providerId: string,
      modelMap: Record<string, string>,
    ) => mocks.updateBindingModelMap(appType, providerId, modelMap),
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
      const entries = Object.entries(options ?? {});
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

const supplier = (
  overrides: Partial<Supplier> & Pick<Supplier, "id" | "name">,
): Supplier => ({
  host: "",
  baseUrl: "",
  upstreamProtocol: "anthropic",
  apps: [],
  consumers: [],
  bindings: [],
  credentialCount: 0,
  credentialKinds: ["literal"],
  ...overrides,
});

/**
 * claude 在用 Acme（anthropic）；codex 在用 Beta（openai_chat）；
 * Gemini 只出现在 Beta 的 apps 里（没有接入明细），用来覆盖单张卡片的空态；
 * Gamma 是还没被 claude 使用的 anthropic 供应商，应当出现在「选用」下拉里。
 */
const overview: Supplier[] = [
  supplier({
    id: "sup-acme",
    name: "Acme Gateway",
    host: "api.acme.dev",
    baseUrl: "https://api.acme.dev",
    upstreamProtocol: "anthropic",
    apps: ["claude"],
    consumers: ["claude:p1"],
    credentialCount: 1,
    bindings: [
      binding({
        appType: "claude",
        providerId: "p1",
        modelMap: { haiku: "kimi-k2-small", sonnet: "kimi-k2" },
      }),
    ],
  }),
  supplier({
    id: "sup-beta",
    name: "Beta Gate",
    host: "beta.dev",
    baseUrl: "https://beta.dev",
    upstreamProtocol: "openai_chat",
    apps: ["codex", "gemini"],
    consumers: ["codex:p3"],
    credentialCount: 1,
    bindings: [
      binding({
        appType: "codex",
        providerId: "p3",
        downstreamProtocol: "openai_responses",
      }),
    ],
  }),
  supplier({
    id: "sup-gamma",
    name: "Gamma Gate",
    host: "gamma.dev",
    baseUrl: "https://gamma.dev/v1",
    upstreamProtocol: "anthropic",
  }),
];

/** 用一份新的 claude 模型映射重建总览（后端保存后返回的形状）。 */
const withClaudeModelMap = (modelMap: Record<string, string>): Supplier[] =>
  overview.map((supplier) =>
    supplier.id === "sup-acme"
      ? {
          ...supplier,
          bindings: [
            binding({ appType: "claude", providerId: "p1", modelMap }),
          ],
        }
      : supplier,
  );

const renderPage = () => {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const invalidateQueries = vi.spyOn(client, "invalidateQueries");
  render(
    <QueryClientProvider client={client}>
      <BindingsPage />
    </QueryClientProvider>,
  );
  return { client, invalidateQueries };
};

/** 卡片里的「选用供应商」下拉，按 app 的显示名找。 */
const picker = (appLabel: string) =>
  screen.getByRole("combobox", {
    name: `bindings.pickSupplierFor:${appLabel}`,
  });

describe("BindingsPage", () => {
  beforeEach(() => {
    // Radix Select 打开时会滚动到选中项，jsdom 没有实现
    Element.prototype.scrollIntoView = vi.fn();
    mocks.overview.mockResolvedValue(overview);
    mocks.adopt.mockReset();
    mocks.updateBindingModelMap.mockReset();
    mocks.success.mockReset();
    mocks.error.mockReset();
  });

  it("shows one card per app in the overview with the suppliers it uses", async () => {
    renderPage();

    // 只展示总览里真正出现过的 app，不是把 APP_IDS 全列一遍
    expect(
      await screen.findByRole("heading", { name: "Claude" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Codex" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Gemini" })).toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: "OpenCode" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: "OpenClaw" }),
    ).not.toBeInTheDocument();

    // 在用的供应商 + 服务根地址 + 上游协议徽标
    expect(screen.getAllByText("Acme Gateway").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Beta Gate").length).toBeGreaterThan(0);
    expect(screen.getByText("https://api.acme.dev")).toBeInTheDocument();
    expect(screen.getByText("https://beta.dev")).toBeInTheDocument();
    expect(screen.getAllByText("Anthropic").length).toBeGreaterThan(0);
    expect(screen.getByText("OpenAI Chat")).toBeInTheDocument();
  });

  it("shows the empty state on a card whose app has no supplier yet", async () => {
    renderPage();

    await screen.findByRole("heading", { name: "Gemini" });

    // Gemini 出现在供应商 apps 里，但没有接入明细：这张卡片给空态
    expect(screen.getByText("bindings.appEmpty")).toBeInTheDocument();
  });

  it("offers only unused Anthropic suppliers and adopts the picked one", async () => {
    // 托管登录账号没有网关地址，不能作为「选用」候选
    mocks.overview.mockResolvedValue([
      ...overview,
      supplier({
        id: "sup-managed",
        name: "Claude Official",
        upstreamProtocol: "anthropic",
        apps: ["claude"],
        consumers: ["claude:official"],
        credentialCount: 1,
        credentialKinds: ["managed"],
        bindings: [binding({ appType: "claude", providerId: "official" })],
      }),
    ]);
    mocks.adopt.mockResolvedValue({
      appType: "claude",
      providerId: "sup-adopted",
      providerName: "Gamma Gate",
    });

    const { invalidateQueries } = renderPage();
    await screen.findByRole("heading", { name: "Claude" });
    const user = userEvent.setup();

    await user.click(picker("Claude"));

    // Claude 已经在用 Acme、Claude Official，候选只剩还没用过的 Gamma；
    // Beta 的 openai_chat 协议不对，不进候选。
    const options = await screen.findAllByRole("option");
    expect(options).toHaveLength(1);
    expect(options[0]).toHaveTextContent("Gamma Gate");
    expect(options[0]).toHaveTextContent("https://gamma.dev/v1");

    await user.click(options[0]);

    await waitFor(() =>
      expect(mocks.adopt).toHaveBeenCalledWith("claude", "sup-gamma"),
    );
    expect(mocks.success).toHaveBeenCalledWith(
      "suppliers.adoptSuccess:Claude|Gamma Gate",
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
    await screen.findByRole("heading", { name: "Claude" });
    const user = userEvent.setup();

    await user.click(picker("Claude"));
    await user.click((await screen.findAllByRole("option"))[0]);

    await waitFor(() =>
      expect(mocks.error).toHaveBeenCalledWith(
        "这家供应商的密钥只存了环境变量名，库里没有明文，无法一键采用",
      ),
    );
    expect(mocks.success).not.toHaveBeenCalled();
    // 失败后下拉恢复可用
    await waitFor(() => expect(picker("Claude")).not.toBeDisabled());
  });

  it("shows an empty state when no app is bound to a supplier", async () => {
    mocks.overview.mockResolvedValue([]);

    renderPage();

    expect(await screen.findByText("bindings.empty")).toBeInTheDocument();
    // 没有可展示的 app 时不该画空卡片
    expect(
      screen.queryByText("bindings.currentSuppliers"),
    ).not.toBeInTheDocument();
  });

  it("shows the load error with a retry action", async () => {
    mocks.overview.mockRejectedValueOnce("database is locked");

    renderPage();

    expect(await screen.findByText("bindings.loadFailed")).toBeInTheDocument();
    expect(screen.getByText("database is locked")).toBeInTheDocument();

    mocks.overview.mockResolvedValue(overview);
    await userEvent
      .setup()
      .click(screen.getByRole("button", { name: "common.retry" }));
    expect(
      await screen.findByRole("heading", { name: "Claude" }),
    ).toBeInTheDocument();
  });

  it("summarizes the current model mapping on each in-use supplier", async () => {
    renderPage();
    await screen.findByRole("heading", { name: "Claude" });

    expect(
      screen.getByText("haiku → kimi-k2-small · sonnet → kimi-k2"),
    ).toBeInTheDocument();
    // Codex 的接入没有映射：如实显示「未配置」，不是留白
    expect(screen.getByText("bindings.modelMapEmpty")).toBeInTheDocument();
  });

  it("prefills the six slots from the binding and saves the edited mapping", async () => {
    mocks.updateBindingModelMap.mockResolvedValue(
      withClaudeModelMap({ sonnet: "kimi-k3" }),
    );

    renderPage();
    await screen.findByRole("heading", { name: "Claude" });
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: "bindings.modelMapEdit" }),
    );

    // 六个槽位固定都在，已配的槽位按后端返回预填
    expect(screen.getByLabelText("bindings.modelSlot.haiku")).toHaveValue(
      "kimi-k2-small",
    );
    expect(screen.getByLabelText("bindings.modelSlot.sonnet")).toHaveValue(
      "kimi-k2",
    );
    for (const slot of ["opus", "fable", "subagent", "default"]) {
      expect(screen.getByLabelText(`bindings.modelSlot.${slot}`)).toHaveValue(
        "",
      );
    }

    const sonnet = screen.getByLabelText("bindings.modelSlot.sonnet");
    await user.clear(sonnet);
    await user.type(sonnet, "kimi-k3");
    // 清空 haiku：空槽位不用提交，后端会删掉对应的模型字段
    await user.clear(screen.getByLabelText("bindings.modelSlot.haiku"));
    await user.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(mocks.updateBindingModelMap).toHaveBeenCalledWith("claude", "p1", {
        sonnet: "kimi-k3",
      }),
    );
    expect(mocks.success).toHaveBeenCalledWith("bindings.modelMapSaved");
    // 新总览落缓存，摘要跟着变，编辑器收起
    expect(await screen.findByText("sonnet → kimi-k3")).toBeInTheDocument();
    expect(
      screen.queryByLabelText("bindings.modelSlot.sonnet"),
    ).not.toBeInTheDocument();
  });

  it("only offers the model mapping editor to Claude-family apps", async () => {
    renderPage();
    await screen.findByRole("heading", { name: "Claude" });

    expect(
      screen.getAllByRole("button", { name: "bindings.modelMapEdit" }),
    ).toHaveLength(1);

    // Codex 的配置里没有可写的模型字段，别给它一个按了会被拒的按钮
    const codexCard = screen
      .getByRole("heading", { name: "Codex" })
      .closest("li") as HTMLElement;
    expect(
      within(codexCard).queryByRole("button", {
        name: "bindings.modelMapEdit",
      }),
    ).toBeNull();
  });

  it("toasts the backend reason and keeps the editor open when saving is rejected", async () => {
    mocks.updateBindingModelMap.mockRejectedValue(
      "claude 的这条配置里没有可写的模型字段（只有 claude 系支持模型映射）",
    );

    renderPage();
    await screen.findByRole("heading", { name: "Claude" });
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: "bindings.modelMapEdit" }),
    );
    const sonnet = screen.getByLabelText("bindings.modelSlot.sonnet");
    await user.clear(sonnet);
    await user.type(sonnet, "kimi-k2-pro");
    await user.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(mocks.error).toHaveBeenCalledWith(
        "claude 的这条配置里没有可写的模型字段（只有 claude 系支持模型映射）",
      ),
    );
    expect(mocks.success).not.toHaveBeenCalled();
    // 输入不丢、不卡加载态
    expect(screen.getByLabelText("bindings.modelSlot.sonnet")).toHaveValue(
      "kimi-k2-pro",
    );
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "common.save" }),
      ).not.toBeDisabled(),
    );
  });

  it("closes the mapping editor on Escape without saving", async () => {
    renderPage();
    await screen.findByRole("heading", { name: "Claude" });
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: "bindings.modelMapEdit" }),
    );
    await user.type(
      screen.getByLabelText("bindings.modelSlot.opus"),
      "kimi-k2-thinking",
    );
    await user.keyboard("{Escape}");

    expect(
      screen.queryByLabelText("bindings.modelSlot.opus"),
    ).not.toBeInTheDocument();
    expect(mocks.updateBindingModelMap).not.toHaveBeenCalled();
    // 收起后摘要还是原来的映射
    expect(
      screen.getByText("haiku → kimi-k2-small · sonnet → kimi-k2"),
    ).toBeInTheDocument();
  });

  it("disables the mapping editor while the request is in flight", async () => {
    let resolveMap!: (value: Supplier[]) => void;
    mocks.updateBindingModelMap.mockReturnValue(
      new Promise<Supplier[]>((resolve) => {
        resolveMap = resolve;
      }),
    );

    renderPage();
    await screen.findByRole("heading", { name: "Claude" });
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: "bindings.modelMapEdit" }),
    );
    await user.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(screen.getByLabelText("bindings.modelSlot.sonnet")).toBeDisabled(),
    );
    expect(screen.getByRole("button", { name: "common.save" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "common.save" })).toHaveAttribute(
      "aria-busy",
      "true",
    );

    await act(async () => {
      resolveMap(withClaudeModelMap({}));
    });

    await waitFor(() =>
      expect(
        screen.queryByLabelText("bindings.modelSlot.sonnet"),
      ).not.toBeInTheDocument(),
    );
  });
});
