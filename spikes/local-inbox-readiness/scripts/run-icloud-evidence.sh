#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXPECTED_ICLOUD_ROOT="$HOME/Library/Mobile Documents/com~apple~CloudDocs/Cancan"
BRCTL_TIMEOUT_SECONDS=10
UPLOAD_READY_POLL_ATTEMPTS=15

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
  done <<< "$output"

  [[ -n "$found" ]] || return 1
  printf '%s\n' "$found"
}

require_line() {
  local output="$1"
  local expected="$2"
  if ! /usr/bin/grep -Fqx -- "$expected" <<< "$output"; then
    printf 'Expected runner output %s, got:\n%s\n' "$expected" "$output" >&2
    exit 1
  fi
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

read_download_status() {
  local file="$1"
  local status_output status

  if ! command -v xcrun >/dev/null 2>&1; then
    printf 'download-status-tool=unavailable\n'
    return
  fi

  if status_output="$(xcrun swift scripts/icloud-download-status.swift "$file" 2>&1)"; then
    printf 'download-status-tool=available\n%s\n' "$status_output"
  else
    status=$?
    printf 'download-status-tool=failed\n'
    printf 'download-status-tool-exit=%s\n' "$status"
    printf '%s\n' "$status_output"
  fi
}

wait_for_upload_ready() {
  local file="$1"
  local attempt

  UPLOAD_READY="unproven"
  for ((attempt = 1; attempt <= UPLOAD_READY_POLL_ATTEMPTS; attempt += 1)); do
    UPLOAD_STATUS="$(read_download_status "$file")"
    if /usr/bin/grep -Fqx 'download-status-tool=available' <<< "$UPLOAD_STATUS" \
      && /usr/bin/grep -Fqx 'uploaded=true' <<< "$UPLOAD_STATUS" \
      && /usr/bin/grep -Fqx 'uploading=false' <<< "$UPLOAD_STATUS"; then
      UPLOAD_READY="observed"
      return
    fi
    if ! /usr/bin/grep -Fqx 'download-status-tool=available' <<< "$UPLOAD_STATUS"; then
      return
    fi
    if [[ "$attempt" -lt "$UPLOAD_READY_POLL_ATTEMPTS" ]]; then
      sleep 1
    fi
  done

  UPLOAD_READY="timed-out"
}

CREATE_OUTPUT="$(cargo run --quiet --locked --bin icloud-evidence -- create "$CANCAN_ICLOUD_EVIDENCE_ROOT")"
printf '%s\n' "$CREATE_OUTPUT"
RUN_DIRECTORY="$(extract_value run-directory "$CREATE_OUTPUT")"
RUN_TOKEN="$(extract_value run-token "$CREATE_OUTPUT")"

STABLE_FILE="$RUN_DIRECTORY/stable.pdf"
STABLE_EXPECTED="$RUN_DIRECTORY/stable.expected"
STABLE_STATE="$RUN_DIRECTORY/stable.snapshot"
printf '%%PDF-1.7\nsynthetic stable evidence\n' >"$STABLE_FILE"
printf '%%PDF-1.7\nsynthetic stable evidence\n' >"$STABLE_EXPECTED"

STABLE_FIRST="$(cargo run --quiet --locked --bin icloud-evidence -- snapshot "$STABLE_FILE" "$STABLE_STATE")"
STABLE_CAPTURE="$(cargo run --quiet --locked --bin icloud-evidence -- capture "$STABLE_FILE" "$STABLE_STATE")"
printf '%s\n%s\n' "$STABLE_FIRST" "$STABLE_CAPTURE"
require_line "$STABLE_CAPTURE" "outcome=captured"
require_line "$STABLE_CAPTURE" "source-snapshot-preserved=true"
if [[ "$(extract_value process-id "$STABLE_FIRST")" == "$(extract_value process-id "$STABLE_CAPTURE")" ]]; then
  printf 'Snapshot and capture did not use independent CLI processes.\n' >&2
  exit 1
fi
if ! cmp -s "$STABLE_FILE" "$STABLE_EXPECTED"; then
  printf 'Stable capture changed synthetic source bytes.\n' >&2
  exit 1
fi
printf 'stable-source-bytes-preserved=true\n'
printf 'restart-rescan-process-boundary=true\n'

CHANGING_FILE="$RUN_DIRECTORY/changing.pdf"
CHANGING_STATE="$RUN_DIRECTORY/changing.snapshot"
printf '%%PDF-1.7\nsynthetic first version\n' >"$CHANGING_FILE"
CHANGING_FIRST="$(cargo run --quiet --locked --bin icloud-evidence -- snapshot "$CHANGING_FILE" "$CHANGING_STATE")"
printf '%%PDF-1.7\nsynthetic changed version with more bytes\n' >"$CHANGING_FILE"
CHANGING_CAPTURE="$(cargo run --quiet --locked --bin icloud-evidence -- capture "$CHANGING_FILE" "$CHANGING_STATE")"
printf '%s\n%s\n' "$CHANGING_FIRST" "$CHANGING_CAPTURE"
require_line "$CHANGING_CAPTURE" "outcome=deferred-changed-before-read"
printf 'changing-source-not-captured-early=true\n'

OFFLINE_FILE="$RUN_DIRECTORY/offline.pdf"
OFFLINE_STATE="$RUN_DIRECTORY/offline.snapshot"
printf '%%PDF-1.7\nsynthetic iCloud eviction evidence\n' >"$OFFLINE_FILE"

printf 'brctl-timeout-seconds=%s\n' "$BRCTL_TIMEOUT_SECONDS"
printf 'upload-ready-poll-attempts=%s\n' "$UPLOAD_READY_POLL_ATTEMPTS"
wait_for_upload_ready "$OFFLINE_FILE"
printf 'upload-ready=%s\n%s\n' "$UPLOAD_READY" "$UPLOAD_STATUS"
run_brctl evict "$OFFLINE_FILE"
EVICT_RESULT="$BRCTL_RESULT"
STATUS_AFTER_EVICT="$(read_download_status "$OFFLINE_FILE")"
printf '%s\n' "$STATUS_AFTER_EVICT"

if [[ "$UPLOAD_READY" != "observed" ]]; then
  printf 'placeholder-observation=evict-not-interpretable-upload-not-ready\n'
elif [[ "$EVICT_RESULT" != "accepted" ]]; then
  printf 'placeholder-observation=brctl-evict-%s\n' "$EVICT_RESULT"
elif EVICT_SNAPSHOT="$(cargo run --quiet --locked --bin icloud-evidence -- snapshot "$OFFLINE_FILE" "$OFFLINE_STATE" 2>&1)"; then
  STATUS_BEFORE_EVICT_CAPTURE="$(read_download_status "$OFFLINE_FILE")"
  EVICT_CAPTURE="$(cargo run --quiet --locked --bin icloud-evidence -- capture "$OFFLINE_FILE" "$OFFLINE_STATE")"
  STATUS_AFTER_EVICT_CAPTURE="$(read_download_status "$OFFLINE_FILE")"
  printf '%s\n%s\n%s\n%s\n' \
    "$EVICT_SNAPSHOT" "$STATUS_BEFORE_EVICT_CAPTURE" "$EVICT_CAPTURE" "$STATUS_AFTER_EVICT_CAPTURE"
  if /usr/bin/grep -Fqx 'download-status=downloaded' <<< "$STATUS_BEFORE_EVICT_CAPTURE" \
    || /usr/bin/grep -Fqx 'download-status=current' <<< "$STATUS_BEFORE_EVICT_CAPTURE"; then
    printf 'placeholder-observation=placeholder-not-produced\n'
  elif /usr/bin/grep -Fqx 'download-status=not-downloaded' <<< "$STATUS_BEFORE_EVICT_CAPTURE" \
    && /usr/bin/grep -Fq 'outcome=deferred-' <<< "$EVICT_CAPTURE"; then
    printf 'placeholder-observation=real-defer-observed\n'
    if /usr/bin/grep -Fqx 'download-status=downloaded' <<< "$STATUS_AFTER_EVICT_CAPTURE" \
      || /usr/bin/grep -Fqx 'download-status=current' <<< "$STATUS_AFTER_EVICT_CAPTURE"; then
      printf 'placeholder-conclusion=download-status-transitioned-during-rust-capture-foundation-preflight-required\n'
    fi
  elif /usr/bin/grep -Fqx 'outcome=captured' <<< "$EVICT_CAPTURE"; then
    printf 'placeholder-observation=no-defer-observed-capture-may-have-triggered-download\n'
    if /usr/bin/grep -Fqx 'download-status=not-downloaded' <<< "$STATUS_BEFORE_EVICT_CAPTURE"; then
      printf 'placeholder-conclusion=rust-protocol-does-not-guarantee-placeholder-defer-foundation-preflight-required\n'
    fi
  else
    printf 'placeholder-observation=unproven\n'
  fi
else
  printf '%s\n' "$EVICT_SNAPSHOT"
  printf 'placeholder-observation=not-snapshottable-after-%s\n' "$EVICT_RESULT"
fi

run_brctl download "$OFFLINE_FILE"
STATUS_AFTER_DOWNLOAD="$(read_download_status "$OFFLINE_FILE")"
printf '%s\n' "$STATUS_AFTER_DOWNLOAD"

printf 'live-iCloud-evidence-run=completed\n'
