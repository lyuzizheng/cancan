import type {
  SourceDocumentImportOutcome,
} from "./command-contracts";
import type { Notice } from "./feedback";

/**
 * What `importNotice` can describe: every status on the wire plus the picker
 * being dismissed. Keeping the union derived from the generated wire type makes
 * the notice table total — a new Rust status fails this `Record` until it is
 * given copy instead of silently falling back.
 */
export type ImportNoticeStatus =
  | SourceDocumentImportOutcome["status"]
  | "cancelled";

export function importNotice(status: ImportNoticeStatus): Notice {
  const notices: Record<ImportNoticeStatus, Notice> = {
    imported: { tone: "success", title: "Added to your Vault", body: "Your file is safely stored. Check its routing when you’re ready." },
    already_present: { tone: "success", title: "Already in CanCan", body: "This exact file is already safely stored in your Vault." },
    restored: { tone: "success", title: "Evidence restored", body: "Your saved evidence is available in the Vault again." },
    restore_confirmation_required: { tone: "attention", title: "Restore this file?", body: "This exact file was removed from CanCan earlier, so nothing was imported. Restore it to bring that evidence back." },
    cancelled: { tone: "attention", title: "No file was imported", body: "You can add a PDF, CSV, PNG, or JPEG whenever you’re ready." },
  };
  return notices[status];
}
