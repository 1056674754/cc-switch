import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const state = vi.hoisted(() => ({ adopted: false }));
const adopt = vi.fn(async () => {
  state.adopted = true;
});
const release = vi.fn(async () => {
  state.adopted = false;
});
const status = vi.fn(async () => state.adopted);

vi.mock("@/lib/api/gateway", () => ({
  gatewayApi: {
    getOpencodeGatewayStatus: () => status(),
    adoptOpencodeToGateway: () => adopt(),
    releaseOpencodeFromGateway: () => release(),
  },
}));

const toastSuccess = vi.fn();
vi.mock("@/lib/toast", () => ({
  toast: { success: (message: string) => toastSuccess(message), error: vi.fn() },
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

import { OpencodeGatewayBar } from "./OpencodeGatewayBar";

function renderBar() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <OpencodeGatewayBar />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  state.adopted = false;
  vi.clearAllMocks();
});

describe("OpencodeGatewayBar", () => {
  it("adopts the gateway and flips the status", async () => {
    const user = userEvent.setup();
    renderBar();
    expect(await screen.findByText("直连（各供应商自己配地址）")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "指向网关" }));
    await waitFor(() => expect(adopt).toHaveBeenCalledTimes(1));
    expect(
      await screen.findByText("全部来自网关（模型 = 网关里的引用）"),
    ).toBeTruthy();
    expect(toastSuccess).toHaveBeenCalled();
  });

  it("releases the gateway when already adopted", async () => {
    state.adopted = true;
    const user = userEvent.setup();
    renderBar();
    expect(
      await screen.findByText("全部来自网关（模型 = 网关里的引用）"),
    ).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "释放" }));
    await waitFor(() => expect(release).toHaveBeenCalledTimes(1));
    expect(await screen.findByText("直连（各供应商自己配地址）")).toBeTruthy();
  });
});
