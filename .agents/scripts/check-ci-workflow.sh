#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
require_tools ruby

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

docs_concurrency = document["concurrency"]
unless docs_concurrency.is_a?(Hash) &&
       docs_concurrency["cancel-in-progress"] == true &&
       docs_concurrency["group"].to_s.include?("github.workflow") &&
       docs_concurrency["group"].to_s.include?("github.event.pull_request.number") &&
       docs_concurrency["group"].to_s.include?("github.ref")
  abort "Docs harness workflow must cancel superseded runs per PR or ref"
end

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
unless application_events.keys.map(&:to_s).sort == %w[pull_request push]
  abort "Fast application workflow must define only pull_request and push events"
end

application_paths = [
  "apps/desktop/**",
  "packages/**",
  "package.json",
  "pnpm-lock.yaml",
  "pnpm-workspace.yaml",
  "tsconfig.base.json",
  ".node-version",
  "rust-toolchain.toml",
  "scripts/dev-toolchain.env",
  application_path,
  ".github/workflows/application-native.yml"
]
%w[pull_request push].each do |event|
  config = application_events[event]
  abort "Application workflow is missing #{event}" unless config.is_a?(Hash)
  paths = Array(config["paths"])
  missing = application_paths - paths
  abort "Application #{event} is missing paths: #{missing.join(', ')}" unless missing.empty?
  forbidden = paths & ["apps/**", "apps/website/**"]
  unless forbidden.empty?
    abort "Fast application #{event} must not re-trigger on the isolated website; the Website workflow owns apps/website/**"
  end
end

application_push_branches = Array(application_events.fetch("push")["branches"])
abort "Application push must include main branch" unless application_push_branches.include?("main")

application_permissions = application["permissions"]
unless application_permissions.is_a?(Hash) && application_permissions["contents"] == "read"
  abort "Fast application workflow must use read-only contents permission"
end

application_jobs = application["jobs"]
application_job = application_jobs.is_a?(Hash) ? application_jobs["fast-application-gate"] : nil
abort "Fast application workflow is missing fast-application-gate job" unless application_job.is_a?(Hash)
abort "Fast application workflow must run on ubuntu-24.04" unless application_job["runs-on"] == "ubuntu-24.04"

application_concurrency = application["concurrency"]
unless application_concurrency.is_a?(Hash) &&
       application_concurrency["cancel-in-progress"] == true &&
       application_concurrency["group"].to_s.include?("github.workflow") &&
       application_concurrency["group"].to_s.include?("github.event.pull_request.number") &&
       application_concurrency["group"].to_s.include?("github.ref")
  abort "Fast application workflow must cancel superseded runs per PR or ref"
end

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
  "pnpm verify:fast"
]
missing_application_runs = required_application_runs - application_runs
unless missing_application_runs.empty?
  abort "Application workflow is missing required run commands: #{missing_application_runs.join(', ')}"
end

combined_runs = application_runs.join("\n")
[
  "source scripts/dev-toolchain.env",
  "corepack@$COREPACK_VERSION",
  "pnpm@$PNPM_VERSION"
].each do |required|
  abort "Fast application workflow is missing toolchain step: #{required}" unless combined_runs.include?(required)
end

native_path = ".github/workflows/application-native.yml"
native = YAML.load_file(native_path)
abort "Workflow must be a mapping: #{native_path}" unless native.is_a?(Hash)

native_events = native["on"] || native[true]
abort "Native application workflow must define pull_request and workflow_dispatch" unless native_events.is_a?(Hash)
unless native_events.keys.map(&:to_s).sort == %w[pull_request workflow_dispatch]
  abort "Native application workflow must not run on every push"
end

native_pull_request = native_events["pull_request"]
abort "Native application workflow is missing pull_request configuration" unless native_pull_request.is_a?(Hash)
native_types = Array(native_pull_request["types"])
required_native_types = %w[opened ready_for_review reopened synchronize]
unless native_types.sort == required_native_types.sort
  abort "Native application pull_request types must run when a stable PR is opened, updated, reopened, or marked ready"
end

native_paths = [
  "apps/desktop/src-tauri/**",
  "apps/desktop/src/generated/presentation-types.ts",
  "apps/desktop/package.json",
  "packages/ai/**",
  "packages/connectors/**",
  "packages/core/**",
  "packages/parsers/**",
  "packages/db/migrations/**",
  "package.json",
  "pnpm-lock.yaml",
  "pnpm-workspace.yaml",
  "tsconfig.base.json",
  ".node-version",
  "rust-toolchain.toml",
  "scripts/dev-toolchain.env",
  native_path
]
unless Array(native_pull_request["paths"]).sort == native_paths.sort
  abort "Native application paths must exactly match native, sidecar, migration, and toolchain inputs"
end

native_permissions = native["permissions"]
unless native_permissions.is_a?(Hash) && native_permissions["contents"] == "read"
  abort "Native application workflow must use read-only contents permission"
end

native_concurrency = native["concurrency"]
unless native_concurrency.is_a?(Hash) &&
       native_concurrency["cancel-in-progress"] == true &&
       native_concurrency["group"].to_s.include?("github.workflow") &&
       native_concurrency["group"].to_s.include?("github.event.pull_request.number") &&
       native_concurrency["group"].to_s.include?("github.ref")
  abort "Native application workflow must cancel superseded runs per PR or ref"
end

native_jobs = native["jobs"]
native_job = native_jobs.is_a?(Hash) ? native_jobs["native-application-gate"] : nil
abort "Native application workflow is missing native-application-gate job" unless native_job.is_a?(Hash)
abort "Native application workflow must run on macos-14" unless native_job["runs-on"] == "macos-14"
unless native_job["if"].to_s.include?("workflow_dispatch") && native_job["if"].to_s.include?("draft == false")
  abort "Native application workflow must skip draft PRs while allowing manual runs"
end

native_steps = Array(native_job["steps"])
native_uses = native_steps.map { |step| step.is_a?(Hash) ? step["uses"] : nil }.compact
%w[actions/checkout@v7 actions/setup-node@v6 actions/cache@v4].each do |required|
  abort "Native application workflow is missing #{required}" unless native_uses.include?(required)
end

native_setup_node = native_steps.find { |step| step.is_a?(Hash) && step["uses"] == "actions/setup-node@v6" }
unless native_setup_node.is_a?(Hash) && native_setup_node.fetch("with", {})["node-version-file"] == ".node-version"
  abort "Native application workflow must source Node from .node-version"
end

native_cache_steps = native_steps.select { |step| step.is_a?(Hash) && step["uses"] == "actions/cache@v4" }
abort "Native application workflow must define exactly one Cargo cache" unless native_cache_steps.length == 1
native_cache = native_cache_steps.first
native_cache_config = native_cache.is_a?(Hash) ? native_cache.fetch("with", {}) : {}
native_cache_paths = native_cache_config["path"].to_s.lines.map(&:strip).reject(&:empty?)
required_native_cache_paths = %w[~/.cargo/registry ~/.cargo/git apps/desktop/src-tauri/target]
unless native_cache_paths.sort == required_native_cache_paths.sort
  abort "Native application workflow must cache only Cargo registry, git, and desktop target data"
end
native_cache_key = native_cache_config["key"].to_s
unless native_cache_key.include?("runner.os") &&
       native_cache_key.include?("rust-toolchain.toml") &&
       native_cache_key.include?("apps/desktop/src-tauri/Cargo.lock")
  abort "Native Cargo cache key must include OS, Rust toolchain, and Cargo.lock"
end

native_runs = native_steps.map { |step| step.is_a?(Hash) ? step["run"] : nil }.compact
required_native_runs = [
  "pnpm install --frozen-lockfile",
  ".agents/scripts/agent-preflight.sh",
  "pnpm verify:native"
]
missing_native_runs = required_native_runs - native_runs
unless missing_native_runs.empty?
  abort "Native application workflow is missing required run commands: #{missing_native_runs.join(', ')}"
end

native_combined_runs = native_runs.join("\n")
[
  "source scripts/dev-toolchain.env",
  "corepack@$COREPACK_VERSION",
  "pnpm@$PNPM_VERSION",
  "rustup show active-toolchain",
  "cargo clippy --version"
].each do |required|
  abort "Native application workflow is missing toolchain step: #{required}" unless native_combined_runs.include?(required)
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

runtime_concurrency = runtime["concurrency"]
unless runtime_concurrency.is_a?(Hash) &&
       runtime_concurrency["cancel-in-progress"] == true &&
       runtime_concurrency["group"].to_s.include?("github.workflow") &&
       runtime_concurrency["group"].to_s.include?("github.event.pull_request.number") &&
       runtime_concurrency["group"].to_s.include?("github.ref")
  abort "Runtime workflow must cancel superseded runs per PR or ref"
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

vault_concurrency = vault["concurrency"]
unless vault_concurrency.is_a?(Hash) &&
       vault_concurrency["cancel-in-progress"] == true &&
       vault_concurrency["group"].to_s.include?("github.workflow") &&
       vault_concurrency["group"].to_s.include?("github.event.pull_request.number") &&
       vault_concurrency["group"].to_s.include?("github.ref")
  abort "Vault workflow must cancel superseded runs per PR or ref"
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

website_path = ".github/workflows/website.yml"
website = YAML.load_file(website_path)
abort "Workflow must be a mapping: #{website_path}" unless website.is_a?(Hash)

website_events = website["on"] || website[true]
abort "Website workflow must define pull_request and push events" unless website_events.is_a?(Hash)
unless website_events.keys.map(&:to_s).sort == %w[pull_request push]
  abort "Website workflow must define only pull_request and push events"
end

website_paths = [
  "apps/website/**",
  "package.json",
  "pnpm-lock.yaml",
  "pnpm-workspace.yaml",
  ".node-version",
  "scripts/dev-toolchain.env",
  website_path
]
%w[pull_request push].each do |event|
  config = website_events[event]
  abort "Website workflow is missing #{event}" unless config.is_a?(Hash)
  paths = Array(config["paths"])
  unless paths.sort == website_paths.sort
    abort "Website #{event} paths must exactly match the isolated static-site inputs"
  end
end

website_push_branches = Array(website_events.fetch("push")["branches"])
abort "Website push must include main branch" unless website_push_branches.include?("main")

website_permissions = website["permissions"]
unless website_permissions.is_a?(Hash) && website_permissions["contents"] == "read"
  abort "Website workflow must use read-only contents permission"
end

website_concurrency = website["concurrency"]
unless website_concurrency.is_a?(Hash) &&
       website_concurrency["cancel-in-progress"] == true &&
       website_concurrency["group"].to_s.include?("github.workflow") &&
       website_concurrency["group"].to_s.include?("github.event.pull_request.number") &&
       website_concurrency["group"].to_s.include?("github.ref")
  abort "Website workflow must cancel superseded runs per PR or ref"
end

website_jobs = website["jobs"]
website_job = website_jobs.is_a?(Hash) ? website_jobs["website-gate"] : nil
abort "Website workflow is missing website-gate job" unless website_job.is_a?(Hash)
abort "Website workflow must run on ubuntu-24.04" unless website_job["runs-on"] == "ubuntu-24.04"

website_steps = Array(website_job["steps"])
website_uses = website_steps.map { |step| step.is_a?(Hash) ? step["uses"] : nil }.compact
%w[actions/checkout@v7 actions/setup-node@v6].each do |required|
  abort "Website workflow is missing #{required}" unless website_uses.include?(required)
end

website_setup_node = website_steps.find { |step| step.is_a?(Hash) && step["uses"] == "actions/setup-node@v6" }
unless website_setup_node.is_a?(Hash) && website_setup_node.fetch("with", {})["node-version-file"] == ".node-version"
  abort "Website workflow must source Node from .node-version"
end

website_runs = website_steps.map { |step| step.is_a?(Hash) ? step["run"] : nil }.compact
unless website_runs.include?("pnpm check:website")
  abort "Website workflow is missing the pnpm check:website gate"
end
unless website_runs.include?("pnpm --filter @cancan/website typecheck")
  abort "Website workflow must type-check the website, which its build does not do"
end
unless website_runs.include?("pnpm install --frozen-lockfile")
  abort "Website workflow must install workspace dependencies with a frozen lockfile"
end

website_combined_runs = website_runs.join("\n")
[
  "source scripts/dev-toolchain.env",
  "corepack@$COREPACK_VERSION",
  "pnpm@$PNPM_VERSION"
].each do |required|
  abort "Website workflow is missing toolchain step: #{required}" unless website_combined_runs.include?(required)
end

package = JSON.parse(File.read("package.json"))
scripts = package.fetch("scripts", {})
required_scripts = %w[typecheck test:unit test:rust check:rust build:web build:desktop verify:fast verify:native verify]
missing_scripts = required_scripts.reject { |name| scripts[name].is_a?(String) && !scripts[name].empty? }
abort "Root package is missing scripts: #{missing_scripts.join(', ')}" unless missing_scripts.empty?

unless scripts.fetch("test:rust").include?("pnpm --filter @cancan/desktop test:rust")
  abort "Root test:rust must delegate to the desktop Rust test suite"
end

unless scripts.fetch("test:unit").include?("--exclude") && scripts.fetch("test:unit").include?("spikes/**")
  abort "Root test:unit must exclude isolated spike tests"
end

%w[typecheck test:unit].each do |name|
  abort "Root verify does not run #{name}" unless scripts.fetch("verify").include?("pnpm #{name}")
end
unless scripts.fetch("verify").include?("pnpm --filter @cancan/desktop verify")
  abort "Root verify must delegate its native checks to the single-sidecar desktop gate"
end

%w[typecheck test:unit build:web].each do |name|
  abort "Root verify:fast does not run #{name}" unless scripts.fetch("verify:fast").include?("pnpm #{name}")
end

unless scripts.fetch("verify:native") == "pnpm --filter @cancan/desktop verify:native"
  abort "Root verify:native must delegate to the single-sidecar desktop native gate"
end

desktop_package = JSON.parse(File.read("apps/desktop/package.json"))
desktop_scripts = desktop_package.fetch("scripts", {})
%w[
  generate:presentation-types:prepared
  check:presentation-types:prepared
  check:rust:prepared
  test:rust:prepared
  build:desktop:prepared
].each do |name|
  command = desktop_scripts[name]
  abort "Desktop #{name} must use Cargo.lock" unless command.is_a?(String) && command.include?("--locked")
end

unless desktop_scripts.fetch("test:rust:prepared").include?("cargo test")
  abort "Desktop prepared Rust test must execute Rust tests"
end

standalone_prepared = {
  "generate:presentation-types" => "generate:presentation-types:prepared",
  "check:presentation-types" => "check:presentation-types:prepared",
  "test:rust" => "test:rust:prepared",
  "check:rust" => "check:rust:prepared",
  "build:desktop" => "build:desktop:prepared"
}
standalone_prepared.each do |name, prepared|
  command = desktop_scripts.fetch(name)
  unless command.include?("pnpm build:sidecar") && command.include?("pnpm #{prepared}")
    abort "Desktop #{name} must remain self-contained and delegate to #{prepared}"
  end
end

{
  "verify:native" => %w[check:presentation-types:prepared test:rust:prepared check:rust:prepared build:desktop:prepared],
  "verify" => %w[check:presentation-types:prepared check:rust:prepared build:desktop:prepared]
}.each do |name, gates|
  command = desktop_scripts.fetch(name)
  abort "Desktop #{name} must build the sidecar exactly once" unless command.scan("pnpm build:sidecar").length == 1
  gates.each do |gate|
    abort "Desktop #{name} is missing #{gate}" unless command.include?("pnpm #{gate}")
  end
end

%w[
  generate:presentation-types:prepared
  check:presentation-types:prepared
  test:rust:prepared
  check:rust:prepared
  build:desktop:prepared
].each do |name|
  if desktop_scripts.fetch(name).include?("build:sidecar")
    abort "Desktop #{name} must reuse the orchestrator's prepared sidecar"
  end
end

tauri_config = JSON.parse(File.read("apps/desktop/src-tauri/tauri.conf.json"))
before_build = tauri_config.fetch("build", {}).fetch("beforeBuildCommand", nil)
unless before_build == "pnpm build:web"
  abort "Tauri beforeBuildCommand must build only the web frontend; package scripts own sidecar preparation"
end
RUBY

echo "CI workflow check passed."
