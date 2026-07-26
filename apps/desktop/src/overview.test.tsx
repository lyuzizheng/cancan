import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type {
  MoneyOverview,
  RecentActivitySummary,
} from "./command-contracts";
import { OverviewView, type OverviewViewProps } from "./overview";

const overview: MoneyOverview = {
  assets: [
    {
      accountId: "account-dbs",
      accountLabel: "DBS Multiplier Account",
      asOf: "2026-07-19",
      currency: "SGD",
      value: "12456.78",
    },
    {
      accountId: "account-wise",
      accountLabel: "Wise USD Balance",
      asOf: "2026-07-18",
      currency: "USD",
      value: "980.5",
    },
  ],
  liabilities: [
    {
      accountId: "account-card",
      accountLabel: "DBS Visa Card",
      asOf: "2026-07-19",
      currency: "SGD",
      value: "1234.56",
    },
  ],
};

const activity: RecentActivitySummary[] = [
  {
    canUndo: true,
    eventDate: "2026-07-18",
    eventId: "event-1",
    eventType: "credit_card_repayment",
    sourceLabels: ["DBS", "DBS Card"],
    spending: false,
  },
  {
    canUndo: false,
    eventDate: "2026-07-17",
    eventId: "event-2",
    eventType: "purchase",
    sourceLabels: ["DBS Card"],
    spending: true,
  },
];

const baseProps: OverviewViewProps = {
  loading: false,
  moneyOverview: overview,
  notice: null,
  onLock: () => undefined,
  onOpenReview: () => undefined,
  onOpenSources: () => undefined,
  onRefresh: () => undefined,
  onUndo: () => undefined,
  recentActivity: activity,
  reviewCount: 0,
  undoingEventId: null,
};

function render(props: Partial<OverviewViewProps> = {}) {
  return renderToStaticMarkup(<OverviewView {...baseProps} {...props} />);
}

describe("OverviewView", () => {
  it("renders native-bucket balances without a net-worth total", () => {
    const markup = render();

    expect(markup).toContain("Money Overview");
    expect(markup).toContain("Assets");
    expect(markup).toContain("Liabilities");
    expect(markup).toContain("DBS Multiplier Account");
    expect(markup).toContain("SGD 12,456.78");
    expect(markup).toContain("USD 980.5");
    expect(markup).toContain("As of 19 Jul 2026");
    expect(markup).not.toContain("Net Worth");
  });

  it("guides to Sources when no balances exist yet", () => {
    const markup = render({
      moneyOverview: { assets: [], liabilities: [] },
      recentActivity: [],
    });

    expect(markup).toContain("Balances appear here after your first records are added");
    expect(markup).toContain("Add a statement from Sources");
    expect(markup).toContain("Nothing added yet");
    expect(markup).toContain("Nothing needs your check");
  });

  it("summarizes the review queue and links to Review", () => {
    const markup = render({ reviewCount: 3 });

    expect(markup).toContain("3 records need your check");
    expect(markup).toContain("Open Review");
  });

  it("renders recent activity with one-click Undo only when allowed", () => {
    const markup = render();

    expect(markup).toContain("Recent activity");
    expect(markup).toContain("Card repayment");
    expect(markup).toContain("DBS · DBS Card");
    expect(markup).toContain("Purchase");
    expect(markup).toContain("Spending");
    const undoCount = (markup.match(/>Undo</g) ?? []).length;
    expect(undoCount).toBe(1);
  });

  it("shows loading status before data arrives", () => {
    const markup = render({
      loading: true,
      moneyOverview: null,
      recentActivity: null,
      reviewCount: null,
    });

    expect(markup).toContain("Loading your balances…");
    expect(markup).toContain("Loading recent activity…");
  });
});
