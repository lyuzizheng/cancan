# CanCan Docs

This folder is the product, architecture, and AI-agent operating manual for **CanCan**.

CanCan is a local-first desktop finance vault that collects financial evidence, parses it into canonical records, reconciles money movements across sources, and stores evidence plus records in an encrypted local vault.

## Canonical documentation model

CanCan docs are intentionally split into a small number of layers.

```text
docs/
  README.md                  entry point
  STRUCTURE.md               documentation rules and source-of-truth model
  specs/                     canonical product + technical specs
  adr/                       status-bearing architecture decision records
  agent/                     AI coding agent operating memory and workflow
  alignment-temp/            temporary grill/alignment workspace, deleted when done
```

## Source of truth

`docs/specs/` is the canonical implementation source of truth.

Older numbered product docs were removed to avoid duplicate and conflicting guidance. If an agent needs product/architecture detail, it should read the relevant spec rather than looking for `00-product-vision.md` style files.

The concern-based source contract and conflict protocol live only in [`STRUCTURE.md`](./STRUCTURE.md). Summary, progress, alignment, and agent files must not override canonical product behavior.

## Product behavior

The root [`README.md`](../README.md) provides the short product identity. Intended behavior and implementation contracts live only in the canonical spec index below; this navigation page is not an implementation contract.

## Canonical specs

See [`specs/README.md`](./specs/README.md).

## AI agent entry point

Run `.agents/scripts/agent-preflight.sh`, then follow [`agent/reading-order.md`](./agent/reading-order.md). The read order is not duplicated here so it has one maintained home.
