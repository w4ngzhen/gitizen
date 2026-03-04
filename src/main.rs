mod branch_selector;
mod branches_view;
mod code_view;
mod diff_code_view;
mod diff_view;
mod git_status;
mod project_selector;
mod repo_actions;
mod tree_view;

use std::env;

use eframe::{egui, App};
use git_status::{
    checkout_local_branch, checkout_reference, checkout_remote_branch, create_local_branch,
    current_local_branch, git_fetch, git_pull, git_push, list_branches, list_changes,
    list_repo_files, read_repo_file, repo_diff, repo_diff_for_path, repo_split_diff,
    repo_split_diff_for_path, BranchScope, ChangeItem, SplitDiffModel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeftView {
    Files,
    Diff,
    Branches,
}

struct GitBrainsApp {
    workspace: String,
    open_projects: Vec<String>,
    recent_projects: Vec<String>,
    changes: Vec<ChangeItem>,
    files: Vec<String>,
    selected_file: Option<String>,
    selected_file_content: String,
    local_branches: Vec<String>,
    remote_branches: Vec<String>,
    selected_local_branch: Option<String>,
    selected_remote_branch: Option<String>,
    branch_panel_state: branches_view::BranchPanelState,
    diff_output: String,
    selected_diff_path: Option<String>,
    selected_diff_output: String,
    selected_split_diff: SplitDiffModel,
    diff_render_mode: diff_code_view::DiffRenderMode,
    diff_panel_state: diff_view::DiffPanelState,
    branch_selector_state: branch_selector::BranchSelectorState,
    left_panel_open: bool,
    left_view: LeftView,
    error: Option<String>,
}

impl Default for GitBrainsApp {
    fn default() -> Self {
        let workspace = env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| String::new());

        Self {
            open_projects: vec![workspace.clone()],
            recent_projects: Vec::new(),
            workspace,
            changes: Vec::new(),
            files: Vec::new(),
            selected_file: None,
            selected_file_content: String::new(),
            local_branches: Vec::new(),
            remote_branches: Vec::new(),
            selected_local_branch: None,
            selected_remote_branch: None,
            branch_panel_state: branches_view::BranchPanelState::default(),
            diff_output: String::new(),
            selected_diff_path: None,
            selected_diff_output: String::new(),
            selected_split_diff: SplitDiffModel::default(),
            diff_render_mode: diff_code_view::DiffRenderMode::default(),
            diff_panel_state: diff_view::DiffPanelState::default(),
            branch_selector_state: branch_selector::BranchSelectorState::default(),
            left_panel_open: true,
            left_view: LeftView::Files,
            error: None,
        }
    }
}

impl GitBrainsApp {
    fn handle_repo_action(&mut self, action: repo_actions::RepoAction) {
        match action {
            repo_actions::RepoAction::Fetch => match git_fetch(&self.workspace) {
                Ok(_) => {
                    self.branch_selector_state.message = Some("fetch completed".to_string());
                    self.refresh();
                }
                Err(err) => {
                    self.branch_selector_state.message = Some(err.to_string());
                }
            },
            repo_actions::RepoAction::PullMerge => match git_pull(&self.workspace, false) {
                Ok(_) => {
                    self.branch_selector_state.message = Some("pull (merge) completed".to_string());
                    self.refresh();
                }
                Err(err) => {
                    self.branch_selector_state.message = Some(err.to_string());
                }
            },
            repo_actions::RepoAction::PullRebase => match git_pull(&self.workspace, true) {
                Ok(_) => {
                    self.branch_selector_state.message = Some("pull (rebase) completed".to_string());
                    self.refresh();
                }
                Err(err) => {
                    self.branch_selector_state.message = Some(err.to_string());
                }
            },
            repo_actions::RepoAction::Commit => {
                self.branch_selector_state.message =
                    Some("commit action is not implemented".to_string());
            }
            repo_actions::RepoAction::Push => match git_push(&self.workspace) {
                Ok(_) => {
                    self.branch_selector_state.message = Some("push completed".to_string());
                    self.refresh();
                }
                Err(err) => {
                    self.branch_selector_state.message = Some(err.to_string());
                }
            },
        }
    }

    fn switch_workspace(&mut self, path: String) {
        if self.workspace != path {
            if !self.recent_projects.iter().any(|p| p == &self.workspace) {
                self.recent_projects.insert(0, self.workspace.clone());
            }
            self.recent_projects.retain(|p| p != &path);
            self.workspace = path.clone();
        }
        if !self.open_projects.iter().any(|p| p == &path) {
            self.open_projects.push(path);
        }
        self.refresh();
    }

    fn refresh(&mut self) {
        let changes = list_changes(&self.workspace);
        let files = list_repo_files(&self.workspace);
        let diff = repo_diff(&self.workspace);
        let local_branches = list_branches(&self.workspace, BranchScope::Local);
        let remote_branches = list_branches(&self.workspace, BranchScope::Remote);

        match (changes, files, diff, local_branches, remote_branches) {
            (Ok(changes), Ok(files), Ok(diff_output), Ok(local_branches), Ok(remote_branches)) => {
                self.error = None;
                self.changes = changes;
                self.files = files;
                self.diff_output = diff_output;
                self.local_branches = local_branches;
                self.remote_branches = remote_branches;

                let selected_missing = match self.selected_file.as_ref() {
                    Some(selected) => !self.files.iter().any(|file| file == selected),
                    None => true,
                };
                if selected_missing {
                    self.selected_file = self.files.first().cloned();
                }
                let selected_diff_missing = match self.selected_diff_path.as_ref() {
                    Some(selected) => !self.changes.iter().any(|change| change.path == *selected),
                    None => false,
                };
                if selected_diff_missing {
                    self.selected_diff_path = None;
                }
                let selected_local_missing = match self.selected_local_branch.as_ref() {
                    Some(selected) => !self.local_branches.iter().any(|branch| branch == selected),
                    None => false,
                };
                if selected_local_missing {
                    self.selected_local_branch = None;
                }
                if let Ok(current) = current_local_branch(&self.workspace) {
                    if current.is_some() {
                        self.selected_local_branch = current;
                    }
                }
                let selected_remote_missing = match self.selected_remote_branch.as_ref() {
                    Some(selected) => !self.remote_branches.iter().any(|branch| branch == selected),
                    None => false,
                };
                if selected_remote_missing {
                    self.selected_remote_branch = None;
                }
                self.load_selected_file_content();
                self.load_selected_diff_content();
            }
            (changes_res, files_res, diff_res, local_res, remote_res) => {
                self.changes.clear();
                self.files.clear();
                self.selected_file = None;
                self.selected_file_content.clear();
                self.local_branches.clear();
                self.remote_branches.clear();
                self.selected_local_branch = None;
                self.selected_remote_branch = None;
                self.diff_output.clear();
                self.selected_diff_path = None;
                self.selected_diff_output.clear();
                self.selected_split_diff = SplitDiffModel::default();

                let message = changes_res
                    .err()
                    .or_else(|| files_res.err())
                    .or_else(|| diff_res.err())
                    .or_else(|| local_res.err())
                    .or_else(|| remote_res.err())
                    .map(|err| err.to_string())
                    .unwrap_or_else(|| "Unknown error while refreshing".to_string());
                self.error = Some(message);
            }
        }
    }

    fn load_selected_file_content(&mut self) {
        let Some(path) = self.selected_file.as_deref() else {
            self.selected_file_content = "No file selected.".to_string();
            return;
        };

        self.selected_file_content = match read_repo_file(&self.workspace, path) {
            Ok(text) => text,
            Err(err) => format!("Unable to read file:\n{err}"),
        };
    }

    fn load_selected_diff_content(&mut self) {
        let Some(path) = self.selected_diff_path.as_deref() else {
            self.selected_diff_output = self.diff_output.clone();
            self.selected_split_diff = repo_split_diff(&self.workspace).unwrap_or_default();
            return;
        };

        self.selected_diff_output = match repo_diff_for_path(&self.workspace, path) {
            Ok(text) => text,
            Err(err) => format!("Unable to load diff for {path}:\n{err}"),
        };
        self.selected_split_diff = repo_split_diff_for_path(&self.workspace, path).unwrap_or_default();
    }
}

impl App for GitBrainsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if let Some(action) = project_selector::render_dropdown(
                    ui,
                    &self.workspace,
                    &self.open_projects,
                    &self.recent_projects,
                ) {
                    match action {
                        project_selector::ProjectAction::OpenFolder => {
                            if let Some(path) = project_selector::open_folder_dialog() {
                                self.switch_workspace(path);
                            }
                        }
                        project_selector::ProjectAction::CloneRepository => {}
                        project_selector::ProjectAction::SwitchWorkspace(path) => {
                            self.switch_workspace(path)
                        }
                    }
                }

                ui.add_space(12.0);
                ui.label("Branch");
                if let Some(action) = branch_selector::render_dropdown(
                    ui,
                    &mut self.branch_selector_state,
                    self.selected_local_branch.as_deref(),
                    &self.local_branches,
                    &self.remote_branches,
                ) {
                    match action {
                        branch_selector::BranchSelectorAction::CreateBranch { name } => {
                            match create_local_branch(&self.workspace, &name) {
                                Ok(_) => {
                                    self.branch_selector_state.message =
                                        Some(format!("branch created: {name}"));
                                    self.refresh();
                                }
                                Err(err) => {
                                    self.branch_selector_state.message = Some(err.to_string());
                                }
                            }
                        }
                        branch_selector::BranchSelectorAction::CheckoutReference { reference } => {
                            match checkout_reference(&self.workspace, &reference) {
                                Ok(_) => {
                                    self.branch_selector_state.message =
                                        Some(format!("checked out: {reference}"));
                                    self.refresh();
                                }
                                Err(err) => {
                                    self.branch_selector_state.message = Some(err.to_string());
                                }
                            }
                        }
                        branch_selector::BranchSelectorAction::CheckoutLocalBranch { name } => {
                            match checkout_local_branch(&self.workspace, &name) {
                                Ok(_) => {
                                    self.branch_selector_state.message =
                                        Some(format!("switched to local branch: {name}"));
                                    self.refresh();
                                }
                                Err(err) => {
                                    self.branch_selector_state.message = Some(err.to_string());
                                }
                            }
                        }
                        branch_selector::BranchSelectorAction::CheckoutRemoteBranch { name } => {
                            match checkout_remote_branch(&self.workspace, &name) {
                                Ok(local) => {
                                    self.branch_selector_state.message = Some(format!(
                                        "switched to remote branch as local: {local}"
                                    ));
                                    self.refresh();
                                }
                                Err(err) => {
                                    self.branch_selector_state.message = Some(err.to_string());
                                }
                            }
                        }
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                if let Some(repo_action) = repo_actions::render_inline(ui) {
                    self.handle_repo_action(repo_action);
                }
            });
            ui.add_space(4.0);
        });

        if self.left_panel_open {
            egui::SidePanel::left("left_panel")
                .resizable(true)
                .default_width(280.0)
                .min_width(180.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.left_view, LeftView::Files, "Files");
                        ui.selectable_value(&mut self.left_view, LeftView::Diff, "Diff");
                        ui.selectable_value(&mut self.left_view, LeftView::Branches, "Branches");
                    });
                    ui.separator();

                    match self.left_view {
                        LeftView::Files => {
                            ui.heading("File Explorer");
                            ui.separator();
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                if self.files.is_empty() {
                                    ui.label("No files");
                                    return;
                                }

                                let tree_items: Vec<_> = self
                                    .files
                                    .iter()
                                    .map(|path| tree_view::TreeItem {
                                        path: path.as_str(),
                                        payload: path,
                                    })
                                    .collect();

                                let mut render_file = |ui: &mut egui::Ui,
                                                       path: &str,
                                                       file_name: &str,
                                                       _payload: &String| {
                                    let is_selected = self
                                        .selected_file
                                        .as_deref()
                                        .map(|selected| selected == path)
                                        .unwrap_or(false);
                                    let response = ui.selectable_label(is_selected, file_name);
                                    let action = if response.clicked() {
                                        Some(path.to_string())
                                    } else {
                                        None
                                    };
                                    (response, action)
                                };

                                let mut dir_context_menu =
                                    |ui: &mut egui::Ui, dir_path: &str, _dir_name: &str| {
                                        if ui.button("Copy Directory Path").clicked() {
                                            ui.ctx().copy_text(dir_path.to_string());
                                            ui.close();
                                        }
                                        if ui.button("Open First File").clicked() {
                                            let prefix = format!("{dir_path}/");
                                            let first = self
                                                .files
                                                .iter()
                                                .find(|file| file.starts_with(&prefix))
                                                .cloned();
                                            ui.close();
                                            return first;
                                        }
                                        None
                                    };

                                let mut file_context_menu =
                                    |ui: &mut egui::Ui, path: &str, _file_name: &str, _payload: &String| {
                                        if ui.button("Open File").clicked() {
                                            ui.close();
                                            return Some(path.to_string());
                                        }
                                        if ui.button("Copy File Path").clicked() {
                                            ui.ctx().copy_text(path.to_string());
                                            ui.close();
                                        }
                                        None
                                    };

                                if let Some(path) = tree_view::render_tree(
                                    ui,
                                    &tree_items,
                                    &mut render_file,
                                    &mut dir_context_menu,
                                    &mut file_context_menu,
                                ) {
                                    self.selected_file = Some(path);
                                    self.load_selected_file_content();
                                }
                            });
                        }
                        LeftView::Diff => {
                            ui.heading("Changed Files");
                            ui.separator();
                            if let Some(action) = diff_view::render_sidebar(
                                ui,
                                &self.changes,
                                &mut self.diff_panel_state,
                                self.selected_diff_path.as_deref(),
                            ) {
                                let diff_view::DiffAction::SelectPath(path) = action;
                                self.selected_diff_path = Some(path);
                                self.load_selected_diff_content();
                            }
                        }
                        LeftView::Branches => {
                            ui.heading("Branches");
                            ui.separator();
                            if let Some(action) = branches_view::render_sidebar(
                                ui,
                                &self.local_branches,
                                &self.remote_branches,
                                &mut self.branch_panel_state,
                                self.selected_local_branch.as_deref(),
                                self.selected_remote_branch.as_deref(),
                            ) {
                                let branches_view::BranchAction::SelectBranch { scope, name } =
                                    action;
                                match scope {
                                    BranchScope::Local => self.selected_local_branch = Some(name),
                                    BranchScope::Remote => self.selected_remote_branch = Some(name),
                                }
                            }
                        }
                    }
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::from_rgb(200, 60, 60), error);
                return;
            }

            match self.left_view {
                LeftView::Files => {
                    ui.heading("File Editor");
                    ui.separator();
                    if let Some(path) = &self.selected_file {
                        ui.label(path);
                    } else {
                        ui.label("No file selected");
                    }
                    ui.separator();
                    code_view::render_readonly_code(
                        ui,
                        &self.selected_file_content,
                        self.selected_file.as_deref(),
                    );
                }
                LeftView::Diff => {
                    if let Some(path) = &self.selected_diff_path {
                        ui.heading(format!("Diff: {path}"));
                    } else {
                        ui.heading("Repository Diff");
                    }
                    ui.separator();
                    diff_code_view::render_mode_switch(ui, &mut self.diff_render_mode);
                    ui.separator();
                    diff_code_view::render(
                        ui,
                        &self.selected_diff_output,
                        Some(&self.selected_split_diff),
                        self.diff_render_mode,
                    );
                }
                LeftView::Branches => {
                    ui.heading("Branch Details");
                    ui.separator();
                    match self.branch_panel_state.scope {
                        BranchScope::Local => {
                            if let Some(branch) = &self.selected_local_branch {
                                ui.label(format!("Scope: Local\nSelected: {branch}"));
                            } else {
                                ui.label("Scope: Local\nNo branch selected");
                            }
                        }
                        BranchScope::Remote => {
                            if let Some(branch) = &self.selected_remote_branch {
                                ui.label(format!("Scope: Remote\nSelected: {branch}"));
                            } else {
                                ui.label("Scope: Remote\nNo branch selected");
                            }
                        }
                    }
                }
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Gitbrains",
        native_options,
        Box::new(|_cc| {
            let mut app = GitBrainsApp::default();
            app.refresh();
            Ok(Box::new(app))
        }),
    )
}
