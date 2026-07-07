# 11. Open Questions

## Product questions

1. What is the base currency for net worth display?
2. Should insurance premiums count as expense, asset transfer, or configurable by policy type?
3. Should credit card statement transactions be shown by statement date, transaction date, or posting date by default?
4. Should user manually create accounts first, or should accounts be auto-created from parsed statements?
5. Should cash/physical wallet be excluded entirely or just hidden by default?
6. How much budgeting/category functionality is required in v1?

## Data model questions

1. Should `ledger_events` model balance snapshots and transactions together, or should snapshots be separate?
2. Should investment trades use generic ledger legs or a specialized trades table?
3. How should realized/unrealized gains be represented?
4. Should `match_edges` link external records, ledger events, or both?
5. How should one-to-many matches be represented, such as one bank payment paying multiple card transactions?
6. How should partial matches be represented?

## Parser questions

1. Which provider should be the first full parser fixture?
2. How should password-protected PDFs be handled?
3. Should OCR be local-only by default?
4. What is the minimum acceptable parse confidence for auto-staging?
5. How should parser version upgrades compare old and new outputs?
6. Should we store full OCR bounding boxes or only text?

## Reconciliation questions

1. What confidence threshold is safe for auto-linking transfers?
2. Should top-ups auto-link if amount/date/provider match strongly?
3. How should payment app channel purchases be reported to avoid double-counting?
4. How should FX fees be detected and allocated?
5. How far apart can bank outflows and broker deposits be before they require manual review?
6. Should known self-transfer relationships be learned from user confirmations?

## Security questions

1. Which secret storage backend should be used in Tauri first?
2. Should SQLCipher be mandatory from v1 or optional initially?
3. Should file vault attachments be individually encrypted in addition to encrypted SQLite?
4. Should AI providers be disabled by default until explicit consent?
5. Should raw document upload to cloud AI require per-source or per-document consent?
6. Should backups include secrets or force re-authentication after restore?

## Platform questions

1. MVP target: macOS only, or macOS + Windows from day one?
2. Is iCloud backup enough initially, or should generic folder backup be the first implementation?
3. Should mobile app v1 be React Native, Flutter, or Tauri mobile?
4. Should mobile v1 be read-only snapshot viewer?
5. Should the app support import/export to plain CSV/JSON for portability?
