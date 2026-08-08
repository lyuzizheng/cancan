/** Joins conditional class fragments. Variants stay exhaustive in each
 * primitive, so no tailwind-merge-style conflict resolution is needed. */
export function cx(...parts: Array<string | false | null | undefined>): string {
  return parts.filter(Boolean).join(" ");
}
