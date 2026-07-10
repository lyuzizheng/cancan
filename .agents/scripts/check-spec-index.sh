#!/usr/bin/env bash
set -euo pipefail

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT"

index="docs/specs/README.md"
fail=0

specs="$(find docs/specs -maxdepth 1 -type f -name '[0-9][0-9][0-9][0-9]-*.md' | sort)"
numbers="$(printf '%s\n' "$specs" | sed -E 's#docs/specs/([0-9]{4})-.*#\1#')"
duplicates="$(printf '%s\n' "$numbers" | sort | uniq -d)"

if [ -n "$duplicates" ]; then
  echo "Duplicate spec numbers:"
  printf '  %s\n' $duplicates
  fail=1
fi

while IFS= read -r spec; do
  [ -n "$spec" ] || continue
  base="$(basename "$spec")"
  number="${base%%-*}"
  first_line="$(sed -n '1p' "$spec")"
  count="$( (rg -F -o "\`$base\`" "$index" || true) | wc -l | tr -d ' ')"

  case "$first_line" in
    "# $number."*) ;;
    *)
      echo "Spec title must start with '# $number.': $spec"
      fail=1
      ;;
  esac

  if ! rg -q '^## Goal$' "$spec"; then
    echo "Spec is missing Goal heading: $spec"
    fail=1
  fi

  if ! rg -q '^## (Tests / )?[Aa]cceptance criteria$' "$spec"; then
    echo "Spec is missing acceptance criteria: $spec"
    fail=1
  fi

  if [ "$count" -ne 1 ]; then
    echo "Spec index must reference $base exactly once; found $count"
    fail=1
  fi
done <<< "$specs"

while IFS= read -r indexed; do
  [ -n "$indexed" ] || continue
  if [ ! -f "docs/specs/$indexed" ]; then
    echo "Spec index references missing file: $indexed"
    fail=1
  fi
done < <((rg -o '`[0-9]{4}-[^`]+[.]md`' "$index" || true) | tr -d '`')

if [ "$fail" -ne 0 ]; then
  echo "Spec index check failed."
  exit 1
fi

echo "Spec index check passed."
