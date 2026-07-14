import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { resetTestDatabase } from "./testing";

const previousTestMarker = process.env.CANCAN_TEST;

afterEach(() => {
  if (previousTestMarker === undefined) {
    delete process.env.CANCAN_TEST;
  } else {
    process.env.CANCAN_TEST = previousTestMarker;
  }
});

describe("resetTestDatabase", () => {
  it("requires the explicit test environment marker", () => {
    delete process.env.CANCAN_TEST;

    expect(() =>
      resetTestDatabase(join(tmpdir(), "cancan-test-guard", "records.sqlite")),
    ).toThrow("CANCAN_TEST=1 is required");
  });

  it("rejects a temporary path that is not visibly test-owned", () => {
    process.env.CANCAN_TEST = "1";

    expect(() => resetTestDatabase(join(tmpdir(), "ordinary-folder", "records.sqlite"))).toThrow(
      "refusing to reset non-test database path",
    );
  });
});
