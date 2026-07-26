import type {
  MoneySourceSummary,
  SourceDocumentImportOutcome,
  SourceDocumentRoutingOutcome,
} from "./command-contracts";
import type { Notice } from "./feedback";

export function importNotice(status: SourceDocumentImportOutcome["status"] | "cancelled"): Notice {
  const notices: Record<SourceDocumentImportOutcome["status"] | "cancelled", Notice> = {
    imported: { tone: "success", title: "Added to your Vault", body: "Your file is safely stored. Check its routing when you’re ready." },
    already_present: { tone: "success", title: "Already in CanCan", body: "This exact file is already safely stored in your Vault." },
    restored: { tone: "success", title: "Evidence restored", body: "Your saved evidence is available in the Vault again." },
    cancelled: { tone: "attention", title: "No file was imported", body: "You can add a PDF, CSV, PNG, or JPEG whenever you’re ready." },
  };
  return notices[status];
}

export function routingNotice(
  outcome: SourceDocumentRoutingOutcome,
  source?: MoneySourceSummary,
): Notice {
  return outcome.status === "routed"
    ? {
        tone: "success",
        title: "Evidence routed",
        body: source
          ? `CanCan matched this evidence to ${source.displayName}.`
          : "CanCan matched this evidence to one configured source and account.",
      }
    : { tone: "attention", title: "Needs attention", body: "CanCan could not match this evidence uniquely, so it was not assigned." };
}
