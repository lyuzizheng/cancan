#!/usr/bin/env ruby

ROOT = File.expand_path(ENV.fetch("CANCAN_ROOT", File.join(__dir__, "../..")))
MANIFEST = File.join(ROOT, "docs/agent/implementation-slices.md")
ALIGNMENT = File.join(ROOT, "docs/alignment-temp/alignment-progress.md")
HEADER = [
  "ID",
  "Status",
  "Depends on",
  "Required specs",
  "Required ADRs",
  "Active blockers",
  "Packages/surfaces",
  "Test gates",
  "Outcome"
].freeze

def fail!(message)
  warn message
  exit 1
end

def cells(line)
  stripped = line.strip
  return nil unless stripped.start_with?("|") && stripped.end_with?("|")

  stripped[1...-1].split("|", -1).map(&:strip)
end

def list(cell)
  return [] if cell == "none"

  cell.delete(96.chr).split(",").map(&:strip).reject(&:empty?)
end

def load_slices
  lines = File.readlines(MANIFEST, chomp: true)
  header_index = lines.index { |line| cells(line) == HEADER }
  fail!("Implementation slice table header is missing or changed.") unless header_index

  separator = cells(lines.fetch(header_index + 1, ""))
  unless separator&.length == HEADER.length && separator.all? { |value| value.match?(/\A-+\z/) }
    fail!("Implementation slice table separator is invalid.")
  end

  rows = []
  lines[(header_index + 2)..].to_a.each do |line|
    row = cells(line)
    break unless row
    fail!("Implementation slice row has #{row.length} columns, expected #{HEADER.length}: #{line}") unless row.length == HEADER.length
    rows << HEADER.zip(row).to_h
  end
  fail!("Implementation slice table has no rows.") if rows.empty?
  rows
end

def active_alignment_areas
  File.readlines(ALIGNMENT, chomp: true).each_with_object([]) do |line, areas|
    match = line.match(/^\|\s*([^|]+?)\s*\|\s*(partial|unresolved|blocked)\s*\|/)
    areas << match[1].strip if match
  end
end

def implementation_blocking_alignment_areas
  priority = nil
  File.readlines(ALIGNMENT, chomp: true).each_with_object([]) do |line, areas|
    heading = line.match(/^## P([0-2]):/)
    priority = heading[1].to_i if heading
    next unless priority && priority <= 1

    match = line.match(/^\|\s*([^|]+?)\s*\|\s*(partial|unresolved|blocked)\s*\|/)
    areas << match[1].strip if match
  end
end

def adr_status(path)
  lines = File.readlines(File.join(ROOT, path), chomp: true)
  heading = lines.index("## Status")
  return nil unless heading

  lines[(heading + 1)..].to_a.find { |line| !line.strip.empty? }&.strip
end

def validate!(slices)
  ids = slices.map { |slice| slice.fetch("ID") }
  duplicate = ids.group_by(&:itself).find { |_id, matches| matches.length > 1 }
  fail!("Duplicate implementation slice ID: #{duplicate[0]}") if duplicate

  areas = active_alignment_areas
  positions = ids.each_with_index.to_h

  slices.each do |slice|
    id = slice.fetch("ID")
    fail!("Invalid implementation slice ID: #{id}") unless id.match?(/\A[a-z0-9]+(?:-[a-z0-9]+)*\z/)
    status = slice.fetch("Status")
    fail!("Invalid status for slice #{id}: #{status}") unless %w[blocked investigating ready in_progress completed].include?(status)

    specs = list(slice.fetch("Required specs"))
    fail!("Slice #{id} has no required specs.") if specs.empty?
    specs.each do |path|
      fail!("Slice #{id} references invalid spec path: #{path}") unless path.match?(%r{\Adocs/specs/\d{4}-[^/]+\.md\z})
      fail!("Slice #{id} references missing spec: #{path}") unless File.file?(File.join(ROOT, path))
    end

    adrs = list(slice.fetch("Required ADRs"))
    adrs.each do |path|
      fail!("Slice #{id} references invalid ADR path: #{path}") unless path.match?(%r{\Adocs/adr/\d{4}-[^/]+\.md\z})
      fail!("Slice #{id} references missing ADR: #{path}") unless File.file?(File.join(ROOT, path))
    end

    blockers = list(slice.fetch("Active blockers"))
    blockers.each do |blocker|
      fail!("Slice #{id} references inactive or missing blocker: #{blocker}") unless areas.include?(blocker)
    end

    if %w[ready in_progress completed].include?(status)
      adrs.each do |path|
        status_value = adr_status(path)
        fail!("Slice #{id} is #{status} but required ADR is not Accepted: #{path} (#{status_value || "missing status"})") unless status_value == "Accepted"
      end
    end

    fail!("Slice #{id} has no packages/surfaces.") if slice.fetch("Packages/surfaces").empty? || slice.fetch("Packages/surfaces") == "none"
    fail!("Slice #{id} has no test gates.") if slice.fetch("Test gates").empty? || slice.fetch("Test gates") == "none"
    fail!("Slice #{id} has no outcome.") if slice.fetch("Outcome").empty? || slice.fetch("Outcome") == "none"

    list(slice.fetch("Depends on")).each do |dependency|
      fail!("Slice #{id} depends on unknown slice: #{dependency}") unless positions.key?(dependency)
      fail!("Slice #{id} must appear after dependency #{dependency}.") unless positions.fetch(dependency) < positions.fetch(id)
    end
  end

  visiting = {}
  visited = {}
  walk = lambda do |id|
    fail!("Implementation slice dependency cycle includes #{id}.") if visiting[id]
    return if visited[id]

    visiting[id] = true
    slice = slices.find { |candidate| candidate.fetch("ID") == id }
    list(slice.fetch("Depends on")).each { |dependency| walk.call(dependency) }
    visiting.delete(id)
    visited[id] = true
  end
  ids.each { |id| walk.call(id) }

  by_id = slices.each_with_object({}) { |slice, result| result[slice.fetch("ID")] = slice }
  referenced_blockers = slices.reject { |slice| slice.fetch("Status") == "completed" }.flat_map do |slice|
    list(slice.fetch("Active blockers"))
  end.uniq
  implementation_blocking_alignment_areas.each do |area|
    fail!("Active P0/P1 alignment area is not referenced by a non-completed slice: #{area}") unless referenced_blockers.include?(area)
  end

  slices.each do |slice|
    id = slice.fetch("ID")
    status = slice.fetch("Status")
    blockers = list(slice.fetch("Active blockers"))
    incomplete_dependencies = list(slice.fetch("Depends on")).reject do |dependency|
      by_id.fetch(dependency).fetch("Status") == "completed"
    end

    if %w[ready in_progress completed].include?(status) && !blockers.empty?
      fail!("Slice #{id} is #{status} but still has active blockers: #{blockers.join(", ")}")
    end
    if %w[ready in_progress completed].include?(status) && !incomplete_dependencies.empty?
      fail!("Slice #{id} is #{status} but dependencies are not completed: #{incomplete_dependencies.join(", ")}")
    end
    if status == "blocked" && blockers.empty? && incomplete_dependencies.empty?
      fail!("Slice #{id} is blocked without an active blocker or incomplete dependency.")
    end
    if status == "investigating" && blockers.empty?
      fail!("Slice #{id} is investigating without an active blocker.")
    end
    if status == "investigating" && !incomplete_dependencies.empty?
      fail!("Slice #{id} is investigating but dependencies are not completed: #{incomplete_dependencies.join(", ")}")
    end
  end
end

def dependency_closure(slice, slices, result = [])
  by_id = slices.each_with_object({}) { |entry, index| index[entry.fetch("ID")] = entry }
  list(slice.fetch("Depends on")).each do |dependency|
    dependency_slice = by_id.fetch(dependency)
    dependency_closure(dependency_slice, slices, result)
    result << dependency_slice unless result.include?(dependency_slice)
  end
  result
end

def print_source_index(path)
  lines = File.readlines(File.join(ROOT, path), chomp: true)
  puts
  puts "## Required source: #{path}"
  headings = lines.each_with_index.each_with_object([]) do |(line, index), result|
    result << "- L#{index + 1}: #{line}" if line.match?(/\A\#{1,6}\s/)
  end
  if headings.empty?
    puts "- No Markdown headings; inspect the file directly."
  else
    puts headings
  end
end

slices = load_slices
validate!(slices)

case ARGV[0] || "check"
when "check"
  puts "Implementation slice check passed (#{slices.length} slices)."
when "list"
  slices.each { |slice| puts slice.fetch("ID") }
when "context"
  id = ARGV[1]
  fail!("Usage: implementation-slices.rb context <slice-id>") if id.nil? || id.empty?
  slice = slices.find { |candidate| candidate.fetch("ID") == id }
  fail!("Unknown slice '#{id}'. Available: #{slices.map { |entry| entry.fetch("ID") }.join(", ")}") unless slice

  puts "# CanCan Implementation Context: #{id}"
  puts
  HEADER.each { |key| puts "- #{key}: #{slice.fetch(key)}" }
  dependencies = dependency_closure(slice, slices)
  blockers = (dependencies + [slice]).flat_map { |entry| list(entry.fetch("Active blockers")) }.uniq
  incomplete_dependencies = dependencies.reject { |entry| entry.fetch("Status") == "completed" }
  puts
  if slice.fetch("Status") == "completed"
    puts "Implementation readiness: COMPLETE"
  elsif slice.fetch("Status") == "investigating"
    puts "Implementation readiness: EVIDENCE ONLY"
    puts "- permitted work: disposable evidence named by this slice's test gates"
    puts "- prohibited work: production implementation and downstream slices"
    puts "- effective active blockers: #{blockers.join(", ")}" unless blockers.empty?
  elsif slice.fetch("Status") == "blocked" || !blockers.empty? || !incomplete_dependencies.empty?
    puts "Implementation readiness: STOP"
    puts "- slice status: #{slice.fetch("Status")}"
    puts "- incomplete dependencies: #{incomplete_dependencies.map { |entry| entry.fetch("ID") }.join(", ")}" unless incomplete_dependencies.empty?
    puts "- effective active blockers: #{blockers.join(", ")}" unless blockers.empty?
  else
    puts "Implementation readiness: READY"
  end

  global_sources = [
    "AGENTS.md",
    "docs/STRUCTURE.md",
    "docs/agent/current-state.md",
    "docs/alignment-temp/alignment-progress.md"
  ]
  puts
  puts "# Canonical Source Index"
  puts
  puts "Open these exact repository files directly. This packet indexes authority and readiness; it does not summarize or replace source content."
  (global_sources + list(slice.fetch("Required specs")) + list(slice.fetch("Required ADRs"))).uniq.each do |path|
    print_source_index(path)
  end
else
  fail!("Unknown command '#{ARGV[0]}'. Use check, list, or context.")
end
