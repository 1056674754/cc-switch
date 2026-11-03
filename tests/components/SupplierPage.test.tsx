import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { SupplierPage } from "@/components/suppliers/SupplierPage";
import type { Supplier } from "@/lib/api/suppliers";

const mocks = vi.hoisted(() => ({
  overview: vi.fn(),
  updateBaseUrl: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
}));

vi.mock("@/lib/api/suppliers", () => ({
  suppliersApi: {
    overview: () => mocks.overview(),
    updateBaseUrl: (endpointId: string, baseUrl: string) =>
      mocks.updateBaseUrl(endpointId, baseUrl),
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
      return count == null ? key : `${key}:${String(count)}`;
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
    },
    {
      id: "ep-2",
      protocol: "gemini_native",
      baseUrl: "",
      apps: [],
      consumers: [],
      credentialCount: 0,
    },
  ],
  ...overrides,
});

const renderPage = () => {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <SupplierPage />
    </QueryClientProvider>,
  );
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
});
