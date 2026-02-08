use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, Context, SharedString, Window,
    WindowBounds, WindowOptions,
};
use similar::{ChangeTag, TextDiff};
use std::env;
use std::fs;

#[derive(Clone)]
struct DiffLine {
    tag: ChangeTag,
    old_lineno: Option<usize>,
    new_lineno: Option<usize>,
    content: SharedString,
}

struct FileDiff {
    old_path: SharedString,
    new_path: SharedString,
    lines: Vec<DiffLine>,
}

impl FileDiff {
    fn compute(old_path: &str, new_path: &str) -> Self {
        let old_content =
            fs::read_to_string(old_path).unwrap_or_else(|e| format!("Error reading file: {e}"));
        let new_content =
            fs::read_to_string(new_path).unwrap_or_else(|e| format!("Error reading file: {e}"));

        let diff = TextDiff::from_lines(&old_content, &new_content);
        let mut lines = Vec::new();
        let mut old_lineno = 0usize;
        let mut new_lineno = 0usize;

        for change in diff.iter_all_changes() {
            let tag = change.tag();
            let (old_ln, new_ln) = match tag {
                ChangeTag::Equal => {
                    old_lineno += 1;
                    new_lineno += 1;
                    (Some(old_lineno), Some(new_lineno))
                }
                ChangeTag::Delete => {
                    old_lineno += 1;
                    (Some(old_lineno), None)
                }
                ChangeTag::Insert => {
                    new_lineno += 1;
                    (None, Some(new_lineno))
                }
            };

            let text = change.to_string_lossy();
            let text = text.trim_end_matches('\n');
            lines.push(DiffLine {
                tag,
                old_lineno: old_ln,
                new_lineno: new_ln,
                content: SharedString::from(text.to_string()),
            });
        }

        Self {
            old_path: SharedString::from(old_path.to_string()),
            new_path: SharedString::from(new_path.to_string()),
            lines,
        }
    }
}

struct DiffViewer {
    diffs: Vec<FileDiff>,
}

impl DiffViewer {
    fn new(file_pairs: Vec<(String, String)>) -> Self {
        let diffs = file_pairs
            .iter()
            .map(|(old, new)| FileDiff::compute(old, new))
            .collect();
        Self { diffs }
    }

    fn render_diff_line(&self, line: &DiffLine, gutter_width: f32) -> impl IntoElement {
        let (bg, text_color, sign) = match line.tag {
            ChangeTag::Delete => (rgb(0x3d1117), rgb(0xffa7a7), "-"),
            ChangeTag::Insert => (rgb(0x1b2e1b), rgb(0xa7ffa7), "+"),
            ChangeTag::Equal => (rgb(0x1e1e1e), rgb(0xd4d4d4), " "),
        };

        let old_ln = line
            .old_lineno
            .map(|n| format!("{n}"))
            .unwrap_or_default();
        let new_ln = line
            .new_lineno
            .map(|n| format!("{n}"))
            .unwrap_or_default();

        div()
            .flex()
            .flex_row()
            .w_full()
            .bg(bg)
            .child(
                div()
                    .w(px(gutter_width))
                    .flex_shrink_0()
                    .text_right()
                    .pr(px(4.0))
                    .text_color(rgb(0x666666))
                    .child(old_ln),
            )
            .child(
                div()
                    .w(px(gutter_width))
                    .flex_shrink_0()
                    .text_right()
                    .pr(px(4.0))
                    .text_color(rgb(0x666666))
                    .child(new_ln),
            )
            .child(
                div()
                    .w(px(16.0))
                    .flex_shrink_0()
                    .text_center()
                    .text_color(text_color)
                    .child(sign),
            )
            .child(
                div()
                    .pl(px(4.0))
                    .flex_grow()
                    .text_color(text_color)
                    .child(line.content.clone()),
            )
    }

    fn render_file_diff(&self, diff: &FileDiff) -> impl IntoElement {
        let max_lineno = diff.lines.iter().fold(0usize, |acc, l| {
            acc.max(l.old_lineno.unwrap_or(0))
                .max(l.new_lineno.unwrap_or(0))
        });
        let gutter_width = format!("{max_lineno}").len() as f32 * 8.0 + 12.0;

        let header_text = if diff.old_path == diff.new_path {
            diff.old_path.clone()
        } else {
            SharedString::from(format!("{} → {}", diff.old_path, diff.new_path))
        };

        let mut content = div().flex().flex_col().w_full();
        for line in &diff.lines {
            content = content.child(self.render_diff_line(line, gutter_width));
        }

        div()
            .flex()
            .flex_col()
            .w_full()
            .mb(px(16.0))
            .child(
                div()
                    .w_full()
                    .px(px(12.0))
                    .py(px(6.0))
                    .bg(rgb(0x2d2d2d))
                    .border_b_1()
                    .border_color(rgb(0x404040))
                    .text_size(px(12.0))
                    .text_color(rgb(0xcccccc))
                    .child(header_text),
            )
            .child(
                div()
                    .w_full()
                    .p(px(4.0))
                    .child(content),
            )
    }
}

impl Render for DiffViewer {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut container = div().flex().flex_col().w_full();
        for diff in &self.diffs {
            container = container.child(self.render_file_diff(diff));
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xd4d4d4))
            .font_family("Menlo")
            .text_size(px(13.0))
            .child(
                div()
                    .id("diff-content")
                    .flex_grow()
                    .overflow_y_scroll()
                    .child(container),
            )
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 || args.len() % 2 == 0 {
        eprintln!("Usage: gpui-diff-tool <old-file> <new-file> [<old-file2> <new-file2> ...]");
        std::process::exit(1);
    }

    let mut file_pairs = Vec::new();
    let mut i = 1;
    while i + 1 < args.len() {
        file_pairs.push((args[i].clone(), args[i + 1].clone()));
        i += 2;
    }

    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(900.0), px(700.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| DiffViewer::new(file_pairs.clone())),
        )
        .unwrap();
    });
}
