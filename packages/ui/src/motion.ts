/**
 * Motion tokens — spec: docs/specs/0011-visual-design-tokens.md (motion tokens).
 * Durations pair with Tailwind `duration-<n>` utilities and the `--ease-mech`
 * curve; this module serves JS-driven animation and the reduced-motion gate.
 * Exit transitions use roughly 75% of the entrance duration.
 */
export const motion = {
  /** Direct feedback (press, hover). */
  feedback: 120,
  /** State change (selection, disclosure). */
  state: 180,
  /** Page/structural transition. */
  transition: 240,
  /** Onboarding or Vault creation/unlock explanation only. */
  slow: 320,
} as const;

export const easeMech = "cubic-bezier(0.19, 1, 0.22, 1)";

/** CSS transition shorthand for the rare JS-driven transition. */
export function transitionFor(
  properties: string[],
  duration: (typeof motion)[keyof typeof motion] = motion.state,
): string {
  return properties
    .map((property) => `${property} ${duration}ms ${easeMech}`)
    .join(", ");
}

/**
 * True when the user asked for reduced motion. Movement must degrade to an
 * instant state change or a short crossfade. SSR/test environments report
 * false (no preference).
 */
export function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}
