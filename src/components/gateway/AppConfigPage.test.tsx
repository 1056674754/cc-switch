import { beforeAll, describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { GatewayCatalogEntry } from "@/types/gateway";

const getCatalog = vi.fn();
const getAppModelConfig = vi.fn();
const setAppModelConfig = vi.fn();

vi.mock("@/lib/api/gateway", () => ({
  gatewayApi: {
    getCatalog: () => getCatalog(),
    getAppModelConfig: (app: string) => getAppModelConfig(app),
    setAppModelConfig: (app: string, config: unknown) =>
      setAppModelConfig(app, config),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
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

vi.mock("@/lib/toast", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

vi.mock("@/lib/query/proxy", () => ({
  proxyKeys: { takeoverStatus: ["takeover"], status: ["status"] },
  useProxyTakeoverStatus: () => ({ data: {}, isPending: false }),
}));

import { AppConfigPage } from "./AppConfigPage";

const entries: GatewayCatalogEntry[] = [
  {
    reference: "CTYun/gpt-5.6-sol",
    upstreamId: "u1",
    upstreamName: "CTYun",
    modelId: "gpt-5.6-sol",
    displayName: "GPT 5.6 Sol",
    oneM: false,
    endpointIds: [],
    sources: ["claude"],
  },
  {
    reference: "Kimi/kimi-k3",
    upstreamId: "u2",
    upstreamName: "Kimi",
    modelId: "kimi-k3",
    displayName: "Kimi K3",
    oneM: false,
    endpointIds: [],
    sources: ["claude"],
  },
];

beforeAll(() => {
  Element.prototype.scrollIntoView = vi.fn();
});

function renderPage(appType = "claude", appLabel = "Claude Code") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <AppConfigPage appType={appType} appLabel={appLabel} />
    </QueryClientProvider>,
  );
}

/** Claude Desktop 的目录条目：来源是 claude-desktop:<供应商>。 */
const desktopEntries: GatewayCatalogEntry[] = entries.map((entry) => ({
  ...entry,
  sources: entry.sources.map((source) => source.replace("claude", "claude-desktop")),
}));

describe("AppConfigPage", () => {
  it("renders the CCR-style form with required fields", async () => {
    getCatalog.mockResolvedValue(entries);
    getAppModelConfig.mockResolvedValue({
      default: "CTYun/gpt-5.6-sol",
      slots: { sonnet: "CTYun/gpt-5.6-sol" },
      allowed: ["CTYun/gpt-5.6-sol"],
    });
    renderPage();

    expect(await screen.findByText("编辑配置")).toBeTruthy();
    expect(screen.getByText("Agent")).toBeTruthy();
    expect(screen.getAllByText("必填").length).toBeGreaterThan(0);
    expect(await screen.findByText("默认模型")).toBeTruthy();
    expect(screen.getByText("Fable 模型")).toBeTruthy();
    expect(screen.getByText("Haiku 模型")).toBeTruthy();
  });

  it("Claude Desktop 也显示固定的四档槽位，值可选", async () => {
    getCatalog.mockResolvedValue(desktopEntries);
    getAppModelConfig.mockResolvedValue({ default: "CTYun/gpt-5.6-sol" });
    setAppModelConfig.mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderPage("claude-desktop", "Claude Desktop");

    for (const label of ["Fable 模型", "Opus 模型", "Sonnet 模型", "Haiku 模型"]) {
      expect(await screen.findByText(label)).toBeTruthy();
    }

    // 找到 Opus 那一格的选择器（表单里还有两个原生 select，不能按下标拿）
    const opusField = screen.getByText("Opus 模型").parentElement;
    expect(opusField).toBeTruthy();
    await user.click(within(opusField as HTMLElement).getByRole("combobox"));
    await user.click(
      within(await screen.findByRole("listbox", { name: "模型" })).getByRole(
        "option",
        { name: /GPT 5.6 Sol/ },
      ),
    );
    await user.click(screen.getByRole("button", { name: "保存" }));
    expect(setAppModelConfig).toHaveBeenCalledWith(
      "claude-desktop",
      expect.objectContaining({ slots: { opus: "CTYun/gpt-5.6-sol" } }),
    );
  });

  it("opens advanced settings with the allowed-model checklist and saves", async () => {
    getCatalog.mockResolvedValue(entries);
    getAppModelConfig.mockResolvedValue({
      default: "CTYun/gpt-5.6-sol",
      allowed: ["CTYun/gpt-5.6-sol"],
    });
    setAppModelConfig.mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByText("高级设置"));
    expect(await screen.findByText("允许模型列表")).toBeTruthy();
    // 目录里的两条都出现在勾选列表里
    expect(await screen.findByText("CTYun/gpt-5.6-sol")).toBeTruthy();
    expect(screen.getByText("Kimi/kimi-k3")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "保存" }));
    expect(setAppModelConfig).toHaveBeenCalledWith(
      "claude",
      expect.objectContaining({
        default: "CTYun/gpt-5.6-sol",
        routing: expect.objectContaining({ scope: "system" }),
      }),
    );
  });
});
