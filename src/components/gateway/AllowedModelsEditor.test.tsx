import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeAll, describe, expect, it, vi } from "vitest";
import type { GatewayCatalogEntry } from "@/types/gateway";
import { AllowedModelsEditor } from "./AllowedModelsEditor";

beforeAll(() => {
  Element.prototype.scrollIntoView = vi.fn();
});

const entries: GatewayCatalogEntry[] = [
  {
    reference: "Kimi/kimi-k3",
    upstreamId: "up-2",
    upstreamName: "Kimi",
    modelId: "kimi-k3",
    displayName: "Kimi K3",
    endpointIds: ["ep-2"],
    sources: ["claude:Kimi"],
  },
];

describe("AllowedModelsEditor", () => {
  it("says unrestricted when the list is empty", () => {
    render(<AllowedModelsEditor entries={entries} onChange={() => {}} />);
    expect(screen.getByText(/未限制/)).toBeInTheDocument();
  });

  it("shows chips, marks the default, and removes one", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <AllowedModelsEditor
        entries={entries}
        allowed={["Kimi/kimi-k3", "Gone/removed"]}
        defaultReference="Kimi/kimi-k3"
        onChange={onChange}
      />,
    );
    expect(screen.getByText("默认")).toBeInTheDocument();
    await user.click(screen.getByLabelText("移除 Kimi/kimi-k3"));
    expect(onChange).toHaveBeenCalledWith(["Gone/removed"]);
  });
});
