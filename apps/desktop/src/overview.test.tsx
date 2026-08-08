import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type {
  MoneyOverview,
  RecentActivitySummary,
  Tasks,
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

const tasks: Tasks = {
  needsActionCount: 1,
  rows: [
    {
      consequence: "password_needed",
      destination: { documentId: "document-1", kind: "password", moneySourceId: "source-1" },
      group: "needs_action",
      rowKey: "task:password:document-1",
      timestamp: "2026-07-19 09:00",
      title: "June statement.pdf",
    },
  ],
};

const baseProps: OverviewViewProps = {
  loading: false,
  moneyOverview: overview,
  notice: null,
  onLock: () => undefined,
  onOpenSources: () => undefined,
  onOpenTask: () => undefined,
  onRefresh: () => undefined,
  onUndo: () => undefined,
  onViewAllTasks: () => undefined,
  recentActivity: activity,
  tasks,
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

  it("renders the unified Tasks section with a View all action", () => {
    const markup = render();

    expect(markup).toContain("Tasks");
    expect(markup).toContain("View all");
    expect(markup).toContain("Password needed");
    expect(markup).toContain("June statement.pdf");
  });

  it("guides to Sources when no balances exist yet", () => {
    const markup = render({
      moneyOverview: { assets: [], liabilities: [] },
      recentActivity: [],
      tasks: { needsActionCount: 0, rows: [] },
    });

    expect(markup).toContain("Balances appear here after your first records are added");
    expect(markup).toContain("Add a statement from Sources");
    expect(markup).toContain("Nothing added yet");
    expect(markup).toContain("all caught up");
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
      tasks: null,
    });

    expect(markup).toContain("Loading your balances…");
    expect(markup).toContain("Loading recent activity…");
    expect(markup).toContain("Loading tasks…");
  });
});
