use std::fs;
use std::process::Command;

use crate::diff::FileDiff;

pub fn git_toplevel() -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    if !output.status.success() {
        return Err("Not a git repository".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn git_diff_files(staged: bool) -> Result<Vec<FileDiff>, String> {
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

        diffs.push(FileDiff::from_contents(
            file,
            file,
            &old_content,
            &new_content,
        ));
    }

    if !staged {
        let untracked_output = Command::new("git")
            .args(["ls-files", "--others", "--exclude-standard"])
            .current_dir(&toplevel)
            .output()
            .map_err(|e| format!("Failed to list untracked files: {e}"))?;

        if untracked_output.status.success() {
            let untracked_list = String::from_utf8_lossy(&untracked_output.stdout);
            for file in untracked_list.lines().filter(|l| !l.is_empty()) {
                let file_path = format!("{toplevel}/{file}");
                let new_content = fs::read_to_string(&file_path).unwrap_or_default();
                diffs.push(FileDiff::from_contents(file, file, "", &new_content));
            }
        }
    }

    if diffs.is_empty() {
        let kind = if staged { "staged" } else { "unstaged" };
        return Err(format!("No {kind} changes found"));
    }

    Ok(diffs)
}
