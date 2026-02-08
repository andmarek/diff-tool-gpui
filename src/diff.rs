use gpui::SharedString;
use similar::{ChangeTag, TextDiff};
use std::fs;

#[derive(Clone)]
pub struct DiffLine {
    pub tag: ChangeTag,
    pub old_lineno: Option<usize>,
    pub new_lineno: Option<usize>,
    pub content: SharedString,
}

pub struct FileDiff {
    pub old_path: SharedString,
    pub new_path: SharedString,
    pub lines: Vec<DiffLine>,
}

impl FileDiff {
    pub fn from_contents(
        old_path: &str,
        new_path: &str,
        old_content: &str,
        new_content: &str,
    ) -> Self {
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

    pub fn from_files(old_path: &str, new_path: &str) -> Self {
        let old_content =
            fs::read_to_string(old_path).unwrap_or_else(|e| format!("Error reading file: {e}"));
        let new_content =
            fs::read_to_string(new_path).unwrap_or_else(|e| format!("Error reading file: {e}"));
        Self::from_contents(old_path, new_path, &old_content, &new_content)
    }
}

pub struct SideBySideLine {
    pub left: Option<DiffLine>,
    pub right: Option<DiffLine>,
}

pub fn to_side_by_side(lines: &[DiffLine]) -> Vec<SideBySideLine> {
    let mut result = Vec::new();
    let mut delete_buf: Vec<DiffLine> = Vec::new();

    for line in lines {
        match line.tag {
            ChangeTag::Delete => {
                delete_buf.push(line.clone());
            }
            ChangeTag::Insert => {
                if let Some(del) = delete_buf.first().cloned() {
                    delete_buf.remove(0);
                    result.push(SideBySideLine {
                        left: Some(del),
                        right: Some(line.clone()),
                    });
                } else {
                    result.push(SideBySideLine {
                        left: None,
                        right: Some(line.clone()),
                    });
                }
            }
            ChangeTag::Equal => {
                for del in delete_buf.drain(..) {
                    result.push(SideBySideLine {
                        left: Some(del),
                        right: None,
                    });
                }
                result.push(SideBySideLine {
                    left: Some(line.clone()),
                    right: Some(line.clone()),
                });
            }
        }
    }

    for del in delete_buf.drain(..) {
        result.push(SideBySideLine {
            left: Some(del),
            right: None,
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_side_by_side_equal_lines() {
        let lines = vec![
            DiffLine {
                tag: ChangeTag::Equal,
                old_lineno: Some(1),
                new_lineno: Some(1),
                content: "hello".into(),
            },
            DiffLine {
                tag: ChangeTag::Equal,
                old_lineno: Some(2),
                new_lineno: Some(2),
                content: "world".into(),
            },
        ];
        let sbs = to_side_by_side(&lines);
        assert_eq!(sbs.len(), 2);
        assert!(sbs[0].left.is_some() && sbs[0].right.is_some());
        assert!(sbs[1].left.is_some() && sbs[1].right.is_some());
    }

    #[test]
    fn test_side_by_side_delete_insert_pairing() {
        let lines = vec![
            DiffLine {
                tag: ChangeTag::Delete,
                old_lineno: Some(1),
                new_lineno: None,
                content: "old".into(),
            },
            DiffLine {
                tag: ChangeTag::Insert,
                old_lineno: None,
                new_lineno: Some(1),
                content: "new".into(),
            },
        ];
        let sbs = to_side_by_side(&lines);
        assert_eq!(sbs.len(), 1);
        assert_eq!(sbs[0].left.as_ref().unwrap().content.as_ref(), "old");
        assert_eq!(sbs[0].right.as_ref().unwrap().content.as_ref(), "new");
    }

    #[test]
    fn test_side_by_side_more_deletes_than_inserts() {
        let lines = vec![
            DiffLine {
                tag: ChangeTag::Delete,
                old_lineno: Some(1),
                new_lineno: None,
                content: "del1".into(),
            },
            DiffLine {
                tag: ChangeTag::Delete,
                old_lineno: Some(2),
                new_lineno: None,
                content: "del2".into(),
            },
            DiffLine {
                tag: ChangeTag::Insert,
                old_lineno: None,
                new_lineno: Some(1),
                content: "ins1".into(),
            },
        ];
        let sbs = to_side_by_side(&lines);
        assert_eq!(sbs.len(), 2);
        assert!(sbs[0].left.is_some() && sbs[0].right.is_some());
        assert!(sbs[1].left.is_some() && sbs[1].right.is_none());
    }

    #[test]
    fn test_side_by_side_more_inserts_than_deletes() {
        let lines = vec![
            DiffLine {
                tag: ChangeTag::Delete,
                old_lineno: Some(1),
                new_lineno: None,
                content: "del1".into(),
            },
            DiffLine {
                tag: ChangeTag::Insert,
                old_lineno: None,
                new_lineno: Some(1),
                content: "ins1".into(),
            },
            DiffLine {
                tag: ChangeTag::Insert,
                old_lineno: None,
                new_lineno: Some(2),
                content: "ins2".into(),
            },
        ];
        let sbs = to_side_by_side(&lines);
        assert_eq!(sbs.len(), 2);
        assert!(sbs[0].left.is_some() && sbs[0].right.is_some());
        assert!(sbs[1].left.is_none() && sbs[1].right.is_some());
    }

    #[test]
    fn test_side_by_side_trailing_deletes() {
        let lines = vec![
            DiffLine {
                tag: ChangeTag::Delete,
                old_lineno: Some(1),
                new_lineno: None,
                content: "del1".into(),
            },
            DiffLine {
                tag: ChangeTag::Delete,
                old_lineno: Some(2),
                new_lineno: None,
                content: "del2".into(),
            },
        ];
        let sbs = to_side_by_side(&lines);
        assert_eq!(sbs.len(), 2);
        assert!(sbs[0].left.is_some() && sbs[0].right.is_none());
        assert!(sbs[1].left.is_some() && sbs[1].right.is_none());
    }
}
