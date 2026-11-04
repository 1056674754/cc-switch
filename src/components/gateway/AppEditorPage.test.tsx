import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { GatewayCatalogEntry } from "@/types/gateway";
import { gatewayApi } from "@/lib/api/gateway";
import { AppEditorPage } from "./AppEditorPage";

vi.mock("@/lib/api/gateway", () => ({
  gatewayApi: {
    getCatalog: vi.fn(),
    getAppModelConfig: vi.fn(),
    setAppModelConfig: vi.fn(),
    migrateFromStack: vi.fn(),
  },
}));

vi.mock("@/lib/toast", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

beforeAll(() => {
  Element.prototype.scrollIntoView = vi.fn();
});

const entries: GatewayCatalogEntry[] = [
  {
    reference: "CTYun/qwen3.8-max",
    upstreamId: "up-1",
    upstreamName: "CTYun",
    modelId: "qwen3.8-max",
    displayName: "Qwen3.8 Max",
    endpointIds: ["ep-1"],
    sources: ["claude:CTYun"],
  },
  {
    reference: "Kimi/kimi-k3",
    upstreamId: "up-2",
    upstreamName: "Kimi",
    modelId: "kimi-k3",
    displayName: "Kimi K3",
    endpointIds: ["ep-2"],
    sources: ["claude:Kimi"],
  },
];

function renderPage() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <AppEditorPage appType="claude" appLabel="Claude Code" />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.mocked(gatewayApi.getCatalog).mockResolvedValue(entries);
  vi.mocked(gatewayApi.getAppModelConfig).mockResolvedValue({
    default: "CTYun/qwen3.8-max",
    slots: { haiku: "Kimi/kimi-k3" },
    allowed: ["CTYun/qwen3.8-max", "Kimi/kimi-k3"],
  });
  vi.mocked(gatewayApi.setAppModelConfig).mockResolvedValue();
  vi.mocked(gatewayApi.migrateFromStack).mockResolvedValue([]);
});

describe("AppEditorPage", () => {
  it("shows the saved config read-only and swaps to editing on demand", async () => {
    const user = userEvent.setup();
    renderPage();

    expect(await screen.findByText("Qwen3.8 Max")).toBeInTheDocument();
    expect(screen.getByText("Kimi K3")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "保存" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "编辑" }));
    expect(screen.getByRole("button", { name: "保存" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "取消" })).toBeInTheDocument();
  });

  it("saves the edited default model", async () => {
    const user = userEvent.setup();
    renderPage();
    await screen.findByText("Qwen3.8 Max");

    await user.click(screen.getByRole("button", { name: "编辑" }));
    await user.click(screen.getAllByRole("combobox")[0]);
    await user.click(await screen.findByRole("option", { name: /Kimi K3/ }));
    await user.click(screen.getByRole("button", { name: "保存" }));

    expect(gatewayApi.setAppModelConfig).toHaveBeenCalledWith(
      "claude",
      expect.objectContaining({ default: "Kimi/kimi-k3" }),
    );
  });

  it("offers migration when the app has no config yet", async () => {
    vi.mocked(gatewayApi.getAppModelConfig).mockResolvedValue(null);
    vi.mocked(gatewayApi.migrateFromStack).mockResolvedValue([
      { app: "claude", status: "migrated", default: "CTYun/qwen3.8-max", models: 2 },
    ]);
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "从聚合配置迁移" }));
    expect(gatewayApi.migrateFromStack).toHaveBeenCalled();
  });
});
