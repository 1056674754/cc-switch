import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { CreateSupplierDialog } from "@/components/suppliers/CreateSupplierDialog";
import { suppliersKeys } from "@/lib/query/suppliers";
import type { Supplier } from "@/lib/api/suppliers";

const mocks = vi.hoisted(() => ({
  create: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
}));

vi.mock("@/lib/api/suppliers", () => ({
  suppliersApi: {
    create: (name: string, baseUrl: string, apiKey: string, apps: string[]) =>
      mocks.create(name, baseUrl, apiKey, apps),
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
      const entries = Object.entries(options ?? {}).filter(
        ([name]) => name !== "defaultValue",
      );
      return entries.length === 0
        ? key
        : `${key}:${entries.map(([, value]) => String(value)).join("|")}`;
    },
  }),
}));

const created: Supplier[] = [
  {
    id: "sup-new",
    name: "Acme Gateway",
    host: "api.acme.dev",
    baseUrl: "https://api.acme.dev",
    upstreamProtocol: "anthropic",
    apps: ["claude", "claude-desktop"],
    consumers: ["claude:p-new", "claude-desktop:p-new"],
    credentialCount: 1,
    credentialKinds: ["literal"],
    bindings: [
      {
        appType: "claude",
        providerId: "p-new",
        modelMap: {},
        downstreamProtocol: "anthropic",
      },
      {
        appType: "claude-desktop",
        providerId: "p-new",
        modelMap: {},
        downstreamProtocol: "anthropic",
      },
    ],
  },
];

/** 真的走 open 状态：成功后关掉的话，对话框要从 DOM 里消失。 */
function Harness({ onClose }: { onClose: () => void }) {
  const [open, setOpen] = useState(true);
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        reopen
      </button>
      <CreateSupplierDialog
        open={open}
        onClose={() => {
          onClose();
          setOpen(false);
        }}
      />
    </>
  );
}

const renderDialog = () => {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const invalidateQueries = vi.spyOn(client, "invalidateQueries");
  const onClose = vi.fn();
  render(
    <QueryClientProvider client={client}>
      <button type="button">outside</button>
      <Harness onClose={onClose} />
    </QueryClientProvider>,
  );
  return { client, invalidateQueries, onClose };
};

const fillRequiredFields = async (
  user: ReturnType<typeof userEvent.setup>,
  overrides: { name?: string; baseUrl?: string; secret?: string } = {},
) => {
  await user.type(
    screen.getByLabelText("suppliers.createNameLabel"),
    overrides.name ?? "  Acme Gateway  ",
  );
  await user.type(
    screen.getByLabelText("suppliers.createBaseUrlLabel"),
    overrides.baseUrl ?? "  https://api.acme.dev  ",
  );
  await user.type(
    screen.getByLabelText("suppliers.createSecretLabel"),
    overrides.secret ?? "  sk-new-key  ",
  );
};

const clickCreate = async (user: ReturnType<typeof userEvent.setup>) => {
  await user.click(
    screen.getByRole("button", { name: "suppliers.createSubmit" }),
  );
};

/** Radix 在下一帧才开始听外部 pointerdown：先让它挂上，再点遮罩外面 */
async function clickOutside() {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
  await act(async () => {
    const target = screen.getByRole("button", {
      name: "outside",
      hidden: true,
    });
    fireEvent.pointerDown(target, { button: 0, pointerType: "mouse" });
    fireEvent.click(target);
  });
}

describe("CreateSupplierDialog", () => {
  beforeEach(() => {
    mocks.create.mockReset();
    mocks.success.mockReset();
    mocks.error.mockReset();
  });

  it("starts with an empty key and every supported app checked", () => {
    renderDialog();

    expect(screen.getByLabelText("suppliers.createNameLabel")).toHaveValue("");
    expect(screen.getByLabelText("suppliers.createBaseUrlLabel")).toHaveValue(
      "",
    );
    const secret = screen.getByLabelText("suppliers.createSecretLabel");
    expect(secret).toHaveValue("");
    expect(secret).toHaveAttribute("type", "password");
    // 默认全勾：claude / claude-desktop 都接上
    expect(screen.getByRole("checkbox", { name: "Claude" })).toBeChecked();
    expect(
      screen.getByRole("checkbox", { name: "Claude Desktop" }),
    ).toBeChecked();
    expect(
      screen.getByRole("button", { name: "suppliers.createSubmit" }),
    ).toBeEnabled();
  });

  it("starts clean every time it reopens", async () => {
    const { onClose } = renderDialog();
    const user = userEvent.setup();

    await fillRequiredFields(user);
    await user.click(screen.getByRole("checkbox", { name: "Claude" }));
    await user.keyboard("{Escape}");
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );

    await user.click(screen.getByRole("button", { name: "reopen" }));

    // 上一轮的输入、勾选、密钥都不带走
    expect(screen.getByLabelText("suppliers.createNameLabel")).toHaveValue("");
    expect(screen.getByLabelText("suppliers.createBaseUrlLabel")).toHaveValue(
      "",
    );
    expect(screen.getByLabelText("suppliers.createSecretLabel")).toHaveValue(
      "",
    );
    expect(screen.getByRole("checkbox", { name: "Claude" })).toBeChecked();
    expect(
      screen.getByRole("checkbox", { name: "Claude Desktop" }),
    ).toBeChecked();
  });

  it("can reveal the key while typing and hide it again", async () => {
    renderDialog();
    const user = userEvent.setup();

    // 空密钥时不显示切换按钮，有内容才给
    expect(
      screen.queryByRole("button", { name: "apiKeyInput.show" }),
    ).not.toBeInTheDocument();
    await user.type(
      screen.getByLabelText("suppliers.createSecretLabel"),
      "sk-live",
    );

    await user.click(screen.getByRole("button", { name: "apiKeyInput.show" }));
    expect(
      screen.getByLabelText("suppliers.createSecretLabel"),
    ).toHaveAttribute("type", "text");

    await user.click(screen.getByRole("button", { name: "apiKeyInput.hide" }));
    expect(
      screen.getByLabelText("suppliers.createSecretLabel"),
    ).toHaveAttribute("type", "password");
  });

  it("refuses an empty name without sending anything", async () => {
    renderDialog();
    const user = userEvent.setup();

    await fillRequiredFields(user, { name: "   " });
    // 名称框里只留下空格
    await clickCreate(user);

    expect(
      screen.getByText("suppliers.createNameRequired"),
    ).toBeInTheDocument();
    expect(mocks.create).not.toHaveBeenCalled();
    expect(mocks.success).not.toHaveBeenCalled();
  });

  it("refuses an address that is not a complete http(s) URL without sending anything", async () => {
    renderDialog();
    const user = userEvent.setup();

    await fillRequiredFields(user, { baseUrl: "api.acme.dev" });
    await clickCreate(user);

    expect(
      screen.getByText("suppliers.createBaseUrlInvalid"),
    ).toBeInTheDocument();
    expect(mocks.create).not.toHaveBeenCalled();
  });

  it("refuses an empty key without sending anything", async () => {
    renderDialog();
    const user = userEvent.setup();

    await fillRequiredFields(user, { secret: "   " });
    await clickCreate(user);

    expect(
      screen.getByText("suppliers.createSecretRequired"),
    ).toBeInTheDocument();
    expect(mocks.create).not.toHaveBeenCalled();
  });

  it("refuses submitting with no app checked", async () => {
    renderDialog();
    const user = userEvent.setup();

    await fillRequiredFields(user);
    await user.click(screen.getByRole("checkbox", { name: "Claude" }));
    await user.click(screen.getByRole("checkbox", { name: "Claude Desktop" }));
    await clickCreate(user);

    expect(
      screen.getByText("suppliers.createAppsRequired"),
    ).toBeInTheDocument();
    expect(mocks.create).not.toHaveBeenCalled();
  });

  it("creates the supplier, replaces the overview, refreshes apps and closes", async () => {
    mocks.create.mockResolvedValue(created);
    const { client, invalidateQueries, onClose } = renderDialog();
    const user = userEvent.setup();

    await fillRequiredFields(user);
    await clickCreate(user);

    await waitFor(() =>
      expect(mocks.create).toHaveBeenCalledWith(
        "Acme Gateway",
        "https://api.acme.dev",
        "sk-new-key",
        ["claude", "claude-desktop"],
      ),
    );
    // 后端返回的总览直接落缓存，不再多拉一次
    expect(client.getQueryData(suppliersKeys.overview)).toEqual(created);
    // 各应用里多了一条配置，供应商列表要重读
    await waitFor(() => {
      const queryKeys = invalidateQueries.mock.calls.map(
        ([filters]) => filters?.queryKey,
      );
      expect(queryKeys).toContainEqual(["providers"]);
    });
    expect(mocks.success).toHaveBeenCalledWith(
      "suppliers.createSuccess:Acme Gateway",
    );
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
  });

  it("keeps the dialog open, toasts the backend reason and allows a retry", async () => {
    mocks.create.mockRejectedValueOnce("供应商名称不能为空");
    mocks.create.mockResolvedValueOnce(created);
    const { onClose } = renderDialog();
    const user = userEvent.setup();

    await fillRequiredFields(user, { name: "Acme Gateway" });
    await clickCreate(user);

    await waitFor(() =>
      expect(mocks.error).toHaveBeenCalledWith("供应商名称不能为空"),
    );
    // 失败不关：输入原样留着，改完能直接重试
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByLabelText("suppliers.createNameLabel")).toHaveValue(
      "Acme Gateway",
    );
    expect(onClose).not.toHaveBeenCalled();

    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "suppliers.createSubmit" }),
      ).not.toBeDisabled(),
    );
    await clickCreate(user);
    await waitFor(() => expect(mocks.create).toHaveBeenCalledTimes(2));
    expect(mocks.success).toHaveBeenCalledWith(
      "suppliers.createSuccess:Acme Gateway",
    );
  });

  it("disables every field and button while the request is in flight", async () => {
    let resolveCreate!: (value: Supplier[]) => void;
    mocks.create.mockReturnValue(
      new Promise<Supplier[]>((resolve) => {
        resolveCreate = resolve;
      }),
    );
    const { onClose } = renderDialog();
    const user = userEvent.setup();

    await fillRequiredFields(user);
    await clickCreate(user);

    await waitFor(() =>
      expect(screen.getByLabelText("suppliers.createNameLabel")).toBeDisabled(),
    );
    expect(
      screen.getByLabelText("suppliers.createBaseUrlLabel"),
    ).toBeDisabled();
    expect(screen.getByLabelText("suppliers.createSecretLabel")).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: "Claude" })).toBeDisabled();
    expect(
      screen.getByRole("checkbox", { name: "Claude Desktop" }),
    ).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "common.cancel" }),
    ).toBeDisabled();
    const submit = screen.getByRole("button", {
      name: "suppliers.createSubmit",
    });
    expect(submit).toBeDisabled();
    expect(submit).toHaveAttribute("aria-busy", "true");

    await act(async () => {
      resolveCreate(created);
    });

    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });

  it("closes on Escape", async () => {
    const { onClose } = renderDialog();
    const user = userEvent.setup();

    await user.keyboard("{Escape}");

    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    expect(mocks.create).not.toHaveBeenCalled();
  });

  it("closes when the overlay is clicked", async () => {
    const { onClose } = renderDialog();

    await clickOutside();

    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("ignores Escape and overlay clicks while submitting", async () => {
    let resolveCreate!: (value: Supplier[]) => void;
    mocks.create.mockReturnValue(
      new Promise<Supplier[]>((resolve) => {
        resolveCreate = resolve;
      }),
    );
    const { onClose } = renderDialog();
    const user = userEvent.setup();

    await fillRequiredFields(user);
    await clickCreate(user);
    await waitFor(() =>
      expect(screen.getByLabelText("suppliers.createNameLabel")).toBeDisabled(),
    );

    await user.keyboard("{Escape}");
    await clickOutside();

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();

    await act(async () => {
      resolveCreate(created);
    });
    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });
});
