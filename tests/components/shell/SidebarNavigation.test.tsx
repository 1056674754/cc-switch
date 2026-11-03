import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { QueryClientProvider } from "@tanstack/react-query";
import { Sidebar } from "@/components/shell/Sidebar";
import type { View } from "@/lib/navigation";
import { DEFAULT_VISIBLE_APPS } from "@/config/appConfig";
import { createTestQueryClient } from "../../utils/testQueryClient";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { value?: string }) =>
      options?.value != null ? `${key}:${options.value}` : key,
    i18n: { resolvedLanguage: "en" },
  }),
}));

vi.mock("@/contexts/UpdateContext", () => ({
  useUpdate: () => ({ hasUpdate: false, updateInfo: null }),
}));

vi.mock("@/hooks/useSidebarStatus", () => ({
  useSidebarStatus: () => ({
    appStatus: () => ({ mode: "direct", alert: false }),
    todayCost: 0,
    todayTokens: 0,
    todayRequests: 0,
    todayHitRate: 0,
    authNeedsAttention: false,
  }),
}));

const onSelectPage = vi.fn();

const renderSidebar = (view: View) => {
  const client = createTestQueryClient();
  // 不让设置查询回头去 mock 后端重取，盖掉这里给的空设置
  client.setQueryDefaults(["settings"], { staleTime: Infinity });
  client.setQueryData(["settings"], {});
  render(
    <QueryClientProvider client={client}>
      <Sidebar
        activeApp="claude"
        view={view}
        visibleApps={DEFAULT_VISIBLE_APPS}
        settingsSection="general"
        onSelectApp={vi.fn()}
        onSelectPage={onSelectPage}
        onSelectSettingsSection={vi.fn()}
        onExitSettings={vi.fn()}
      />
    </QueryClientProvider>,
  );
};

/** a 在文档里出现在 b 之前 */
const precedes = (a: HTMLElement, b: HTMLElement) =>
  Boolean(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);

describe("Sidebar navigation order", () => {
  it("puts the gateway group before every app and keeps one supplier entry", () => {
    renderSidebar("providers");

    const suppliers = screen.getByRole("button", { name: "nav.suppliers" });
    const bindings = screen.getByRole("button", { name: "nav.bindings" });
    const claude = screen.getByRole("button", { name: "Claude Code" });
    const usage = screen.getByRole("button", { name: "nav.usage" });

    // 网关（供应商 → 接入端）→ 应用 → 全局页
    expect(precedes(suppliers, bindings)).toBe(true);
    expect(precedes(bindings, claude)).toBe(true);
    expect(precedes(claude, usage)).toBe(true);

    // 供应商以前贴在全局页最底下，现在只出现一次
    expect(
      screen.getAllByRole("button", { name: "nav.suppliers" }),
    ).toHaveLength(1);
  });

  it("opens the bindings page from the gateway group", async () => {
    renderSidebar("providers");

    await userEvent
      .setup()
      .click(screen.getByRole("button", { name: "nav.bindings" }));

    expect(onSelectPage).toHaveBeenCalledWith("bindings");
  });

  it("highlights the gateway group when the view is a gateway page", () => {
    renderSidebar("bindings");

    expect(
      screen.getByRole("button", { name: "nav.bindings" }),
    ).toHaveAttribute("aria-current", "page");
    expect(
      screen.getByRole("button", { name: "nav.suppliers" }),
    ).not.toHaveAttribute("aria-current");
  });
});
