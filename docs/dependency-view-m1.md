# Dependency View Milestone 1

## Summary

Add a dependency inspection flow from the Browse version context menu.

For this milestone:

- the version context menu gains `View dependencies`
- selecting it opens a modal dialog
- the modal shows a recursive dependency tree for the chosen exact version
- dependency resolution is performed against the local cached catalog only
- resolved dependency rows can jump to the exact dependency version in Browse
- unresolved dependencies stay visible as raw metadata

This milestone is inspection and navigation only.

It does not yet change install behavior, profile mutation, or dependency auto-install.

## Locked Product Decisions

- entry point: version context menu in Browse package detail
- surface: modal dialog
- depth: recursive dependency tree
- default expansion:
  - first-level dependencies expanded
  - deeper levels collapsible
- action on resolved dependency click:
  - close modal
  - switch to Browse if needed
  - open dependency package detail
  - scroll to the exact required version row
  - temporarily highlight that version row
- unresolved dependency handling:
  - keep it visible
  - show the raw dependency string
  - mark it unresolved
  - no click action
- cycle handling:
  - mark the node as a cycle
  - stop recursion below that node
- data source: local SQLite catalog only

## Goals

- let users inspect exact-version dependency requirements before install
- make dependency chains understandable without overloading the existing context menu
- preserve trust by showing unresolved metadata rather than hiding it
- make resolved dependencies actionable by jumping to the exact referenced version

## Non-Goals

- installing dependencies automatically
- changing package/profile install state
- adding a graph view or dedicated dependency screen
- fetching missing dependency data from the network
- editing dependency metadata

## Backend Design

Add a dedicated dependency command instead of inflating `get_package_detail`.

Recommended additions:

- `src-tauri/src/commands/dependencies.rs`
- `src-tauri/src/services/dependency_service.rs`

New command:

- `get_version_dependencies`

Input:

- `packageId`
- `versionId`

Output root object:

- root package id
- root package name
- root version id
- root version number
- `items` tree

Per dependency node:

- raw dependency string
- resolved package id if available
- resolved package name if available
- resolved version id if available
- resolved version number if available
- effective status if resolved
- reference note if resolved and locally meaningful
- resolution kind:
  - `resolved`
  - `unresolved`
  - `cycle`
- recursive `children`

## Resolution Rules

Thunderstore dependency strings should be parsed as exact package/version references.

Resolution order:

1. parse raw dependency string
2. normalize package id using the same package-id logic already used by catalog persistence
3. normalize version id using the same version-id logic already used by catalog persistence
4. attempt exact local package/version lookup
5. if found:
   - mark resolved
   - attach effective status
   - attach local reference note if available
   - recurse into that version's dependencies
6. if not found:
   - mark unresolved
   - keep raw string
7. if recursion revisits a package/version already in the current ancestry path:
   - mark cycle
   - do not recurse further

If one dependency entry is malformed, degrade just that node to unresolved rather than failing the whole tree.

## Frontend Design

Add dependency DTOs and API plumbing in:

- `src/lib/types.ts`
- `src/lib/api/dependencies.ts`
- `src/lib/api/client.ts`
- `src/lib/api/mock-backend.ts`

Add store state for:

- dependency modal loading/data/error state
- currently focused dependency target version in Browse

Recommended store actions:

- `openDependencyModal`
- `closeDependencyModal`
- `jumpToDependency`

## Browse / Package Detail UX

In `src/components/PackageDetail.svelte`:

- add `View dependencies` near the top of the existing version context menu
- keep the reference actions below it
- pass the selected version context into the new dependency action

Add a new modal component:

- `src/components/DependencyModal.svelte`

The modal should show:

- title with package and version
- helper copy that the tree comes from the cached local catalog
- loading state
- error state
- empty state for versions with no dependencies
- recursive dependency rows

Each resolved row should show:

- package name
- exact version number
- compatibility/status pill
- optional note when locally verified/broken

Each unresolved row should show:

- raw dependency string
- unresolved marker

## Jump-To-Version Behavior

Resolved dependency click should:

1. close the dependency modal
2. switch to Browse
3. select the dependency package
4. load that package detail if needed
5. scroll the exact target version row into view
6. apply a temporary highlight

Highlight behavior:

- one highlighted version at a time
- lasts about 2 seconds
- obvious but lightweight

## Acceptance Criteria

This milestone is complete when:

1. A version context menu item named `View dependencies` exists.
2. Selecting it opens a modal dialog.
3. The modal shows a recursive dependency tree for the chosen exact version.
4. Resolved nodes display exact version and compatibility state.
5. Unresolved nodes remain visible with raw dependency strings.
6. Cycles do not recurse forever and are marked clearly.
7. Clicking a resolved node jumps to the exact dependency version in Browse.
8. The target version row scrolls into view and highlights temporarily.
9. Existing context-menu reference actions still work.
10. Existing install/cache flows remain unchanged.

## Validation

Backend/unit coverage should verify:

- dependency string parsing
- exact package/version resolution
- unresolved package/version cases
- recursive traversal
- cycle detection
- resolved effective-status mapping

Manual validation should cover:

- version with no dependencies
- version with direct dependencies
- version with transitive dependencies
- unresolved dependency entry
- cycle case
- jump-to-version flow
- modal load/error state
- context-menu regression checks
