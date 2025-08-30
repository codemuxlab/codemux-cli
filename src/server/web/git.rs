use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::process::Command;

use super::types::{AppState, GitDiff, GitFileDiff, GitFileStatus, GitStatus};

pub async fn get_git_status(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let working_dir = match get_session_working_dir(&session_id, &state).await {
        Some(dir) => dir,
        None => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Session not found"))
                .unwrap()
        }
    };

    match execute_git_status(&working_dir).await {
        Ok(status) => Json(status).into_response(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(format!("Git error: {}", e)))
            .unwrap(),
    }
}

pub async fn get_git_diff(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let working_dir = match get_session_working_dir(&session_id, &state).await {
        Some(dir) => dir,
        None => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Session not found"))
                .unwrap()
        }
    };

    match execute_git_diff(&working_dir).await {
        Ok(diff) => Json(diff).into_response(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(format!("Git error: {}", e)))
            .unwrap(),
    }
}

pub async fn get_git_file_diff(
    Path((session_id, file_path)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let working_dir = match get_session_working_dir(&session_id, &state).await {
        Some(dir) => dir,
        None => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Session not found"))
                .unwrap()
        }
    };

    match execute_git_file_diff(&working_dir, &file_path).await {
        Ok(diff) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from(diff))
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(format!("Git error: {}", e)))
            .unwrap(),
    }
}

// Helper functions
async fn get_session_working_dir(session_id: &str, state: &AppState) -> Option<String> {
    // For run mode, use current directory
    // Get session working directory from session manager
    let _session_info = state.session_manager.get_session(session_id).await?;
    // TODO: Get actual working directory from session info
    // For now, return current directory
    Some(std::env::current_dir().ok()?.to_string_lossy().to_string())
}

async fn execute_git_status(
    working_dir: &str,
) -> Result<GitStatus, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("git")
        .args(["status", "--porcelain", "-b", "--untracked-files=all"])
        .current_dir(working_dir)
        .output()?;

    if !output.status.success() {
        return Err("Not a git repository or git command failed".into());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();
    let mut branch = None;

    for line in output_str.lines() {
        if line.starts_with("##") {
            // Branch information
            let branch_info = line.strip_prefix("## ").unwrap_or("");
            branch = Some(
                branch_info
                    .split("...")
                    .next()
                    .unwrap_or(branch_info)
                    .to_string(),
            );
        } else if line.len() >= 3 {
            let status_chars = &line[0..2];
            let file_path = line[3..].to_string();

            let status = match status_chars {
                " M" | "M " | "MM" => "modified",
                "A " | "AM" => "added",
                " D" | "D " => "deleted",
                "R " => "renamed",
                "??" => "untracked",
                _ => "unknown",
            };

            files.push(GitFileStatus {
                path: file_path,
                status: status.to_string(),
                additions: None, // TODO: Get from git diff --numstat
                deletions: None,
            });
        }
    }

    let is_clean = files.is_empty();
    Ok(GitStatus {
        files,
        branch,
        clean: is_clean,
    })
}

async fn execute_git_diff(
    working_dir: &str,
) -> Result<GitDiff, Box<dyn std::error::Error + Send + Sync>> {
    let mut files = Vec::new();

    // Get tracked file changes
    let output = Command::new("git")
        .args(["diff", "--name-status"])
        .current_dir(working_dir)
        .output()?;

    if !output.status.success() {
        return Err("Git diff failed".into());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);

    for line in output_str.lines() {
        if let Some((status_char, file_path)) = line.split_once('\t') {
            let status = match status_char {
                "M" => "modified",
                "A" => "added",
                "D" => "deleted",
                "R" => "renamed",
                _ => "unknown",
            };

            // Get detailed diff for this file
            let diff_output = Command::new("git")
                .args(["diff", file_path])
                .current_dir(working_dir)
                .output()?;

            let diff_content = String::from_utf8_lossy(&diff_output.stdout).to_string();

            // Parse additions/deletions from diff
            let (additions, deletions) = parse_diff_stats(&diff_content);

            files.push(GitFileDiff {
                path: file_path.to_string(),
                old_path: None, // TODO: Handle renamed files
                status: status.to_string(),
                additions,
                deletions,
                diff: diff_content,
            });
        }
    }

    // Add untracked files (show full content as "added")
    let untracked_output = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(working_dir)
        .output()?;

    if untracked_output.status.success() {
        let untracked_str = String::from_utf8_lossy(&untracked_output.stdout);

        for line in untracked_str.lines() {
            if line.starts_with("??") && line.len() >= 3 {
                let file_path = &line[3..];

                // Read the full content of the untracked file
                let file_content =
                    std::fs::read_to_string(std::path::Path::new(working_dir).join(file_path))
                        .unwrap_or_else(|_| String::from("Binary file or read error"));

                // Create a fake diff showing the entire file as added
                let fake_diff = if file_content.is_empty() {
                    format!("diff --git a/{} b/{}\nnew file mode 100644\nindex 0000000..0000000\n--- /dev/null\n+++ b/{}\n", file_path, file_path, file_path)
                } else {
                    let mut diff_lines = vec![
                        format!("diff --git a/{} b/{}", file_path, file_path),
                        "new file mode 100644".to_string(),
                        "index 0000000..0000000".to_string(),
                        "--- /dev/null".to_string(),
                        format!("+++ b/{}", file_path),
                    ];

                    // Add each line of the file as an addition
                    for line in file_content.lines() {
                        diff_lines.push(format!("+{}", line));
                    }

                    diff_lines.join("\n")
                };

                let line_count = file_content.lines().count() as u32;

                files.push(GitFileDiff {
                    path: file_path.to_string(),
                    old_path: None,
                    status: "untracked".to_string(),
                    additions: line_count,
                    deletions: 0,
                    diff: fake_diff,
                });
            }
        }
    }

    Ok(GitDiff { files })
}

async fn execute_git_file_diff(
    working_dir: &str,
    file_path: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("git")
        .args(["diff", file_path])
        .current_dir(working_dir)
        .output()?;

    if output.status.success() {
        let diff_content = String::from_utf8_lossy(&output.stdout).to_string();
        if !diff_content.trim().is_empty() {
            return Ok(diff_content);
        }
    }

    // If git diff returns empty or fails, check if it's an untracked file
    let status_output = Command::new("git")
        .args(["status", "--porcelain", file_path])
        .current_dir(working_dir)
        .output()?;

    if status_output.status.success() {
        let status_str = String::from_utf8_lossy(&status_output.stdout);
        if status_str.starts_with("??") {
            // It's an untracked file, show full content as additions
            let file_content =
                std::fs::read_to_string(std::path::Path::new(working_dir).join(file_path))
                    .unwrap_or_else(|_| String::from("Binary file or read error"));

            let fake_diff = if file_content.is_empty() {
                format!("diff --git a/{} b/{}\nnew file mode 100644\nindex 0000000..0000000\n--- /dev/null\n+++ b/{}\n", file_path, file_path, file_path)
            } else {
                let mut diff_lines = vec![
                    format!("diff --git a/{} b/{}", file_path, file_path),
                    "new file mode 100644".to_string(),
                    "index 0000000..0000000".to_string(),
                    "--- /dev/null".to_string(),
                    format!("+++ b/{}", file_path),
                ];

                // Add each line of the file as an addition
                for line in file_content.lines() {
                    diff_lines.push(format!("+{}", line));
                }

                diff_lines.join("\n")
            };

            return Ok(fake_diff);
        }
    }

    Err("No diff found for file".into())
}

fn parse_diff_stats(diff_content: &str) -> (u32, u32) {
    let mut additions = 0;
    let mut deletions = 0;

    for line in diff_content.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
        }
    }

    (additions, deletions)
}
