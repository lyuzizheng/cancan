const EXACT_DECIMAL = /^(-?)(0|[1-9]\d*)(?:\.(\d+))?$/;
const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

const LEDGER_MONTHS = [
  "Jan", "Feb", "Mar", "Apr", "May", "Jun",
  "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
] as const;

/**
 * Groups the integer part of a host-supplied exact decimal string without
 * float conversion, preserving the source decimal scale.
 */
export function formatNativeAmount(value: string): string {
  const match = EXACT_DECIMAL.exec(value);
  if (!match) {
    return value;
  }
  const sign = match[1] ?? "";
  const grouped = (match[2] ?? "").replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  const fraction = match[3];
  return `${sign}${grouped}${fraction ? `.${fraction}` : ""}`;
}

export function formatCurrencyAmount(
  currency: string | null,
  value: string | null,
): string {
  if (value === null) {
    return "Amount missing";
  }
  const amount = formatNativeAmount(value);
  return currency ? `${currency} ${amount}` : amount;
}

/** Renders a host ISO calendar date such as 2026-07-19 without timezone shifts. */
export function formatLedgerDate(iso: string): string {
  if (!ISO_DATE.test(iso)) {
    return iso;
  }
  const year = Number(iso.slice(0, 4));
  const month = Number(iso.slice(5, 7));
  const day = Number(iso.slice(8, 10));
  if (month < 1 || month > 12 || day < 1 || day > 31) {
    return iso;
  }
  return `${day} ${LEDGER_MONTHS[month - 1]} ${year}`;
}

const EVENT_TYPE_LABELS: Record<string, string> = {
  balance_observation: "Balance update",
  credit_card_purchase: "Card purchase",
  credit_card_repayment: "Card repayment",
  credit_card_repayment_reversal: "Repayment undone",
  purchase: "Purchase",
  same_currency_transfer: "Transfer",
  same_currency_transfer_reversal: "Transfer undone",
};

export function eventTypeLabel(eventType: string | null): string {
  if (eventType === null) {
    return "Record";
  }
  const known = EVENT_TYPE_LABELS[eventType];
  if (known) {
    return known;
  }
  const words = eventType.replaceAll("_", " ").trim();
  return words.length === 0 ? "Record" : `${words[0]!.toUpperCase()}${words.slice(1)}`;
}

const REVIEW_REASON_LABELS: Record<string, string> = {
  auto_commit_disabled: "Automatic add is off",
  classification_conflict: "Source needs checking",
  possible_card_repayment: "Looks like a card repayment",
  possible_transfer: "Looks like a transfer",
  record_edited: "Edited record",
  source_file_deleted: "Source file deleted",
};

export function reviewReasonLabel(reasonCode: string): string {
  return REVIEW_REASON_LABELS[reasonCode] ?? "Needs your check";
}

const REVIEW_CONFLICT_MESSAGES: Record<string, string> = {
  invalid_relationship_request: "CanCan couldn’t confirm that link.",
  invalid_review_edit: "That edit isn’t valid. Check the amount and date.",
  invalid_review_request: "CanCan couldn’t apply that change.",
  relationship_changed: "That link changed while you worked. CanCan reloaded the latest version.",
  relationship_needs_review: "CanCan can’t confirm that link yet.",
  stale_record: "This item changed while you worked. CanCan reloaded the latest version.",
  stale_relationship: "That link changed while you worked. CanCan reloaded the latest version.",
  stale_relationship_candidate: "That linked record changed while you worked. CanCan reloaded the latest version.",
  stale_review_item: "This item changed while you worked. CanCan reloaded the latest version.",
};

export function reviewConflictMessage(reason: string | null): string {
  if (reason === null) {
    return "CanCan couldn’t apply that change.";
  }
  return REVIEW_CONFLICT_MESSAGES[reason] ?? "CanCan couldn’t apply that change.";
}

const BATCH_GROUP_STATUS_LABELS: Record<string, string> = {
  already_committed: "Already added",
  committed: "Added",
  stale: "Changed while adding",
  still_needs_review: "Still needs your check",
};

export function batchGroupStatusLabel(status: string): string {
  return BATCH_GROUP_STATUS_LABELS[status] ?? "Needs your check";
}

const BATCH_GROUP_REASON_LABELS: Record<string, string> = {
  ambiguous_relationship: "it has more than one possible link",
  core_preflight_failed: "its details aren’t complete",
  job_lease_changed: "it changed while adding",
  relationship_changed: "its link changed while adding",
  relationship_not_confirmed: "its link isn’t confirmed yet",
  relationship_not_selected: "select both linked records before adding",
  stale_record: "it changed while adding",
  stale_relationship: "its link changed while adding",
  stale_review_item: "it changed while adding",
};

export function batchGroupReasonLabel(reason: string | null): string {
  if (reason === null) {
    return "check its details";
  }
  return BATCH_GROUP_REASON_LABELS[reason] ?? "check its details";
}
