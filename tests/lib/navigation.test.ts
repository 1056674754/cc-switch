import { beforeEach, describe, expect, it } from "vitest";
import {
  isGlobalPage,
  parseView,
  readStoredView,
  storeView,
  VIEW_STORAGE_KEY,
} from "@/lib/navigation";

describe("navigation views", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("accepts the bindings view", () => {
    expect(parseView("bindings")).toBe("bindings");
    expect(isGlobalPage("bindings")).toBe(true);
    // 供应商挪到侧栏顶部之后仍然是一个合法页面
    expect(parseView("suppliers")).toBe("suppliers");
  });

  it("round-trips a stored view through localStorage", () => {
    storeView("bindings");
    expect(localStorage.getItem(VIEW_STORAGE_KEY)).toBe("bindings");
    expect(readStoredView()).toBe("bindings");
  });

  it("falls back to the providers page for an unknown stored view", () => {
    localStorage.setItem(VIEW_STORAGE_KEY, "not-a-view");
    expect(readStoredView()).toBe("providers");
  });
});
