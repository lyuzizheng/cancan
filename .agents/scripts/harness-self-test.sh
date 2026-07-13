#!/usr/bin/env bash
set -euo pipefail

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT"

TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/cancan-harness.XXXXXX")"
trap 'rm -rf "$TEST_ROOT"' EXIT

cp -R docs .agents .github scripts "$TEST_ROOT/"
cp AGENTS.md README.md .gitignore .node-version rust-toolchain.toml \
  package.json pnpm-lock.yaml pnpm-workspace.yaml tsconfig.base.json "$TEST_ROOT/"
mkdir -p "$TEST_ROOT/apps/desktop"
cp apps/desktop/package.json "$TEST_ROOT/apps/desktop/"
mkdir -p "$TEST_ROOT/spikes/desktop-feasibility/scripts"
cp spikes/desktop-feasibility/package.json spikes/desktop-feasibility/EVIDENCE.md \
  "$TEST_ROOT/spikes/desktop-feasibility/"
cp spikes/desktop-feasibility/scripts/verify.sh "$TEST_ROOT/spikes/desktop-feasibility/scripts/"

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

run_check .agents/scripts/check-spec-index.sh
run_check .agents/scripts/check-links.sh
run_check .agents/scripts/check-docs-consistency.sh
run_check .agents/scripts/check-agent-skills.sh
run_check .agents/scripts/check-ci-workflow.sh
run_check .agents/scripts/check-implementation-slices.sh
run_check scripts/test-setup-dev.sh

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
sed 's/| synthetic-core-flow | blocked |/| synthetic-core-flow | ready |/' "$manifest.bak" > "$manifest"
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

printf 'implementation packet untracked probe\n' > "$TEST_ROOT/implementation-packet-probe.txt"
review_packet="$TEST_ROOT/../cancan-implementation-review-packet.txt"
CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/implementation-review-packet.sh" desktop-feasibility HEAD > "$review_packet"
if ! rg -q 'implementation-packet-probe[.]txt' "$review_packet"; then
  echo "Implementation review packet omitted an untracked file."
  exit 1
fi
rm "$review_packet" "$TEST_ROOT/implementation-packet-probe.txt"

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

application_workflow="$TEST_ROOT/.github/workflows/application.yml"
cp "$application_workflow" "$application_workflow.bak"
grep -v '^        run: pnpm verify$' "$application_workflow.bak" > "$application_workflow"
expect_failure "application CI missing verify gate" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$application_workflow.bak" "$application_workflow"

desktop_package="$TEST_ROOT/apps/desktop/package.json"
cp "$desktop_package" "$desktop_package.bak"
sed 's/ --locked//g' "$desktop_package.bak" > "$desktop_package"
expect_failure "desktop CI permits unlocked Cargo resolution" env CANCAN_ROOT="$TEST_ROOT" "$TEST_ROOT/.agents/scripts/check-ci-workflow.sh"
mv "$desktop_package.bak" "$desktop_package"

echo "Harness self-test passed."
