#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXPECTED_ICLOUD_ROOT="$HOME/Library/Mobile Documents/com~apple~CloudDocs/Cancan"
BRCTL_TIMEOUT_SECONDS=10
STATUS_POLL_ATTEMPTS=15

: "${CANCAN_ICLOUD_EVIDENCE_ROOT:?Set this to the exact iCloud Drive CanCan root to opt in.}"
if [[ "$CANCAN_ICLOUD_EVIDENCE_ROOT" != "$EXPECTED_ICLOUD_ROOT" ]]; then
  printf 'Refusing iCloud evidence outside the exact root: %s\n' "$EXPECTED_ICLOUD_ROOT" >&2
  exit 2
fi

cd "$ROOT"

RUN_DIRECTORY=""
RUN_TOKEN=""

cleanup() {
  local original_status=$?
  trap - EXIT
  if [[ -n "$RUN_DIRECTORY" && -n "$RUN_TOKEN" ]]; then
    if ! cargo run --quiet --locked --bin icloud-evidence -- cleanup "$RUN_DIRECTORY" "$RUN_TOKEN"; then
      printf 'Synthetic iCloud evidence directory was not removed: %s\n' "$RUN_DIRECTORY" >&2
      exit 1
    fi
  fi
  exit "$original_status"
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

read_preflight() {
  local file="$1"

  if ! command -v xcrun >/dev/null 2>&1; then
    printf 'xcrun is unavailable; Foundation preflight evidence cannot run.\n' >&2
    return 1
  fi
  xcrun swift scripts/icloud-download-status.swift "$file"
}

wait_for_preflight() {
  local file="$1"
  local expected_status="$2"
  local expected_decision="$3"
  local attempt

  for ((attempt = 1; attempt <= STATUS_POLL_ATTEMPTS; attempt += 1)); do
    PREFLIGHT_OUTPUT="$(read_preflight "$file")"
    if /usr/bin/grep -Fqx "download-status=$expected_status" <<<"$PREFLIGHT_OUTPUT" \
      && /usr/bin/grep -Fqx "capture-decision=$expected_decision" <<<"$PREFLIGHT_OUTPUT"; then
      return 0
    fi
    if [[ "$attempt" -lt "$STATUS_POLL_ATTEMPTS" ]]; then
      sleep 1
    fi
  done

  return 1
}

wait_for_upload_ready() {
  local file="$1"
  local attempt

  for ((attempt = 1; attempt <= STATUS_POLL_ATTEMPTS; attempt += 1)); do
    PREFLIGHT_OUTPUT="$(read_preflight "$file")"
    if /usr/bin/grep -Fqx 'download-status=current' <<<"$PREFLIGHT_OUTPUT" \
      && /usr/bin/grep -Fqx 'capture-decision=ready' <<<"$PREFLIGHT_OUTPUT" \
      && /usr/bin/grep -Fqx 'uploaded=true' <<<"$PREFLIGHT_OUTPUT" \
      && /usr/bin/grep -Fqx 'uploading=false' <<<"$PREFLIGHT_OUTPUT"; then
      return 0
    fi
    if [[ "$attempt" -lt "$STATUS_POLL_ATTEMPTS" ]]; then
      sleep 1
    fi
  done

  return 1
}

run_brctl() {
  local action="$1"
  local file="$2"
  local log_file="$RUN_DIRECTORY/brctl-${action}.log"
  local command_pid elapsed status

  if [[ ! -x /usr/bin/brctl ]]; then
    BRCTL_RESULT="unavailable"
    printf 'brctl-%s-result=%s\n' "$action" "$BRCTL_RESULT"
    return
  fi

  /usr/bin/brctl "$action" "$file" >"$log_file" 2>&1 &
  command_pid=$!
  elapsed=0
  while kill -0 "$command_pid" 2>/dev/null && [[ "$elapsed" -lt "$BRCTL_TIMEOUT_SECONDS" ]]; do
    sleep 1
    elapsed=$((elapsed + 1))
  done

  if kill -0 "$command_pid" 2>/dev/null; then
    kill -TERM "$command_pid" 2>/dev/null || true
    sleep 1
    if kill -0 "$command_pid" 2>/dev/null; then
      kill -KILL "$command_pid" 2>/dev/null || true
    fi
    wait "$command_pid" 2>/dev/null || true
    status=124
    BRCTL_RESULT="timeout"
  elif wait "$command_pid"; then
    status=0
    BRCTL_RESULT="accepted"
  else
    status=$?
    if /usr/bin/grep -Eiq 'unrecognized command|unknown command|not supported' "$log_file"; then
      BRCTL_RESULT="unsupported"
    else
      BRCTL_RESULT="failed"
    fi
  fi

  printf 'brctl-%s-result=%s\n' "$action" "$BRCTL_RESULT"
  printf 'brctl-%s-exit=%s\n' "$action" "$status"
  if [[ -s "$log_file" ]]; then
    printf 'brctl-%s-output:\n' "$action"
    /usr/bin/sed -n '1,20p' "$log_file"
  fi
}

CREATE_OUTPUT="$(cargo run --quiet --locked --bin icloud-evidence -- create "$CANCAN_ICLOUD_EVIDENCE_ROOT")"
printf '%s\n' "$CREATE_OUTPUT"
RUN_DIRECTORY="$(extract_value run-directory "$CREATE_OUTPUT")"
RUN_TOKEN="$(extract_value run-token "$CREATE_OUTPUT")"

OFFLINE_FILE="$RUN_DIRECTORY/offline.pdf"
OFFLINE_EXPECTED="$RUN_DIRECTORY/offline.expected"
printf '%%PDF-1.7\nsynthetic iCloud eviction evidence\n' >"$OFFLINE_FILE"
cp "$OFFLINE_FILE" "$OFFLINE_EXPECTED"

printf 'brctl-timeout-seconds=%s\n' "$BRCTL_TIMEOUT_SECONDS"
printf 'status-poll-attempts=%s\n' "$STATUS_POLL_ATTEMPTS"
if ! wait_for_upload_ready "$OFFLINE_FILE"; then
  printf 'iCloud synthetic file did not reach uploaded/current readiness:\n%s\n' "$PREFLIGHT_OUTPUT" >&2
  exit 1
fi
printf 'upload-ready=observed\n%s\n' "$PREFLIGHT_OUTPUT"

run_brctl evict "$OFFLINE_FILE"
if [[ "$BRCTL_RESULT" != "accepted" ]]; then
  printf 'Could not request synthetic-file eviction: %s\n' "$BRCTL_RESULT" >&2
  exit 1
fi
if ! wait_for_preflight "$OFFLINE_FILE" not-downloaded defer; then
  printf 'Eviction did not produce an observable not-downloaded placeholder:\n%s\n' "$PREFLIGHT_OUTPUT" >&2
  exit 1
fi
PREFLIGHT_BEFORE_SECOND_CHECK="$PREFLIGHT_OUTPUT"
printf '%s\n' "$PREFLIGHT_BEFORE_SECOND_CHECK"
require_line "$PREFLIGHT_BEFORE_SECOND_CHECK" 'capture-reason=not-downloaded'

PREFLIGHT_AFTER_SECOND_CHECK="$(read_preflight "$OFFLINE_FILE")"
printf '%s\n' "$PREFLIGHT_AFTER_SECOND_CHECK"
require_line "$PREFLIGHT_AFTER_SECOND_CHECK" 'download-status=not-downloaded'
require_line "$PREFLIGHT_AFTER_SECOND_CHECK" 'capture-decision=defer'
require_line "$PREFLIGHT_AFTER_SECOND_CHECK" 'capture-reason=not-downloaded'
printf 'placeholder-preflight-defer=true\n'
printf 'placeholder-preflight-did-not-hydrate=true\n'

run_brctl download "$OFFLINE_FILE"
if [[ "$BRCTL_RESULT" != "accepted" ]]; then
  printf 'Could not request synthetic-file download: %s\n' "$BRCTL_RESULT" >&2
  exit 1
fi
if ! wait_for_preflight "$OFFLINE_FILE" current ready; then
  printf 'Explicit download did not reach current readiness:\n%s\n' "$PREFLIGHT_OUTPUT" >&2
  exit 1
fi
printf 'explicit-download-current=true\n%s\n' "$PREFLIGHT_OUTPUT"

OBSERVE_OUTPUT="$(cargo run --quiet --locked --bin icloud-evidence -- observe "$OFFLINE_FILE")"
printf '%s\n' "$OBSERVE_OUTPUT"
require_line "$OBSERVE_OUTPUT" 'phase=observe-only'
require_line "$OBSERVE_OUTPUT" 'observation-snapshot-persisted=false'
if find "$RUN_DIRECTORY" -maxdepth 1 -name '*.snapshot' -print -quit | /usr/bin/grep -q .; then
  printf 'Observe process persisted scan state.\n' >&2
  exit 1
fi

CAPTURE_OUTPUT="$(cargo run --quiet --locked --bin icloud-evidence -- restart-capture "$OFFLINE_FILE")"
printf '%s\n' "$CAPTURE_OUTPUT"
require_line "$CAPTURE_OUTPUT" 'phase=fresh-first-scan-after-restart'
require_line "$CAPTURE_OUTPUT" 'restart-snapshot-imported=false'
require_line "$CAPTURE_OUTPUT" 'settle-seconds=2'
require_line "$CAPTURE_OUTPUT" 'outcome=captured'
require_line "$CAPTURE_OUTPUT" 'source-snapshot-preserved=true'
require_settle_interval "$CAPTURE_OUTPUT"
if [[ "$(extract_value process-id "$OBSERVE_OUTPUT")" == "$(extract_value process-id "$CAPTURE_OUTPUT")" ]]; then
  printf 'Observe and restart capture did not use independent processes.\n' >&2
  exit 1
fi
if ! cmp -s "$OFFLINE_FILE" "$OFFLINE_EXPECTED"; then
  printf 'Explicit-download capture changed source bytes.\n' >&2
  exit 1
fi
printf 'restart-rescan-fresh-process=true\n'
printf 'restart-snapshot-state-transferred=false\n'
printf 'iCloud-source-bytes-identity-size-mtime-preserved=true\n'
printf 'live-iCloud-evidence-run=passed\n'
