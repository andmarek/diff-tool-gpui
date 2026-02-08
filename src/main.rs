use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, Context, SharedString, Window,
    WindowBounds, WindowOptions,
};
use std::env;
use std::fs;

struct FileViewer {
    filename: SharedString,
    lines: Vec<SharedString>,
}

impl FileViewer {
    fn new(path: &str) -> Self {
        let content = fs::read_to_string(path).unwrap_or_else(|e| format!("Error reading file: {e}"));
        let lines: Vec<SharedString> = content.lines().map(|l| SharedString::from(l.to_string())).collect();
        Self {
            filename: SharedString::from(path.to_string()),
            lines,
        }
    }
}

impl Render for FileViewer {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let line_count = self.lines.len();
        let gutter_width = format!("{}", line_count).len() as f32 * 8.0 + 16.0;

        let mut content = div().flex().flex_col().w_full();

        for (i, line) in self.lines.iter().enumerate() {
            let line_num = i + 1;
            content = content.child(
                div()
                    .flex()
                    .flex_row()
                    .w_full()
                    .child(
                        div()
                            .w(px(gutter_width))
                            .flex_shrink_0()
                            .text_right()
                            .pr(px(8.0))
                            .text_color(rgb(0x888888))
                            .child(format!("{line_num}")),
                    )
                    .child(
                        div()
                            .pl(px(8.0))
                            .flex_grow()
                            .child(line.clone()),
                    ),
            );
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
                    .w_full()
                    .px(px(12.0))
                    .py(px(6.0))
                    .bg(rgb(0x2d2d2d))
                    .border_b_1()
                    .border_color(rgb(0x404040))
                    .text_size(px(12.0))
                    .text_color(rgb(0xcccccc))
                    .child(self.filename.clone()),
            )
            .child(
                div()
                    .id("file-content")
                    .flex_grow()
                    .overflow_y_scroll()
                    .p(px(4.0))
                    .child(content),
            )
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: gpui-diff-tool <file>");
        std::process::exit(1);
    }
    let path = args[1].clone();

    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| FileViewer::new(&path)),
        )
        .unwrap();
    });
}
