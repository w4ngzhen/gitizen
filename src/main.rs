mod code_view;
mod diff_view;
mod git_status;
mod tree_view;

use std::env;

use eframe::{egui, App};
use git_status::{
    list_changes, list_repo_files, read_repo_file, repo_diff, repo_diff_for_path, ChangeItem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeftView {
    Files,
    Diff,
}

struct GitizenApp {
    workspace: String,
    changes: Vec<ChangeItem>,
    files: Vec<String>,
    selected_file: Option<String>,
    selected_file_content: String,
    diff_output: String,
    selected_diff_path: Option<String>,
    selected_diff_output: String,
    diff_panel_state: diff_view::DiffPanelState,
    left_panel_open: bool,
    left_view: LeftView,
    error: Option<String>,
}

impl Default for GitizenApp {
    fn default() -> Self {
        let workspace = env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| String::new());

        Self {
            workspace,
            changes: Vec::new(),
            files: Vec::new(),
            selected_file: None,
            selected_file_content: String::new(),
            diff_output: String::new(),
            selected_diff_path: None,
            selected_diff_output: String::new(),
            diff_panel_state: diff_view::DiffPanelState::default(),
            left_panel_open: true,
            left_view: LeftView::Files,
            error: None,
        }
    }
}

impl GitizenApp {
    fn refresh(&mut self) {
        let changes = list_changes(&self.workspace);
        let files = list_repo_files(&self.workspace);
        let diff = repo_diff(&self.workspace);

        match (changes, files, diff) {
            (Ok(changes), Ok(files), Ok(diff_output)) => {
                self.error = None;
                self.changes = changes;
                self.files = files;
                self.diff_output = diff_output;

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
                self.load_selected_file_content();
                self.load_selected_diff_content();
            }
            (changes_res, files_res, diff_res) => {
                self.changes.clear();
                self.files.clear();
                self.selected_file = None;
                self.selected_file_content.clear();
                self.diff_output.clear();
                self.selected_diff_path = None;
                self.selected_diff_output.clear();

                let message = changes_res
                    .err()
                    .or_else(|| files_res.err())
                    .or_else(|| diff_res.err())
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
            return;
        };

        self.selected_diff_output = match repo_diff_for_path(&self.workspace, path) {
            Ok(text) => text,
            Err(err) => format!("Unable to load diff for {path}:\n{err}"),
        };
    }
}

impl App for GitizenApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let toggle_label = if self.left_panel_open {
                    "Hide Left Panel"
                } else {
                    "Show Left Panel"
                };
                if ui.button(toggle_label).clicked() {
                    self.left_panel_open = !self.left_panel_open;
                }
                ui.label("Workspace:");
                ui.text_edit_singleline(&mut self.workspace);
                if ui.button("Refresh Status").clicked() {
                    self.refresh();
                }
            });
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
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.monospace(&self.selected_diff_output);
                    });
                }
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Gitizen",
        native_options,
        Box::new(|_cc| {
            let mut app = GitizenApp::default();
            app.refresh();
            Ok(Box::new(app))
        }),
    )
}
