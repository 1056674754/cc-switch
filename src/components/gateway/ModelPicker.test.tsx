import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeAll, describe, expect, it, vi } from "vitest";
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

beforeAll(() => {
  // jsdom 没有 scrollIntoView，cmdk 选中项时会调
  Element.prototype.scrollIntoView = vi.fn();
});

describe("ModelPicker", () => {
  it("shows the selected model label", () => {
    render(<ModelPicker entries={entries} value="Kimi/kimi-k3" onChange={() => {}} />);
    expect(screen.getByRole("combobox")).toHaveTextContent("Kimi K3");
  });

  it("marks a value missing from the catalog", () => {
    render(<ModelPicker entries={entries} value="Gone/model" onChange={() => {}} />);
    expect(screen.getByRole("combobox")).toHaveTextContent("Gone/model");
  });

  it("lists models grouped by supplier and selects one", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ModelPicker entries={entries} onChange={onChange} />);
    await user.click(screen.getByRole("combobox"));
    expect(await screen.findByText("DeepSeek V4.1")).toBeInTheDocument();
    expect(screen.getByText("Kimi K3")).toBeInTheDocument();
    await user.click(screen.getByText("Kimi K3"));
    expect(onChange).toHaveBeenCalledWith("Kimi/kimi-k3");
  });
});
