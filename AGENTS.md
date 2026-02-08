# AGENTS.md

## About

A native diff viewer built with GPUI (Zed's UI framework). Currently supports opening and viewing a single file with line numbers and vertical scrolling. The goal is to evolve this into a full side-by-side diff viewer.

When you add or change a major feature, update this section to reflect the current capabilities. Keep it to a few sentences.

## Build

```
cargo check
cargo run
```

## GPUI Setup


GPUI cannot be used as a simple crates.io dependency. It must be pulled from the Zed git repo with specific `[patch.crates-io]` entries to resolve a `core-graphics` version conflict in `zed-font-kit`.

Required patches in `Cargo.toml`:

```toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed", rev = "83ca31055cf3e56aa8a704ac49e1686434f4e640" }

[patch.crates-io]
core-text = { git = "https://github.com/servo/core-foundation-rs", rev = "b10f1efc48343fc5590127ec6d890ffbb8b5bd02" }
core-graphics = { git = "https://github.com/servo/core-foundation-rs", rev = "b10f1efc48343fc5590127ec6d890ffbb8b5bd02" }
core-graphics-types = { git = "https://github.com/servo/core-foundation-rs", rev = "b10f1efc48343fc5590127ec6d890ffbb8b5bd02" }
core-foundation = { git = "https://github.com/servo/core-foundation-rs", rev = "b10f1efc48343fc5590127ec6d890ffbb8b5bd02" }
core-foundation-sys = { git = "https://github.com/servo/core-foundation-rs", rev = "b10f1efc48343fc5590127ec6d890ffbb8b5bd02" }
```

The `core-foundation-rs` rev must have versions compatible with the gpui rev's pinned `core-foundation = "=0.10.0"`. If updating the gpui rev, check whether the `core-foundation` pin has changed and find a matching `core-foundation-rs` rev.

## GPUI Docs

- API docs: https://github.com/zed-industries/zed/tree/main/crates/gpui
- Examples: https://github.com/zed-industries/zed/tree/main/crates/gpui/examples
- Hello world: https://www.gpui.rs/
- A list of example apps: https://github.com/zed-industries/awesome-gpui/blob/main/README.md
