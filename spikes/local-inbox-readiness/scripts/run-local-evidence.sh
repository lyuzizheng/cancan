#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
cd "$ROOT"

RUN_DIRECTORY="$(mktemp -d "${TMPDIR:-/tmp}/cancan-local-inbox-readiness.XXXXXX")"

cleanup() {
  local status=$?
  rm -rf "$RUN_DIRECTORY"
  exit "$status"
}
trap cleanup EXIT

extract_value() {
  local wanted_key="$1"
  local output="$2"
  local key value found=""

  while IFS='=' read -r key value; do
    if [[ "$key" == "$wanted_key" ]]; then
      if [[ -n "$found" ]]; then
        return 1
      fi
      found="$value"
    fi
  done <<<"$output"

  [[ -n "$found" ]] || return 1
  printf '%s\n' "$found"
}

require_line() {
  local output="$1"
  local expected="$2"
  if ! /usr/bin/grep -Fqx -- "$expected" <<<"$output"; then
    printf 'Expected runner output %s, got:\n%s\n' "$expected" "$output" >&2
    exit 1
  fi
}

require_settle_interval() {
  local output="$1"
  local milliseconds

  milliseconds="$(extract_value settle-wait-milliseconds "$output")"
  if (( milliseconds < 2000 )); then
    printf 'Capture waited only %sms; expected at least 2000ms.\n' "$milliseconds" >&2
    exit 1
  fi
}

LOCAL_FILE="$RUN_DIRECTORY/local.pdf"
LOCAL_EXPECTED="$RUN_DIRECTORY/local.expected"
printf '%%PDF-1.7\nsynthetic local evidence\n' >"$LOCAL_FILE"
cp "$LOCAL_FILE" "$LOCAL_EXPECTED"

LOCAL_PREFLIGHT="$(xcrun swift scripts/icloud-download-status.swift "$LOCAL_FILE")"
printf '%s\n' "$LOCAL_PREFLIGHT"
require_line "$LOCAL_PREFLIGHT" 'capture-decision=ready'
require_line "$LOCAL_PREFLIGHT" 'capture-reason=non-ubiquitous-regular'

OBSERVE_OUTPUT="$(cargo run --quiet --locked --bin icloud-evidence -- observe "$LOCAL_FILE")"
printf '%s\n' "$OBSERVE_OUTPUT"
require_line "$OBSERVE_OUTPUT" 'phase=observe-only'
require_line "$OBSERVE_OUTPUT" 'observation-snapshot-persisted=false'
if find "$RUN_DIRECTORY" -maxdepth 1 -name '*.snapshot' -print -quit | /usr/bin/grep -q .; then
  printf 'Observe process persisted scan state.\n' >&2
  exit 1
fi

RESTART_CAPTURE_OUTPUT="$(cargo run --quiet --locked --bin icloud-evidence -- restart-capture "$LOCAL_FILE")"
printf '%s\n' "$RESTART_CAPTURE_OUTPUT"
require_line "$RESTART_CAPTURE_OUTPUT" 'phase=fresh-first-scan-after-restart'
require_line "$RESTART_CAPTURE_OUTPUT" 'restart-snapshot-imported=false'
require_line "$RESTART_CAPTURE_OUTPUT" 'settle-seconds=2'
require_line "$RESTART_CAPTURE_OUTPUT" 'outcome=captured'
require_line "$RESTART_CAPTURE_OUTPUT" 'source-snapshot-preserved=true'
require_settle_interval "$RESTART_CAPTURE_OUTPUT"
if [[ "$(extract_value process-id "$OBSERVE_OUTPUT")" == "$(extract_value process-id "$RESTART_CAPTURE_OUTPUT")" ]]; then
  printf 'Observe and restart capture did not use independent processes.\n' >&2
  exit 1
fi
if ! cmp -s "$LOCAL_FILE" "$LOCAL_EXPECTED"; then
  printf 'Local capture changed source bytes.\n' >&2
  exit 1
fi
printf 'restart-rescan-fresh-process=true\n'
printf 'restart-snapshot-state-transferred=false\n'

WRITING_FILE="$RUN_DIRECTORY/continuous.pdf"
printf '%%PDF-1.7\nwriter initial version\n' >"$WRITING_FILE"
(
  for attempt in {1..16}; do
    printf '%%PDF-1.7\nwriter version %02d\n' "$attempt" >"$WRITING_FILE"
    sleep 0.25
  done
) &
WRITER_PID=$!

CHANGING_CAPTURE_OUTPUT="$(cargo run --quiet --locked --bin icloud-evidence -- restart-capture "$WRITING_FILE")"
printf '%s\n' "$CHANGING_CAPTURE_OUTPUT"
require_line "$CHANGING_CAPTURE_OUTPUT" 'outcome=deferred-changed-before-read'
require_settle_interval "$CHANGING_CAPTURE_OUTPUT"
if ! kill -0 "$WRITER_PID" 2>/dev/null; then
  printf 'Continuous writer ended before the two-second capture attempt finished.\n' >&2
  exit 1
fi
wait "$WRITER_PID"

STABLE_CAPTURE_OUTPUT="$(cargo run --quiet --locked --bin icloud-evidence -- restart-capture "$WRITING_FILE")"
printf '%s\n' "$STABLE_CAPTURE_OUTPUT"
require_line "$STABLE_CAPTURE_OUTPUT" 'outcome=captured'
require_line "$STABLE_CAPTURE_OUTPUT" 'source-snapshot-preserved=true'
require_settle_interval "$STABLE_CAPTURE_OUTPUT"
printf 'continuous-writer-not-captured-before-final-two-second-settle=true\n'
printf 'continuous-writer-captured-after-final-two-second-settle=true\n'
