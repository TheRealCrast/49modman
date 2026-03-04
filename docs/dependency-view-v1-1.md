# Dependency View v1.1

## Summary

Rework the dependency inspection flow so it is easier for casual users to read and significantly cheaper to resolve.

This is the next planned pass after the first implemented dependency modal.

Locked decisions:

- the modal should default to a deduplicated `Summary` view
- the recursive `Tree` view should remain available as an advanced inspection mode
- deduplication identity is the exact package version, not package-only
- repeated nodes in the advanced tree should collapse into lightweight `Already shown above` reference rows
- backend optimization should stay within the current schema for now
- resolver optimization should use request-local in-memory indexes and memoized traversal, not a new persistent dependency edge table yet

## Current Implemented State

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
2. `Transitive dependencies`
3. `Unresolved`

Rules:

- deduplicate resolved dependencies by exact `(package_id, version_id)`
- deduplicate unresolved dependencies by exact raw dependency string
- classify a resolved dependency as `Direct` if minimum depth is `1`
- classify a resolved dependency as `Transitive` if minimum depth is `2+`
- if the same package appears in two different exact versions, keep them as separate entries

Resolved summary rows should show:

- package name
- exact version
- compatibility/status pill
- optional local reference note
- short helper copy indicating whether the dependency is direct or transitive

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

## Backend Optimization Plan

### Key Change

Stop doing one SQLite query per dependency edge.

Instead, each request should:

1. load a dependency-capable catalog snapshot with one batched query
2. build request-local indexes in memory
3. traverse the graph using those indexes
4. memoize traversal results within the request

### Request-Local Indexes

Recommended internal indexes:

- `versions_by_id`
- `version_id_by_dependency_raw`

The second map should use keys in this form:

- `{full_name}-{version_number}`

That preserves exact-version resolution without live string-concatenation lookups in SQLite.

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

Do not add a schema migration in this pass.

If performance is still unacceptable after request-local indexing and memoization, that follow-up can become a later milestone with a normalized dependency edge table.

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
2. summary deduplicates resolved dependencies by exact version
3. unresolved dependencies remain visible in both summary and tree modes
4. advanced tree still supports exact dependency-path inspection
5. repeated exact-version nodes in tree mode no longer render full duplicate subtrees
6. repeated exact-version nodes are clearly labeled
7. jump-to-exact-version still works from the modal
8. backend resolution no longer performs one SQLite lookup per dependency edge
9. one dependency request expands a given exact version at most once in tree construction
10. heavy dependency queries no longer peg one worker thread for long periods in normal use

## Validation

### Backend Tests

Cover:

- exact dependency resolution from request-local index
- unresolved dependency handling
- cycle detection
- repeated-node compaction
- summary deduplication by exact version
- same package with different versions remaining split in summary
- direct vs transitive classification by minimum depth
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

If request-local indexing and memoization still are not enough:

- add a normalized dependency edge table during catalog sync
- query dependency graphs from normalized rows instead of on-demand `dependencies_json` parsing
