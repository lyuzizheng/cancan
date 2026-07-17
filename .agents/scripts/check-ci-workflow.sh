#!/usr/bin/env bash
set -euo pipefail

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT"

if ! command -v ruby >/dev/null 2>&1; then
  echo "Missing required harness dependency: ruby"
  exit 1
fi

ruby - .github/workflows/docs-harness.yml <<'RUBY'
require "json"
require "yaml"

path = ARGV.fetch(0)
document = YAML.load_file(path)
abort "Workflow must be a mapping: #{path}" unless document.is_a?(Hash)

events = document["on"] || document[true]
abort "Workflow must define pull_request and push events" unless events.is_a?(Hash)

required_paths = [
  "AGENTS.md",
  "README.md",
  ".agents/**",
  ".codex/**",
  "scripts/**",
  ".node-version",
  "rust-toolchain.toml",
  "spikes/desktop-feasibility/package.json",
  "docs/**",
  ".github/workflows/docs-harness.yml"
]
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
docs_uses = steps.map { |step| step.is_a?(Hash) ? step["uses"] : nil }.compact
abort "Docs workflow is missing actions/checkout@v7" unless docs_uses.include?("actions/checkout@v7")

runs = steps.map { |step| step.is_a?(Hash) ? step["run"] : nil }.compact
required_runs = [
  ".agents/scripts/agent-preflight.sh",
  ".agents/scripts/harness-self-test.sh"
]
missing_runs = required_runs.reject { |required| runs.include?(required) }
abort "Workflow is missing required run commands: #{missing_runs.join(', ')}" unless missing_runs.empty?

application_path = ".github/workflows/application.yml"
application = YAML.load_file(application_path)
abort "Workflow must be a mapping: #{application_path}" unless application.is_a?(Hash)

application_events = application["on"] || application[true]
abort "Application workflow must define pull_request and push events" unless application_events.is_a?(Hash)

application_paths = [
  "apps/**",
  "packages/**",
  "package.json",
  "pnpm-lock.yaml",
  "pnpm-workspace.yaml",
  "tsconfig.base.json",
  ".node-version",
  "rust-toolchain.toml",
  "scripts/dev-toolchain.env",
  application_path
]
%w[pull_request push].each do |event|
  config = application_events[event]
  abort "Application workflow is missing #{event}" unless config.is_a?(Hash)
  missing = application_paths - Array(config["paths"])
  abort "Application #{event} is missing paths: #{missing.join(', ')}" unless missing.empty?
end

application_push_branches = Array(application_events.fetch("push")["branches"])
abort "Application push must include main branch" unless application_push_branches.include?("main")

application_permissions = application["permissions"]
unless application_permissions.is_a?(Hash) && application_permissions["contents"] == "read"
  abort "Application workflow must use read-only contents permission"
end

application_jobs = application["jobs"]
application_job = application_jobs.is_a?(Hash) ? application_jobs["application-gate"] : nil
abort "Application workflow is missing application-gate job" unless application_job.is_a?(Hash)
abort "Application workflow must run on macos-14" unless application_job["runs-on"] == "macos-14"

application_steps = Array(application_job["steps"])
application_uses = application_steps.map { |step| step.is_a?(Hash) ? step["uses"] : nil }.compact
%w[actions/checkout@v7 actions/setup-node@v6].each do |required|
  abort "Application workflow is missing #{required}" unless application_uses.include?(required)
end

setup_node = application_steps.find { |step| step.is_a?(Hash) && step["uses"] == "actions/setup-node@v6" }
unless setup_node.is_a?(Hash) && setup_node.fetch("with", {})["node-version-file"] == ".node-version"
  abort "Application workflow must source Node from .node-version"
end

application_runs = application_steps.map { |step| step.is_a?(Hash) ? step["run"] : nil }.compact
required_application_runs = [
  "pnpm install --frozen-lockfile",
  ".agents/scripts/agent-preflight.sh",
  "pnpm test:rust",
  "pnpm verify"
]
missing_application_runs = required_application_runs - application_runs
unless missing_application_runs.empty?
  abort "Application workflow is missing required run commands: #{missing_application_runs.join(', ')}"
end

combined_runs = application_runs.join("\n")
[
  "source scripts/dev-toolchain.env",
  "corepack@$COREPACK_VERSION",
  "pnpm@$PNPM_VERSION",
  "rustup show active-toolchain",
  "cargo clippy --version"
].each do |required|
  abort "Application workflow is missing toolchain step: #{required}" unless combined_runs.include?(required)
end

runtime_path = ".github/workflows/document-normalizer-runtime.yml"
runtime = YAML.load_file(runtime_path)
abort "Workflow must be a mapping: #{runtime_path}" unless runtime.is_a?(Hash)

runtime_events = runtime["on"] || runtime[true]
abort "Runtime workflow must define pull_request and push events" unless runtime_events.is_a?(Hash)
allowed_runtime_events = %w[pull_request push]
unless runtime_events.keys.map(&:to_s).sort == allowed_runtime_events.sort
  abort "Runtime workflow must define only pull_request and push events"
end

runtime_paths = [
  "spikes/document-normalizer-runtime/**",
  "packages/parsers/**",
  "tsconfig.base.json",
  ".node-version",
  "rust-toolchain.toml",
  "scripts/dev-toolchain.env",
  runtime_path
]
%w[pull_request push].each do |event|
  config = runtime_events[event]
  abort "Runtime workflow is missing #{event}" unless config.is_a?(Hash)
  paths = Array(config["paths"])
  unless paths.sort == runtime_paths.sort
    abort "Runtime #{event} paths must exactly match the allowed paths"
  end
end

runtime_push_branches = Array(runtime_events.fetch("push")["branches"])
abort "Runtime push must include main branch" unless runtime_push_branches.include?("main")

runtime_permissions = runtime["permissions"]
unless runtime_permissions.is_a?(Hash) && runtime_permissions["contents"] == "read"
  abort "Runtime workflow must use read-only contents permission"
end

runtime_jobs = runtime["jobs"]
runtime_job = runtime_jobs.is_a?(Hash) ? runtime_jobs["document-normalizer-runtime-gate"] : nil
abort "Runtime workflow is missing document-normalizer-runtime-gate job" unless runtime_job.is_a?(Hash)
abort "Runtime workflow must run on macos-14" unless runtime_job["runs-on"] == "macos-14"

runtime_steps = Array(runtime_job["steps"])
runtime_uses = runtime_steps.map { |step| step.is_a?(Hash) ? step["uses"] : nil }.compact
%w[actions/checkout@v7 actions/setup-node@v6].each do |required|
  abort "Runtime workflow is missing #{required}" unless runtime_uses.include?(required)
end

runtime_setup_node = runtime_steps.find { |step| step.is_a?(Hash) && step["uses"] == "actions/setup-node@v6" }
unless runtime_setup_node.is_a?(Hash) && runtime_setup_node.fetch("with", {})["node-version-file"] == ".node-version"
  abort "Runtime workflow must source Node from .node-version"
end

runtime_runs = runtime_steps.map { |step| step.is_a?(Hash) ? step["run"] : nil }.compact
unless runtime_runs.include?("bash spikes/document-normalizer-runtime/scripts/verify.sh")
  abort "Runtime workflow is missing the document normalizer verification command"
end

runtime_combined_runs = runtime_runs.join("\n")
[
  "source scripts/dev-toolchain.env",
  "corepack@$COREPACK_VERSION",
  "pnpm@$PNPM_VERSION",
  "rustup show active-toolchain",
  "cargo clippy --version"
].each do |required|
  abort "Runtime workflow is missing toolchain step: #{required}" unless runtime_combined_runs.include?(required)
end

vault_path = ".github/workflows/vault-security-validation.yml"
vault = YAML.load_file(vault_path)
abort "Workflow must be a mapping: #{vault_path}" unless vault.is_a?(Hash)

vault_events = vault["on"] || vault[true]
abort "Vault workflow must define pull_request and push events" unless vault_events.is_a?(Hash)
allowed_vault_events = %w[pull_request push]
unless vault_events.keys.map(&:to_s).sort == allowed_vault_events.sort
  abort "Vault workflow must define only pull_request and push events"
end

vault_paths = [
  "spikes/vault-security-validation/**",
  "rust-toolchain.toml",
  vault_path
]
%w[pull_request push].each do |event|
  config = vault_events[event]
  abort "Vault workflow is missing #{event}" unless config.is_a?(Hash)
  paths = Array(config["paths"])
  unless paths.sort == vault_paths.sort
    abort "Vault #{event} paths must exactly match the allowed paths"
  end
end

vault_push_branches = Array(vault_events.fetch("push")["branches"])
abort "Vault push must include main branch" unless vault_push_branches.include?("main")

vault_permissions = vault["permissions"]
unless vault_permissions.is_a?(Hash) && vault_permissions["contents"] == "read"
  abort "Vault workflow must use read-only contents permission"
end

vault_jobs = vault["jobs"]
vault_job = vault_jobs.is_a?(Hash) ? vault_jobs["vault-security-validation-gate"] : nil
abort "Vault workflow is missing vault-security-validation-gate job" unless vault_job.is_a?(Hash)
abort "Vault workflow must run on macos-14" unless vault_job["runs-on"] == "macos-14"

vault_steps = Array(vault_job["steps"])
vault_uses = vault_steps.map { |step| step.is_a?(Hash) ? step["uses"] : nil }.compact
abort "Vault workflow is missing actions/checkout@v7" unless vault_uses.include?("actions/checkout@v7")

vault_runs = vault_steps.map { |step| step.is_a?(Hash) ? step["run"] : nil }.compact
unless vault_runs.include?("bash spikes/vault-security-validation/scripts/verify.sh")
  abort "Vault workflow is missing the security validation command"
end

vault_combined_runs = vault_runs.join("\n")
%w[rustup\ show\ active-toolchain cargo\ clippy\ --version].each do |required|
  abort "Vault workflow is missing toolchain step: #{required}" unless vault_combined_runs.include?(required)
end

package = JSON.parse(File.read("package.json"))
scripts = package.fetch("scripts", {})
required_scripts = %w[typecheck test:unit test:rust check:rust build:web build:desktop verify]
missing_scripts = required_scripts.reject { |name| scripts[name].is_a?(String) && !scripts[name].empty? }
abort "Root package is missing scripts: #{missing_scripts.join(', ')}" unless missing_scripts.empty?

unless scripts.fetch("test:rust").include?("pnpm --filter @cancan/desktop test:rust")
  abort "Root test:rust must delegate to the desktop Rust test suite"
end

%w[typecheck test:unit check:rust build:desktop].each do |name|
  abort "Root verify does not run #{name}" unless scripts.fetch("verify").include?("pnpm #{name}")
end

desktop_package = JSON.parse(File.read("apps/desktop/package.json"))
desktop_scripts = desktop_package.fetch("scripts", {})
%w[check:rust test:rust build:desktop].each do |name|
  command = desktop_scripts[name]
  abort "Desktop #{name} must use Cargo.lock" unless command.is_a?(String) && command.include?("--locked")
end

unless desktop_scripts.fetch("test:rust").include?("cargo test")
  abort "Desktop test:rust must execute Rust tests"
end
RUBY

echo "CI workflow check passed."
