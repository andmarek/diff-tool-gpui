use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, Context, SharedString, Window,
    WindowBounds, WindowOptions,
};
use similar::{ChangeTag, TextDiff};
use std::env;
use std::fs;
use std::process::Command;

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
    fn from_contents(old_path: &str, new_path: &str, old_content: &str, new_content: &str) -> Self {
        let diff = TextDiff::from_lines(old_content, new_content);
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

    fn from_files(old_path: &str, new_path: &str) -> Self {
        let old_content =
            fs::read_to_string(old_path).unwrap_or_else(|e| format!("Error reading file: {e}"));
        let new_content =
            fs::read_to_string(new_path).unwrap_or_else(|e| format!("Error reading file: {e}"));
        Self::from_contents(old_path, new_path, &old_content, &new_content)
    }
}

fn git_toplevel() -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    if !output.status.success() {
        return Err("Not a git repository".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_diff_files(staged: bool) -> Result<Vec<FileDiff>, String> {
    let toplevel = git_toplevel()?;

    let mut args = vec!["diff", "--name-only"];
    if staged {
        args.push("--cached");
    }

    let output = Command::new("git")
        .args(&args)
        .current_dir(&toplevel)
        .output()
        .map_err(|e| format!("Failed to run git diff: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git diff failed: {stderr}"));
    }

    let file_list = String::from_utf8_lossy(&output.stdout);
    let files: Vec<&str> = file_list.lines().filter(|l| !l.is_empty()).collect();

    if files.is_empty() {
        let kind = if staged { "staged" } else { "unstaged" };
        return Err(format!("No {kind} changes found"));
    }

    let mut diffs = Vec::new();
    for file in files {
        let mut show_args = vec!["show".to_string()];
        let ref_prefix = if staged { "" } else { "" };
        show_args.push(format!(":{ref_prefix}{file}"));

        let old_output = Command::new("git")
            .args(&show_args)
            .current_dir(&toplevel)
            .output()
            .map_err(|e| format!("Failed to get index version of {file}: {e}"))?;

        let old_content = if old_output.status.success() {
            String::from_utf8_lossy(&old_output.stdout).to_string()
        } else {
            String::new()
        };

        let file_path = format!("{toplevel}/{file}");
        let new_content = if staged {
            let staged_output = Command::new("git")
                .args(["show", &format!(":{file}")])
                .current_dir(&toplevel)
                .output()
                .map_err(|e| format!("Failed to get staged version of {file}: {e}"))?;
            String::from_utf8_lossy(&staged_output.stdout).to_string()
        } else {
            fs::read_to_string(&file_path).unwrap_or_default()
        };

        diffs.push(FileDiff::from_contents(file, file, &old_content, &new_content));
    }

    Ok(diffs)
}

struct DiffViewer {
    diffs: Vec<FileDiff>,
}

impl DiffViewer {
    fn from_file_pairs(file_pairs: Vec<(String, String)>) -> Self {
        let diffs = file_pairs
            .iter()
            .map(|(old, new)| FileDiff::from_files(old, new))
            .collect();
        Self { diffs }
    }

    fn from_diffs(diffs: Vec<FileDiff>) -> Self {
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

enum Mode {
    FilePairs(Vec<(String, String)>),
    Git { staged: bool },
}

fn parse_args() -> Mode {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("  gpui-diff-tool --git            Show unstaged git changes");
        eprintln!("  gpui-diff-tool --git --staged    Show staged git changes");
        eprintln!("  gpui-diff-tool <old> <new> ...   Diff file pairs");
        std::process::exit(1);
    }

    if args.iter().any(|a| a == "--git") {
        let staged = args.iter().any(|a| a == "--staged");
        return Mode::Git { staged };
    }

    if args.len() < 3 || args.len() % 2 == 0 {
        eprintln!("Usage: gpui-diff-tool <old-file> <new-file> [<old-file2> <new-file2> ...]");
        std::process::exit(1);
    }

    let mut pairs = Vec::new();
    let mut i = 1;
    while i + 1 < args.len() {
        pairs.push((args[i].clone(), args[i + 1].clone()));
        i += 2;
    }
    Mode::FilePairs(pairs)
}

fn main() {
    let mode = parse_args();

    let viewer = match mode {
        Mode::FilePairs(pairs) => DiffViewer::from_file_pairs(pairs),
        Mode::Git { staged } => match git_diff_files(staged) {
            Ok(diffs) => DiffViewer::from_diffs(diffs),
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        },
    };

    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(900.0), px(700.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| viewer),
        )
        .unwrap();
    });
}
