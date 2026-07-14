import { rmSync } from "node:fs";
import { basename, resolve, sep } from "node:path";

export function resetTestDatabase(databasePath: string): void {
  const resolvedPath = resolve(databasePath);
  const hasTestDirectory = resolvedPath
    .split(sep)
    .some((segment) => segment === ".cancan-test" || segment.startsWith("cancan-test-"));
  const unsafeName = /(?:^|[._-])(vault|backup)(?:[._-]|$)/i.test(basename(resolvedPath));

  if (process.env.CANCAN_TEST !== "1") {
    throw new Error("CANCAN_TEST=1 is required to reset a test database");
  }
  if (!hasTestDirectory || unsafeName) {
    throw new Error(`refusing to reset non-test database path: ${resolvedPath}`);
  }

  console.info(`[cancan test db] reset ${resolvedPath}`);
  rmSync(resolvedPath, { force: true });
}
