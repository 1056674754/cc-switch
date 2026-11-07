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
  ensureManaged: vi.fn(),
  authGetStatus: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
}));

vi.mock("@/lib/api/suppliers", () => ({
  suppliersApi: {
    create: (
      name: string,
      baseUrl: string,
      apiKey: string,
      apps: string[],
      protocol?: string,
    ) => mocks.create(name, baseUrl, apiKey, apps, protocol),
    ensureManaged: (authProvider: string) => mocks.ensureManaged(authProvider),
  },
}));

vi.mock("@/lib/api/auth", () => ({
  authApi: {
    authGetStatus: (provider: string) => mocks.authGetStatus(provider),
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
    enabled: true,
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

/** 向导第一步是选来源；绝大多数用例走「自定义地址」这套表单。 */
const openCustomForm = async (user: ReturnType<typeof userEvent.setup>) => {
  await user.click(
    screen.getByRole("button", { name: /suppliers\.createSourceCustom/ }),
  );
};

const fillRequiredFields = async (
  user: ReturnType<typeof userEvent.setup>,
  overrides: { name?: string; baseUrl?: string; secret?: string } = {},
) => {
  await openCustomForm(user);
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
    mocks.ensureManaged.mockReset();
    mocks.authGetStatus.mockReset();
    mocks.authGetStatus.mockResolvedValue({ accounts: [] });
    mocks.success.mockReset();
    mocks.error.mockReset();
  });

  it("starts on the source chooser with preset / import / custom entries", () => {
    renderDialog();

    expect(
      screen.getByText("suppliers.createSourcePreset"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("suppliers.createSourceImport"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("suppliers.createSourceCustom"),
    ).toBeInTheDocument();
    // 还没有进表单：不显示任何输入框
    expect(
      screen.queryByLabelText("suppliers.createNameLabel"),
    ).not.toBeInTheDocument();
  });

  it("opens an empty form with no app pre-checked (creation is decoupled)", async () => {
    renderDialog();
    const user = userEvent.setup();
    await openCustomForm(user);

    expect(screen.getByLabelText("suppliers.createNameLabel")).toHaveValue("");
    expect(screen.getByLabelText("suppliers.createBaseUrlLabel")).toHaveValue(
      "",
    );
    const secret = screen.getByLabelText("suppliers.createSecretLabel");
    expect(secret).toHaveValue("");
    expect(secret).toHaveAttribute("type", "password");
    // 创建供应商与接入 app 解耦：默认一个都不勾
    expect(screen.getByRole("checkbox", { name: "Claude" })).not.toBeChecked();
    expect(
      screen.getByRole("checkbox", { name: "Claude Desktop" }),
    ).not.toBeChecked();
    expect(
      screen.getByRole("button", { name: "suppliers.createSubmit" }),
    ).toBeEnabled();
  });

  it("starts clean every time it reopens", async () => {
    const { onClose } = renderDialog();
    const user = userEvent.setup();

    await fillRequiredFields(user);
    await user.keyboard("{Escape}");
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );

    await user.click(screen.getByRole("button", { name: "reopen" }));

    // 上一轮的输入、勾选、密钥都不带走：回到来源选择这一步
    expect(
      screen.getByText("suppliers.createSourceCustom"),
    ).toBeInTheDocument();
    expect(
      screen.queryByLabelText("suppliers.createNameLabel"),
    ).not.toBeInTheDocument();
  });

  it("can reveal the key while typing and hide it again", async () => {
    renderDialog();
    const user = userEvent.setup();

    await openCustomForm(user);
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

  it("creates the supplier with no app checked (creation is independent of access)", async () => {
    mocks.create.mockResolvedValue(created);
    const { onClose } = renderDialog();
    const user = userEvent.setup();

    await fillRequiredFields(user);
    await clickCreate(user);

    await waitFor(() =>
      expect(mocks.create).toHaveBeenCalledWith(
        "Acme Gateway",
        "https://api.acme.dev",
        "sk-new-key",
        [],
        "anthropic",
      ),
    );
    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });

  it("creates the supplier, replaces the overview, refreshes apps and closes", async () => {
    mocks.create.mockResolvedValue(created);
    const { client, invalidateQueries, onClose } = renderDialog();
    const user = userEvent.setup();

    await fillRequiredFields(user);
    await user.click(screen.getByRole("checkbox", { name: "Claude" }));
    await user.click(screen.getByRole("checkbox", { name: "Claude Desktop" }));
    await clickCreate(user);

    await waitFor(() =>
      expect(mocks.create).toHaveBeenCalledWith(
        "Acme Gateway",
        "https://api.acme.dev",
        "sk-new-key",
        ["claude", "claude-desktop"],
        "anthropic",
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
    await user.click(screen.getByRole("checkbox", { name: "Claude" }));
    await user.click(screen.getByRole("checkbox", { name: "Claude Desktop" }));
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

  it("filters presets and fills name / address / protocol from the picked one", async () => {
    renderDialog();
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: /suppliers\.createSourcePreset/ }),
    );
    const search = screen.getByLabelText("suppliers.createPresetSearch");
    await user.type(search, "deepseek");
    // 结果里只剩匹配项：点进去表单已经带着预设值
    expect(screen.queryByText("suppliers.createPresetEmpty")).toBeNull();
    const [firstPreset] = screen.getAllByRole("button", {
      name: /https?:\/\//,
    });
    await user.click(firstPreset);

    expect(screen.getByLabelText("suppliers.createNameLabel")).not.toHaveValue(
      "",
    );
    expect(
      screen.getByLabelText("suppliers.createBaseUrlLabel"),
    ).not.toHaveValue("");
    expect(screen.getByLabelText("suppliers.createProtocolLabel")).toHaveValue(
      "anthropic",
    );
  });

  it("hides the app picker for non-anthropic protocols and creates without apps", async () => {
    mocks.create.mockResolvedValue(created);
    const { onClose } = renderDialog();
    const user = userEvent.setup();

    await fillRequiredFields(user);
    await user.selectOptions(
      screen.getByLabelText("suppliers.createProtocolLabel"),
      "openai_responses",
    );
    // 非 anthropic 协议先不接应用
    expect(
      screen.queryByRole("checkbox", { name: "Claude" }),
    ).not.toBeInTheDocument();
    await clickCreate(user);

    await waitFor(() =>
      expect(mocks.create).toHaveBeenCalledWith(
        "Acme Gateway",
        "https://api.acme.dev",
        "sk-new-key",
        [],
        "openai_responses",
      ),
    );
    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });

  it("imports a detected local sign-in as a managed supplier", async () => {
    mocks.authGetStatus.mockImplementation((provider: string) =>
      Promise.resolve({
        accounts:
          provider === "codex_oauth" ? [{ id: "acc-1" }, { id: "acc-2" }] : [],
      }),
    );
    mocks.ensureManaged.mockResolvedValue(created);
    const { client, onClose } = renderDialog();
    const user = userEvent.setup();

    await user.click(
      screen.getByRole("button", { name: /suppliers\.createSourceImport/ }),
    );
    // 检测到 Codex 登录态：显示账号数并提供导入
    expect(
      await screen.findByText("suppliers.createImportAccounts:2"),
    ).toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "suppliers.createImportAction" }),
    );

    await waitFor(() =>
      expect(mocks.ensureManaged).toHaveBeenCalledWith("codex_oauth"),
    );
    expect(client.getQueryData(suppliersKeys.overview)).toEqual(created);
    expect(mocks.success).toHaveBeenCalledWith("suppliers.createImportSuccess");
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
  });

  it("goes back to the source chooser without losing the page", async () => {
    renderDialog();
    const user = userEvent.setup();

    await openCustomForm(user);
    await user.click(
      screen.getByRole("button", { name: "suppliers.createBack" }),
    );
    expect(
      screen.getByText("suppliers.createSourcePreset"),
    ).toBeInTheDocument();
    expect(
      screen.queryByLabelText("suppliers.createNameLabel"),
    ).not.toBeInTheDocument();
  });
});
