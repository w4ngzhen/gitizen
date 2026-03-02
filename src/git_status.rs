use std::fs;
use std::path::{Path, PathBuf};

use git2::{Diff, DiffFormat, DiffOptions, Repository, Status, StatusEntry, StatusOptions};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeItem {
    pub code: String,
    pub path: String,
}

#[derive(Debug, Error)]
pub enum GitStatusError {
    #[error("Workspace path does not exist: {0}")]
    WorkspaceNotFound(String),
    #[error("Failed to open git repository: {0}")]
    OpenRepo(#[from] git2::Error),
    #[error("Failed to read file {path}: {source}")]
    ReadFile { path: String, source: std::io::Error },
}

pub fn list_changes(workspace: &str) -> Result<Vec<ChangeItem>, GitStatusError> {
    let workspace_path = PathBuf::from(workspace.trim());
    if !workspace_path.exists() {
        return Err(GitStatusError::WorkspaceNotFound(
            workspace_path.display().to_string(),
        ));
    }

    let repo = Repository::discover(&workspace_path)?;
    let repo_root = repo
        .workdir()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .include_ignored(false)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut options))?;
    let mut result = Vec::with_capacity(statuses.len());

    for entry in statuses.iter() {
        result.push(map_entry(&repo_root, &entry));
    }

    result.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(result)
}

pub fn list_repo_files(workspace: &str) -> Result<Vec<String>, GitStatusError> {
    let workspace_path = PathBuf::from(workspace.trim());
    if !workspace_path.exists() {
        return Err(GitStatusError::WorkspaceNotFound(
            workspace_path.display().to_string(),
        ));
    }

    let repo = Repository::discover(&workspace_path)?;
    let repo_root = repo
        .workdir()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let mut files = Vec::new();
    let mut dirs = vec![repo_root.clone()];

    while let Some(dir) = dirs.pop() {
        let mut entries = match fs::read_dir(&dir) {
            Ok(entries) => entries.filter_map(Result::ok).collect::<Vec<_>>(),
            Err(_) => continue,
        };
        entries.sort_by_key(|entry| entry.path());

        for entry in entries {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name == ".git" {
                continue;
            }

            let relative = match path.strip_prefix(&repo_root) {
                Ok(relative) => relative,
                Err(_) => continue,
            };
            let relative_text = relative.to_string_lossy().replace('\\', "/");

            if path.is_dir() {
                if is_ignored_path(&repo, relative) {
                    continue;
                }
                dirs.push(path);
                continue;
            }

            if path.is_file() {
                if is_ignored_path(&repo, relative) {
                    continue;
                }
                files.push(relative_text);
            }
        }
    }

    files.sort();
    Ok(files)
}

pub fn read_repo_file(workspace: &str, relative_path: &str) -> Result<String, GitStatusError> {
    let workspace_path = PathBuf::from(workspace.trim());
    if !workspace_path.exists() {
        return Err(GitStatusError::WorkspaceNotFound(
            workspace_path.display().to_string(),
        ));
    }

    let repo = Repository::discover(&workspace_path)?;
    let repo_root = repo
        .workdir()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let file_path = repo_root.join(relative_path);
    fs::read_to_string(&file_path).map_err(|source| GitStatusError::ReadFile {
        path: file_path.display().to_string(),
        source,
    })
}

pub fn repo_diff(workspace: &str) -> Result<String, GitStatusError> {
    let workspace_path = PathBuf::from(workspace.trim());
    if !workspace_path.exists() {
        return Err(GitStatusError::WorkspaceNotFound(
            workspace_path.display().to_string(),
        ));
    }

    let repo = Repository::discover(&workspace_path)?;
    let mut output = String::new();

    let mut staged_opts = DiffOptions::new();
    staged_opts
        .include_untracked(true)
        .include_typechange(true)
        .recurse_untracked_dirs(true);

    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
    let staged = repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut staged_opts))?;
    if staged.deltas().len() > 0 {
        output.push_str("## Staged\n");
        output.push_str(&render_diff(&staged)?);
        output.push('\n');
    }

    let mut unstaged_opts = DiffOptions::new();
    unstaged_opts
        .include_untracked(true)
        .include_typechange(true)
        .recurse_untracked_dirs(true);
    let unstaged = repo.diff_index_to_workdir(None, Some(&mut unstaged_opts))?;
    if unstaged.deltas().len() > 0 {
        output.push_str("## Unstaged\n");
        output.push_str(&render_diff(&unstaged)?);
    }

    if output.trim().is_empty() {
        Ok("No diff changes.".to_string())
    } else {
        Ok(output)
    }
}

pub fn repo_diff_for_path(workspace: &str, relative_path: &str) -> Result<String, GitStatusError> {
    let workspace_path = PathBuf::from(workspace.trim());
    if !workspace_path.exists() {
        return Err(GitStatusError::WorkspaceNotFound(
            workspace_path.display().to_string(),
        ));
    }

    let repo = Repository::discover(&workspace_path)?;
    let mut output = String::new();

    let mut staged_opts = DiffOptions::new();
    staged_opts
        .include_untracked(true)
        .include_typechange(true)
        .recurse_untracked_dirs(true)
        .pathspec(relative_path);

    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
    let staged = repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut staged_opts))?;
    if staged.deltas().len() > 0 {
        output.push_str("## Staged\n");
        output.push_str(&render_diff(&staged)?);
        output.push('\n');
    }

    let mut unstaged_opts = DiffOptions::new();
    unstaged_opts
        .include_untracked(true)
        .include_typechange(true)
        .recurse_untracked_dirs(true)
        .pathspec(relative_path);
    let unstaged = repo.diff_index_to_workdir(None, Some(&mut unstaged_opts))?;
    if unstaged.deltas().len() > 0 {
        output.push_str("## Unstaged\n");
        output.push_str(&render_diff(&unstaged)?);
    }

    if output.trim().is_empty() {
        Ok(format!("No diff changes for {relative_path}."))
    } else {
        Ok(output)
    }
}

fn render_diff(diff: &Diff<'_>) -> Result<String, git2::Error> {
    let mut out = String::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        if let Ok(text) = std::str::from_utf8(line.content()) {
            out.push_str(text);
        }
        true
    })?;
    Ok(out)
}

fn is_ignored_path(repo: &Repository, relative_path: &Path) -> bool {
    repo.status_should_ignore(relative_path).unwrap_or(false)
}

fn map_entry(repo_root: &Path, entry: &StatusEntry<'_>) -> ChangeItem {
    let status = entry.status();

    let code = format_status_code(status);

    let path = entry
        .head_to_index()
        .and_then(|d| d.new_file().path())
        .or_else(|| entry.index_to_workdir().and_then(|d| d.new_file().path()))
        .or_else(|| entry.path().map(Path::new))
        .map(|p| normalize_path(repo_root, p))
        .unwrap_or_else(|| "<unknown>".to_string());

    ChangeItem { code, path }
}

fn normalize_path(repo_root: &Path, relative_or_absolute: &Path) -> String {
    if relative_or_absolute.is_absolute() {
        return relative_or_absolute.display().to_string();
    }
    let _ = repo_root;
    relative_or_absolute.display().to_string()
}

fn format_status_code(status: Status) -> String {
    let staged = staged_code(status);
    let unstaged = unstaged_code(status);
    format!("{staged}{unstaged}")
}

fn staged_code(status: Status) -> char {
    if status.contains(Status::INDEX_NEW) {
        'A'
    } else if status.contains(Status::INDEX_MODIFIED) {
        'M'
    } else if status.contains(Status::INDEX_DELETED) {
        'D'
    } else if status.contains(Status::INDEX_RENAMED) {
        'R'
    } else if status.contains(Status::INDEX_TYPECHANGE) {
        'T'
    } else {
        ' '
    }
}

fn unstaged_code(status: Status) -> char {
    if status.contains(Status::WT_NEW) {
        '?'
    } else if status.contains(Status::WT_MODIFIED) {
        'M'
    } else if status.contains(Status::WT_DELETED) {
        'D'
    } else if status.contains(Status::WT_TYPECHANGE) {
        'T'
    } else if status.contains(Status::WT_RENAMED) {
        'R'
    } else {
        ' '
    }
}
