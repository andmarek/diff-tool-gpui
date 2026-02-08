# Side-by-Side Diff View

## Current State

The app renders a **unified diff** — one column of lines with `+`/`-`/` ` markers, old and new line numbers in a shared gutter. It has a resizable file panel on the right and a scrollable diff content area on the left. Everything lives in `src/main.rs`.

## Step 1: Split into modules

Before adding new features, break `main.rs` into modules to keep things manageable.

- `src/diff.rs` — `DiffLine`, `FileDiff`, and the new `SideBySideLine` + conversion logic
- `src/git.rs` — `git_toplevel()`, `git_diff_files()`
- `src/viewer.rs` — `DiffViewer`, `PanelResizeDrag`, `ViewMode`, all rendering
- `src/main.rs` — `parse_args()`, `Mode`, `main()`

No behavior changes, just reorganization.

## Step 2: Add `SideBySideLine` data model

```rust
struct SideBySideLine {
    left: Option<DiffLine>,   // old file line (or None if this row is an insert)
    right: Option<DiffLine>,  // new file line (or None if this row is a delete)
}
```

Add a conversion function:

```rust
fn to_side_by_side(lines: &[DiffLine]) -> Vec<SideBySideLine>
```

**Alignment logic:**
1. Walk through `lines` sequentially.
2. Collect consecutive `Delete` lines into a buffer.
3. When an `Insert` is encountered and the delete buffer is non-empty, pair them: one delete + one insert per `SideBySideLine`. If counts differ, the shorter side gets `None` entries.
4. `Equal` lines flush any remaining delete buffer (as left-only rows), then emit a row with both sides populated.

## Step 3: Add `ViewMode` enum and toggle

```rust
enum ViewMode {
    Unified,
    SideBySide,
}
```

- Add `view_mode: ViewMode` field to `DiffViewer`.
- In `Render::render()`, dispatch to `render_file_diff()` or `render_side_by_side_diff()` based on `self.view_mode`.

## Step 4: Render side-by-side view

### Layout

```
┌──────────────────────────────────────────────────────┬───┬────────────┐
│ [Unified] [Side-by-Side]              filename.rs    │   │ FILES (3)  │
├────────────────────────┬─┬───────────────────────────┤ ↕ │            │
│  10 │ old line content  │ │  10 │ new line content    │   │ file1.rs   │
│  11 │ deleted line      │ │     │                     │   │ file2.rs   │
│     │                   │ │  11 │ inserted line       │   │ file3.rs   │
│  12 │ equal line        │ │  12 │ equal line          │   │            │
└────────────────────────┴─┴───────────────────────────┘   └────────────┘
         left pane        div         right pane         drag   panel
```

### `render_side_by_side_diff()`

Each row is a single flex-row containing:
1. **Left gutter** — old line number, right-aligned, fixed width
2. **Left content** — old file text, flex-grow, horizontal overflow scroll
3. **Center divider** — 1px vertical border
4. **Right gutter** — new line number, right-aligned, fixed width
5. **Right content** — new file text, flex-grow, horizontal overflow scroll

### Colors

| Row type       | Left bg      | Right bg     |
|---------------|-------------|-------------|
| Equal          | `0x1e1e1e`   | `0x1e1e1e`   |
| Delete (left)  | `0x3d1117`   | `0x262626` (dimmed) |
| Insert (right) | `0x262626` (dimmed) | `0x1b2e1b`   |

### Scrolling

Both panes are children of the same scrollable container — one row per `SideBySideLine`. Vertical scroll is inherently synchronized since they share the same parent. Each side's content area can scroll horizontally independently.

## Step 5: Add toolbar with view mode toggle

Add a small bar above the diff content (inside the diff area, below the file header):

- Two buttons: **Unified** | **Side-by-Side**
- Active button gets a highlight/accent color (`0x007acc`)
- Clicking toggles `self.view_mode`

## Step 6: Polish and follow-ups

These are not part of the initial implementation but worth noting:

- **Intra-line highlighting**: highlight changed characters within modified lines (word-level diff using `similar`'s word diffing)
- **Collapse equal regions**: hide long stretches of unchanged lines, show "N lines hidden" expander
- **Keyboard shortcut**: toggle view mode with a key binding (e.g., `t`)
- **Persist preference**: remember last-used view mode across sessions

## Implementation Order

1. Step 1 (module split) — mechanical refactor, verify `cargo run` still works
2. Step 2 (data model) — add `SideBySideLine` + conversion, write unit tests
3. Steps 3–5 (UI) — add `ViewMode`, render function, toggle button
4. Step 6 (polish) — optional follow-ups
