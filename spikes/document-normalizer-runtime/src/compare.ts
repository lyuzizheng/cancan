import { createRuntimeFixture, type RuntimeResult } from "./harness";
import {
  happyPathTranscript,
  runPiAgentCore,
  runSinglePass,
  runToolLoopAgent,
} from "./runtimes";

interface ComparisonResult extends RuntimeResult {
  elapsedMs: number;
  mockTokens: number;
}

async function measure(run: () => Promise<RuntimeResult>): Promise<ComparisonResult> {
  const started = performance.now();
  const result = await run();
  return {
    ...result,
    elapsedMs: Number((performance.now() - started).toFixed(2)),
    mockTokens: result.modelSteps * 15,
  };
}

export async function collectComparisonEvidence(): Promise<ComparisonResult[]> {
  const fixture = createRuntimeFixture();
  const transcript = happyPathTranscript(fixture.proposal);
  const results = await Promise.all([
    measure(() => runSinglePass({ proposal: fixture.proposal, fixture })),
    measure(() => runToolLoopAgent({ transcript, fixture })),
    measure(() => runPiAgentCore({ transcript, fixture })),
  ]);
  if (
    results.some(({ outcome }) => outcome !== "accepted") ||
    new Set(results.map(({ proposalSha256 }) => proposalSha256)).size !== 1
  ) {
    throw new Error("runtime comparison did not produce one accepted proposal");
  }
  return results;
}
