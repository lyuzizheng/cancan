import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type {
  RelationshipCandidateSummary,
  ReviewItemDetail,
  ReviewItemSummary,
} from "./command-contracts";
import {
  ReviewView,
  batchOutcomeSummary,
  type ReviewDetailState,
  type ReviewViewProps,
} from "./review";

const item: ReviewItemSummary = {
  accountLabel: "DBS Multiplier Account",
  amountValue: "512.34",
  currency: "SGD",
  eventType: "credit_card_repayment",
  postedOn: "2026-07-15",
  reasonCode: "possible_card_repayment",
  recordCommitted: false,
  recordId: "record-1",
  recordVersion: 1,
  reviewItemId: "review-1",
};

const secondItem: ReviewItemSummary = {
  accountLabel: "DBS Visa Card",
  amountValue: "512.34",
  currency: "SGD",
  eventType: "credit_card_repayment",
  postedOn: "2026-07-17",
  reasonCode: "possible_card_repayment",
  recordCommitted: false,
  recordId: "record-2",
  recordVersion: 1,
  reviewItemId: "review-2",
};

const committedItem: ReviewItemSummary = {
  accountLabel: "HSBC Everyday",
  amountValue: "800.00",
  currency: "SGD",
  eventType: "credit_card_repayment",
  postedOn: "2026-06-30",
  reasonCode: "reparse_divergence",
  recordCommitted: true,
  recordId: "record-4",
  recordVersion: 1,
  reviewItemId: "review-4",
};

const detail: ReviewItemDetail = {
  ...item,
  documentLabel: "July statement.pdf",
  sourceLabel: "DBS",
};

const candidate: RelationshipCandidateSummary = {
  accountLabel: "DBS Visa Card",
  amountValue: "512.34",
  currency: "SGD",
  eventType: "credit_card_repayment",
  postedOn: "2026-07-17",
  recordId: "record-2",
  recordVersion: 1,
};

const baseProps: ReviewViewProps = {
  detail: null,
  items: [item, secondItem],
  job: null,
  mutatingItemId: null,
  notice: null,
  onAcceptCandidate: () => undefined,
  onAcknowledge: () => undefined,
  onCancelEdit: () => undefined,
  onCancelRemove: () => undefined,
  onClearSelection: () => undefined,
  onCloseDetail: () => undefined,
  onConfirmRemove: () => undefined,
  onEditChange: () => undefined,
  onEnqueue: () => undefined,
  onLock: () => undefined,
  onOpenDetail: () => undefined,
  onRefresh: () => undefined,
  onRemove: () => undefined,
  onSaveEdit: () => undefined,
  onSelectAll: () => undefined,
  onStartEdit: () => undefined,
  onToggleSelect: () => undefined,
  selectedIds: new Set(),
};

function render(props: Partial<ReviewViewProps> = {}) {
  return renderToStaticMarkup(<ReviewView {...baseProps} {...props} />);
}

function expandedDetail(overrides: Partial<ReviewDetailState> = {}): ReviewDetailState {
  return {
    candidates: [candidate],
    confirmingRemove: false,
    detail,
    editing: null,
    reviewItemId: item.reviewItemId,
    summary: item,
    ...overrides,
  };
}

describe("ReviewView", () => {
  it("renders the queue with personal-finance language and no selection by default", () => {
    const markup = render();

    expect(markup).toContain("Needs your check");
    expect(markup).toContain("SGD 512.34");
    expect(markup).toContain("DBS Multiplier Account");
    expect(markup).toContain("15 Jul 2026");
    expect(markup).toContain("Looks like a card repayment");
    expect(markup).toContain("None selected");
    expect(markup).toContain("Add selected");
    expect(markup).not.toContain("allocation");
    expect(markup).not.toContain("match edge");
    const checked = (markup.match(/checked=""/g) ?? []).length;
    expect(checked).toBe(0);
  });

  it("guides when the queue is empty", () => {
    const markup = render({ items: [] });

    expect(markup).toContain("Nothing needs your check");
    expect(markup).not.toContain("Add selected");
  });

  it("shows a loading state before items arrive", () => {
    expect(render({ items: null })).toContain("Loading review items…");
  });

  it("expands detail with evidence, candidates, and actions", () => {
    const markup = render({ detail: expandedDetail() });

    expect(markup).toContain("July statement.pdf");
    expect(markup).toContain("Looks related");
    expect(markup).toContain("DBS Visa Card");
    expect(markup).toContain("Accept link");
    expect(markup).toContain("Edit record");
    expect(markup).toContain("Remove record");
    expect(markup).toContain("This record");
  });

  it("shows a loading state while detail and candidates load", () => {
    const markup = render({
      detail: expandedDetail({ candidates: null, detail: null }),
    });

    expect(markup).toContain("Loading details…");
    expect(markup).toContain("Looking for related records…");
  });

  it("explains when no related records exist", () => {
    const markup = render({ detail: expandedDetail({ candidates: [] }) });

    expect(markup).toContain("No related records found");
  });

  it("renders the edit form with prefilled amount and date", () => {
    const markup = render({
      detail: expandedDetail({
        editing: {
          accountBalanceDelta: "",
          amountValue: "512.34",
          error: null,
          postedOn: "2026-07-15",
          saving: false,
        },
      }),
    });

    expect(markup).toContain("Save edit");
    expect(markup).toContain('value="512.34"');
    expect(markup).toContain('value="2026-07-15"');
    expect(markup).toContain("Balance change");
    expect(markup).toContain("Leave it blank to keep the current value");
  });

  it("offers only acknowledge on a record that is already added", () => {
    const markup = render({
      detail: {
        candidates: [],
        confirmingRemove: false,
        detail: {
          ...committedItem,
          documentLabel: "June statement.pdf",
          sourceLabel: "HSBC",
        },
        editing: null,
        reviewItemId: committedItem.reviewItemId,
        summary: committedItem,
      },
      items: [committedItem],
    });

    expect(markup).toContain("Acknowledge");
    expect(markup).toContain("This record is already added");
    expect(markup).toContain("Re-parsed values differ");
    expect(markup).not.toContain("Edit record");
    expect(markup).not.toContain("Remove record");
    expect(markup).not.toContain("Accept link");
    expect(markup).not.toContain("Looks related");
    expect(markup).not.toContain("Add selected");
    expect(markup).not.toContain("Select SGD 800.00");
  });

  it("keeps already-added records out of a mixed selection", () => {
    const markup = render({ items: [committedItem, item] });

    expect(markup).toContain("Add selected");
    expect(markup).toContain("Select SGD 512.34 for DBS Multiplier Account");
    expect(markup).not.toContain("Select SGD 800.00 for HSBC Everyday");
  });

  it("asks for confirmation before removing a record", () => {
    const markup = render({ detail: expandedDetail({ confirmingRemove: true }) });
    expect(markup).toContain("Remove this record from Review?");
    expect(markup).toContain("Confirm remove");
    expect(markup).toContain("Keep");
  });

  it("reflects selection and the running commit job", () => {
    const markup = render({
      job: { jobId: "job-1", outcomes: [], status: "running" },
      selectedIds: new Set([item.reviewItemId, secondItem.reviewItemId]),
    });

    expect(markup).toContain("2 selected");
    expect(markup).toContain("Adding your records…");
    expect(markup).toContain("Adding…");
  });

  it("summarizes finished job outcomes with follow-up reasons", () => {
    const markup = render({
      job: {
        jobId: "job-1",
        outcomes: [
          { reason: null, recordIds: ["record-1", "record-2"], status: "committed" },
          { reason: "relationship_not_confirmed", recordIds: ["record-3"], status: "still_needs_review" },
        ],
        status: "done",
      },
      items: [item],
    });

    expect(markup).toContain("Added 2 records; 1 still needs your check.");
    expect(markup).toContain("its link isn’t confirmed yet");
  });

  it("reports a failed job safely", () => {
    const markup = render({
      job: { jobId: "job-1", outcomes: [], status: "failed" },
    });

    expect(markup).toContain("CanCan couldn’t finish adding records");
  });
});

describe("batchOutcomeSummary", () => {
  it("builds a plain-language summary per outcome kind", () => {
    expect(
      batchOutcomeSummary([
        { reason: null, recordIds: ["a", "b"], status: "committed" },
        { reason: null, recordIds: ["c"], status: "already_committed" },
        { reason: "stale_record", recordIds: [], status: "stale" },
        { reason: "core_preflight_failed", recordIds: ["d"], status: "still_needs_review" },
      ]),
    ).toBe("Added 2 records; 1 was already added; 1 still needs your check; 1 changed while adding.");
    expect(batchOutcomeSummary([])).toBe("Nothing was added.");
  });
});
