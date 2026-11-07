import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AuthSettingsPanel } from "@/components/providers/AuthSettingsPanel";

vi.mock("@/components/common/FullScreenPanel", () => ({
  FullScreenPanel: ({
    isOpen,
    title,
    children,
  }: {
    isOpen: boolean;
    title: string;
    children: React.ReactNode;
  }) =>
    isOpen ? (
      <div>
        <h1>{title}</h1>
        {children}
      </div>
    ) : null,
}));

// 账号区必须是供应商卡片里那一套组件，而不是授权中心那套页面
vi.mock("@/components/suppliers/SupplierManagedAccounts", () => ({
  isManagedAuthProvider: (value: string) =>
    [
      "github_copilot",
      "codex_oauth",
      "xai_oauth",
      "agy_oauth",
      "claude_official",
    ].includes(value),
  SupplierManagedAccounts: ({ provider }: { provider: string }) => (
    <div>SHARED-ACCOUNTS:{provider}</div>
  ),
}));

vi.mock("@/components/providers/KimiAccountLogin", () => ({
  KimiAccountLogin: () => <div>KIMI-ACCOUNTS</div>,
}));

vi.mock("@/components/providers/DshAccount", () => ({
  DshAccountLogin: () => <div>DSH-ACCOUNTS</div>,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe("AuthSettingsPanel", () => {
  it("renders the shared managed-account area for an official subscription", () => {
    render(<AuthSettingsPanel target="codex_oauth" onClose={() => {}} />);

    expect(screen.getByText("SHARED-ACCOUNTS:codex_oauth")).toBeInTheDocument();
    expect(screen.getByText("OpenAI Official")).toBeInTheDocument();
  });

  it("renders the Kimi account group for kimi_oauth", () => {
    render(<AuthSettingsPanel target="kimi_oauth" onClose={() => {}} />);

    expect(screen.getByText("KIMI-ACCOUNTS")).toBeInTheDocument();
    expect(screen.getByText("Kimi Code")).toBeInTheDocument();
  });

  it("renders the DeepSeek Harness account group for dsh_account", () => {
    render(<AuthSettingsPanel target="dsh_account" onClose={() => {}} />);

    expect(screen.getByText("DSH-ACCOUNTS")).toBeInTheDocument();
    expect(screen.getByText("DeepSeek Harness")).toBeInTheDocument();
  });

  it("stays closed without a target", () => {
    render(<AuthSettingsPanel target={null} onClose={() => {}} />);

    expect(screen.queryByText(/ACCOUNTS/)).toBeNull();
  });
});
