use std::fs;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use git2::{BranchType, Diff, DiffFormat, DiffOptions, Patch, Repository, Status, StatusEntry, StatusOptions};
use similar::{ChangeTag, TextDiff};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeItem {
    pub code: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchScope {
    Local,
    Remote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitCellKind {
    None,
    Context,
    Added,
    Removed,
    Meta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitDiffRow {
    pub left: String,
    pub right: String,
    pub language: String,
    pub left_kind: SplitCellKind,
    pub right_kind: SplitCellKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SplitDiffModel {
    pub rows: Vec<SplitDiffRow>,
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

pub fn list_branches(workspace: &str, scope: BranchScope) -> Result<Vec<String>, GitStatusError> {
    let workspace_path = PathBuf::from(workspace.trim());
    if !workspace_path.exists() {
        return Err(GitStatusError::WorkspaceNotFound(
            workspace_path.display().to_string(),
        ));
    }

    let repo = Repository::discover(&workspace_path)?;
    let branch_type = match scope {
        BranchScope::Local => BranchType::Local,
        BranchScope::Remote => BranchType::Remote,
    };

    let mut branches = Vec::new();
    for branch_result in repo.branches(Some(branch_type))? {
        let (branch, _) = branch_result?;
        let Some(name) = branch.name()? else {
            continue;
        };

        if name.ends_with("/HEAD") {
            continue;
        }
        branches.push(name.to_string());
    }

    branches.sort();
    Ok(branches)
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

pub fn repo_split_diff(workspace: &str) -> Result<SplitDiffModel, GitStatusError> {
    build_split_diff_model(workspace, None)
}

pub fn repo_split_diff_for_path(
    workspace: &str,
    relative_path: &str,
) -> Result<SplitDiffModel, GitStatusError> {
    build_split_diff_model(workspace, Some(relative_path))
}

fn build_split_diff_model(
    workspace: &str,
    relative_path: Option<&str>,
) -> Result<SplitDiffModel, GitStatusError> {
    let workspace_path = PathBuf::from(workspace.trim());
    if !workspace_path.exists() {
        return Err(GitStatusError::WorkspaceNotFound(
            workspace_path.display().to_string(),
        ));
    }

    let repo = Repository::discover(&workspace_path)?;
    let mut model = SplitDiffModel::default();

    let mut staged_opts = DiffOptions::new();
    staged_opts
        .include_untracked(true)
        .include_typechange(true)
        .recurse_untracked_dirs(true);
    if let Some(path) = relative_path {
        staged_opts.pathspec(path);
    }
    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
    let staged = repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut staged_opts))?;
    append_split_rows_for_diff(&staged, "Staged", &mut model)?;

    let mut unstaged_opts = DiffOptions::new();
    unstaged_opts
        .include_untracked(true)
        .include_typechange(true)
        .recurse_untracked_dirs(true);
    if let Some(path) = relative_path {
        unstaged_opts.pathspec(path);
    }
    let unstaged = repo.diff_index_to_workdir(None, Some(&mut unstaged_opts))?;
    append_split_rows_for_diff(&unstaged, "Unstaged", &mut model)?;

    if model.rows.is_empty() {
        let message = relative_path
            .map(|path| format!("No diff changes for {path}."))
            .unwrap_or_else(|| "No diff changes.".to_string());
        model.rows.push(meta_row(&message));
    }

    Ok(model)
}

fn append_split_rows_for_diff(
    diff: &Diff<'_>,
    section_name: &str,
    model: &mut SplitDiffModel,
) -> Result<(), git2::Error> {
    if diff.deltas().len() == 0 {
        return Ok(());
    }

    model.rows.push(meta_row(&format!("## {section_name}")));

    for idx in 0..diff.deltas().len() {
        let Some(delta) = diff.get_delta(idx) else {
            continue;
        };

        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<unknown>".to_string());
        let language = language_from_path(&path).to_string();
        model.rows.push(meta_row(&format!("diff -- {path}")));

        let Some(patch) = Patch::from_diff(diff, idx)? else {
            model.rows.push(meta_row("Binary or unchanged content"));
            continue;
        };

        for hunk_idx in 0..patch.num_hunks() {
            let (hunk, line_count) = patch.hunk(hunk_idx)?;
            let hunk_header = String::from_utf8_lossy(hunk.header())
                .trim_end_matches(['\n', '\r'])
                .to_string();
            if !hunk_header.is_empty() {
                model.rows.push(meta_row(&hunk_header));
            }

            let mut removed_run: Vec<String> = Vec::new();
            let mut added_run: Vec<String> = Vec::new();
            for line_idx in 0..line_count {
                let line = patch.line_in_hunk(hunk_idx, line_idx)?;
                let text = normalized_line_content(line.content());
                match line.origin() {
                    ' ' | '=' => {
                        flush_change_run(&mut model.rows, &mut removed_run, &mut added_run, &language);
                        model.rows.push(SplitDiffRow {
                            left: text.clone(),
                            right: text,
                            language: language.clone(),
                            left_kind: SplitCellKind::Context,
                            right_kind: SplitCellKind::Context,
                        });
                    }
                    '-' | '<' => removed_run.push(text),
                    '+' | '>' => added_run.push(text),
                    _ => {}
                }
            }
            flush_change_run(&mut model.rows, &mut removed_run, &mut added_run, &language);
        }
    }

    Ok(())
}

fn flush_change_run(
    rows: &mut Vec<SplitDiffRow>,
    removed_run: &mut Vec<String>,
    added_run: &mut Vec<String>,
    language: &str,
) {
    if removed_run.is_empty() && added_run.is_empty() {
        return;
    }

    let removed_refs: Vec<&str> = removed_run.iter().map(String::as_str).collect();
    let added_refs: Vec<&str> = added_run.iter().map(String::as_str).collect();
    let diff = TextDiff::from_slices(&removed_refs, &added_refs);
    let mut pending_removed: VecDeque<String> = VecDeque::new();
    let mut pending_added: VecDeque<String> = VecDeque::new();

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                flush_pending_changes(rows, &mut pending_removed, &mut pending_added, language);
                let text = change.value().to_string();
                rows.push(SplitDiffRow {
                    left: text.clone(),
                    right: text,
                    language: language.to_string(),
                    left_kind: SplitCellKind::Context,
                    right_kind: SplitCellKind::Context,
                });
            }
            ChangeTag::Delete => pending_removed.push_back(change.value().to_string()),
            ChangeTag::Insert => pending_added.push_back(change.value().to_string()),
        }
    }

    flush_pending_changes(rows, &mut pending_removed, &mut pending_added, language);
    removed_run.clear();
    added_run.clear();
}

fn flush_pending_changes(
    rows: &mut Vec<SplitDiffRow>,
    pending_removed: &mut VecDeque<String>,
    pending_added: &mut VecDeque<String>,
    language: &str,
) {
    while !pending_removed.is_empty() || !pending_added.is_empty() {
        let left = pending_removed.pop_front().unwrap_or_default();
        let right = pending_added.pop_front().unwrap_or_default();
        let left_kind = if left.is_empty() {
            SplitCellKind::None
        } else {
            SplitCellKind::Removed
        };
        let right_kind = if right.is_empty() {
            SplitCellKind::None
        } else {
            SplitCellKind::Added
        };

        rows.push(SplitDiffRow {
            left,
            right,
            language: language.to_string(),
            left_kind,
            right_kind,
        });
    }
}

fn meta_row(text: &str) -> SplitDiffRow {
    SplitDiffRow {
        left: text.to_string(),
        right: text.to_string(),
        language: "diff".to_string(),
        left_kind: SplitCellKind::Meta,
        right_kind: SplitCellKind::Meta,
    }
}

fn normalized_line_content(content: &[u8]) -> String {
    String::from_utf8_lossy(content)
        .trim_end_matches(['\n', '\r'])
        .to_string()
}

fn language_from_path(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if let Some(ext) = lower.rsplit('.').next() {
        return match ext {
            "rs" => "rust",
            "toml" => "toml",
            "py" => "python",
            "c" | "h" | "hpp" | "cpp" | "cc" | "cxx" => "cpp",
            "js" | "mjs" | "cjs" => "javascript",
            "ts" | "tsx" => "typescript",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "md" | "markdown" => "markdown",
            "sh" | "bash" | "zsh" => "bash",
            "html" | "htm" => "html",
            "css" => "css",
            "xml" => "xml",
            "java" => "java",
            "go" => "go",
            "rb" => "ruby",
            "php" => "php",
            "sql" => "sql",
            _ => "",
        };
    }

    ""
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
