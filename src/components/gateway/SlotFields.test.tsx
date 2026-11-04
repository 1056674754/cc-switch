import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeAll, describe, expect, it, vi } from "vitest";
import type { GatewayCatalogEntry } from "@/types/gateway";
import { SlotFields } from "./SlotFields";

beforeAll(() => {
  Element.prototype.scrollIntoView = vi.fn();
});

const entries: GatewayCatalogEntry[] = [
  {
    reference: "CTYun/qwen3.8-max",
    upstreamId: "up-1",
    upstreamName: "CTYun",
    modelId: "qwen3.8-max",
    displayName: "Qwen3.8 Max",
    endpointIds: ["ep-1"],
    sources: ["claude:CTYun"],
  },
];

describe("SlotFields", () => {
  it("renders the four claude slots and nothing for other apps", () => {
    const { rerender } = render(
      <SlotFields appType="claude" entries={entries} onChange={() => {}} />,
    );
    expect(screen.getByText("Fable")).toBeInTheDocument();
    expect(screen.getByText("Opus")).toBeInTheDocument();
    expect(screen.getByText("Sonnet")).toBeInTheDocument();
    expect(screen.getByText("Haiku")).toBeInTheDocument();

    rerender(<SlotFields appType="codex" entries={entries} onChange={() => {}} />);
    expect(screen.queryByText("Fable")).not.toBeInTheDocument();
  });

  it("selects a model for a slot", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<SlotFields appType="claude" entries={entries} onChange={onChange} />);
    await user.click(screen.getAllByRole("combobox")[0]);
    await user.click(await screen.findByText("Qwen3.8 Max"));
    expect(onChange).toHaveBeenCalledWith({ fable: "CTYun/qwen3.8-max" });
  });
});
