import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SubscriptionQuota } from "@/types/subscription";
import CodexOauthAccountQuota from "@/components/CodexOauthAccountQuota";

const { refetch, consumeCodexResetCredit, state } = vi.hoisted(() => ({
  refetch: vi.fn(),
  consumeCodexResetCredit: vi.fn(),
  state: { quota: undefined as SubscriptionQuota | undefined },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { defaultValue?: string }) =>
      options?.defaultValue ?? key,
    i18n: { language: "zh" },
  }),
}));

vi.mock("@/lib/query/subscription", () => ({
  useCodexOauthQuotaByAccountId: () => ({
    data: state.quota,
    isFetching: false,
    refetch,
  }),
}));

vi.mock("@/lib/api/subscription", () => ({
  subscriptionApi: { consumeCodexResetCredit },
}));

vi.mock("@/components/quota/useQuotaDisplay", () => ({
  useQuotaDisplay: () => "left",
}));

vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }));

const baseQuota = (
  overrides: Partial<SubscriptionQuota> = {},
): SubscriptionQuota => ({
  tool: "codex_oauth",
  credentialStatus: "valid",
  credentialMessage: null,
  success: true,
  tiers: [
    {
      name: "seven_day",
      utilization: 100,
      resetsAt: "2026-11-16T00:00:00Z",
    },
  ],
  extraUsage: null,
  error: null,
  queriedAt: Date.parse("2026-11-09T08:00:00Z"),
  ...overrides,
});

beforeEach(() => {
  vi.clearAllMocks();
  refetch.mockResolvedValue(undefined);
  consumeCodexResetCredit.mockResolvedValue({
    windowsReset: 1,
    creditExpiresAt: "2026-12-02T00:00:00Z",
  });
});

describe("ChatGPT 账号额度的重置卡", () => {
  it("额度已用完且有存下的重置卡时给出「使用重置卡」，确认后才用掉一张", async () => {
    state.quota = baseQuota({
      resetCredits: {
        expiresAt: ["2026-12-02T00:00:00Z"],
        credits: [{ id: "credit-1", expiresAt: "2026-12-02T00:00:00Z" }],
      },
    });
    render(<CodexOauthAccountQuota accountId="acct-1" />);

    // 按钮收在「剩余 N 次」的明细弹窗里，先点开明细
    fireEvent.click(
      screen.getByRole("button", { name: "quota.resetCredits.showAll" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "使用重置卡" }));
    // 用掉不可恢复：先弹确认，不点确认就不发请求
    expect(screen.getByText("使用一张重置卡？")).toBeInTheDocument();
    expect(consumeCodexResetCredit).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "使用" }));

    await waitFor(() =>
      expect(consumeCodexResetCredit).toHaveBeenCalledWith("acct-1"),
    );
    await waitFor(() => expect(refetch).toHaveBeenCalled());
  });

  it("额度没用完时不显示按钮（免得白白用掉一张卡）", () => {
    state.quota = baseQuota({
      tiers: [{ name: "seven_day", utilization: 40, resetsAt: null }],
      resetCredits: {
        expiresAt: ["2026-12-02T00:00:00Z"],
        credits: [{ id: "credit-1", expiresAt: "2026-12-02T00:00:00Z" }],
      },
    });
    render(<CodexOauthAccountQuota accountId="acct-1" />);
    expect(
      screen.queryByRole("button", { name: "使用重置卡" }),
    ).not.toBeInTheDocument();
  });

  it("没有存下的重置卡时不显示按钮", () => {
    state.quota = baseQuota({ resetCredits: null });
    render(<CodexOauthAccountQuota accountId="acct-1" />);
    expect(
      screen.queryByRole("button", { name: "使用重置卡" }),
    ).not.toBeInTheDocument();
  });
});
