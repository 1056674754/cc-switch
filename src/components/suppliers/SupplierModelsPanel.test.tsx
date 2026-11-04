import { beforeAll, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { GatewayCatalogEntry } from "@/types/gateway";

const getCatalog = vi.fn();
const getModelOverrides = vi.fn();
const setModelOverride = vi.fn();
const clearModelOverride = vi.fn();

vi.mock("@/lib/api/gateway", () => ({
  gatewayApi: {
    getCatalog: () => getCatalog(),
    getModelOverrides: () => getModelOverrides(),
    setModelOverride: (input: unknown) => setModelOverride(input),
    clearModelOverride: (input: unknown) => clearModelOverride(input),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (_key: string, options?: Record<string, unknown>) => {
      const raw = (options?.defaultValue as string | undefined) ?? _key;
      return Object.entries(options ?? {}).reduce(
        (text, [name, value]) =>
          text.replace(new RegExp(`\\\\{\\\\{${name}\\\\}\\\\}`, "g"), String(value)),
        raw,
      );
    },
  }),
}));

vi.mock("@/lib/toast", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

import { SupplierModelsPanel } from "./SupplierModelsPanel";

const entry = (
  upstreamId: string,
  modelId: string,
  displayName: string,
): GatewayCatalogEntry => ({
  reference: `CTYun/${modelId}`,
  upstreamId,
  upstreamName: "CTYun",
  modelId,
  displayName,
  oneM: false,
  endpointIds: [],
  sources: ["claude"],
});

beforeAll(() => {
  Element.prototype.scrollIntoView = vi.fn();
});

function renderPanel(upstreamIds: string[]) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <SupplierModelsPanel upstreamIds={upstreamIds} />
    </QueryClientProvider>,
  );
}

describe("SupplierModelsPanel", () => {
  it("shows only the models of this supplier's upstreams", async () => {
    getCatalog.mockResolvedValue([
      entry("u1", "gpt-5.6-sol", "GPT 5.6 Sol"),
      entry("u2", "gpt-5.6-mini", "GPT 5.6 Mini"),
    ]);
    getModelOverrides.mockResolvedValue([]);
    const user = userEvent.setup();
    renderPanel(["u1"]);

    await user.click(screen.getByRole("button", { name: /模型/ }));
    expect(await screen.findByText("gpt-5.6-sol")).toBeTruthy();
    expect(screen.queryByText("gpt-5.6-mini")).toBeNull();
  });

  it("hides a model through the override command and resets it", async () => {
    getCatalog.mockResolvedValue([entry("u1", "gpt-5.6-sol", "GPT 5.6 Sol")]);
    getModelOverrides.mockResolvedValue([
      {
        upstreamId: "u1",
        modelId: "gpt-5.6-sol",
        displayName: null,
        hidden: false,
      },
    ]);
    setModelOverride.mockResolvedValue(undefined);
    clearModelOverride.mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderPanel(["u1"]);

    await user.click(await screen.findByRole("button", { name: /模型/ }));
    const checkbox = await screen.findByRole("checkbox");
    await user.click(checkbox);
    await waitFor(() =>
      expect(setModelOverride).toHaveBeenCalledWith(
        expect.objectContaining({
          upstreamId: "u1",
          modelId: "gpt-5.6-sol",
          hidden: true,
        }),
      ),
    );

    await user.click(screen.getByLabelText("还原"));
    await waitFor(() =>
      expect(clearModelOverride).toHaveBeenCalledWith({
        upstreamId: "u1",
        modelId: "gpt-5.6-sol",
      }),
    );
  });
});
