# gpui-diff-tool

A native diff viewer built with [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui) (Zed's UI framework).

## Build

```
cargo build
```

## Usage

### Git diff (unstaged changes)

```
cargo run -- --git
```

### Git diff (staged changes)

```
cargo run -- --git --staged
```

### Diff specific file pairs

```
cargo run -- <old-file> <new-file>
cargo run -- old1.txt new1.txt old2.txt new2.txt
```

Each file diff is shown as an inline unified diff with colored additions (green) and deletions (red), stacked vertically in a single scrollable window.
