use std::collections::{BTreeMap, HashSet};

use gpui::{
    div, prelude::*, px, rgb, CursorStyle, ElementId, Pixels, SharedString, Window,
    Context,
};
use similar::ChangeTag;

use crate::diff::{to_side_by_side, DiffLine, FileDiff, SideBySideLine};

pub const MIN_PANEL_WIDTH: f32 = 100.0;
pub const MAX_PANEL_WIDTH: f32 = 600.0;
pub const DEFAULT_PANEL_WIDTH: f32 = 220.0;
pub const DRAG_HANDLE_WIDTH: f32 = 4.0;

#[derive(Clone, Copy, PartialEq)]
pub enum ViewMode {
    Unified,
    SideBySide,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PanelMode {
    List,
    Tree,
}

pub struct PanelResizeDrag {
    pub initial_width: Pixels,
}

impl Render for PanelResizeDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Debug)]
enum TreeNode {
    Directory {
        name: String,
        children: BTreeMap<String, TreeNode>,
    },
    File {
        diff_index: usize,
    },
}

fn insert_into_tree(root: &mut BTreeMap<String, TreeNode>, parts: &[&str], diff_index: usize) {
    if parts.is_empty() {
        return;
    }
    if parts.len() == 1 {
        root.insert(
            parts[0].to_string(),
            TreeNode::File { diff_index },
        );
        return;
    }
    let dir = root
        .entry(parts[0].to_string())
        .or_insert_with(|| TreeNode::Directory {
            name: parts[0].to_string(),
            children: BTreeMap::new(),
        });
    if let TreeNode::Directory { children, .. } = dir {
        insert_into_tree(children, &parts[1..], diff_index);
    }
}

fn build_file_tree(diffs: &[FileDiff]) -> BTreeMap<String, TreeNode> {
    let mut root: BTreeMap<String, TreeNode> = BTreeMap::new();

    for (i, diff) in diffs.iter().enumerate() {
        let path = diff.new_path.to_string();
        let parts: Vec<&str> = path.split('/').collect();
        insert_into_tree(&mut root, &parts, i);
    }

    root
}

pub struct DiffViewer {
    pub diffs: Vec<FileDiff>,
    pub selected_index: Option<usize>,
    pub panel_width: Pixels,
    pub view_mode: ViewMode,
    pub panel_mode: PanelMode,
    pub collapsed_dirs: HashSet<String>,
}

impl DiffViewer {
    pub fn from_file_pairs(file_pairs: Vec<(String, String)>) -> Self {
        let diffs: Vec<FileDiff> = file_pairs
            .iter()
            .map(|(old, new)| FileDiff::from_files(old, new))
            .collect();
        let selected = if diffs.is_empty() { None } else { Some(0) };
        Self {
            diffs,
            selected_index: selected,
            panel_width: px(DEFAULT_PANEL_WIDTH),
            view_mode: ViewMode::Unified,
            panel_mode: PanelMode::List,
            collapsed_dirs: HashSet::new(),
        }
    }

    pub fn from_diffs(diffs: Vec<FileDiff>) -> Self {
        let selected = if diffs.is_empty() { None } else { Some(0) };
        Self {
            diffs,
            selected_index: selected,
            panel_width: px(DEFAULT_PANEL_WIDTH),
            view_mode: ViewMode::Unified,
            panel_mode: PanelMode::List,
            collapsed_dirs: HashSet::new(),
        }
    }

    fn file_display_name(diff: &FileDiff) -> SharedString {
        if diff.old_path == diff.new_path {
            diff.old_path.clone()
        } else {
            SharedString::from(format!("{} → {}", diff.old_path, diff.new_path))
        }
    }

    fn render_diff_line(&self, line: &DiffLine, gutter_width: f32) -> impl IntoElement {
        let (bg, text_color, sign) = match line.tag {
            ChangeTag::Delete => (rgb(0x3d1117), rgb(0xffa7a7), "-"),
            ChangeTag::Insert => (rgb(0x1b2e1b), rgb(0xa7ffa7), "+"),
            ChangeTag::Equal => (rgb(0x1e1e1e), rgb(0xd4d4d4), " "),
        };

        let old_ln = line.old_lineno.map(|n| format!("{n}")).unwrap_or_default();
        let new_ln = line.new_lineno.map(|n| format!("{n}")).unwrap_or_default();

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

        let mut content = div().flex().flex_col().w_full();
        for line in &diff.lines {
            content = content.child(self.render_diff_line(line, gutter_width));
        }

        div()
            .flex()
            .flex_col()
            .w_full()
            .mb(px(16.0))
            .child(div().w_full().p(px(4.0)).child(content))
    }

    fn render_side_by_side_line(
        &self,
        sbs_line: &SideBySideLine,
        gutter_width: f32,
    ) -> impl IntoElement {
        let (left_bg, left_text, left_ln, left_content) = match &sbs_line.left {
            Some(line) => {
                let (bg, tc) = match line.tag {
                    ChangeTag::Delete => (rgb(0x3d1117), rgb(0xffa7a7)),
                    ChangeTag::Equal => (rgb(0x1e1e1e), rgb(0xd4d4d4)),
                    _ => (rgb(0x1e1e1e), rgb(0xd4d4d4)),
                };
                let ln = line.old_lineno.map(|n| format!("{n}")).unwrap_or_default();
                (bg, tc, ln, line.content.clone())
            }
            None => (
                rgb(0x262626),
                rgb(0x666666),
                String::new(),
                SharedString::from(""),
            ),
        };

        let (right_bg, right_text, right_ln, right_content) = match &sbs_line.right {
            Some(line) => {
                let (bg, tc) = match line.tag {
                    ChangeTag::Insert => (rgb(0x1b2e1b), rgb(0xa7ffa7)),
                    ChangeTag::Equal => (rgb(0x1e1e1e), rgb(0xd4d4d4)),
                    _ => (rgb(0x1e1e1e), rgb(0xd4d4d4)),
                };
                let ln = line.new_lineno.map(|n| format!("{n}")).unwrap_or_default();
                (bg, tc, ln, line.content.clone())
            }
            None => (
                rgb(0x262626),
                rgb(0x666666),
                String::new(),
                SharedString::from(""),
            ),
        };

        div()
            .flex()
            .flex_row()
            .w_full()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_w(px(0.0))
                    .bg(left_bg)
                    .child(
                        div()
                            .w(px(gutter_width))
                            .flex_shrink_0()
                            .text_right()
                            .pr(px(4.0))
                            .text_color(rgb(0x666666))
                            .child(left_ln),
                    )
                    .child(
                        div()
                            .pl(px(4.0))
                            .flex_grow()
                            .min_w(px(0.0))
                            .overflow_x_hidden()
                            .text_color(left_text)
                            .child(left_content),
                    ),
            )
            .child(
                div()
                    .w(px(1.0))
                    .flex_shrink_0()
                    .bg(rgb(0x404040)),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_w(px(0.0))
                    .bg(right_bg)
                    .child(
                        div()
                            .w(px(gutter_width))
                            .flex_shrink_0()
                            .text_right()
                            .pr(px(4.0))
                            .text_color(rgb(0x666666))
                            .child(right_ln),
                    )
                    .child(
                        div()
                            .pl(px(4.0))
                            .flex_grow()
                            .min_w(px(0.0))
                            .overflow_x_hidden()
                            .text_color(right_text)
                            .child(right_content),
                    ),
            )
    }

    fn render_side_by_side_diff(&self, diff: &FileDiff) -> impl IntoElement {
        let sbs_lines = to_side_by_side(&diff.lines);

        let max_lineno = diff.lines.iter().fold(0usize, |acc, l| {
            acc.max(l.old_lineno.unwrap_or(0))
                .max(l.new_lineno.unwrap_or(0))
        });
        let gutter_width = format!("{max_lineno}").len() as f32 * 8.0 + 12.0;

        let mut content = div().flex().flex_col().w_full();
        for sbs_line in &sbs_lines {
            content = content.child(self.render_side_by_side_line(sbs_line, gutter_width));
        }

        div()
            .flex()
            .flex_col()
            .w_full()
            .mb(px(16.0))
            .child(div().w_full().p(px(4.0)).child(content))
    }

    fn render_toolbar(&self, diff: &FileDiff, cx: &mut Context<Self>) -> impl IntoElement {
        let header_text = Self::file_display_name(diff);
        let unified_active = self.view_mode == ViewMode::Unified;
        let sbs_active = self.view_mode == ViewMode::SideBySide;

        let unified_bg = if unified_active {
            rgb(0x007acc)
        } else {
            rgb(0x3c3c3c)
        };
        let sbs_bg = if sbs_active {
            rgb(0x007acc)
        } else {
            rgb(0x3c3c3c)
        };

        div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .px(px(12.0))
            .py(px(6.0))
            .bg(rgb(0x2d2d2d))
            .border_b_1()
            .border_color(rgb(0x404040))
            .gap(px(4.0))
            .child(
                div()
                    .id("btn-unified")
                    .px(px(8.0))
                    .py(px(2.0))
                    .bg(unified_bg)
                    .rounded(px(3.0))
                    .cursor_pointer()
                    .text_size(px(11.0))
                    .text_color(rgb(0xffffff))
                    .child("Unified")
                    .on_click(cx.listener(|this, _event, _window, _cx| {
                        this.view_mode = ViewMode::Unified;
                    })),
            )
            .child(
                div()
                    .id("btn-side-by-side")
                    .px(px(8.0))
                    .py(px(2.0))
                    .bg(sbs_bg)
                    .rounded(px(3.0))
                    .cursor_pointer()
                    .text_size(px(11.0))
                    .text_color(rgb(0xffffff))
                    .child("Side-by-Side")
                    .on_click(cx.listener(|this, _event, _window, _cx| {
                        this.view_mode = ViewMode::SideBySide;
                    })),
            )
            .child(
                div()
                    .flex_grow()
                    .text_size(px(12.0))
                    .text_color(rgb(0xcccccc))
                    .text_right()
                    .child(header_text),
            )
    }

    fn render_file_item(&self, i: usize, diff: &FileDiff, indent: f32, cx: &mut Context<Self>) -> impl IntoElement {
        let is_selected = self.selected_index == Some(i);
        let name = match self.panel_mode {
            PanelMode::List => Self::file_display_name(diff),
            PanelMode::Tree => {
                let path = diff.new_path.to_string();
                SharedString::from(
                    path.rsplit('/').next().unwrap_or(&path).to_string(),
                )
            }
        };

        let additions = diff.lines.iter().filter(|l| l.tag == ChangeTag::Insert).count();
        let deletions = diff.lines.iter().filter(|l| l.tag == ChangeTag::Delete).count();
        let stats = SharedString::from(format!("+{additions} −{deletions}"));

        let bg = if is_selected {
            rgb(0x37373d)
        } else {
            rgb(0x252526)
        };

        div()
            .id(ElementId::NamedInteger("file-item".into(), i as u64))
            .w_full()
            .pl(px(12.0 + indent))
            .pr(px(12.0))
            .py(px(4.0))
            .bg(bg)
            .cursor_pointer()
            .hover(|style| style.bg(rgb(0x2a2d2e)))
            .on_click(cx.listener(move |this, _event, _window, _cx| {
                this.selected_index = Some(i);
            }))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(0x888888))
                            .child("📄"),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(0xcccccc))
                            .overflow_x_hidden()
                            .child(name),
                    ),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(rgb(0x888888))
                    .pl(px(18.0))
                    .child(stats),
            )
    }

    fn render_tree_nodes(
        &self,
        nodes: &BTreeMap<String, TreeNode>,
        parent_path: &str,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        let mut elements: Vec<gpui::AnyElement> = Vec::new();
        let indent = depth as f32 * 16.0;

        let mut dirs: Vec<(&String, &TreeNode)> = Vec::new();
        let mut files: Vec<(&String, &TreeNode)> = Vec::new();

        for (key, node) in nodes {
            match node {
                TreeNode::Directory { .. } => dirs.push((key, node)),
                TreeNode::File { .. } => files.push((key, node)),
            }
        }

        for (_key, node) in &dirs {
            if let TreeNode::Directory { name, children } = node {
                let dir_path = if parent_path.is_empty() {
                    name.clone()
                } else {
                    format!("{parent_path}/{name}")
                };

                let is_collapsed = self.collapsed_dirs.contains(&dir_path);
                let arrow = if is_collapsed { "▶" } else { "▼" };
                let dir_path_clone = dir_path.clone();

                let dir_header = div()
                    .id(ElementId::Name(SharedString::from(format!("dir-{dir_path}"))))
                    .w_full()
                    .pl(px(12.0 + indent))
                    .pr(px(12.0))
                    .py(px(4.0))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0x2a2d2e)))
                    .on_click(cx.listener(move |this, _event, _window, _cx| {
                        if this.collapsed_dirs.contains(&dir_path_clone) {
                            this.collapsed_dirs.remove(&dir_path_clone);
                        } else {
                            this.collapsed_dirs.insert(dir_path_clone.clone());
                        }
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(rgb(0x888888))
                                    .w(px(10.0))
                                    .child(arrow),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(rgb(0x888888))
                                    .child("📁"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(rgb(0xcccccc))
                                    .child(SharedString::from(name.clone())),
                            ),
                    );

                elements.push(dir_header.into_any_element());

                if !is_collapsed {
                    let child_elements =
                        self.render_tree_nodes(children, &dir_path, depth + 1, cx);
                    elements.extend(child_elements);
                }
            }
        }

        for (_key, node) in &files {
            if let TreeNode::File { diff_index, .. } = node {
                let diff = &self.diffs[*diff_index];
                elements.push(
                    self.render_file_item(*diff_index, diff, indent, cx)
                        .into_any_element(),
                );
            }
        }

        elements
    }

    fn render_file_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let list_active = self.panel_mode == PanelMode::List;
        let tree_active = self.panel_mode == PanelMode::Tree;

        let list_bg = if list_active { rgb(0x007acc) } else { rgb(0x3c3c3c) };
        let tree_bg = if tree_active { rgb(0x007acc) } else { rgb(0x3c3c3c) };

        let mut panel = div()
            .flex()
            .flex_col()
            .w(self.panel_width)
            .flex_shrink_0()
            .h_full()
            .bg(rgb(0x252526))
            .border_l_1()
            .border_color(rgb(0x404040))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    .py(px(6.0))
                    .bg(rgb(0x2d2d2d))
                    .border_b_1()
                    .border_color(rgb(0x404040))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(0x999999))
                            .child(SharedString::from(format!(
                                "FILES ({})",
                                self.diffs.len()
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .id("btn-list-view")
                                    .px(px(6.0))
                                    .py(px(1.0))
                                    .bg(list_bg)
                                    .rounded(px(3.0))
                                    .cursor_pointer()
                                    .text_size(px(10.0))
                                    .text_color(rgb(0xffffff))
                                    .child("List")
                                    .on_click(cx.listener(|this, _event, _window, _cx| {
                                        this.panel_mode = PanelMode::List;
                                    })),
                            )
                            .child(
                                div()
                                    .id("btn-tree-view")
                                    .px(px(6.0))
                                    .py(px(1.0))
                                    .bg(tree_bg)
                                    .rounded(px(3.0))
                                    .cursor_pointer()
                                    .text_size(px(10.0))
                                    .text_color(rgb(0xffffff))
                                    .child("Tree")
                                    .on_click(cx.listener(|this, _event, _window, _cx| {
                                        this.panel_mode = PanelMode::Tree;
                                    })),
                            ),
                    ),
            );

        match self.panel_mode {
            PanelMode::List => {
                for (i, diff) in self.diffs.iter().enumerate() {
                    panel = panel.child(self.render_file_item(i, diff, 0.0, cx));
                }
            }
            PanelMode::Tree => {
                let tree = build_file_tree(&self.diffs);
                let elements = self.render_tree_nodes(&tree, "", 0, cx);
                for el in elements {
                    panel = panel.child(el);
                }
            }
        }

        panel
    }
}

impl Render for DiffViewer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let diff_content = if let Some(idx) = self.selected_index {
            if let Some(diff) = self.diffs.get(idx) {
                let toolbar = self.render_toolbar(diff, cx);
                let body = match self.view_mode {
                    ViewMode::Unified => self.render_file_diff(diff).into_any_element(),
                    ViewMode::SideBySide => {
                        self.render_side_by_side_diff(diff).into_any_element()
                    }
                };
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .child(toolbar)
                    .child(body)
                    .into_any_element()
            } else {
                div().into_any_element()
            }
        } else {
            div()
                .p(px(20.0))
                .text_color(rgb(0x888888))
                .child("No file selected")
                .into_any_element()
        };

        let initial_width = self.panel_width;

        let drag_handle = div()
            .id("panel-resize-handle")
            .w(px(DRAG_HANDLE_WIDTH))
            .h_full()
            .flex_shrink_0()
            .cursor(CursorStyle::ResizeLeftRight)
            .bg(rgb(0x404040))
            .hover(|style| style.bg(rgb(0x007acc)))
            .on_drag(
                PanelResizeDrag { initial_width },
                |drag, _offset, _window, cx| {
                    cx.new(|_| PanelResizeDrag {
                        initial_width: drag.initial_width,
                    })
                },
            )
            .on_drag_move::<PanelResizeDrag>(cx.listener(
                move |this, event: &gpui::DragMoveEvent<PanelResizeDrag>, window, _cx| {
                    let window_width = window.bounds().size.width;
                    let mouse_x = event.event.position.x;
                    let new_width = window_width - mouse_x - px(DRAG_HANDLE_WIDTH);
                    let clamped = new_width
                        .max(px(MIN_PANEL_WIDTH))
                        .min(px(MAX_PANEL_WIDTH));
                    this.panel_width = clamped;
                },
            ));

        div()
            .flex()
            .flex_row()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xd4d4d4))
            .font_family("Menlo")
            .text_size(px(13.0))
            .child(
                div()
                    .id("diff-content")
                    .flex_grow()
                    .min_w(px(0.0))
                    .overflow_y_scroll()
                    .overflow_x_hidden()
                    .child(diff_content),
            )
            .child(drag_handle)
            .child(self.render_file_panel(cx))
    }
}
