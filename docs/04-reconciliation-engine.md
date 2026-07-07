# 04. Reconciliation Engine

## Goal

The reconciliation engine links records across different sources so the user can understand true money movement and avoid double-counting.

It answers:

```text
Is this a duplicate?
Is this an internal transfer?
Is this a credit card repayment?
Is this a payment app or Wise top-up?
Is this a broker deposit?
Is this an FX conversion?
Is this a stock/crypto trade?
Is this part of a longer money-flow chain?
```

## Duplicate vs transfer

Duplicate:

```text
Same real-world event imported more than once.
Example: Wise API transaction and Wise CSV row describe the same purchase.
Action: keep one canonical event, retain duplicate source as evidence.
```

Transfer/link:

```text
Two or more records are both true but represent internal movement.
Example: UOB -1000 SGD and DBS credit card payment +1000 SGD.
Action: keep both, link them, and do not count the repayment as spending.
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
trade_execution
valuation_snapshot_link
```

## Candidate generation

Use deterministic rules first, then AI for fuzzy understanding, reranking, and explanation.

Exact duplicate candidates:

```text
same external_transaction_id
same source_document_id + row_hash
same provider + account + posted_at + amount + normalized_description + balance_after
```

Transfer candidates:

```text
outflow from account A
inflow to account B
same currency
same amount within tolerance
date distance 0-5 days
self/counterparty/top-up keywords
known source relationship
```

Credit card payment candidates:

```text
bank account outflow
credit card liability reduction/payment row
same amount
same currency
date distance 0-5 days
card/payment keywords
```

FX conversion candidates:

```text
outflow currency X
inflow currency Y
same source or known conversion path
close dates
implied FX rate within tolerance
provider fee plausible
```

Broker/trade candidates:

```text
cash outflow or broker cash reduction
position quantity increase/decrease
trade date/post date compatible
instrument recognized
fees/commission plausible
```

## Scoring

Example deterministic factors:

```text
amount match:              0.30
currency/instrument match: 0.15
date proximity:            0.15
source/provider keywords:  0.15
account relationship:      0.10
historical user behavior:  0.10
balance validation:        0.05
```

Confidence bands:

```text
>= 0.98: auto-link only for explicitly safe policies
0.90-0.98: review, preselected
0.70-0.90: review, needs explanation
< 0.70: keep unmatched unless user searches manually
```

## AI role

AI may:

```text
normalize merchant/counterparty names
recognize provider-specific descriptions
suggest match type
rerank candidates
explain why records may match
flag suspicious or ambiguous cases
```

AI must not:

```text
commit match edges without deterministic validation or auto-policy
ignore records permanently
delete records
read secrets
make payments
place trades
```

## Ledger impact classification

Each committed event should know whether it affects spending, income, and net worth.

Examples:

```text
merchant purchase:       spending yes, net worth down
salary:                  income yes, net worth up
bank transfer:           spending no, net worth unchanged
credit card payment:     spending no, cash down and liability down
broker deposit:          spending no, internal asset move
stock trade:             spending no, asset type changes
unrealized gain:         valuation change, not cash income
insurance premium:       expense or asset-linked depending policy rules
balance snapshot:        proof/validation, not a transaction
```

## Money Flow graph

The reconciliation graph should support chains:

```text
UOB -1000 SGD
  --credit_card_payment-->
DBS Visa liability -1000 SGD

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
