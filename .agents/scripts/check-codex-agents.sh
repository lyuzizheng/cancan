#!/usr/bin/env bash
set -euo pipefail

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT"

if ! command -v ruby >/dev/null 2>&1; then
  echo "Missing required agent-config dependency: ruby"
  exit 1
fi

# Parse only the small TOML subset used by project agent bindings. Keeping the
# parser constrained makes duplicate keys and unsupported syntax fail closed
# without adding a package or depending on Python's version-specific tomllib.
ruby - <<'RUBY'
def invalid(path, line, message)
  location = line ? "#{path}:#{line}" : path.to_s
  raise "Invalid Codex agent configuration #{location}: #{message}"
end

def parse_toml(path, sectioned:)
  invalid(path, nil, "file is missing") unless File.file?(path)

  values = {}
  section = nil
  seen_sections = {}
  multiline_key = nil
  multiline_start = nil
  multiline_value = []

  File.readlines(path, chomp: true).each_with_index do |line, index|
    line_number = index + 1

    if multiline_key
      if line.strip == '"""'
        values[multiline_key] = multiline_value.join("\n")
        multiline_key = nil
        multiline_start = nil
        multiline_value = []
      else
        multiline_value << line
      end
      next
    end

    stripped = line.strip
    next if stripped.empty? || stripped.start_with?("#")

    if (match = stripped.match(/^\[([A-Za-z_][A-Za-z0-9_-]*)\]$/))
      invalid(path, line_number, "sections are not allowed") unless sectioned
      section = match[1]
      invalid(path, line_number, "duplicate section #{section}") if seen_sections[section]
      seen_sections[section] = true
      next
    end

    key_match = stripped.match(/^([A-Za-z_][A-Za-z0-9_-]*)\s*=\s*(.*)$/)
    invalid(path, line_number, "unsupported syntax") unless key_match

    key = key_match[1]
    qualified_key = section ? "#{section}.#{key}" : key
    invalid(path, line_number, "key outside a section") if sectioned && section.nil?
    invalid(path, line_number, "duplicate key #{qualified_key}") if values.key?(qualified_key)

    raw_value = key_match[2]
    if raw_value == '"""'
      multiline_key = qualified_key
      multiline_start = line_number
    elsif (match = raw_value.match(/^"([^"\\]*)"$/))
      values[qualified_key] = match[1]
    elsif raw_value.match?(/^-?[0-9]+$/)
      values[qualified_key] = raw_value.to_i
    else
      invalid(path, line_number, "unsupported value for #{qualified_key}")
    end
  end

  invalid(path, multiline_start, "unterminated multiline string for #{multiline_key}") if multiline_key
  values
end

begin
  config_path = ".codex/config.toml"
  expected_config = {
    "agents.max_threads" => 4,
    "agents.max_depth" => 1,
  }
  config = parse_toml(config_path, sectioned: true)
  raise "Unexpected Codex concurrency configuration in #{config_path}: #{config.inspect}" unless config == expected_config

  expected_agents = {
    "explorer" => ["gpt-5.6-sol", "high", "read-only"],
    "implementer" => ["gpt-5.6-terra", "max", "workspace-write"],
    "tester" => ["gpt-5.6-luna", "max", "workspace-write"],
    "reviewer" => ["gpt-5.6-sol", "high", "read-only"],
  }
  agents_dir = ".codex/agents"
  expected_files = expected_agents.keys.map { |name| "#{name}.toml" }.sort
  actual_files = Dir.glob("#{agents_dir}/*.toml").map { |path| File.basename(path) }.sort
  unless actual_files == expected_files
    raise "Unexpected Codex agent files in #{agents_dir}: expected #{expected_files.inspect}, found #{actual_files.inspect}"
  end

  required_keys = %w[
    name
    description
    developer_instructions
    model
    model_reasoning_effort
    sandbox_mode
  ].sort

  expected_agents.each do |name, (model, effort, sandbox)|
    path = ".codex/agents/#{name}.toml"
    agent = parse_toml(path, sectioned: false)
    raise "Unexpected keys in #{path}: #{agent.keys.sort.inspect}" unless agent.keys.sort == required_keys

    {
      "name" => name,
      "model" => model,
      "model_reasoning_effort" => effort,
      "sandbox_mode" => sandbox,
    }.each do |key, expected|
      raise "Unexpected #{key} in #{path}: #{agent[key].inspect}" unless agent[key] == expected
    end

    %w[description developer_instructions].each do |key|
      raise "#{path} requires non-empty #{key}" unless agent[key].is_a?(String) && !agent[key].strip.empty?
    end

    if name == "reviewer" && !agent["developer_instructions"].include?(".agents/workflows/review-code.md")
      raise "#{path} must delegate the detailed review loop to .agents/workflows/review-code.md"
    end
    if name == "tester" && !agent["developer_instructions"].include?(".agents/workflows/development-cycle.md")
      raise "#{path} must delegate the detailed testing loop to .agents/workflows/development-cycle.md"
    end
  end
rescue StandardError => error
  warn error.message
  exit 1
end
RUBY

echo "Codex agent configuration check passed."
