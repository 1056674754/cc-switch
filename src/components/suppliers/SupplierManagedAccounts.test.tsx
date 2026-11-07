import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

vi.mock("@/components/providers/forms/CodexOAuthSection", () => ({
  CodexOAuthSection: (props: {
    onSetDefaultAccount?: (accountId: string) => void;
    settingDefaultAccount?: boolean;
    helpNote?: React.ReactNode;
  }) => (
    <div>
      CHATGPT-ACCOUNTS
      <span data-testid="codex-on-set-default">
        {String(Boolean(props.onSetDefaultAccount))}
      </span>
      <span data-testid="codex-help-note">{props.helpNote ?? "none"}</span>
    </div>
  ),
}));
vi.mock("@/components/providers/forms/ClaudeOfficialSection", () => ({
  ClaudeOfficialSection: () => <div>CLAUDE-ACCOUNTS</div>,
}));
vi.mock("@/components/providers/forms/CopilotAuthSection", () => ({
  CopilotAuthSection: () => <div>COPILOT-ACCOUNTS</div>,
}));
vi.mock("@/components/providers/forms/XaiOAuthSection", () => ({
  XaiOAuthSection: () => <div>XAI-ACCOUNTS</div>,
}));
vi.mock("@/components/providers/forms/AgyOAuthSection", () => ({
  AgyOAuthSection: () => <div>AGY-ACCOUNTS</div>,
}));

import {
  SupplierManagedAccounts,
  isManagedAuthProvider,
} from "./SupplierManagedAccounts";

describe("SupplierManagedAccounts", () => {
  it.each([
    ["codex_oauth", "CHATGPT-ACCOUNTS"],
    ["claude_official", "CLAUDE-ACCOUNTS"],
    ["github_copilot", "COPILOT-ACCOUNTS"],
    ["xai_oauth", "XAI-ACCOUNTS"],
    ["agy_oauth", "AGY-ACCOUNTS"],
  ] as const)("%s 嵌对应的账号组", (provider, expected) => {
    render(<SupplierManagedAccounts provider={provider} />);
    expect(screen.getByText(expected)).toBeTruthy();
  });

  it("供应商卡片把「设为默认」和补充说明透传给账号组", () => {
    const onSetDefaultAccount = vi.fn();
    render(
      <SupplierManagedAccounts
        provider="codex_oauth"
        onSetDefaultAccount={onSetDefaultAccount}
      />,
    );
    expect(screen.getByTestId("codex-on-set-default").textContent).toBe("true");
    // 「?」里补一句：这里的「设为默认」是让这家供应商改用该账号
    expect(screen.getByTestId("codex-help-note").textContent).toContain(
      "设为默认",
    );
  });

  it("授权中心语义（不传 onSetDefaultAccount）时不补那句说明", () => {
    render(<SupplierManagedAccounts provider="codex_oauth" />);
    expect(screen.getByTestId("codex-on-set-default").textContent).toBe(
      "false",
    );
    expect(screen.getByTestId("codex-help-note").textContent).toBe("none");
  });

  it("只认这几个托管登录 provider，其它值不当账号池", () => {
    expect(isManagedAuthProvider("codex_oauth")).toBe(true);
    expect(isManagedAuthProvider("kimi")).toBe(false);
    expect(isManagedAuthProvider("gemini_official")).toBe(false);
    expect(isManagedAuthProvider(null)).toBe(false);
    expect(isManagedAuthProvider(undefined)).toBe(false);
  });
});
