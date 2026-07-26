import {
  selectProviderDocumentPackage,
  validateStructuredProposal,
  type ExtractionBundle,
  type ProviderDocumentPackage,
  type StructuredProposalValidation,
} from "@cancan/parsers";
import {
  createSyntheticProviderStatementFixture,
  createSyntheticTransferFixture,
  syntheticBankRecordContract,
} from "@cancan/parsers/testing";

import type { NormalizeDocumentInput } from "./worker-protocol";

const LEGACY_FIXTURE_MARKER = "CANCAN_SYNTHETIC_STATEMENT_V1";
const PROVIDER_FIXTURE_MARKER = "CANCAN_SYNTHETIC_PROVIDER_STATEMENT_V1";
const NORMALIZER_RUNTIME = "single-pass-mock";
const INPUT_STRATEGY = "native-observations-v1";
const MOCK_MODEL_PROVIDER = "cancan-deterministic-mock";
const MOCK_MODEL = "fixture-v1";

export interface NormalizationExtractionEngine {
  engine: string;
  kind: "native_text" | "table_cell";
  version: string;
}

export interface NormalizationOcrEngine {
  engine: string;
  version: string;
}

export interface NormalizationProfile {
  documentType: string;
  extractionEngines: NormalizationExtractionEngine[];
  id: string;
  inputStrategy: typeof INPUT_STRATEGY;
  model: typeof MOCK_MODEL;
  modelProvider: typeof MOCK_MODEL_PROVIDER;
  normalizerRuntime: typeof NORMALIZER_RUNTIME;
  ocrEngines: NormalizationOcrEngine[];
  packageId: string;
  packageVersion: string;
  parserVersion: string;
  promptVersion: string;
  providerKey: string;
  reviewOnly: true;
  schemaVersion: string;
  skillVersion: string;
  toolContractVersion: string;
  validatorVersion: string;
}

export type MockNormalizerResult =
  | {
      status: "classified";
      profile: NormalizationProfile;
      proposal: Extract<StructuredProposalValidation, { status: "valid" }>;
    }
  | { status: "needs_attention"; reason: "unsupported_document" };

function fixtureTokens(input: NormalizeDocumentInput): Set<string> {
  return new Set(
    input.extractionBundle.observations.flatMap(({ text }) => text.split(/\s+/).filter(Boolean)),
  );
}

function markerValue(tokens: Set<string>, prefix: string): string | undefined {
  const values = [...tokens]
    .filter((token) => token.startsWith(prefix))
    .map((token) => token.slice(prefix.length));
  return values.length === 1 && values[0] ? values[0] : undefined;
}

function sorted<T>(values: Iterable<T>, key: (value: T) => string): T[] {
  return [...values].sort((left, right) => {
    const leftKey = key(left);
    const rightKey = key(right);
    return leftKey < rightKey ? -1 : leftKey > rightKey ? 1 : 0;
  });
}

function profileEngines(extractionBundle: ExtractionBundle): {
  extractionEngines: NormalizationExtractionEngine[];
  ocrEngines: NormalizationOcrEngine[];
} {
  const extractionEngines = new Map<string, NormalizationExtractionEngine>();
  const ocrEngines = new Map<string, NormalizationOcrEngine>();
  for (const observation of extractionBundle.observations) {
    if (observation.kind === "ocr_text") {
      const engine = { engine: observation.engine, version: observation.engineVersion };
      ocrEngines.set(`${engine.engine}\u0000${engine.version}`, engine);
      continue;
    }
    const engine = {
      kind: observation.kind,
      engine: observation.engine,
      version: observation.engineVersion,
    };
    extractionEngines.set(`${engine.kind}\u0000${engine.engine}\u0000${engine.version}`, engine);
  }
  return {
    extractionEngines: sorted(extractionEngines.values(), ({ kind, engine, version }) =>
      `${kind}\u0000${engine}\u0000${version}`,
    ),
    ocrEngines: sorted(ocrEngines.values(), ({ engine, version }) => `${engine}\u0000${version}`),
  };
}

function providerProfileId(
  packageId: string,
  extractionEngines: readonly NormalizationExtractionEngine[],
  ocrEngines: readonly NormalizationOcrEngine[],
): string {
  const engines = [
    ...extractionEngines.map(({ kind, engine, version }) => `extract-${kind}-${engine}-${version}`),
    ...ocrEngines.map(({ engine, version }) => `ocr-${engine}-${version}`),
  ].join("+");
  return `mock:${packageId}:${INPUT_STRATEGY}:${engines}`;
}

function providerProfile(
  providerPackage: ProviderDocumentPackage,
  extractionBundle: ExtractionBundle,
): NormalizationProfile {
  const { extractionEngines, ocrEngines } = profileEngines(extractionBundle);
  return {
    id: providerProfileId(providerPackage.packageId, extractionEngines, ocrEngines),
    providerKey: providerPackage.providerKey,
    documentType: providerPackage.documentType,
    packageId: providerPackage.packageId,
    packageVersion: providerPackage.versions.package,
    parserVersion: providerPackage.versions.parser,
    skillVersion: providerPackage.versions.skill,
    promptVersion: providerPackage.versions.prompt,
    schemaVersion: providerPackage.versions.schema,
    validatorVersion: providerPackage.versions.validator,
    normalizerRuntime: NORMALIZER_RUNTIME,
    toolContractVersion: providerPackage.versions.toolContract,
    inputStrategy: INPUT_STRATEGY,
    extractionEngines,
    ocrEngines,
    modelProvider: MOCK_MODEL_PROVIDER,
    model: MOCK_MODEL,
    reviewOnly: true,
  };
}

function legacyProfile(extractionBundle: ExtractionBundle): NormalizationProfile {
  const { extractionEngines, ocrEngines } = profileEngines(extractionBundle);
  return {
    id: "synthetic-bank-transfer-export-v1",
    providerKey: "synthetic-bank",
    documentType: "transfer_export",
    packageId: "synthetic/bank_transfer_export@1",
    packageVersion: "1.0.0",
    parserVersion: "synthetic-bank-v1",
    skillVersion: "synthetic-bank-v1",
    promptVersion: "synthetic-bank-v1",
    schemaVersion: "structured-proposal-v1",
    validatorVersion: "synthetic-bank-v1",
    normalizerRuntime: NORMALIZER_RUNTIME,
    toolContractVersion: "synthetic-bank-v1",
    inputStrategy: INPUT_STRATEGY,
    extractionEngines,
    ocrEngines,
    modelProvider: MOCK_MODEL_PROVIDER,
    model: MOCK_MODEL,
    reviewOnly: true,
  };
}

async function normalizeProviderFixture(
  input: NormalizeDocumentInput,
  tokens: Set<string>,
): Promise<MockNormalizerResult> {
  const providerKey = markerValue(tokens, "provider=");
  const documentType = markerValue(tokens, "document_type=");
  const packageId = markerValue(tokens, "package_id=");
  const statementId = markerValue(tokens, "statement_id=");
  if (!providerKey || !documentType || !packageId || !statementId) {
    return { status: "needs_attention", reason: "unsupported_document" };
  }
  const providerPackage = selectProviderDocumentPackage({
    providerKey,
    documentType,
    mimeType: input.extractionBundle.mimeType,
  });
  if (!providerPackage || providerPackage.packageId !== packageId) {
    return { status: "needs_attention", reason: "unsupported_document" };
  }
  const fixture = createSyntheticProviderStatementFixture(providerPackage);
  if (fixture.proposal.document.statementId !== statementId) {
    return { status: "needs_attention", reason: "unsupported_document" };
  }
  const validation = await providerPackage.validate({
    semanticDocumentKey: fixture.semanticDocumentKey,
    extractionBundle: input.extractionBundle,
    proposal: fixture.proposal,
  });
  if (validation.status !== "valid") {
    return { status: "needs_attention", reason: "unsupported_document" };
  }
  return {
    status: "classified",
    proposal: validation,
    profile: providerProfile(providerPackage, input.extractionBundle),
  };
}

export async function normalizeWithMock(
  input: NormalizeDocumentInput,
): Promise<MockNormalizerResult> {
  const tokens = fixtureTokens(input);
  if (tokens.has(PROVIDER_FIXTURE_MARKER)) {
    return normalizeProviderFixture(input, tokens);
  }
  if (
    !tokens.has(LEGACY_FIXTURE_MARKER) ||
    !tokens.has("provider=synthetic-bank") ||
    !tokens.has("statement_id=transfer-2026-07")
  ) {
    return { status: "needs_attention", reason: "unsupported_document" };
  }

  const fixture = createSyntheticTransferFixture();
  const proposal = {
    ...fixture.proposal,
    document: {
      ...fixture.proposal.document,
      statementId: "transfer-2026-07",
      statementPeriod: { from: "2026-07-01", to: "2026-07-31" },
    },
  };
  const validation = await validateStructuredProposal({
    semanticDocumentKey: "synthetic-bank:transfer-2026-07",
    extractionBundle: input.extractionBundle,
    proposal,
    recordContract: syntheticBankRecordContract,
  });
  if (validation.status !== "valid") {
    return { status: "needs_attention", reason: "unsupported_document" };
  }

  return {
    status: "classified",
    proposal: validation,
    profile: legacyProfile(input.extractionBundle),
  };
}
