# gpui-diff-tool

A native diff viewer built with [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui) (Zed's UI framework).

## Build

```
cargo build
```

## Usage

Diff two files:

```
cargo run -- <old-file> <new-file>
```

Diff multiple file pairs:

```
cargo run -- old1.txt new1.txt old2.txt new2.txt
```

Each pair is shown as an inline unified diff with colored additions (green) and deletions (red), stacked vertically in a single scrollable window.
