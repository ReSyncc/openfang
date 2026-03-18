//! Git repo registry and scoped git operations.
//!
//! Agents can clone, commit, push, pull repos within their drive scope.
//! Git credentials are resolved via the kernel's CredentialResolver.

use std::path::Path;
use std::process::Stdio;

use crate::{DriveError, DriveResult};

/// Run a git command in the given repo directory and return stdout.
pub async fn git_command(repo_path: &Path, args: &[&str]) -> DriveResult<String> {
    let output = tokio::process::Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| DriveError::Backend(format!("git exec failed: {e}")))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(DriveError::Backend(format!(
            "git {} failed: {}",
            args.first().unwrap_or(&""),
            stderr.trim()
        )))
    }
}

/// Clone a repo into the given path.
pub async fn clone_repo(url: &str, dest: &Path) -> DriveResult<()> {
    let output = tokio::process::Command::new("git")
        .args(["clone", url, &dest.to_string_lossy()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| DriveError::Backend(format!("git clone failed: {e}")))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(DriveError::Backend(format!(
            "git clone failed: {}",
            stderr.trim()
        )))
    }
}

/// Get the current branch name.
pub async fn current_branch(repo_path: &Path) -> DriveResult<String> {
    let out = git_command(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"]).await?;
    Ok(out.trim().to_string())
}

/// Get repo status (short format).
pub async fn repo_status(repo_path: &Path) -> DriveResult<String> {
    git_command(repo_path, &["status", "--short"]).await
}

/// Get the diff of uncommitted changes.
pub async fn repo_diff(repo_path: &Path) -> DriveResult<String> {
    git_command(repo_path, &["diff"]).await
}

/// Create and switch to a new branch.
pub async fn create_branch(repo_path: &Path, branch: &str) -> DriveResult<()> {
    git_command(repo_path, &["checkout", "-b", branch]).await?;
    Ok(())
}

/// Switch to an existing branch.
pub async fn switch_branch(repo_path: &Path, branch: &str) -> DriveResult<()> {
    git_command(repo_path, &["checkout", branch]).await?;
    Ok(())
}

/// Stage all changes and commit with a message.
pub async fn commit(repo_path: &Path, message: &str) -> DriveResult<String> {
    git_command(repo_path, &["add", "-A"]).await?;
    git_command(repo_path, &["commit", "-m", message]).await
}

/// Push to remote.
pub async fn push(repo_path: &Path, remote: &str, branch: &str) -> DriveResult<String> {
    git_command(repo_path, &["push", remote, branch]).await
}

/// Pull from remote.
pub async fn pull(repo_path: &Path) -> DriveResult<String> {
    git_command(repo_path, &["pull"]).await
}

/// Get remote URL.
pub async fn remote_url(repo_path: &Path) -> DriveResult<Option<String>> {
    match git_command(repo_path, &["remote", "get-url", "origin"]).await {
        Ok(url) => Ok(Some(url.trim().to_string())),
        Err(_) => Ok(None),
    }
}

/// Check if a directory is a git repo.
pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}
