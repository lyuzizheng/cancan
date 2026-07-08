#!/usr/bin/env bash
set -euo pipefail

prompt="${*:-}"
if [ -z "$prompt" ]; then
  echo "Usage: .agents/scripts/role-for-prompt.sh \"user prompt\""
  exit 2
fi

lower="$(printf '%s' "$prompt" | tr '[:upper:]' '[:lower:]')"

case "$lower" in
  *grill*|*stress-test*|*align*|*clarify*|*question*)
    echo "role=design-griller workflow=design-grill"
    ;;
  *review*|*audit*|*risk*|*pr\ feedback*)
    echo "role=code-reviewer workflow=review-code"
    ;;
  *test*|*simulate*|*simulation*|*qa*|*fixture*)
    echo "role=qa-simulator workflow=simulated-testing"
    ;;
  *ui*|*visual*|*polish*|*design\ system*|*layout*)
    echo "role=ui-polisher workflow=refine-ui"
    ;;
  *architecture*|*refactor*|*structure*|*boundary*|*package*)
    echo "role=architect workflow=refine-architecture"
    ;;
  *plugin*|*connector*|*gmail*|*source*)
    echo "role=implementer workflow=plugin-work"
    ;;
  *implement*|*build*|*add*|*fix*|*change*)
    echo "role=implementer workflow=implement-feature"
    ;;
  *)
    echo "role=implementer workflow=development-cycle"
    ;;
esac
