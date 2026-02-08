mod diff;
mod git;
mod viewer;

use gpui::{px, size, App, AppContext, Application, Bounds, WindowBounds, WindowOptions};
use std::env;

use crate::git::git_diff_files;
use crate::viewer::DiffViewer;

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
