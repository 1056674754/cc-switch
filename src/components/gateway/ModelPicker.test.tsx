import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { GatewayCatalogEntry } from "@/types/gateway";
import { ModelPicker } from "./ModelPicker";

const entries: GatewayCatalogEntry[] = [
  {
    reference: "CTYun/deepseek-v4.1-flash-1017",
    upstreamId: "up-ctyun",
    upstreamName: "CTYun",
    modelId: "deepseek-v4.1-flash-1017",
    displayName: "DeepSeek V4.1",
    oneM: true,
    window: 200000,
    endpointIds: ["ep-1"],
    sources: ["claude:CTYun"],
  },
  {
    reference: "CTYun/qwen3.8-max",
    upstreamId: "up-ctyun",
    upstreamName: "CTYun",
    modelId: "qwen3.8-max",
    oneM: false,
    window: 1_000_000,
    endpointIds: ["ep-1"],
    sources: ["claude:CTYun"],
  },
  {
    reference: "Kimi/kimi-k3",
    upstreamId: "up-kimi",
    upstreamName: "Kimi",
    modelId: "kimi-k3",
    displayName: "Kimi K3",
    oneM: false,
    window: 262144,
    endpointIds: ["ep-2"],
    sources: ["claude:Kimi"],
  },
];

const supplierPane = () =>
  screen.getByRole("listbox", { name: "供应商" }) as HTMLElement;
const modelPane = () =>
  screen.getByRole("listbox", { name: "模型" }) as HTMLElement;

describe("ModelPicker", () => {
  it("shows the selected model label", () => {
    render(<ModelPicker entries={entries} value="Kimi/kimi-k3" onChange={() => {}} />);
    expect(screen.getByRole("combobox")).toHaveTextContent("Kimi K3");
  });

  it("marks a value missing from the catalog", () => {
    render(<ModelPicker entries={entries} value="Gone/model" onChange={() => {}} />);
    expect(screen.getByRole("combobox")).toHaveTextContent("Gone/model");
  });

  it("opens on the current supplier and lists only that supplier's models", async () => {
    const user = userEvent.setup();
    render(<ModelPicker entries={entries} value="Kimi/kimi-k3" onChange={() => {}} />);
    await user.click(screen.getByRole("combobox"));

    // 左栏：每个供应商 + 模型数
    const suppliers = within(supplierPane()).getAllByRole("option");
    expect(suppliers.map((node) => node.textContent)).toEqual(["CTYun2", "Kimi1"]);
    // 打开时定位到当前值所在的供应商
    expect(suppliers[1]).toHaveAttribute("aria-selected", "true");
    expect(within(modelPane()).getAllByRole("option")).toHaveLength(1);
    expect(within(modelPane()).getByText("Kimi K3")).toBeInTheDocument();
  });

  it("switches supplier and selects a model", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ModelPicker entries={entries} onChange={onChange} />);
    await user.click(screen.getByRole("combobox"));

    // 没有当前值时停在第一个供应商
    expect(within(modelPane()).getByText("DeepSeek V4.1")).toBeInTheDocument();
    expect(within(modelPane()).queryByText("Kimi K3")).not.toBeInTheDocument();

    await user.click(within(supplierPane()).getByText("Kimi"));
    await user.click(within(modelPane()).getByText("Kimi K3"));
    expect(onChange).toHaveBeenCalledWith("Kimi/kimi-k3");
  });

  it("searches suppliers and models at once", async () => {
    const user = userEvent.setup();
    render(<ModelPicker entries={entries} onChange={() => {}} />);
    await user.click(screen.getByRole("combobox"));

    const search = screen.getByLabelText("搜索供应商或模型");
    await user.type(search, "kimi");
    const suppliers = within(supplierPane()).getAllByRole("option");
    expect(suppliers.map((node) => node.textContent)).toEqual(["Kimi1"]);

    await user.clear(search);
    await user.type(search, "qwen");
    // 供应商名不命中，但它的模型命中：左栏保留该供应商，右栏只剩命中的模型
    expect(within(supplierPane()).getAllByRole("option").map((node) => node.textContent)).toEqual([
      "CTYun1",
    ]);
    expect(within(modelPane()).getAllByRole("option").map((node) => node.textContent?.[0])).toEqual([
      "q",
    ]);

    await user.clear(search);
    await user.type(search, "nothing-here");
    expect(
      within(supplierPane()).getByText("没有匹配项"),
    ).toBeInTheDocument();
  });
});
