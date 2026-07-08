#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

fail=0

for skill in .agents/skills/*/SKILL.md; do
  name="$(sed -n '2p' "$skill")"
  description="$(sed -n '3p' "$skill")"
  close="$(sed -n '4p' "$skill")"

  if [ "$(sed -n '1p' "$skill")" != "---" ] || [ "$close" != "---" ]; then
    echo "Invalid frontmatter fence: $skill"
    fail=1
  fi

  if ! printf '%s\n' "$name" | grep -Eq '^name: [a-z0-9-]+$'; then
    echo "Invalid skill name line: $skill"
    fail=1
  fi

  if ! printf '%s\n' "$description" | grep -Eq '^description: .+Use when .+'; then
    echo "Description must include trigger phrase 'Use when': $skill"
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "Agent skill check failed."
  exit 1
fi

echo "Agent skill check passed."
