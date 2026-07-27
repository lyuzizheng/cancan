#!/usr/bin/env bash
set -euo pipefail

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT"

TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/cancan-harness.XXXXXX")"
trap 'rm -rf "$TEST_ROOT"' EXIT

cp -R docs resources .agents .codex .github scripts "$TEST_ROOT/"
cp AGENTS.md README.md .gitignore .node-version rust-toolchain.toml \
  package.json pnpm-lock.yaml pnpm-workspace.yaml tsconfig.base.json "$TEST_ROOT/"
mkdir -p "$TEST_ROOT/apps/desktop"
cp apps/desktop/package.json "$TEST_ROOT/apps/desktop/"
mkdir -p "$TEST_ROOT/apps/desktop/src-tauri"
cp apps/desktop/src-tauri/tauri.conf.json "$TEST_ROOT/apps/desktop/src-tauri/"
mkdir -p "$TEST_ROOT/spikes/desktop-feasibility/scripts"
cp spikes/desktop-feasibility/package.json spikes/desktop-feasibility/EVIDENCE.md \
  "$TEST_ROOT/spikes/desktop-feasibility/"
cp spikes/desktop-feasibility/scripts/verify.sh "$TEST_ROOT/spikes/desktop-feasibility/scripts/"
mkdir -p "$TEST_ROOT/spikes/document-normalizer-runtime"
cp spikes/document-normalizer-runtime/EVIDENCE.md \
  "$TEST_ROOT/spikes/document-normalizer-runtime/"
mkdir -p "$TEST_ROOT/spikes/vault-security-validation"
cp spikes/vault-security-validation/EVIDENCE.md \
  "$TEST_ROOT/spikes/vault-security-validation/"
mkdir -p "$TEST_ROOT/spikes/local-inbox-readiness"
cp spikes/local-inbox-readiness/EVIDENCE.md \
  "$TEST_ROOT/spikes/local-inbox-readiness/"

(
  cd "$TEST_ROOT"
  git init -q
  git add .
  git -c user.name='Harness Self-Test' -c user.email='harness@example.invalid' commit -qm baseline
)

run_check() {
  CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/$1" >/dev/null
}

expect_failure() {
  label="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    echo "Harness failed to detect: $label"
    exit 1
  fi
  echo "Detected injected fault: $label"
}

packet_omits_untracked_probe() {
  ! rg -q '^[+]?implementation packet untracked probe$' "$1"
}

run_check .agents/scripts/check-spec-index.sh
run_check .agents/scripts/check-links.sh
run_check .agents/scripts/check-docs-consistency.sh
run_check .agents/scripts/check-agent-skills.sh
run_check .agents/scripts/check-codex-agents.sh
run_check .agents/scripts/check-ci-workflow.sh
run_check .agents/scripts/check-implementation-slices.sh
run_check scripts/test-setup-dev.sh

explorer_agent="$TEST_ROOT/.codex/agents/explorer.toml"
cp "$explorer_agent" "$explorer_agent.bak"
sed 's/model = "gpt-5.6-sol"/model = "gpt-5.6-terra"/' "$explorer_agent.bak" > "$explorer_agent"
expect_failure "explorer model drift" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh"
mv "$explorer_agent.bak" "$explorer_agent"

reviewer_agent="$TEST_ROOT/.codex/agents/reviewer.toml"

cp "$TEST_ROOT/.codex/agents/implementer.toml" "$TEST_ROOT/.codex/agents/extra-writer.toml"
expect_failure "unexpected extra writer role" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh"
rm "$TEST_ROOT/.codex/agents/extra-writer.toml"

cp "$explorer_agent" "$explorer_agent.bak"
printf '\nbroken = [\n' >> "$explorer_agent"
expect_failure "malformed agent TOML" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh"
mv "$explorer_agent.bak" "$explorer_agent"

cp "$explorer_agent" "$explorer_agent.bak"
ruby -0pi -e '
  valid = %q{Valid escapes: \" \\\\ \u0041 \U0001F600 \"""}
  $_.sub!("Plan or explore when") { "#{valid}\nPlan or explore when" }
  continuation = "Final continuation: " + "\\" + "\n"
  $_.sub!("\n\"\"\"\n") { "\n#{continuation}\"\"\"\n" }
' "$explorer_agent"
if ! env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh" >/dev/null; then
  echo "Valid multiline TOML escapes or line continuation were rejected."
  exit 1
fi
mv "$explorer_agent.bak" "$explorer_agent"

cp "$explorer_agent" "$explorer_agent.bak"
awk '1; /^Plan or explore when/ { print "Invalid TOML escape: \\q" }' "$explorer_agent.bak" > "$explorer_agent"
expect_failure "invalid multiline TOML escape" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh"
mv "$explorer_agent.bak" "$explorer_agent"

cp "$explorer_agent" "$explorer_agent.bak"
awk '1; /^Plan or explore when/ { print "Invalid delimiter \"\"\" trailing" }' "$explorer_agent.bak" > "$explorer_agent"
expect_failure "unescaped multiline TOML delimiter" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh"
mv "$explorer_agent.bak" "$explorer_agent"

cp "$explorer_agent" "$explorer_agent.bak"
ruby -0pi -e 'sub("Plan or explore when", "Invalid TOML control: \x7F\nPlan or explore when")' "$explorer_agent"
expect_failure "invalid multiline TOML control character" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh"
mv "$explorer_agent.bak" "$explorer_agent"

implementer_agent="$TEST_ROOT/.codex/agents/implementer.toml"
cp "$implementer_agent" "$implementer_agent.bak"
printf '\nmodel = "gpt-5.6-terra"\n' >> "$implementer_agent"
expect_failure "duplicate agent key" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh"
mv "$implementer_agent.bak" "$implementer_agent"

cp "$implementer_agent" "$implementer_agent.bak"
printf '\nsandbox_mode = "danger-full-access"\n' >> "$implementer_agent"
expect_failure "implementer gains a repo-local sandbox default" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh"
mv "$implementer_agent.bak" "$implementer_agent"

codex_config="$TEST_ROOT/.codex/config.toml"
cp "$codex_config" "$codex_config.bak"
sed 's/max_threads = 4/max_threads = 5/' "$codex_config.bak" > "$codex_config"
expect_failure "agent concurrency drift" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh"
mv "$codex_config.bak" "$codex_config"

tester_agent="$TEST_ROOT/.codex/agents/tester.toml"
cp "$tester_agent" "$tester_agent.bak"
sed 's/model_reasoning_effort = "max"/model_reasoning_effort = "high"/' "$tester_agent.bak" > "$tester_agent"
expect_failure "tester reasoning drift" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh"
mv "$tester_agent.bak" "$tester_agent"

cp "$tester_agent" "$tester_agent.bak"
grep -v '[.]agents/workflows/development-cycle[.]md' "$tester_agent.bak" > "$tester_agent"
expect_failure "tester loses workflow reference" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh"
mv "$tester_agent.bak" "$tester_agent"

cp "$reviewer_agent" "$reviewer_agent.bak"
sed 's/sandbox_mode = "read-only"/sandbox_mode = "workspace-write"/' "$reviewer_agent.bak" > "$reviewer_agent"
expect_failure "reviewer read-only default drift" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh"
mv "$reviewer_agent.bak" "$reviewer_agent"

cp "$reviewer_agent" "$reviewer_agent.bak"
grep -v '[.]agents/workflows/review-code[.]md' "$reviewer_agent.bak" > "$reviewer_agent"
expect_failure "reviewer loses workflow reference" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-codex-agents.sh"
mv "$reviewer_agent.bak" "$reviewer_agent"

printf 'name = "packet-probe"\n' > "$TEST_ROOT/.codex/agents/packet-probe.toml"
docs_review_packet="$TEST_ROOT/../cancan-docs-review-packet.txt"
CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/docs-review-packet.sh" HEAD > "$docs_review_packet"
if ! rg -q '[.]codex/agents/packet-probe[.]toml' "$docs_review_packet"; then
  echo "Docs review packet omitted an untracked project agent file."
  exit 1
fi
rm "$docs_review_packet" "$TEST_ROOT/.codex/agents/packet-probe.toml"

context_packet="$TEST_ROOT/../cancan-context-packet.txt"
CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/context-for-slice.sh" desktop-feasibility > "$context_packet"
if ! rg -q 'docs/specs/0001-repo-structure[.]md' "$context_packet" || ! rg -q 'Implementation readiness: COMPLETE' "$context_packet"; then
  echo "Slice context packet is missing required sources or blocker state."
  exit 1
fi
rm "$context_packet"

context_packet="$TEST_ROOT/../cancan-dependent-context-packet.txt"
CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/context-for-slice.sh" app-foundation > "$context_packet"
if ! rg -q 'Depends on: desktop-feasibility' "$context_packet" || ! rg -q 'Implementation readiness: COMPLETE' "$context_packet"; then
  echo "Completed slice context omitted dependency or completion state."
  exit 1
fi
rm "$context_packet"

CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/new-spec.sh" "Harness Probe" >/dev/null
generated_spec="$(find "$TEST_ROOT/docs/specs" -maxdepth 1 -type f -name '*-harness-probe.md')"
if [ -z "$generated_spec" ]; then
  echo "new-spec did not create the expected file"
  exit 1
fi
expect_failure "new spec requires index update" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-spec-index.sh"
rm "$generated_spec"

cp "$TEST_ROOT/docs/specs/0017-evidence-documents-source-ux.md" "$TEST_ROOT/docs/specs/0017-duplicate.md"
expect_failure "duplicate spec number" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-spec-index.sh"
rm "$TEST_ROOT/docs/specs/0017-duplicate.md"

index="$TEST_ROOT/docs/specs/README.md"
cp "$index" "$index.bak"
grep -v '0017-evidence-documents-source-ux.md' "$index.bak" > "$index"
expect_failure "spec missing from index" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-spec-index.sh"
mv "$index.bak" "$index"

readme="$TEST_ROOT/docs/README.md"
cp "$readme" "$readme.bak"
printf '\n[broken harness link](missing-harness-target.md)\n' >> "$readme"
expect_failure "broken local Markdown link" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-links.sh"
mv "$readme.bak" "$readme"

router="$TEST_ROOT/.agents/ROUTER.md"
cp "$router" "$router.bak"
sed 's#\./workflows/design-grill[.]md#./workflows/missing-design-grill.md#' "$router.bak" > "$router"
expect_failure "broken router target" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-links.sh"
mv "$router.bak" "$router"

skill="$TEST_ROOT/.agents/skills/cancan-docs-orientation/SKILL.md"
cp "$skill" "$skill.bak"
sed '2s/.*/name: wrong-name/' "$skill.bak" > "$skill"
expect_failure "skill name mismatch" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-agent-skills.sh"
mv "$skill.bak" "$skill"

agent_readme="$TEST_ROOT/.agents/README.md"
cp "$agent_readme" "$agent_readme.bak"
printf '\n## Current Priority\n' >> "$agent_readme"
expect_failure "static priority in harness" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-docs-consistency.sh"
mv "$agent_readme.bak" "$agent_readme"

cp "$readme" "$readme.bak"
printf '\ntrailing whitespace probe \n' >> "$readme"
expect_failure "whole-tree trailing whitespace" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-docs-consistency.sh"
mv "$readme.bak" "$readme"

adr="$TEST_ROOT/docs/adr/0001-local-first-tauri-react-sqlite.md"
cp "$adr" "$adr.bak"
awk 'found && $0 == "Accepted" {$0 = "Unknown"} /^## Status$/ {found = 1} {print}' "$adr.bak" > "$adr"
expect_failure "invalid ADR status" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-docs-consistency.sh"
mv "$adr.bak" "$adr"

cp "$agent_readme" "$agent_readme.bak"
printf '\nSee `.agents/roles/removed.md`.\n' >> "$agent_readme"
expect_failure "reference to removed harness layer" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-docs-consistency.sh"
mv "$agent_readme.bak" "$agent_readme"

mkdir -p "$TEST_ROOT/fixtures-private"
printf 'private fixture probe\n' > "$TEST_ROOT/fixtures-private/probe.txt"
(
  cd "$TEST_ROOT"
  git add -f fixtures-private/probe.txt
)
expect_failure "tracked private fixture" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-docs-consistency.sh"
(
  cd "$TEST_ROOT"
  git rm -q --cached fixtures-private/probe.txt
)
rm -rf "$TEST_ROOT/fixtures-private"

syntax_probe="$TEST_ROOT/.agents/scripts/syntax-probe.sh"
printf '#!/usr/bin/env bash\nif then\n' > "$syntax_probe"
expect_failure "invalid shell syntax" bash -n "$syntax_probe"
rm "$syntax_probe"

ruby_probe="$TEST_ROOT/.agents/scripts/syntax-probe.rb"
printf 'def broken(\n' > "$ruby_probe"
expect_failure "invalid Ruby syntax" ruby -c "$ruby_probe"
rm "$ruby_probe"

manifest="$TEST_ROOT/docs/agent/implementation-slices.md"
cp "$manifest" "$manifest.bak"
awk 'BEGIN {changed=0} !changed && /docs\/specs\/0001-repo-structure[.]md/ {sub(/docs\/specs\/0001-repo-structure[.]md/, "docs/specs/9999-missing.md"); changed=1} {print}' "$manifest.bak" > "$manifest"
expect_failure "slice references missing spec" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$manifest.bak" "$manifest"

cp "$manifest" "$manifest.bak"
awk 'BEGIN {changed=0} !changed && /Public project identity and website launch/ {sub(/Public project identity and website launch/, "Missing public-project blocker"); changed=1} {print}' "$manifest.bak" > "$manifest"
expect_failure "slice references missing blocker" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$manifest.bak" "$manifest"

cp "$manifest" "$manifest.bak"
sed 's/desktop build spike, SQLCipher smoke, FTS5 smoke, file encryption and Keychain\/recovery smoke/none/' "$manifest.bak" > "$manifest"
expect_failure "slice missing test gates" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$manifest.bak" "$manifest"

cp "$manifest" "$manifest.bak"
sed 's/| desktop-feasibility | completed | none |/| desktop-feasibility | completed | backup-release |/' "$manifest.bak" > "$manifest"
expect_failure "slice dependency cycle or invalid order" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$manifest.bak" "$manifest"

cp "$manifest" "$manifest.bak"
sed 's/| desktop-feasibility | completed |/| desktop-feasibility | unknown |/' "$manifest.bak" > "$manifest"
expect_failure "slice invalid status" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$manifest.bak" "$manifest"

cp "$manifest" "$manifest.bak"
sed 's/| public-project-surface | blocked |/| public-project-surface | ready |/' "$manifest.bak" > "$manifest"
expect_failure "ready slice retains blockers" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$manifest.bak" "$manifest"

cp "$manifest" "$manifest.bak"
sed 's/| high-confidence-auto-commit | blocked |/| high-confidence-auto-commit | ready |/' "$manifest.bak" > "$manifest"
expect_failure "ready slice has incomplete dependency" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$manifest.bak" "$manifest"

cp "$manifest" "$manifest.bak"
sed 's/, Security observability and sensitive-data lifecycle | connectors/ | connectors/' "$manifest.bak" > "$manifest"
expect_failure "sensitive slice loses safety blocker" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$manifest.bak" "$manifest"

adr="$TEST_ROOT/docs/adr/0001-local-first-tauri-react-sqlite.md"
cp "$adr" "$adr.bak"
awk 'found && $0 == "Accepted" {$0 = "Proposed"} /^## Status$/ {found = 1} {print}' "$adr.bak" > "$adr"
expect_failure "ready slice uses proposed ADR" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$adr.bak" "$adr"

node_version="$TEST_ROOT/.node-version"
cp "$node_version" "$node_version.bak"
printf '0.0.0\n' > "$node_version"
expect_failure "setup toolchain version drift" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/scripts/test-setup-dev.sh"
mv "$node_version.bak" "$node_version"

setup_script="$TEST_ROOT/scripts/setup-dev.sh"
cp "$setup_script" "$setup_script.bak"
grep -v 'pnpm verify)' "$setup_script.bak" > "$setup_script"
expect_failure "setup omits production application gate" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/scripts/test-setup-dev.sh"
mv "$setup_script.bak" "$setup_script"

cp "$setup_script" "$setup_script.bak"
grep -v 'pnpm spike:verify)' "$setup_script.bak" > "$setup_script"
expect_failure "setup omits isolated spike gate" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/scripts/test-setup-dev.sh"
mv "$setup_script.bak" "$setup_script"

implementation_review="$TEST_ROOT/.agents/scripts/implementation-review-packet.sh"
cp "$implementation_review" "$implementation_review.bak"
grep -v 'context-for-slice[.]sh.*slice_id' "$implementation_review.bak" > "$implementation_review"
expect_failure "implementation review loses shared context" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$implementation_review.bak" "$implementation_review"

cp "$implementation_review" "$implementation_review.bak"
grep -v '^[.]agents/scripts/context-for-slice[.]sh "[$]slice_id" >/dev/null$' "$implementation_review.bak" > "$implementation_review"
expect_failure "implementation review keeps only the context instruction" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$implementation_review.bak" "$implementation_review"

expect_failure "implementation review accepts an unknown slice" env CANCAN_ROOT="$TEST_ROOT" "$implementation_review" unknown-slice HEAD

cp "$implementation_review" "$implementation_review.bak"
sed 's#context-for-slice[.]sh "[$]slice_id" >/dev/null#context-for-slice.sh "$slice_id"#' "$implementation_review.bak" > "$implementation_review"
expect_failure "implementation review embeds shared context" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$implementation_review.bak" "$implementation_review"

implementation_slices="$TEST_ROOT/.agents/scripts/implementation-slices.rb"
cp "$implementation_slices" "$implementation_slices.bak"
ruby -0pi -e 'sub("def print_source_index(path)\n", "def print_source_index(path)\n  puts File.read(File.join(ROOT, path))\n")' "$implementation_slices"
expect_failure "implementation context copies canonical source content" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$implementation_slices.bak" "$implementation_slices"

root_instructions="$TEST_ROOT/AGENTS.md"
cp "$root_instructions" "$root_instructions.bak"
sed 's/open the full contents of every listed source from the exact working tree and head/use the generated index/' "$root_instructions.bak" > "$root_instructions"
expect_failure "root instructions lose full canonical-source inspection" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$root_instructions.bak" "$root_instructions"

implement_workflow="$TEST_ROOT/.agents/workflows/implement-feature.md"
cp "$implement_workflow" "$implement_workflow.bak"
sed 's/Open the full contents of every source listed by the generated index/Inspect selected indexed sources/' "$implement_workflow.bak" > "$implement_workflow"
expect_failure "implement workflow loses full canonical-source inspection" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$implement_workflow.bak" "$implement_workflow"

testing_workflow="$TEST_ROOT/.agents/workflows/simulated-testing.md"
cp "$testing_workflow" "$testing_workflow.bak"
sed 's/open the full contents of every source listed by the generated index/inspect selected indexed sources/' "$testing_workflow.bak" > "$testing_workflow"
expect_failure "testing workflow loses full canonical-source inspection" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$testing_workflow.bak" "$testing_workflow"

development_cycle="$TEST_ROOT/.agents/workflows/development-cycle.md"
cp "$development_cycle" "$development_cycle.bak"
sed 's#[.]agents/scripts/agent-preflight[.]sh#.agents/scripts/preflight-removed.sh#' "$development_cycle.bak" > "$development_cycle"
expect_failure "development cycle loses preflight evaluation" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$development_cycle.bak" "$development_cycle"

cp "$development_cycle" "$development_cycle.bak"
sed 's/`pnpm verify`/`pnpm evaluate`/' "$development_cycle.bak" > "$development_cycle"
expect_failure "development cycle loses application evaluation" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$development_cycle.bak" "$development_cycle"

review_workflow="$TEST_ROOT/.agents/workflows/review-code.md"
cp "$review_workflow" "$review_workflow.bak"
grep -v '^## Critical cleanup gate$' "$review_workflow.bak" > "$review_workflow"
expect_failure "review workflow loses critical cleanup gate" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$review_workflow.bak" "$review_workflow"

cp "$review_workflow" "$review_workflow.bak"
sed 's/Open the full contents of every source listed by the generated index/Inspect selected indexed sources/' "$review_workflow.bak" > "$review_workflow"
expect_failure "review workflow loses full canonical-source inspection" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$review_workflow.bak" "$review_workflow"

cp "$review_workflow" "$review_workflow.bak"
sed 's/Reject overengineering:/Consider complexity:/' "$review_workflow.bak" > "$review_workflow"
expect_failure "review workflow loses critical cleanup content" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$review_workflow.bak" "$review_workflow"

cp "$development_cycle" "$development_cycle.bak"
sed 's/re-review the entire cumulative diff/review the latest fix/' "$development_cycle.bak" > "$development_cycle"
expect_failure "development cycle loses whole cumulative diff re-review" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$development_cycle.bak" "$development_cycle"

cp "$development_cycle" "$development_cycle.bak"
sed 's/Do not duplicate the root transcript or tool history/Copy the root transcript and tool history/' "$development_cycle.bak" > "$development_cycle"
expect_failure "development cycle loses compact role handoff" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$development_cycle.bak" "$development_cycle"

cp "$review_workflow" "$review_workflow.bak"
sed 's/inspect the complete cumulative diff directly from the shared repository/use only the packet copy of the diff/' "$review_workflow.bak" > "$review_workflow"
expect_failure "review workflow stops reading the repository diff" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$review_workflow.bak" "$review_workflow"

printf 'implementation packet untracked probe\n' > "$TEST_ROOT/implementation-packet-probe.txt"
review_packet="$TEST_ROOT/../cancan-implementation-review-packet.txt"
CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/implementation-review-packet.sh" desktop-feasibility HEAD > "$review_packet"
if ! rg -q 'implementation-packet-probe[.]txt' "$review_packet" ||
   ! rg -q '^# CanCan Implementation Review Handoff$' "$review_packet" ||
   ! rg -q '^- Base commit: [0-9a-f]{40}$' "$review_packet" ||
   ! rg -q '^- Head commit: [0-9a-f]{40}$' "$review_packet" ||
   ! rg -q '^- Working tree fingerprint: [0-9a-f]{40}$' "$review_packet" ||
   ! rg -q '^# Required External Handoff$' "$review_packet" ||
   ! rg -q '^## Diff stat$' "$review_packet" ||
   ! rg -q '^## Rename and deletion summary$' "$review_packet" ||
   ! rg -q '^## Repository inspection$' "$review_packet"; then
  echo "Implementation review packet omitted required context or evidence sections."
  exit 1
fi
if ! packet_omits_untracked_probe "$review_packet"; then
  echo "Implementation review packet copied untracked file content instead of indexing it."
  exit 1
fi

original_fingerprint="$(sed -n 's/^- Working tree fingerprint: //p' "$review_packet")"
printf 'implementation packet changed untracked probe\n' > "$TEST_ROOT/implementation-packet-probe.txt"
changed_review_packet="$TEST_ROOT/../cancan-implementation-review-packet-changed.txt"
CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/implementation-review-packet.sh" desktop-feasibility HEAD > "$changed_review_packet"
changed_fingerprint="$(sed -n 's/^- Working tree fingerprint: //p' "$changed_review_packet")"
if [ "$original_fingerprint" = "$changed_fingerprint" ]; then
  echo "Implementation review packet fingerprint ignored untracked content changes."
  exit 1
fi
echo "Detected injected fault: implementation review checkout fingerprint drift"

printf '+implementation packet untracked probe\n' >> "$review_packet"
expect_failure "implementation review copies diff-prefixed untracked content" packet_omits_untracked_probe "$review_packet"
rm "$review_packet" "$changed_review_packet" "$TEST_ROOT/implementation-packet-probe.txt"

cp "$implementation_review" "$implementation_review.bak"
sed 's/^echo "- Committed changes: git diff --no-ext-diff [$]base_sha [$]head_sha -- [.]"$/git diff --no-ext-diff "$base_sha" "$head_sha" -- ./' "$implementation_review.bak" > "$implementation_review"
expect_failure "implementation review copies the full cumulative diff" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$implementation_review.bak" "$implementation_review"

cp "$implementation_review" "$implementation_review.bak"
grep -v 'Exact user request' "$implementation_review.bak" > "$implementation_review"
expect_failure "implementation review loses required handoff evidence" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$implementation_review.bak" "$implementation_review"

cp "$implementation_review" "$implementation_review.bak"
grep -v 'Selected execution tier and justification' "$implementation_review.bak" > "$implementation_review"
expect_failure "implementation review loses execution tier" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$implementation_review.bak" "$implementation_review"

cp "$implementation_review" "$implementation_review.bak"
grep -v 'Canonical sources inspected at the exact head commit' "$implementation_review.bak" > "$implementation_review"
expect_failure "implementation review loses canonical-source evidence" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$implementation_review.bak" "$implementation_review"

cp "$implementation_review" "$implementation_review.bak"
grep -v 'git diff --summary --find-renames' "$implementation_review.bak" > "$implementation_review"
expect_failure "implementation review loses rename and deletion evidence" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-implementation-slices.sh"
mv "$implementation_review.bak" "$implementation_review"

workflow="$TEST_ROOT/.github/workflows/docs-harness.yml"
cp "$workflow" "$workflow.bak"
printf '\ninvalid: [\n' >> "$workflow"
expect_failure "invalid workflow YAML" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$workflow.bak" "$workflow"

cp "$workflow" "$workflow.bak"
grep -v '^        run: [. ]*agents/scripts/harness-self-test[.]sh$' "$workflow.bak" > "$workflow"
expect_failure "docs CI missing harness self-test" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$workflow.bak" "$workflow"

cp "$workflow" "$workflow.bak"
sed 's/^      - main$/      - develop/' "$workflow.bak" > "$workflow"
expect_failure "docs CI missing main push branch" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$workflow.bak" "$workflow"

cp "$workflow" "$workflow.bak"
sed 's#actions/checkout@v7#actions/checkout@v4#' "$workflow.bak" > "$workflow"
expect_failure "docs CI uses deprecated action runtime" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$workflow.bak" "$workflow"

application_workflow="$TEST_ROOT/.github/workflows/application.yml"
cp "$application_workflow" "$application_workflow.bak"
grep -v '^        run: pnpm verify:fast$' "$application_workflow.bak" > "$application_workflow"
expect_failure "fast application CI missing verify gate" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$application_workflow.bak" "$application_workflow"

cp "$application_workflow" "$application_workflow.bak"
grep -v '^  cancel-in-progress: true$' "$application_workflow.bak" > "$application_workflow"
expect_failure "fast application CI loses superseded-run cancellation" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$application_workflow.bak" "$application_workflow"

native_workflow="$TEST_ROOT/.github/workflows/application-native.yml"
cp "$native_workflow" "$native_workflow.bak"
grep -v '^        run: pnpm verify:native$' "$native_workflow.bak" > "$native_workflow"
expect_failure "native application CI missing native gate" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$native_workflow.bak" "$native_workflow"

cp "$native_workflow" "$native_workflow.bak"
grep -v '^        uses: actions/cache@v4$' "$native_workflow.bak" > "$native_workflow"
expect_failure "native application CI loses Cargo cache" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$native_workflow.bak" "$native_workflow"

cp "$native_workflow" "$native_workflow.bak"
sed 's#apps/desktop/src-tauri/target#apps/desktop/src-tauri/binaries#' "$native_workflow.bak" > "$native_workflow"
expect_failure "native application CI caches sidecar binaries" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$native_workflow.bak" "$native_workflow"

cp "$native_workflow" "$native_workflow.bak"
ruby -0pi -e 'sub("      - name: Install workspace dependencies\n", "      - name: Cache sidecar binaries\n        uses: actions/cache@v4\n        with:\n          path: apps/desktop/src-tauri/binaries\n          key: forbidden-sidecar-cache\n      - name: Install workspace dependencies\n")' "$native_workflow"
expect_failure "native application CI gains a second cache" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$native_workflow.bak" "$native_workflow"

cp "$native_workflow" "$native_workflow.bak"
sed 's#      - apps/desktop/src-tauri/[*][*]#      - apps/**#' "$native_workflow.bak" > "$native_workflow"
expect_failure "native application CI gains renderer-wide trigger" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$native_workflow.bak" "$native_workflow"

cp "$native_workflow" "$native_workflow.bak"
grep -v '^      - apps/desktop/src/generated/presentation-types[.]ts$' "$native_workflow.bak" > "$native_workflow"
expect_failure "native application CI loses generated type drift trigger" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$native_workflow.bak" "$native_workflow"

cp "$native_workflow" "$native_workflow.bak"
sed "s/github.event.pull_request.draft == false/true/" "$native_workflow.bak" > "$native_workflow"
expect_failure "native application CI runs for draft PRs" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$native_workflow.bak" "$native_workflow"

root_package="$TEST_ROOT/package.json"
cp "$root_package" "$root_package.bak"
sed "s/ --exclude 'spikes\/[*][*]'//" "$root_package.bak" > "$root_package"
expect_failure "application unit gate includes isolated spikes" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$root_package.bak" "$root_package"

runtime_workflow="$TEST_ROOT/.github/workflows/document-normalizer-runtime.yml"
cp "$runtime_workflow" "$runtime_workflow.bak"
grep -v '^        run: bash spikes/document-normalizer-runtime/scripts/verify[.]sh$' "$runtime_workflow.bak" > "$runtime_workflow"
expect_failure "runtime CI missing spike verify gate" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$runtime_workflow.bak" "$runtime_workflow"

cp "$runtime_workflow" "$runtime_workflow.bak"
ruby -0pi -e 'sub("      - .github/workflows/document-normalizer-runtime.yml\n", "      - .github/workflows/document-normalizer-runtime.yml\n      - unrelated/**\n")' "$runtime_workflow"
expect_failure "runtime CI gains unrelated trigger path" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$runtime_workflow.bak" "$runtime_workflow"

vault_workflow="$TEST_ROOT/.github/workflows/vault-security-validation.yml"
cp "$vault_workflow" "$vault_workflow.bak"
grep -v '^        run: bash spikes/vault-security-validation/scripts/verify[.]sh$' "$vault_workflow.bak" > "$vault_workflow"
expect_failure "vault CI missing security validation gate" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$vault_workflow.bak" "$vault_workflow"

desktop_package="$TEST_ROOT/apps/desktop/package.json"
cp "$desktop_package" "$desktop_package.bak"
sed 's/ --locked//g' "$desktop_package.bak" > "$desktop_package"
expect_failure "desktop CI permits unlocked Cargo resolution" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$desktop_package.bak" "$desktop_package"

cp "$desktop_package" "$desktop_package.bak"
sed 's/pnpm build:sidecar && pnpm test:rust:prepared/pnpm test:rust:prepared/' "$desktop_package.bak" > "$desktop_package"
expect_failure "standalone Rust test loses sidecar preparation" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$desktop_package.bak" "$desktop_package"

cp "$desktop_package" "$desktop_package.bak"
ruby -rjson -e '
  path = ARGV.fetch(0)
  package = JSON.parse(File.read(path))
  command = package.fetch("scripts").fetch("verify:native")
  package["scripts"]["verify:native"] = command.sub(
    "pnpm build:sidecar",
    "pnpm build:sidecar && pnpm build:sidecar"
  )
  File.write(path, JSON.pretty_generate(package) + "\n")
' "$desktop_package"
expect_failure "native gate rebuilds the sidecar" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$desktop_package.bak" "$desktop_package"

cp "$desktop_package" "$desktop_package.bak"
ruby -rjson -e '
  path = ARGV.fetch(0)
  package = JSON.parse(File.read(path))
  package["scripts"]["check:presentation-types"] = "pnpm check:presentation-types:prepared"
  File.write(path, JSON.pretty_generate(package) + "\n")
' "$desktop_package"
expect_failure "standalone presentation check loses sidecar preparation" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$desktop_package.bak" "$desktop_package"

tauri_config="$TEST_ROOT/apps/desktop/src-tauri/tauri.conf.json"
cp "$tauri_config" "$tauri_config.bak"
sed 's/pnpm build:web/pnpm build:sidecar \&\& pnpm build:web/' "$tauri_config.bak" > "$tauri_config"
expect_failure "Tauri build hook rebuilds the sidecar" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$tauri_config.bak" "$tauri_config"

echo "Harness self-test passed."
