/**
 * Display-format amount normalization for statement rows.
 *
 * DBS/HSBC PDFs render amounts with thousands separators (`1,234.56`) while
 * the canonical ledger form is plain exact-decimal (`1234.56`). The raw row
 * keeps the display string so evidence grounding matches the source text;
 * the proposal's canonical money fields carry the normalized form so schema
 * and arithmetic validators see exact-decimal only.
 *
 * Every rule here is deterministic and auditable: {@link normalizeDisplayAmount}
 * either returns the exact normalized string or rejects, never a rounded or
 * approximated value.
 */

/** Strict display-decimal: optional `-`, thousands-grouped or plain digits, optional fraction. */
const DISPLAY_AMOUNT_PATTERN = /^(-?)(0|[1-9]\d{0,2}(?:,\d{3})+|[1-9]\d*)(?:\.(\d+))?$/;

/** Canonical exact-decimal: no separators, no exponent, no leading zeros. */
const CANONICAL_AMOUNT_PATTERN = /^-?(0|[1-9]\d*)(?:\.\d+)?$/;

/**
 * Normalize one display amount to canonical exact-decimal.
 *
 * Accepts plain (`1234.56`) and thousands-grouped (`1,234.56`) forms with an
 * optional leading `-`. Returns the canonical string (commas stripped) or
 * `undefined` when the input is not a well-formed display amount — wrong
 * group sizes, leading zeros, whitespace, `+` sign, or any non-numeric text.
 */
export function normalizeDisplayAmount(value: string): string | undefined {
  const match = DISPLAY_AMOUNT_PATTERN.exec(value);
  if (match === null) {
    return undefined;
  }
  const sign = match[1] ?? "";
  const whole = match[2];
  if (whole === undefined) {
    return undefined;
  }
  const fraction = match[3];
  const canonical = `${sign === "-" ? "-" : ""}${whole.replace(/,/g, "")}${fraction === undefined ? "" : `.${fraction}`}`;
  return CANONICAL_AMOUNT_PATTERN.test(canonical) ? canonical : undefined;
}

/**
 * Parse a canonical two-decimal SGD value into minor units. Accepts display
 * grouping as well, so `twoDecimalMinorUnits("1,234.56") === 123456n`.
 */
export function twoDecimalMinorUnits(value: string): bigint | undefined {
  const canonical = normalizeDisplayAmount(value);
  if (canonical === undefined) {
    return undefined;
  }
  const match = /^(-?)(0|[1-9]\d*)\.(\d{2})$/.exec(canonical);
  if (match === null) {
    return undefined;
  }
  const sign = match[1];
  const whole = match[2] ?? "";
  const fraction = match[3] ?? "";
  const minorUnits = BigInt(`${whole}${fraction}`);
  return sign === "-" ? -minorUnits : minorUnits;
}
