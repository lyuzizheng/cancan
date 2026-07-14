#!/usr/bin/env bash
set -euo pipefail

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT"

if ! command -v python3 >/dev/null 2>&1; then
  echo "Missing required agent-config dependency: python3 with tomllib"
  exit 1
fi

python3 - <<'PY'
from pathlib import Path
import tomllib


def load(path: Path) -> dict:
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise SystemExit(f"Invalid Codex agent configuration {path}: {error}") from error


config_path = Path(".codex/config.toml")
config = load(config_path)
expected_config = {"agents": {"max_threads": 4, "max_depth": 1}}
if config != expected_config:
    raise SystemExit(f"Unexpected Codex concurrency configuration in {config_path}: {config!r}")

expected_agents = {
    "explorer": ("gpt-5.6-sol", "high", "read-only"),
    "implementer": ("gpt-5.6-terra", "max", "workspace-write"),
    "tester": ("gpt-5.6-luna", "max", "workspace-write"),
    "reviewer": ("gpt-5.6-sol", "high", "read-only"),
}
agents_dir = Path(".codex/agents")
expected_files = {f"{name}.toml" for name in expected_agents}
actual_files = {path.name for path in agents_dir.glob("*.toml")}
if actual_files != expected_files:
    raise SystemExit(
        f"Unexpected Codex agent files in {agents_dir}: "
        f"expected {sorted(expected_files)}, found {sorted(actual_files)}"
    )
required_keys = {
    "name",
    "description",
    "developer_instructions",
    "model",
    "model_reasoning_effort",
    "sandbox_mode",
}

for name, (model, effort, sandbox) in expected_agents.items():
    path = Path(f".codex/agents/{name}.toml")
    agent = load(path)
    if set(agent) != required_keys:
        raise SystemExit(f"Unexpected keys in {path}: {sorted(agent)}")
    expected_values = {
        "name": name,
        "model": model,
        "model_reasoning_effort": effort,
        "sandbox_mode": sandbox,
    }
    for key, expected in expected_values.items():
        if agent.get(key) != expected:
            raise SystemExit(f"Unexpected {key} in {path}: {agent.get(key)!r}")
    for key in ("description", "developer_instructions"):
        if not isinstance(agent.get(key), str) or not agent[key].strip():
            raise SystemExit(f"{path} requires non-empty {key}")
PY

echo "Codex agent configuration check passed."
