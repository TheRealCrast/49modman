# Dependency View v1.1

## Summary

Rework the dependency inspection flow so it is easier for casual users to read and significantly cheaper to resolve.

This pass is now implemented.

Locked decisions:

- the modal should default to a deduplicated `Summary` view
- the recursive `Tree` view should remain available as an advanced inspection mode
- Summary deduplication identity is package-level, not exact-version-level
- for each package in Summary, the highest required version is used as the primary row target
- lower required versions for that package remain visible as strikethrough metadata in the row
- repeated nodes in the advanced tree should collapse into lightweight `Already shown above` reference rows
- backend optimization should stay within the current schema for now
- resolver optimization should use an app-lifetime in-memory index cache with warm/invalidate flow, not a new persistent dependency edge table yet

## Implementation Status (2026-03-04)

Implemented behavior:

- dependency modal opens on `Summary` by default and supports `Summary` / `Tree` switching
- backend command `get_version_dependencies` now resolves from a cached app-lifetime in-memory index
- backend command `warm_dependency_index` now prewarms that in-memory index
- dependency index cache is invalidated on catalog/reference/reset mutations
- dependency-array JSON is parsed lazily for visited nodes during traversal
- response shape is summary + compact tree
- Summary sections are `Direct`, `Indirect`, and `Unresolved`
- Summary resolved rows are deduplicated by package:
  - highest required version shown as the row target
  - lower required versions shown with strikethrough in the same row
- repeated exact-version tree nodes are marked `repeated` and do not expand duplicate subtrees
- Tree stays exact-version for inspection/jump behavior
- lower-version rows in Tree are visually de-emphasized:
  - italic title
  - strikethrough + italic version
  - muted grey row background
- unresolved rows remain visible in both Summary and Tree
- jump-to-exact-version still closes modal, opens Browse, and highlights the target row
- startup overlay now includes dependency warmup so the first dependency modal open avoids cold index build cost
- existing-catalog startup now performs a non-force catalog freshness check (`sync_catalog` with `force=false`) before dismissing the startup overlay
- post-overlay background refresh was removed from cached-catalog startup to avoid a confusing second refresh phase after the app becomes visible

## Original Plan Context

The existing dependency view already provides:

- `View dependencies` in the version context menu
- a dependency modal
- recursive local-catalog dependency resolution
- unresolved dependency visibility
- cycle detection
- jump-to-version navigation back into Browse
- temporary exact-version row highlight after jump

Current shortcomings:

- duplicate exact-version nodes can appear multiple times in the recursive tree
- the default tree-heavy presentation is not ideal for casual users
- backend resolution currently does repeated per-node SQLite work
- heavy dependency requests can peg one blocking worker thread for a noticeable period

## Goals

- make the default dependency modal easier to understand at a glance
- preserve exact-version fidelity
- remove duplicate-heavy tree output from the primary UX
- eliminate repeated per-node SQLite dependency lookups during one request
- keep the existing jump-to-exact-version behavior

## Non-Goals

- no install behavior changes
- no automatic dependency installation
- no remote fetch fallback for missing dependencies
- no schema migration in this pass
- no graph visualization

## Product Decisions

### Modal Views

The dependency modal should have two internal tabs or segmented views:

1. `Summary`
2. `Tree`

Default view:

- `Summary`

### Summary View

Show three sections:

1. `Direct dependencies`
2. `Indirect dependencies`
3. `Unresolved`

Rules:

- deduplicate resolved dependencies by `package_id`
- choose the highest required version per package as the primary row target
- keep lower required versions in that same row as strikethrough metadata
- deduplicate unresolved dependencies by exact raw dependency string
- classify a resolved dependency as `Direct` if minimum depth is `1`
- classify a resolved dependency as `Indirect` if minimum depth is `2+`

Resolved summary rows should show:

- package name
- selected highest required version
- lower required versions in strikethrough when present
- compatibility/status pill
- optional local reference note
- short helper copy indicating whether the dependency is direct or indirect

Unresolved summary rows should show:

- raw dependency string
- unresolved marker
- no click action

### Tree View

Keep the recursive tree for advanced inspection, but compact duplicate branches.

Rules:

- first-level nodes expanded by default
- deeper nodes collapsed by default
- unresolved nodes remain visible
- cycle nodes remain visible and stop recursion
- repeated exact-version nodes should not render their subtree again
- repeated exact-version nodes should render as lightweight reference rows with text such as:
  - `Already shown above`

Deduplication identity for repeats:

- exact `(package_id, version_id)`

### Jump Behavior

For resolved summary rows and resolved/repeated tree rows:

1. close the modal
2. switch to Browse
3. select the dependency package
4. load its package detail if needed
5. scroll the exact target version into view
6. highlight that version row temporarily

## Backend Design

Keep the command name:

- `get_version_dependencies`

Change the result shape from tree-only to summary + tree.

Recommended root DTO:

- root package id
- root package name
- root version id
- root version number
- `summary`
- `treeItems`

Recommended summary payload:

- `direct`
- `transitive`
- `unresolved`

Tree node resolution kinds should become:

- `resolved`
- `unresolved`
- `cycle`
- `repeated`

## Backend Optimization Status

### Key Change

Stop rebuilding dependency lookup structures for every dependency modal request.

Implemented approach:

1. build one dependency-capable catalog snapshot in memory
2. keep it in an app-lifetime cache
3. warm it during startup overlay
4. invalidate and rebuild after dependency-relevant mutations

### Request-Local Indexes

Current internal indexes:

- `versions_by_id`
- `version_id_by_dependency_raw`

Dependency raw lookup keys stay in this form:

- `{full_name}-{version_number}`

This preserves exact-version resolution without live string-concatenation lookups in SQLite.

Version dependency arrays now use lazy parsing per record so cold warmup avoids eagerly deserializing every `dependencies_json` row.

### Traversal Split

Use two passes:

1. summary collection pass
2. tree construction pass

Summary pass responsibilities:

- collect unique resolved exact-version dependencies
- collect unresolved raw strings
- record minimum depth
- avoid re-traversing already visited resolved nodes

Tree pass responsibilities:

- preserve ancestry-aware cycle detection
- expand each exact version at most once in the rendered tree
- mark later occurrences as `repeated`

### Important Constraint

No schema migration was added in this pass.

If performance remains unacceptable after app-lifetime cache + warm/invalidate + lazy parsing, the next follow-up can be a normalized dependency edge table populated at sync time.

## Frontend Design

Update:

- `src/lib/types.ts`
- `src/lib/api/dependencies.ts`
- `src/lib/store.ts`
- `src/components/DependencyModal.svelte`
- `src/components/DependencyTreeNode.svelte`

Frontend work should include:

- summary/tree view switcher in the modal
- summary section rendering
- repeated-node rendering in tree mode
- updated DTO handling

The existing entry point should stay the same:

- `View dependencies` in the version context menu

## Acceptance Criteria

This pass is complete when:

1. dependency modal opens on `Summary` by default
2. summary deduplicates resolved dependencies by package
3. summary rows choose the highest required version per package
4. summary rows preserve lower required versions as strikethrough metadata
5. unresolved dependencies remain visible in both summary and tree modes
6. advanced tree still supports exact dependency-path inspection
7. repeated exact-version nodes in tree mode no longer render full duplicate subtrees
8. repeated exact-version nodes are clearly labeled
9. jump-to-exact-version still works from the modal
10. backend resolution no longer performs repeated per-request full-catalog index rebuild work for each modal open
11. one dependency request expands a given exact version at most once in tree construction
12. first dependency open after startup avoids cold index build in normal use

## Validation

### Backend Tests

Cover:

- exact dependency resolution from the warmed app-lifetime in-memory index
- unresolved dependency handling
- cycle detection
- repeated-node compaction
- summary deduplication by package with highest-version selection
- lower required versions retained in summary row metadata
- direct vs indirect classification by minimum depth
- malformed dependency entries degrading to unresolved rather than failing the response

### Manual Validation

Check:

- version with zero dependencies
- version with one direct dependency
- version with shared transitive dependency
- version with unresolved dependency entry
- version with a cycle
- graph containing two exact versions of the same package
- summary click-to-jump flow
- repeated tree node click-to-jump flow
- performance against a known heavy dependency package

## Follow-Up If Needed

If app-lifetime cache warm/invalidate and lazy parsing still are not enough:

- add a normalized dependency edge table during catalog sync
- query dependency graphs from normalized rows instead of on-demand `dependencies_json` parsing
