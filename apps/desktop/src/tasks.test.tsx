import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { TaskConsequence, TaskRow, Tasks } from "./command-contracts";
import {
  taskConsequenceLabel,
  taskGroupLabel,
  TaskRowList,
  TasksSection,
  type TasksSectionProps,
} from "./tasks";
import { TasksView, type TasksViewProps } from "./tasks-view";

function taskRow(overrides: Partial<TaskRow> = {}): TaskRow {
  return {
    consequence: "password_needed",
    destination: { documentId: "document-1", kind: "password", moneySourceId: "source-1" },
    group: "needs_action",
    rowKey: "task:password:document-1",
    timestamp: "2026-07-19 09:00",
    title: "June statement.pdf",
    ...overrides,
  };
}

const mixedRows: TaskRow[] = [
  taskRow(),
  taskRow({
    consequence: "new_source_detected",
    destination: { kind: "source_confirmation", moneySourceCandidateId: "candidate-dbs" },
    rowKey: "task:new_source:candidate-dbs",
    timestamp: "2026-07-18 10:00",
    title: "New source detected: DBS",
  }),
  taskRow({
    consequence: "processing",
    destination: { documentId: "document-2", kind: "document" },
    group: "in_progress",
    rowKey: "task:processing:document-2",
    timestamp: "2026-07-18 08:00",
    title: "May statement.pdf",
  }),
  taskRow({
    consequence: "ready",
    destination: { intakeItemId: "intake-1", kind: "receipt" },
    group: "recently_completed",
    rowKey: "task:ready:document-3",
    timestamp: "2026-07-17 08:00",
    title: "April statement.pdf",
  }),
  taskRow({
    consequence: "password_parked",
    destination: { documentId: "document-4", kind: "document" },
    group: "parked",
    rowKey: "task:parked:document-4",
    timestamp: "2026-07-10 08:00",
    title: "Old export.csv",
  }),
];

const tasks: Tasks = { needsActionCount: 2, rows: mixedRows };

const baseSectionProps: TasksSectionProps = {
  onOpenTask: () => undefined,
  onViewAll: () => undefined,
  tasks,
};

describe("TasksSection", () => {
  it("renders rows with group captions, consequence labels, and a needs-action count", () => {
    const html = renderToStaticMarkup(<TasksSection {...baseSectionProps} />);

    expect(html).toContain("Tasks");
    expect(html).toContain("View all");
    expect(html).toContain("Needs action");
    expect(html).toContain("In progress");
    expect(html).toContain("Recently completed");
    expect(html).toContain("Parked");
    expect(html).toContain("Password needed");
    expect(html).toContain("New source detected");
    expect(html).toContain("June statement.pdf");
    expect(html).toContain(">2</span>");
  });

  it("shows the caught-up empty state when the host returns no rows", () => {
    const html = renderToStaticMarkup(
      <TasksSection {...baseSectionProps} tasks={{ needsActionCount: 0, rows: [] }} />,
    );

    expect(html).toContain("all caught up");
    expect(html).not.toContain(">0</span>");
  });

  it("announces loading before the first payload arrives", () => {
    const html = renderToStaticMarkup(
      <TasksSection {...baseSectionProps} tasks={null} />,
    );

    expect(html).toContain("Loading tasks…");
  });
});

describe("TaskRowList", () => {
  it("omits group captions when a single-group surface renders it", () => {
    const html = renderToStaticMarkup(
      <TaskRowList
        captions={false}
        onOpenTask={() => undefined}
        rows={[mixedRows[0]!, mixedRows[1]!]}
      />,
    );

    expect(html).not.toContain("Needs action");
    expect(html).toContain("Password needed");
  });
});

const baseViewProps: TasksViewProps = {
  filter: "needs_action",
  onFilterChange: () => undefined,
  onLock: () => undefined,
  onOpenTask: () => undefined,
  onRefresh: () => undefined,
  tasks,
};

describe("TasksView", () => {
  it("renders the four group filters with per-group counts", () => {
    const html = renderToStaticMarkup(<TasksView {...baseViewProps} />);

    expect(html).toContain('aria-pressed="true"');
    expect(html).toContain("Needs action");
    expect(html).toContain("In progress");
    expect(html).toContain("Recently completed");
    expect(html).toContain("Parked");
    expect(html).toContain("June statement.pdf");
    expect(html).toContain("New source detected: DBS");
    expect(html).not.toContain("May statement.pdf");
  });

  it("shows the active filter's empty state copy", () => {
    const html = renderToStaticMarkup(
      <TasksView {...baseViewProps} filter="in_progress" tasks={{ needsActionCount: 0, rows: [] }} />,
    );

    expect(html).toContain("No work in progress.");
  });
});

describe("task labels", () => {
  it("labels every task group", () => {
    expect(taskGroupLabel("needs_action")).toBe("Needs action");
    expect(taskGroupLabel("in_progress")).toBe("In progress");
    expect(taskGroupLabel("recently_completed")).toBe("Recently completed");
    expect(taskGroupLabel("parked")).toBe("Parked");
  });

  it("labels every host consequence without falling through", () => {
    const consequences: TaskConsequence[] = [
      "already_in_cancan",
      "file_not_added",
      "import_interrupted",
      "inbox_file_could_not_be_added",
      "inbox_file_parked",
      "needs_attention",
      "needs_review",
      "new_source_detected",
      "password_needed",
      "password_parked",
      "processing",
      "ready",
      "restore_source_file",
      "save_recovery_file",
      "setup_reminder_postponed",
      "source_file_left_deleted",
      "source_file_restored",
      "source_unassigned",
    ];

    for (const consequence of consequences) {
      expect(taskConsequenceLabel(consequence).length).toBeGreaterThan(0);
    }
  });
});
