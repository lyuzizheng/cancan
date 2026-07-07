# 04. Reconciliation Engine

## Goal

The reconciliation engine links records across different sources so that the user can understand true money movement and avoid double-counting.

It answers:

```text
Is this a duplicate?
Is this an internal transfer?
Is this a card repayment?
Is this a payment app top-up?
Is this a broker deposit?
Is this an FX conversion?
Is this part of a longer money-flow chain?
```

## Important concepts

### Duplicate

Same real-world event imported more than once.

Example:

```text
Wise API transaction and Wise CSV statement row describe the same Wise card purchase.
```

Action:

```text
Keep one canonical event. Mark the other source record as duplicate evidence.
```

### Transfer/link

Two or more real-world events are both valid but represent internal movement.

Example:

```text
DBS -1000 SGD
Wise +1000 SGD
```

Action:

```text
Keep both events. Link them as top-up/transfer so they do not count as spending.
```

### Money-flow chain

Multiple linked movements form a path.

Example:

```text
DBS -1000 SGD
-> Wise +1000 SGD
-> Wise FX SGD to USD
-> Moomoo USD deposit
-> Buy AAPL
```

Action:

```text
Show the chain in Money Flow view.
```

## Match types

```text
duplicate
transfer
bank_transfer
credit_card_payment
payment_app_topup
wise_topup
revolut_topup
broker_deposit
crypto_deposit
crypto_withdrawal
fx_conversion
refund
reimbursement
channel_payment
insurance_premium
fee_allocation
interest_income
```

## Candidate generation

Use deterministic rules first.

### Exact duplicate candidates

```text
same external_transaction_id
same source_document_id + row_hash
same provider + account + posted_at + amount + normalized_description + balance_after
```

### Fuzzy duplicate candidates

```text
same account
same amount and currency
posted date within 0-3 days
similar normalized description
same balance_after if available
```

### Transfer candidates

```text
outflow from account A
inflow to account B
same currency
same amount within tolerance
date distance 0-5 days
self/counterparty/top-up keywords
```

### FX conversion candidates

```text
outflow currency X
inflow currency Y
same source or known conversion path
close dates
implied FX rate within tolerance
provider fee plausible
```

### Credit card payment candidates

```text
bank account outflow
credit card account payment/inflow
same amount
same currency
date distance 0-5 days
card/payment keywords
```

### Payment app top-up candidates

```text
bank/card outflow
Wise/Revolut/payment app inflow
same or FX-compatible amount
top-up/funding/provider keywords
```

### Broker deposit candidates

```text
bank/Wise/Revolut outflow
broker cash inflow
same currency or FX-compatible
date distance 0-7 days
broker/provider keywords
```

### Crypto deposit/withdrawal candidates

```text
bank/card/payment app fiat outflow
crypto exchange fiat inflow
or on-chain withdrawal/deposit pair
amount/currency/token/date compatible
```

## Scoring

Each candidate gets a deterministic score.

Example factors:

```text
amount match:              0.30
currency match:            0.15
date proximity:            0.15
source/provider keywords:  0.15
account relationship:      0.10
historical user behavior:  0.10
balance validation:        0.05
```

Confidence bands:

```text
>= 0.95: auto-link if safe
0.80-0.95: show in review, preselected
0.55-0.80: show as lower-confidence candidate
< 0.55: keep unmatched unless user searches manually
```

## AI role

AI may rerank or explain candidates, but should not directly commit ledger changes.

AI can help with:

```text
normalizing merchant/counterparty names
recognizing descriptions like "FAST TRANSFER", "CARD FUNDING", "REVOLUT*", "WISE*"
explaining why two records may match
ranking ambiguous candidates
suggesting match type
```

AI should not:

```text
commit match edges without validation
ignore records permanently
delete records
read secrets
make payments
place trades
```

## Review item shape

```text
review_items
- type: possible_transfer
- title: DBS outflow may match Wise top-up
- left_event_id
- right_event_id
- confidence
- evidence_json
- explanation
- actions: confirm, reject, edit, link manually
```

## Ledger impact classification

Each event should know whether it affects spending/income/net worth.

Examples:

```text
merchant purchase:       spending yes, net worth down
salary:                  income yes, net worth up
bank transfer:           spending no, net worth unchanged
credit card payment:     spending no, liability/cash move
broker deposit:          spending no, internal asset move
stock trade:             spending no, asset type changes
investment gain:         valuation change, not cash income unless realized
insurance premium:       expense or asset-linked depending user rules
```

## Money Flow view

The reconciliation graph should support chains.

Example graph:

```text
DBS -1000 SGD
  --wise_topup-->
Wise +1000 SGD
  --fx_conversion-->
Wise +740 USD
  --broker_deposit-->
Moomoo +740 USD
  --trade_execution-->
AAPL +1 share
```

This is a graph of events and edges, not just a flat transaction table.
