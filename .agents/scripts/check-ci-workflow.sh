#!/usr/bin/env bash
set -euo pipefail

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT"

if ! command -v ruby >/dev/null 2>&1; then
  echo "Missing required harness dependency: ruby"
  exit 1
fi

ruby - .github/workflows/docs-harness.yml <<'RUBY'
require "yaml"

path = ARGV.fetch(0)
document = YAML.load_file(path)
abort "Workflow must be a mapping: #{path}" unless document.is_a?(Hash)

events = document["on"] || document[true]
abort "Workflow must define pull_request and push events" unless events.is_a?(Hash)

required_paths = ["AGENTS.md", ".agents/**", "docs/**", ".github/workflows/docs-harness.yml"]
%w[pull_request push].each do |event|
  config = events[event]
  abort "Workflow is missing #{event}" unless config.is_a?(Hash)
  paths = Array(config["paths"])
  missing = required_paths - paths
  abort "#{event} is missing paths: #{missing.join(', ')}" unless missing.empty?
end

push_branches = Array(events.fetch("push")["branches"])
abort "push must include main branch" unless push_branches.include?("main")

permissions = document["permissions"]
abort "Workflow must use read-only contents permission" unless permissions.is_a?(Hash) && permissions["contents"] == "read"

jobs = document["jobs"]
job = jobs.is_a?(Hash) ? jobs["deterministic-docs-gate"] : nil
abort "Workflow is missing deterministic-docs-gate job" unless job.is_a?(Hash)

steps = Array(job["steps"])
runs = steps.map { |step| step.is_a?(Hash) ? step["run"] : nil }.compact
required_runs = [
  ".agents/scripts/agent-preflight.sh",
  ".agents/scripts/harness-self-test.sh"
]
missing_runs = required_runs.reject { |required| runs.include?(required) }
abort "Workflow is missing required run commands: #{missing_runs.join(', ')}" unless missing_runs.empty?
RUBY

echo "Docs harness workflow check passed."
