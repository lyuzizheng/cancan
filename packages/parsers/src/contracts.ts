export type ExactDecimalString = string;

export interface ExactMoneyInput {
  value: ExactDecimalString;
  currency: string;
}

export interface SourceObservation {
  id: string;
  kind: "native_text" | "ocr_text" | "table_cell" | "document_region";
  page?: number;
  row?: number;
  column?: number;
  text: string;
  textSpan?: { start: number; end: number };
  boundingBox?: { x: number; y: number; width: number; height: number };
  engine: string;
  engineVersion: string;
  confidence?: number;
}

export interface ExtractionBundle {
  sourceDocumentId: string;
  fileSha256: string;
  mimeType: string;
  observations: SourceObservation[];
  metadata: Record<string, unknown>;
}

export interface CanonicalExternalRecordInput {
  proposalRecordId: string;
  providerRecordId?: string;
  recordType: "transaction" | "balance" | "position" | "trade" | "valuation" | "fee" | "interest";
  eventType?: string;
  proposalAccountId?: string;
  instrumentSymbol?: string;
  postedOn?: string;
  transactionOn?: string;
  postedAt?: string;
  descriptionRaw?: string;
  descriptionNormalized?: string;
  amount?: ExactMoneyInput;
  quantity?: ExactDecimalString;
  statementEntrySide?: "debit" | "credit";
  accountBalanceDelta?: ExactMoneyInput;
  balanceAfter?: ExactMoneyInput;
  valuation?: ExactMoneyInput;
  raw: Record<string, unknown>;
}

export interface StructuredDocumentIdentity {
  providerKey: string;
  documentType: string;
  statementId?: string;
  statementPeriod?: { from?: string; to?: string };
}

export interface StructuredAccountCandidate {
  proposalAccountId: string;
  accountType:
    | "deposit_account"
    | "credit_card"
    | "currency_balance"
    | "brokerage_account"
    | "cash_balance"
    | "position_group"
    | "insurance_policy"
    | "manual_asset"
    | "manual_liability";
  providerAccountId?: string;
  maskedIdentifier?: string;
  currency?: string;
}

export interface StructuredParseProposal {
  document: StructuredDocumentIdentity;
  accounts: StructuredAccountCandidate[];
  openingSnapshots: CanonicalExternalRecordInput[];
  records: CanonicalExternalRecordInput[];
  closingSnapshots: CanonicalExternalRecordInput[];
}

export interface ProviderRecordInspection {
  groundingValues: string[];
  identityProjection: Record<string, unknown>;
  canonical: Partial<CanonicalExternalRecordInput>;
}

export interface ProviderRecordContract {
  inspect(raw: Record<string, unknown>): ProviderRecordInspection;
}

export type ValidatedExternalRecord = CanonicalExternalRecordInput & {
  stableRecordKey: string;
  validation: {
    schemaValid: true;
    rawGrounded: true;
    deterministicValidationPassed: true;
  };
};

export type StructuredProposalValidation =
  | {
      status: "valid";
      document: StructuredDocumentIdentity;
      accounts: StructuredAccountCandidate[];
      openingSnapshots: ValidatedExternalRecord[];
      records: ValidatedExternalRecord[];
      closingSnapshots: ValidatedExternalRecord[];
    }
  | {
      status: "invalid";
      errors: Array<{
        proposalRecordId?: string;
        code: string;
      }>;
    };
